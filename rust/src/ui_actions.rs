use crate::models::{Station, TuneTimer};
use crate::podcasts::discovery::SearchResult;
use crate::podcasts::repository::{EpisodeRow, NotificationEntry, QueueEntry, SubscriptionRow};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PodcastUiModel {
    pub subscriptions: Vec<String>,
    pub episodes: Vec<String>,
    pub queue: Vec<String>,
    pub inbox: Vec<String>,
}

impl PodcastUiModel {
    pub fn from_rows(
        subscriptions: Vec<SubscriptionRow>,
        episodes: Vec<EpisodeRow>,
        queue: Vec<QueueEntry>,
        inbox: Vec<NotificationEntry>,
    ) -> Self {
        Self {
            subscriptions: subscriptions.iter().map(subscription_label).collect(),
            episodes: episodes.iter().map(episode_label).collect(),
            queue: queue.iter().map(queue_label).collect(),
            inbox: inbox.iter().map(inbox_label).collect(),
        }
    }
}

pub fn subscription_label(row: &SubscriptionRow) -> String {
    let title = row.title.as_deref().unwrap_or("Untitled show");
    let author = row.author.as_deref().unwrap_or("unknown author");
    format!("{title} by {author}; feed {}", row.feed_url)
}

pub fn podcast_search_result_label(row: &SearchResult) -> String {
    let author = row.author.as_deref().unwrap_or("unknown author");
    format!(
        "{} by {author}; source {}; feed {}",
        row.title, row.source, row.feed_url
    )
}

pub fn station_label(station: &Station) -> String {
    let subtitle = station.subtitle();
    if subtitle.is_empty() {
        format!("{}; {}", station.name, station.quality_label())
    } else {
        format!(
            "{}; {}; {}",
            station.name,
            subtitle,
            station.quality_label()
        )
    }
}

pub fn timer_label(timer: &TuneTimer) -> String {
    let mut parts = vec![
        format!("{} at {}", timer.station_name, timer.time),
        if timer.enabled { "enabled" } else { "disabled" }.to_string(),
    ];
    if timer.auto_play {
        parts.push("auto play".to_string());
    }
    if !timer.end_time.trim().is_empty() {
        parts.push(format!("stop at {}", timer.end_time));
    }
    if !timer.show_name.trim().is_empty() {
        parts.push(format!("show {}", timer.show_name));
    }
    if timer.track_playlist {
        parts.push("track playlist".to_string());
    }
    parts.join("; ")
}

pub fn episode_label(row: &EpisodeRow) -> String {
    let mut parts = vec![
        row.title
            .as_deref()
            .unwrap_or("Untitled episode")
            .to_string(),
    ];
    parts.push(if row.is_played {
        "played".to_string()
    } else {
        "unplayed".to_string()
    });
    if let Some(position) = row.queue_position.filter(|_| row.is_queued) {
        parts.push(format!("queued position {position}"));
    }
    if row.position_ms > 0 {
        parts.push(format!(
            "listened {}",
            human_duration(row.position_ms / 1_000)
        ));
    }
    if let Some(duration) = row.duration_seconds {
        parts.push(format!("duration {}", human_duration(i64::from(duration))));
    }
    if let Some(pub_date) = row.pub_date.as_deref() {
        parts.push(format!("published {pub_date}"));
    }
    if row.transcript_url.is_some() {
        parts.push("transcript available".to_string());
    }
    parts.join("; ")
}

pub fn queue_label(row: &QueueEntry) -> String {
    let title = row.title.as_deref().unwrap_or("Untitled episode");
    format!(
        "Queue position {}: {title}; added {}",
        row.position, row.added_at
    )
}

pub fn inbox_label(row: &NotificationEntry) -> String {
    let episode = row.episode_title.as_deref().unwrap_or("Untitled episode");
    let show = row.show_title.as_deref().unwrap_or("unknown show");
    let mut parts = vec![
        format!("{episode} from {show}"),
        format!("action {}", row.action),
    ];
    parts.push(if row.toast_fired {
        "toast fired".to_string()
    } else {
        "toast pending".to_string()
    });
    if let Some(reason) = row.suppression_reason.as_deref() {
        parts.push(format!("suppressed: {reason}"));
    }
    parts.push(format!("classified {}", row.classified_at));
    parts.join("; ")
}

fn human_duration(total_seconds: i64) -> String {
    let total_seconds = total_seconds.max(0);
    let hours = total_seconds / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;
    let mut parts = Vec::new();
    push_unit(&mut parts, hours, "hour");
    push_unit(&mut parts, minutes, "minute");
    push_unit(&mut parts, seconds, "second");
    if parts.is_empty() {
        "0 seconds".to_string()
    } else {
        parts.join(" ")
    }
}

fn push_unit(parts: &mut Vec<String>, value: i64, unit: &str) {
    if value > 0 {
        let suffix = if value == 1 { "" } else { "s" };
        parts.push(format!("{value} {unit}{suffix}"));
    }
}
