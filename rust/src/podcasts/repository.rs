use crate::notifications::NotificationRule;
use crate::podcasts::feed::{Episode, ParsedFeed};
use chrono::Utc;
use rusqlite::{Connection, Result, params};
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionRow {
    pub id: i64,
    pub feed_url: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub image_url: Option<String>,
    pub description: Option<String>,
    pub subscribed_at: String,
    pub unsubscribed_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionSettings {
    pub notification_rule: NotificationRule,
    pub skip_silence: bool,
    pub intro_skip_seconds: i32,
    pub outro_skip_seconds: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayedState {
    pub episode_id: i64,
    pub position_ms: i64,
    pub is_played: bool,
    pub played_at: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueueEntry {
    pub episode_id: i64,
    pub position: i64,
    pub title: Option<String>,
    pub added_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeRow {
    pub id: i64,
    pub subscription_id: i64,
    pub guid: Option<String>,
    pub title: Option<String>,
    pub enclosure_url: Option<String>,
    pub pub_date: Option<String>,
    pub duration_seconds: Option<i32>,
    pub description_html: Option<String>,
    pub transcript_url: Option<String>,
    pub transcript_format: Option<String>,
    pub chapters_url: Option<String>,
    pub position_ms: i64,
    pub is_played: bool,
    pub played_at: Option<String>,
    pub is_queued: bool,
    pub queue_position: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationEntry {
    pub id: i64,
    pub episode_id: i64,
    pub episode_title: Option<String>,
    pub show_title: Option<String>,
    pub classified_at: String,
    pub action: String,
    pub toast_fired: bool,
    pub fired_at: Option<String>,
    pub suppression_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheEntry {
    pub episode_id: i64,
    pub cache_path: String,
    pub bytes_cached: i64,
    pub total_bytes: Option<i64>,
    pub lease_count: i64,
    pub was_playing: bool,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionPollRow {
    pub id: i64,
    pub feed_url: String,
    pub title: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub next_poll_at: Option<String>,
    pub health_state: String,
    pub consecutive_fails: i64,
}

pub fn open_memory_database() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    initialize_database(&conn)?;
    Ok(conn)
}

pub fn open_database(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    initialize_database(&conn)?;
    Ok(conn)
}

pub fn initialize_database(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );
        INSERT OR IGNORE INTO schema_version(version) VALUES (1);
        CREATE TABLE IF NOT EXISTS subscriptions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            feed_url TEXT NOT NULL UNIQUE,
            title TEXT,
            author TEXT,
            image_url TEXT,
            description TEXT,
            etag TEXT,
            last_modified TEXT,
            last_polled_at TEXT,
            next_poll_at TEXT,
            health_state TEXT NOT NULL DEFAULT 'ok',
            consecutive_fails INTEGER NOT NULL DEFAULT 0,
            notification_rule TEXT NOT NULL DEFAULT 'always',
            skip_silence INTEGER,
            intro_skip_seconds INTEGER,
            outro_skip_seconds INTEGER,
            subscribed_at TEXT NOT NULL,
            unsubscribed_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_subscriptions_active
            ON subscriptions(unsubscribed_at) WHERE unsubscribed_at IS NULL;
        CREATE TABLE IF NOT EXISTS episodes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            subscription_id INTEGER NOT NULL REFERENCES subscriptions(id) ON DELETE RESTRICT,
            guid TEXT,
            enclosure_url_norm TEXT,
            title_pubdate_hash TEXT,
            title TEXT,
            enclosure_url TEXT,
            pub_date TEXT,
            duration_seconds INTEGER,
            description_html TEXT,
            transcript_url TEXT,
            transcript_format TEXT,
            chapters_url TEXT,
            first_seen_at TEXT NOT NULL,
            UNIQUE(subscription_id, guid, enclosure_url_norm, title_pubdate_hash)
        );
        CREATE INDEX IF NOT EXISTS idx_episodes_sub_pubdate
            ON episodes(subscription_id, pub_date DESC);
        CREATE TABLE IF NOT EXISTS played_state (
            episode_id INTEGER PRIMARY KEY REFERENCES episodes(id) ON DELETE CASCADE,
            position_ms INTEGER NOT NULL DEFAULT 0,
            is_played INTEGER NOT NULL DEFAULT 0,
            played_at TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS queue (
            episode_id INTEGER PRIMARY KEY REFERENCES episodes(id) ON DELETE CASCADE,
            position INTEGER NOT NULL,
            added_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_queue_position ON queue(position);
        CREATE TABLE IF NOT EXISTS notifications_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            episode_id INTEGER NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
            classified_at TEXT NOT NULL,
            action TEXT NOT NULL,
            toast_fired INTEGER NOT NULL DEFAULT 0,
            fired_at TEXT,
            suppression_reason TEXT
        );
        CREATE TABLE IF NOT EXISTS cache_index (
            episode_id INTEGER PRIMARY KEY REFERENCES episodes(id) ON DELETE CASCADE,
            cache_path TEXT NOT NULL,
            bytes_cached INTEGER NOT NULL DEFAULT 0,
            total_bytes INTEGER,
            lease_count INTEGER NOT NULL DEFAULT 0,
            was_playing INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );
        "#,
    )
}

pub fn subscribe(
    conn: &Connection,
    feed_url: &str,
    parsed: Option<&ParsedFeed>,
    title: Option<&str>,
) -> Result<i64> {
    validate_http_url(feed_url)?;
    let now = now_iso();
    let row_title = parsed.and_then(|feed| feed.feed_title.as_deref()).or(title);
    let row_author = parsed.and_then(|feed| feed.feed_author.as_deref());
    let row_image = parsed.and_then(|feed| feed.feed_image_url.as_deref());
    let row_description = parsed.and_then(|feed| feed.feed_description.as_deref());

    let existing: Result<i64> = conn.query_row(
        "SELECT id FROM subscriptions WHERE feed_url = ?1",
        [feed_url],
        |row| row.get(0),
    );
    if let Ok(id) = existing {
        conn.execute(
            r#"
            UPDATE subscriptions
               SET unsubscribed_at = NULL,
                   title = COALESCE(?1, title),
                   author = COALESCE(?2, author),
                   image_url = COALESCE(?3, image_url),
                   description = COALESCE(?4, description)
             WHERE id = ?5
            "#,
            params![row_title, row_author, row_image, row_description, id],
        )?;
        return Ok(id);
    }

    conn.execute(
        r#"
        INSERT INTO subscriptions
            (feed_url, title, author, image_url, description, subscribed_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            feed_url,
            row_title,
            row_author,
            row_image,
            row_description,
            now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn unsubscribe(conn: &Connection, subscription_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE subscriptions SET unsubscribed_at = ?1 WHERE id = ?2",
        params![now_iso(), subscription_id],
    )?;
    Ok(())
}

pub fn list_active_subscriptions(conn: &Connection) -> Result<Vec<SubscriptionRow>> {
    let mut statement = conn.prepare(
        r#"
        SELECT id, feed_url, title, author, image_url, description, subscribed_at, unsubscribed_at
          FROM subscriptions
         WHERE unsubscribed_at IS NULL
         ORDER BY COALESCE(title, feed_url) COLLATE NOCASE
        "#,
    )?;
    let rows = statement.query_map([], |row| {
        Ok(SubscriptionRow {
            id: row.get(0)?,
            feed_url: row.get(1)?,
            title: row.get(2)?,
            author: row.get(3)?,
            image_url: row.get(4)?,
            description: row.get(5)?,
            subscribed_at: row.get(6)?,
            unsubscribed_at: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn get_subscription_settings(
    conn: &Connection,
    subscription_id: i64,
) -> Result<Option<SubscriptionSettings>> {
    let result = conn.query_row(
        r#"
        SELECT notification_rule,
               COALESCE(skip_silence, 0),
               COALESCE(intro_skip_seconds, 0),
               COALESCE(outro_skip_seconds, 0)
          FROM subscriptions
         WHERE id = ?1
        "#,
        [subscription_id],
        |row| {
            Ok(SubscriptionSettings {
                notification_rule: notification_rule_from_db(row.get::<_, Option<String>>(0)?),
                skip_silence: row.get::<_, i64>(1)? != 0,
                intro_skip_seconds: clamp_skip_seconds(row.get(2)?),
                outro_skip_seconds: clamp_skip_seconds(row.get(3)?),
            })
        },
    );
    match result {
        Ok(settings) => Ok(Some(settings)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn update_subscription_settings(
    conn: &Connection,
    subscription_id: i64,
    settings: &SubscriptionSettings,
) -> Result<()> {
    conn.execute(
        r#"
        UPDATE subscriptions
           SET notification_rule = ?1,
               skip_silence = ?2,
               intro_skip_seconds = ?3,
               outro_skip_seconds = ?4
         WHERE id = ?5
        "#,
        params![
            notification_rule_to_db(settings.notification_rule),
            if settings.skip_silence { 1_i64 } else { 0_i64 },
            clamp_skip_seconds(settings.intro_skip_seconds),
            clamp_skip_seconds(settings.outro_skip_seconds),
            subscription_id
        ],
    )?;
    Ok(())
}

pub fn get_feed_urls(conn: &Connection) -> Result<std::collections::HashSet<String>> {
    let mut statement = conn.prepare("SELECT feed_url FROM subscriptions")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect()
}

pub fn list_due_subscriptions(
    conn: &Connection,
    now: &str,
    limit: usize,
) -> Result<Vec<SubscriptionPollRow>> {
    let mut statement = conn.prepare(
        r#"
        SELECT id, feed_url, title, etag, last_modified, next_poll_at,
               health_state, consecutive_fails
          FROM subscriptions
         WHERE unsubscribed_at IS NULL
           AND (next_poll_at IS NULL OR next_poll_at <= ?1)
         ORDER BY next_poll_at IS NOT NULL, next_poll_at, COALESCE(title, feed_url) COLLATE NOCASE
         LIMIT ?2
        "#,
    )?;
    let rows = statement.query_map(params![now, limit.max(1) as i64], |row| {
        Ok(SubscriptionPollRow {
            id: row.get(0)?,
            feed_url: row.get(1)?,
            title: row.get(2)?,
            etag: row.get(3)?,
            last_modified: row.get(4)?,
            next_poll_at: row.get(5)?,
            health_state: row.get(6)?,
            consecutive_fails: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn mark_poll_success(
    conn: &Connection,
    subscription_id: i64,
    etag: Option<&str>,
    last_modified: Option<&str>,
    next_poll_at: &str,
) -> Result<()> {
    conn.execute(
        r#"
        UPDATE subscriptions
           SET etag = COALESCE(?1, etag),
               last_modified = COALESCE(?2, last_modified),
               last_polled_at = ?3,
               next_poll_at = ?4,
               health_state = 'ok',
               consecutive_fails = 0
         WHERE id = ?5
        "#,
        params![
            etag.filter(|value| !value.trim().is_empty()),
            last_modified.filter(|value| !value.trim().is_empty()),
            now_iso(),
            next_poll_at,
            subscription_id
        ],
    )?;
    Ok(())
}

pub fn mark_poll_failure(
    conn: &Connection,
    subscription_id: i64,
    next_poll_at: &str,
) -> Result<()> {
    conn.execute(
        r#"
        UPDATE subscriptions
           SET last_polled_at = ?1,
               next_poll_at = ?2,
               consecutive_fails = consecutive_fails + 1,
               health_state = CASE
                   WHEN consecutive_fails + 1 >= 3 THEN 'unhealthy'
                   ELSE health_state
               END
         WHERE id = ?3
        "#,
        params![now_iso(), next_poll_at, subscription_id],
    )?;
    Ok(())
}

pub fn upsert_episodes(
    conn: &Connection,
    subscription_id: i64,
    episodes: &[Episode],
) -> Result<usize> {
    let mut inserted = 0;
    let now = now_iso();
    for episode in episodes {
        let transcript_url = episode
            .transcript
            .as_ref()
            .map(|transcript| transcript.url.as_str());
        let transcript_format = episode
            .transcript
            .as_ref()
            .and_then(|transcript| transcript.mime_type.as_deref())
            .and_then(transcript_format);
        let changed = conn.execute(
            r#"
            INSERT OR IGNORE INTO episodes
                (subscription_id, guid, enclosure_url_norm, title_pubdate_hash,
                 title, enclosure_url, pub_date, duration_seconds,
                 description_html, transcript_url, transcript_format,
                 chapters_url, first_seen_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                subscription_id,
                episode.guid.as_deref(),
                episode.enclosure_url_norm.as_deref(),
                episode.title_pubdate_hash.as_deref(),
                episode.title.as_deref(),
                episode.enclosure_url.as_deref(),
                episode.pub_date.as_deref(),
                episode.duration_seconds,
                episode.description_html.as_deref(),
                transcript_url,
                transcript_format,
                episode.chapters_url.as_deref(),
                now
            ],
        )?;
        inserted += changed;
    }
    Ok(inserted)
}

pub fn list_episodes(conn: &Connection, subscription_id: i64) -> Result<Vec<EpisodeRow>> {
    let mut statement = conn.prepare(
        r#"
        SELECT e.id, e.subscription_id, e.guid, e.title, e.enclosure_url,
               e.pub_date, e.duration_seconds, e.description_html,
               e.transcript_url, e.transcript_format, e.chapters_url,
               COALESCE(ps.position_ms, 0), COALESCE(ps.is_played, 0),
               ps.played_at, q.position
          FROM episodes e
          LEFT JOIN played_state ps ON ps.episode_id = e.id
          LEFT JOIN queue q ON q.episode_id = e.id
         WHERE e.subscription_id = ?1
         ORDER BY e.pub_date DESC, e.first_seen_at DESC, e.id ASC
        "#,
    )?;
    let rows = statement.query_map([subscription_id], |row| {
        let queue_position: Option<i64> = row.get(14)?;
        Ok(EpisodeRow {
            id: row.get(0)?,
            subscription_id: row.get(1)?,
            guid: row.get(2)?,
            title: row.get(3)?,
            enclosure_url: row.get(4)?,
            pub_date: row.get(5)?,
            duration_seconds: row.get(6)?,
            description_html: row.get(7)?,
            transcript_url: row.get(8)?,
            transcript_format: row.get(9)?,
            chapters_url: row.get(10)?,
            position_ms: row.get(11)?,
            is_played: row.get::<_, i64>(12)? != 0,
            played_at: row.get(13)?,
            is_queued: queue_position.is_some(),
            queue_position,
        })
    })?;
    rows.collect()
}

pub fn set_playback_position(conn: &Connection, episode_id: i64, position_ms: i64) -> Result<()> {
    let now = now_iso();
    conn.execute(
        r#"
        INSERT INTO played_state (episode_id, position_ms, is_played, updated_at)
        VALUES (?1, ?2, 0, ?3)
        ON CONFLICT(episode_id) DO UPDATE SET
            position_ms = excluded.position_ms,
            updated_at = excluded.updated_at
        "#,
        params![episode_id, position_ms.max(0), now],
    )?;
    Ok(())
}

pub fn mark_episode_played(conn: &Connection, episode_id: i64) -> Result<()> {
    let now = now_iso();
    conn.execute(
        r#"
        INSERT INTO played_state (episode_id, position_ms, is_played, played_at, updated_at)
        VALUES (?1, 0, 1, ?2, ?2)
        ON CONFLICT(episode_id) DO UPDATE SET
            is_played = 1,
            played_at = excluded.played_at,
            updated_at = excluded.updated_at
        "#,
        params![episode_id, now],
    )?;
    Ok(())
}

pub fn get_played_state(conn: &Connection, episode_id: i64) -> Result<Option<PlayedState>> {
    let result = conn.query_row(
        r#"
        SELECT episode_id, position_ms, is_played, played_at, updated_at
          FROM played_state
         WHERE episode_id = ?1
        "#,
        [episode_id],
        |row| {
            Ok(PlayedState {
                episode_id: row.get(0)?,
                position_ms: row.get(1)?,
                is_played: row.get::<_, i64>(2)? != 0,
                played_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    );
    match result {
        Ok(state) => Ok(Some(state)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn enqueue_episode(conn: &Connection, episode_id: i64) -> Result<()> {
    let exists: Result<i64> = conn.query_row(
        "SELECT episode_id FROM queue WHERE episode_id = ?1",
        [episode_id],
        |row| row.get(0),
    );
    if exists.is_ok() {
        return Ok(());
    }
    let next_position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM queue",
        [],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT INTO queue (episode_id, position, added_at) VALUES (?1, ?2, ?3)",
        params![episode_id, next_position, now_iso()],
    )?;
    Ok(())
}

pub fn list_queue(conn: &Connection) -> Result<Vec<QueueEntry>> {
    let mut statement = conn.prepare(
        r#"
        SELECT q.episode_id, q.position, e.title, q.added_at
          FROM queue q
          JOIN episodes e ON e.id = q.episode_id
         ORDER BY q.position ASC
        "#,
    )?;
    let rows = statement.query_map([], |row| {
        Ok(QueueEntry {
            episode_id: row.get(0)?,
            position: row.get(1)?,
            title: row.get(2)?,
            added_at: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn move_queued_episode(conn: &Connection, episode_id: i64, new_position: i64) -> Result<()> {
    let entries = list_queue(conn)?;
    let Some(current_index) = entries
        .iter()
        .position(|entry| entry.episode_id == episode_id)
    else {
        return Ok(());
    };
    let mut ids: Vec<i64> = entries.into_iter().map(|entry| entry.episode_id).collect();
    let id = ids.remove(current_index);
    let target = new_position.max(0).min(ids.len() as i64) as usize;
    ids.insert(target, id);
    for (position, id) in ids.into_iter().enumerate() {
        conn.execute(
            "UPDATE queue SET position = ?1 WHERE episode_id = ?2",
            params![position as i64, id],
        )?;
    }
    Ok(())
}

pub fn remove_queued_episode(conn: &Connection, episode_id: i64) -> Result<()> {
    conn.execute("DELETE FROM queue WHERE episode_id = ?1", [episode_id])?;
    let entries = list_queue(conn)?;
    for (position, entry) in entries.into_iter().enumerate() {
        conn.execute(
            "UPDATE queue SET position = ?1 WHERE episode_id = ?2",
            params![position as i64, entry.episode_id],
        )?;
    }
    Ok(())
}

pub fn clear_queue(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM queue", [])?;
    Ok(())
}

pub fn log_notification_event(
    conn: &Connection,
    episode_id: i64,
    classified_at: &str,
    action: &str,
    suppression_reason: Option<&str>,
) -> Result<i64> {
    conn.execute(
        r#"
        INSERT INTO notifications_log
            (episode_id, classified_at, action, suppression_reason)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        params![episode_id, classified_at, action, suppression_reason],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn mark_notification_toast_fired(
    conn: &Connection,
    notification_id: i64,
    fired_at: &str,
) -> Result<()> {
    conn.execute(
        r#"
        UPDATE notifications_log
           SET toast_fired = 1,
               fired_at = ?1
         WHERE id = ?2
        "#,
        params![fired_at, notification_id],
    )?;
    Ok(())
}

pub fn list_notification_inbox(conn: &Connection, limit: usize) -> Result<Vec<NotificationEntry>> {
    let mut statement = conn.prepare(
        r#"
        SELECT n.id, n.episode_id, e.title, s.title, n.classified_at,
               n.action, n.toast_fired, n.fired_at, n.suppression_reason
          FROM notifications_log n
          JOIN episodes e ON e.id = n.episode_id
          JOIN subscriptions s ON s.id = e.subscription_id
         ORDER BY n.classified_at DESC, n.id DESC
         LIMIT ?1
        "#,
    )?;
    let rows = statement.query_map([limit.max(1) as i64], |row| {
        Ok(NotificationEntry {
            id: row.get(0)?,
            episode_id: row.get(1)?,
            episode_title: row.get(2)?,
            show_title: row.get(3)?,
            classified_at: row.get(4)?,
            action: row.get(5)?,
            toast_fired: row.get::<_, i64>(6)? != 0,
            fired_at: row.get(7)?,
            suppression_reason: row.get(8)?,
        })
    })?;
    rows.collect()
}

pub fn clear_notification_inbox(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM notifications_log", [])?;
    Ok(())
}

pub fn update_cache_entry(
    conn: &Connection,
    episode_id: i64,
    cache_path: &str,
    bytes_cached: i64,
    total_bytes: Option<i64>,
) -> Result<()> {
    let now = now_iso();
    conn.execute(
        r#"
        INSERT INTO cache_index
            (episode_id, cache_path, bytes_cached, total_bytes, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(episode_id) DO UPDATE SET
            cache_path = excluded.cache_path,
            bytes_cached = excluded.bytes_cached,
            total_bytes = excluded.total_bytes,
            updated_at = excluded.updated_at
        "#,
        params![
            episode_id,
            cache_path,
            bytes_cached.max(0),
            total_bytes.map(|value| value.max(0)),
            now
        ],
    )?;
    Ok(())
}

pub fn get_cache_entry(conn: &Connection, episode_id: i64) -> Result<Option<CacheEntry>> {
    let result = conn.query_row(
        r#"
        SELECT episode_id, cache_path, bytes_cached, total_bytes, lease_count,
               was_playing, updated_at
          FROM cache_index
         WHERE episode_id = ?1
        "#,
        [episode_id],
        cache_entry_from_row,
    );
    match result {
        Ok(entry) => Ok(Some(entry)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn lease_cache_entry(conn: &Connection, episode_id: i64) -> Result<()> {
    conn.execute(
        r#"
        UPDATE cache_index
           SET lease_count = lease_count + 1,
               updated_at = ?1
         WHERE episode_id = ?2
        "#,
        params![now_iso(), episode_id],
    )?;
    Ok(())
}

pub fn release_cache_entry(conn: &Connection, episode_id: i64) -> Result<()> {
    conn.execute(
        r#"
        UPDATE cache_index
           SET lease_count = CASE
                WHEN lease_count > 0 THEN lease_count - 1
                ELSE 0
               END,
               updated_at = ?1
         WHERE episode_id = ?2
        "#,
        params![now_iso(), episode_id],
    )?;
    Ok(())
}

pub fn mark_cache_was_playing(conn: &Connection, episode_id: i64, was_playing: bool) -> Result<()> {
    conn.execute(
        r#"
        UPDATE cache_index
           SET was_playing = ?1,
               updated_at = ?2
         WHERE episode_id = ?3
        "#,
        params![was_playing as i64, now_iso(), episode_id],
    )?;
    Ok(())
}

fn validate_http_url(value: &str) -> Result<()> {
    let parsed = url::Url::parse(value)
        .map_err(|_| rusqlite::Error::InvalidParameterName("unsupported feed URL".to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(rusqlite::Error::InvalidParameterName(
            "unsupported feed URL".to_string(),
        ));
    }
    Ok(())
}

fn transcript_format(mime_type: &str) -> Option<&'static str> {
    let mime = mime_type.to_lowercase();
    if mime.contains("vtt") {
        Some("vtt")
    } else if mime.contains("srt") || mime.contains("subrip") {
        Some("srt")
    } else if mime.contains("json") {
        Some("json")
    } else {
        None
    }
}

fn notification_rule_to_db(rule: NotificationRule) -> &'static str {
    match rule {
        NotificationRule::Always => "always",
        NotificationRule::Digest => "digest",
        NotificationRule::Never => "never",
    }
}

fn notification_rule_from_db(value: Option<String>) -> NotificationRule {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "digest" => NotificationRule::Digest,
        "never" => NotificationRule::Never,
        _ => NotificationRule::Always,
    }
}

fn clamp_skip_seconds(value: i32) -> i32 {
    value.clamp(0, 600)
}

fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn cache_entry_from_row(row: &rusqlite::Row<'_>) -> Result<CacheEntry> {
    Ok(CacheEntry {
        episode_id: row.get(0)?,
        cache_path: row.get(1)?,
        bytes_cached: row.get(2)?,
        total_bytes: row.get(3)?,
        lease_count: row.get(4)?,
        was_playing: row.get::<_, i64>(5)? != 0,
        updated_at: row.get(6)?,
    })
}
