use crate::models::{BrowseFacet, BrowseFacetKind, Station, TuneTimer};
use crate::podcasts::discovery::{
    DiscoveryError, PodcastSearchClient, SearchOutcome, podcast_index_credentials_from_state,
};
use crate::podcasts::opml::OpmlImportResult;
use crate::podcasts::repository::{
    CacheEntry, EpisodeRow, NotificationEntry, QueueEntry, SubscriptionRow, SubscriptionSettings,
    clear_notification_inbox, clear_queue, enqueue_episode, get_cache_entry,
    get_subscription_settings, lease_cache_entry, list_active_subscriptions, list_episodes,
    list_notification_inbox, list_queue, log_notification_event, mark_cache_was_playing,
    mark_episode_played, mark_notification_toast_fired, move_queued_episode, open_database,
    release_cache_entry, remove_queued_episode, set_playback_position, unsubscribe,
    update_cache_entry, update_subscription_settings,
};
use crate::podcasts::service::{
    PodcastServiceError, PollSummary, SubscribeResult, export_opml_subscriptions,
    import_opml_subscriptions, poll_due_feeds, subscribe_to_feed,
};
use crate::radio_browser::{RadioBrowserClient, RadioBrowserError};
use crate::storage::{
    AppState, CredentialStore, SettingsStore, add_unique_station, station_scout_settings_path,
};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub const FORCE_PORTABLE_ENV: &str = "STATION_SCOUT_FORCE_PORTABLE";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStoragePaths {
    pub config_root: PathBuf,
    pub data_root: PathBuf,
    pub cache_root: PathBuf,
    pub logs_root: PathBuf,
    pub portable_mode: bool,
    pub custom_config_dir: bool,
}

pub struct StationScoutRuntime {
    pub settings_path: PathBuf,
    pub podcast_database_path: PathBuf,
    pub track_log_folder: PathBuf,
    pub has_active_podcast_subscriptions: bool,
    pub state: AppState,
    pub podcast_database: Connection,
    settings_store: SettingsStore,
}

impl RuntimeStoragePaths {
    pub fn new(
        config_root: PathBuf,
        data_root: PathBuf,
        cache_root: PathBuf,
        logs_root: PathBuf,
        portable_mode: bool,
        custom_config_dir: bool,
    ) -> Self {
        Self {
            config_root,
            data_root,
            cache_root,
            logs_root,
            portable_mode,
            custom_config_dir,
        }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config_root.join("station_scout.json")
    }

    pub fn podcast_database_file(&self) -> PathBuf {
        self.data_root.join("podcatcher.sqlite3")
    }

    pub fn episode_cache_root(&self) -> PathBuf {
        self.cache_root.join("episodes")
    }

    pub fn log_file(&self) -> PathBuf {
        self.logs_root.join("station_scout.log")
    }

