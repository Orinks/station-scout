use crate::podcasts::feed::parse_feed;
use crate::podcasts::feed_client::fetch_feed_sync;
use crate::podcasts::opml::{OpmlImportResult, import_opml, render_opml};
use crate::podcasts::repository::{
    get_feed_urls, list_active_subscriptions, list_due_subscriptions, mark_poll_failure,
    mark_poll_success, subscribe, upsert_episodes,
};
use rusqlite::Connection;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscribeResult {
    pub subscription_id: i64,
    pub title: Option<String>,
    pub episodes_added: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PollSummary {
    pub feeds_checked: usize,
    pub new_episodes: usize,
    pub not_modified: usize,
    pub failures: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PodcastServiceError {
    message: String,
}

impl PodcastServiceError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PodcastServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PodcastServiceError {}

pub fn subscribe_to_feed(
    conn: &Connection,
    feed_url: &str,
    next_poll_at: &str,
) -> Result<SubscribeResult, PodcastServiceError> {
    let response = fetch_feed_sync(feed_url, None, None)
        .map_err(|error| PodcastServiceError::new(error.to_string()))?;
    let parsed =
        parse_feed(&response.body).map_err(|error| PodcastServiceError::new(error.to_string()))?;
    let title = parsed.feed_title.clone();
    let subscription_id = subscribe(conn, feed_url, Some(&parsed), None).map_err(sqlite_error)?;
    let episodes_added =
        upsert_episodes(conn, subscription_id, &parsed.episodes).map_err(sqlite_error)?;
    mark_poll_success(
        conn,
        subscription_id,
        response.etag.as_deref(),
        response.last_modified.as_deref(),
        next_poll_at,
    )
    .map_err(sqlite_error)?;
    Ok(SubscribeResult {
        subscription_id,
        title,
        episodes_added,
    })
}

pub fn import_opml_subscriptions(
    conn: &Connection,
    source: &[u8],
) -> Result<OpmlImportResult, PodcastServiceError> {
    let existing = get_feed_urls(conn).map_err(sqlite_error)?;
    let result = import_opml(source, existing)
        .map_err(|error| PodcastServiceError::new(error.to_string()))?;
    for row in &result.rows {
        if row.status == "added" {
            if let Some(feed_url) = row.feed_url.as_deref() {
                subscribe(conn, feed_url, None, row.title.as_deref()).map_err(sqlite_error)?;
            }
        }
    }
    Ok(result)
}

pub fn export_opml_subscriptions(conn: &Connection) -> Result<Vec<u8>, PodcastServiceError> {
    let rows = list_active_subscriptions(conn).map_err(sqlite_error)?;
    Ok(render_opml(
        rows.into_iter()
            .map(|row| {
                let title = row.title.unwrap_or_else(|| row.feed_url.clone());
                (title, row.feed_url, None)
            })
            .collect(),
    ))
}

pub fn poll_due_feeds(
    conn: &Connection,
    now: &str,
    next_poll_at: &str,
    limit: usize,
) -> Result<PollSummary, PodcastServiceError> {
    let due = list_due_subscriptions(conn, now, limit).map_err(sqlite_error)?;
    let mut summary = PollSummary::default();
    for row in due {
        summary.feeds_checked += 1;
        let response = fetch_feed_sync(
            &row.feed_url,
            row.etag.as_deref(),
            row.last_modified.as_deref(),
        );
        match response {
            Ok(response) if response.not_modified => {
                mark_poll_success(
                    conn,
                    row.id,
                    response.etag.as_deref().or(row.etag.as_deref()),
                    response
                        .last_modified
                        .as_deref()
                        .or(row.last_modified.as_deref()),
                    next_poll_at,
                )
                .map_err(sqlite_error)?;
                summary.not_modified += 1;
            }
            Ok(response) => {
                let parsed = match parse_feed(&response.body) {
                    Ok(parsed) => parsed,
                    Err(_) => {
                        mark_poll_failure(conn, row.id, next_poll_at).map_err(sqlite_error)?;
                        summary.failures += 1;
                        continue;
                    }
                };
                subscribe(conn, &row.feed_url, Some(&parsed), None).map_err(sqlite_error)?;
                summary.new_episodes +=
                    upsert_episodes(conn, row.id, &parsed.episodes).map_err(sqlite_error)?;
                mark_poll_success(
                    conn,
                    row.id,
                    response.etag.as_deref(),
                    response.last_modified.as_deref(),
                    next_poll_at,
                )
                .map_err(sqlite_error)?;
            }
            Err(_) => {
                mark_poll_failure(conn, row.id, next_poll_at).map_err(sqlite_error)?;
                summary.failures += 1;
            }
        }
    }
    Ok(summary)
}

fn sqlite_error(error: rusqlite::Error) -> PodcastServiceError {
    PodcastServiceError::new(error.to_string())
}