    pub fn ensure_dirs(&self) -> Result<(), String> {
        for directory in [
            &self.config_root,
            &self.data_root,
            &self.cache_root,
            &self.logs_root,
            &self.episode_cache_root(),
        ] {
            std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

impl StationScoutRuntime {
    pub fn open(
        settings_path: PathBuf,
        credentials: Box<dyn CredentialStore>,
    ) -> Result<Self, String> {
        let podcast_database_path = settings_path
            .parent()
            .map(|parent| parent.join("podcatcher.sqlite3"))
            .unwrap_or_else(|| PathBuf::from("podcatcher.sqlite3"));
        Self::open_with_database_path(settings_path, podcast_database_path, credentials)
    }

    pub fn open_with_storage_paths(
        storage_paths: RuntimeStoragePaths,
        credentials: Box<dyn CredentialStore>,
    ) -> Result<Self, String> {
        storage_paths.ensure_dirs()?;
        Self::open_with_database_path(
            storage_paths.settings_file(),
            storage_paths.podcast_database_file(),
            credentials,
        )
    }

    pub fn open_default(credentials: Box<dyn CredentialStore>) -> Result<Self, String> {
        let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from);
        let app_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let storage_paths = resolve_runtime_storage(
            None,
            is_forced_portable_mode(),
            appdata.as_deref(),
            home.as_deref(),
            &app_dir,
        );
        Self::open_with_storage_paths(storage_paths, credentials)
    }

    fn open_with_database_path(
        settings_path: PathBuf,
        podcast_database_path: PathBuf,
        credentials: Box<dyn CredentialStore>,
    ) -> Result<Self, String> {
        let store = SettingsStore::new(settings_path.clone()).with_credentials(credentials);
        let state = store.load()?;
        let track_log_folder = store.track_log_folder(&state);
        std::fs::create_dir_all(&track_log_folder).map_err(|error| error.to_string())?;
        let podcast_database =
            open_database(&podcast_database_path).map_err(|error| error.to_string())?;
        let has_active_podcast_subscriptions =
            active_podcast_subscriptions_exist(&podcast_database)
                .map_err(|error| error.to_string())?;
        Ok(Self {
            settings_path,
            podcast_database_path,
            track_log_folder,
            has_active_podcast_subscriptions,
            state,
            podcast_database,
            settings_store: store,
        })
    }

    pub fn save_settings(&self) -> Result<(), String> {
        self.settings_store.save(&self.state)
    }

    pub fn import_podcast_opml(
        &mut self,
        source: &[u8],
    ) -> Result<OpmlImportResult, PodcastServiceError> {
        let result = import_opml_subscriptions(&self.podcast_database, source)?;
        self.has_active_podcast_subscriptions =
            active_podcast_subscriptions_exist(&self.podcast_database).map_err(sqlite_error)?;
        Ok(result)
    }

    pub fn export_podcast_opml(&self) -> Result<Vec<u8>, PodcastServiceError> {
        export_opml_subscriptions(&self.podcast_database)
    }

    pub fn subscribe_to_podcast_feed(
        &mut self,
        feed_url: &str,
        next_poll_at: &str,
    ) -> Result<SubscribeResult, PodcastServiceError> {
        let result = subscribe_to_feed(&self.podcast_database, feed_url, next_poll_at)?;
        self.has_active_podcast_subscriptions =
            active_podcast_subscriptions_exist(&self.podcast_database).map_err(sqlite_error)?;
        Ok(result)
    }

    pub fn poll_due_podcast_feeds(
        &self,
        now: &str,
        next_poll_at: &str,
        limit: usize,
    ) -> Result<PollSummary, PodcastServiceError> {
        poll_due_feeds(&self.podcast_database, now, next_poll_at, limit)
    }

    pub fn search_podcasts_with_client(
        &self,
        client: &PodcastSearchClient,
        query: &str,
        auth_date: &str,
        max_results: i32,
    ) -> Result<SearchOutcome, DiscoveryError> {
        client.search(
            query,
            podcast_index_credentials_from_state(&self.state).as_ref(),
            auth_date,
            max_results,
        )
    }

    pub fn search_radio_with_client(
        &self,
        client: &RadioBrowserClient,
        name: &str,
        country: &str,
        language: &str,
        tag: &str,
        limit: i32,
    ) -> Result<Vec<Station>, RadioBrowserError> {
        client.search(name, country, language, tag, limit)
    }

    pub fn browse_radio_facets_with_client(
        &self,
        client: &RadioBrowserClient,
        kind: BrowseFacetKind,
        limit: i32,
    ) -> Result<Vec<BrowseFacet>, RadioBrowserError> {
        client.browse_facets(kind, limit)
    }

    pub fn browse_radio_stations_with_client(
        &self,
        client: &RadioBrowserClient,
        facet: &BrowseFacet,
        limit: i32,
    ) -> Result<Vec<Station>, RadioBrowserError> {
        client.browse_stations(facet, limit)
    }

    pub fn add_radio_favorite(&mut self, station: Station) -> Result<(), String> {
        self.state.favorites = add_unique_station(self.state.favorites.clone(), station, None);
        Ok(())
    }

    pub fn record_radio_recent(&mut self, station: Station) -> Result<(), String> {
        self.state.recents = add_unique_station(self.state.recents.clone(), station, Some(50));
        Ok(())
    }

    pub fn add_tune_timer(&mut self, timer: TuneTimer) -> Result<(), String> {
        self.state.timers.push(timer);
        Ok(())
    }

    pub fn list_podcast_subscriptions(&self) -> Result<Vec<SubscriptionRow>, PodcastServiceError> {
        list_active_subscriptions(&self.podcast_database).map_err(sqlite_error)
    }

    pub fn unsubscribe_from_podcast(
        &mut self,
        subscription_id: i64,
    ) -> Result<(), PodcastServiceError> {
        unsubscribe(&self.podcast_database, subscription_id).map_err(sqlite_error)?;
        self.has_active_podcast_subscriptions =
            active_podcast_subscriptions_exist(&self.podcast_database).map_err(sqlite_error)?;
        Ok(())
    }

    pub fn get_podcast_subscription_settings(
        &self,
        subscription_id: i64,
    ) -> Result<Option<SubscriptionSettings>, PodcastServiceError> {
        get_subscription_settings(&self.podcast_database, subscription_id).map_err(sqlite_error)
    }

    pub fn update_podcast_subscription_settings(
        &self,
        subscription_id: i64,
        settings: &SubscriptionSettings,
    ) -> Result<(), PodcastServiceError> {
        update_subscription_settings(&self.podcast_database, subscription_id, settings)
            .map_err(sqlite_error)
    }

    pub fn list_podcast_episodes(
        &self,
        subscription_id: i64,
    ) -> Result<Vec<EpisodeRow>, PodcastServiceError> {
        list_episodes(&self.podcast_database, subscription_id).map_err(sqlite_error)
    }

    pub fn set_podcast_playback_position(
        &self,
        episode_id: i64,
        position_ms: i64,
    ) -> Result<(), PodcastServiceError> {
        set_playback_position(&self.podcast_database, episode_id, position_ms).map_err(sqlite_error)
    }

    pub fn mark_podcast_episode_played(&self, episode_id: i64) -> Result<(), PodcastServiceError> {
        mark_episode_played(&self.podcast_database, episode_id).map_err(sqlite_error)
    }

    pub fn enqueue_podcast_episode(&self, episode_id: i64) -> Result<(), PodcastServiceError> {
        enqueue_episode(&self.podcast_database, episode_id).map_err(sqlite_error)
    }

    pub fn list_podcast_queue(&self) -> Result<Vec<QueueEntry>, PodcastServiceError> {
        list_queue(&self.podcast_database).map_err(sqlite_error)
    }

    pub fn move_queued_podcast_episode(
        &self,
        episode_id: i64,
        new_position: i64,
    ) -> Result<(), PodcastServiceError> {
        move_queued_episode(&self.podcast_database, episode_id, new_position).map_err(sqlite_error)
    }

    pub fn remove_queued_podcast_episode(
        &self,
        episode_id: i64,
    ) -> Result<(), PodcastServiceError> {
        remove_queued_episode(&self.podcast_database, episode_id).map_err(sqlite_error)
    }

    pub fn clear_podcast_queue(&self) -> Result<(), PodcastServiceError> {
        clear_queue(&self.podcast_database).map_err(sqlite_error)
    }

    pub fn log_podcast_notification_event(
        &self,
        episode_id: i64,
        classified_at: &str,
        action: &str,
        suppression_reason: Option<&str>,
    ) -> Result<i64, PodcastServiceError> {
        log_notification_event(
            &self.podcast_database,
            episode_id,
            classified_at,
            action,
            suppression_reason,
        )
        .map_err(sqlite_error)
    }

    pub fn mark_podcast_notification_toast_fired(
        &self,
        notification_id: i64,
        fired_at: &str,
    ) -> Result<(), PodcastServiceError> {
        mark_notification_toast_fired(&self.podcast_database, notification_id, fired_at)
            .map_err(sqlite_error)
    }

    pub fn list_podcast_notification_inbox(
        &self,
        limit: usize,
    ) -> Result<Vec<NotificationEntry>, PodcastServiceError> {
        list_notification_inbox(&self.podcast_database, limit).map_err(sqlite_error)
    }

    pub fn clear_podcast_notification_inbox(&self) -> Result<(), PodcastServiceError> {
        clear_notification_inbox(&self.podcast_database).map_err(sqlite_error)
    }

    pub fn update_podcast_cache_entry(
        &self,
        episode_id: i64,
        cache_path: &str,
        bytes_cached: i64,
        total_bytes: Option<i64>,
    ) -> Result<(), PodcastServiceError> {
        update_cache_entry(
            &self.podcast_database,
            episode_id,
            cache_path,
            bytes_cached,
            total_bytes,
        )
        .map_err(sqlite_error)
    }

    pub fn get_podcast_cache_entry(
        &self,
        episode_id: i64,
    ) -> Result<Option<CacheEntry>, PodcastServiceError> {
        get_cache_entry(&self.podcast_database, episode_id).map_err(sqlite_error)
    }

    pub fn lease_podcast_cache_entry(&self, episode_id: i64) -> Result<(), PodcastServiceError> {
        lease_cache_entry(&self.podcast_database, episode_id).map_err(sqlite_error)
    }

    pub fn release_podcast_cache_entry(&self, episode_id: i64) -> Result<(), PodcastServiceError> {
        release_cache_entry(&self.podcast_database, episode_id).map_err(sqlite_error)
    }

    pub fn mark_podcast_cache_was_playing(
        &self,
        episode_id: i64,
        was_playing: bool,
    ) -> Result<(), PodcastServiceError> {
        mark_cache_was_playing(&self.podcast_database, episode_id, was_playing)
            .map_err(sqlite_error)
    }
}

pub fn is_forced_portable_mode() -> bool {
    std::env::var(FORCE_PORTABLE_ENV)
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

pub fn resolve_runtime_storage(
    config_dir: Option<PathBuf>,
    portable_mode: bool,
    appdata: Option<&Path>,
    home: Option<&Path>,
    app_dir: &Path,
) -> RuntimeStoragePaths {
    if let Some(root) = config_dir {
        return RuntimeStoragePaths::new(
            root.clone(),
            root.clone(),
            root.join("cache"),
            root.join("logs"),
            portable_mode,
            true,
        );
    }

    if portable_mode {
        let root = app_dir.join("config");
        return RuntimeStoragePaths::new(
            root.clone(),
            root.clone(),
            root.join("cache"),
            root.join("logs"),
            true,
            false,
        );
    }

    let config_root = station_scout_settings_path(appdata, home)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("Station Scout"));
    RuntimeStoragePaths::new(
        config_root.clone(),
        config_root.clone(),
        config_root.join("cache"),
        config_root.join("logs"),
        false,
        false,
    )
}

fn active_podcast_subscriptions_exist(conn: &Connection) -> rusqlite::Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM subscriptions WHERE unsubscribed_at IS NULL LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn sqlite_error(error: rusqlite::Error) -> PodcastServiceError {
    PodcastServiceError::new(error.to_string())
}
