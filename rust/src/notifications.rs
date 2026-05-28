#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotifierBackend {
    Toasted,
    DesktopNotifier,
    Wx,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationRule {
    Always,
    Digest,
    Never,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuietHours {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_quiet_hours_start")]
    pub start: String,
    #[serde(default = "default_quiet_hours_end")]
    pub end: String,
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: false,
            start: "22:00".to_string(),
            end: "07:00".to_string(),
        }
    }
}

fn default_quiet_hours_start() -> String {
    "22:00".to_string()
}

fn default_quiet_hours_end() -> String {
    "07:00".to_string()
}

const ALLOWED_ANNOUNCEMENT_EVENTS: &[&str] = &[
    "play",
    "pause",
    "seek_completed",
    "chapter_change",
    "sleep_timer_fired",
    "end_of_episode",
    "buffering_start",
    "buffering_end",
    "new_toast",
    "notifications_paused",
    "notifications_resumed",
    "error",
    "subscriptions_refreshed",
    "opml_import_complete",
    "opml_export_complete",
    "subscribed",
    "unsubscribed",
    "no_selection",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AccessibleAnnouncer {
    spoken: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnouncementError {
    message: String,
}

impl AccessibleAnnouncer {
    pub fn announce_event(
        &mut self,
        event: &str,
        context: &[(&str, &str)],
    ) -> Result<(), AnnouncementError> {
        if !ALLOWED_ANNOUNCEMENT_EVENTS.contains(&event) {
            return Err(AnnouncementError {
                message: format!("unknown announcer event: {event}"),
            });
        }
        self.speak(announcement_phrase(event, context));
        Ok(())
    }

    pub fn announce_position_on_demand(&mut self, position_text: &str) {
        self.speak(position_text.to_string());
    }

    pub fn spoken(&self) -> &[String] {
        &self.spoken
    }

    fn speak(&mut self, phrase: String) {
        if !phrase.trim().is_empty() {
            self.spoken.push(phrase);
        }
    }
}

impl std::fmt::Display for AnnouncementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AnnouncementError {}

pub fn allowed_announcement_events() -> &'static [&'static str] {
    ALLOWED_ANNOUNCEMENT_EVENTS
}

pub fn announcement_phrase(event: &str, context: &[(&str, &str)]) -> String {
    let get = |name: &str| {
        context
            .iter()
            .find_map(|(key, value)| (*key == name).then_some(*value))
            .unwrap_or("")
    };
    match event {
        "play" => "Playing".to_string(),
        "pause" => "Paused".to_string(),
        "seek_completed" => "Seek complete".to_string(),
        "chapter_change" => {
            let title = get("chapter_title");
            if title.is_empty() {
                "Chapter changed".to_string()
            } else {
                format!("Chapter: {title}")
            }
        }
        "sleep_timer_fired" => "Sleep timer ended".to_string(),
        "end_of_episode" => "End of episode".to_string(),
        "buffering_start" => "Buffering".to_string(),
        "buffering_end" => "Buffering complete".to_string(),
        "new_toast" => {
            let show = get("show_title");
            if show.is_empty() {
                "New episode".to_string()
            } else {
                format!("New episode: {show}")
            }
        }
        "notifications_paused" => {
            let until = get("until");
            if until.is_empty() {
                "Notifications paused".to_string()
            } else {
                format!("Notifications paused until {until}")
            }
        }
        "notifications_resumed" => "Notifications resumed".to_string(),
        "error" => {
            let detail = get("detail");
            if detail.is_empty() {
                "Error".to_string()
            } else {
                format!("Error: {detail}")
            }
        }
        "subscriptions_refreshed" => {
            let count = get("new_episodes");
            if count.is_empty() || count == "0" {
                "Subscriptions refreshed".to_string()
            } else {
                format!("Subscriptions refreshed; {count} new")
            }
        }
        "opml_import_complete" => format!(
            "OPML import complete: {} added, {} skipped.",
            get("added").parse::<i32>().unwrap_or(0),
            get("skipped").parse::<i32>().unwrap_or(0)
        ),
        "opml_export_complete" => {
            format!(
                "Exported {} subscriptions to OPML.",
                get("count").parse::<i32>().unwrap_or(0)
            )
        }
        "subscribed" => {
            let title = get("title");
            if title.is_empty() {
                "Subscribed".to_string()
            } else {
                format!("Subscribed to {title}")
            }
        }
        "unsubscribed" => {
            let title = get("title");
            if title.is_empty() {
                "Unsubscribed".to_string()
            } else {
                format!("Unsubscribed from {title}")
            }
        }
        "no_selection" => "No show selected".to_string(),
        _ => event.to_string(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationContext {
    pub rule: NotificationRule,
    pub quiet_hours: QuietHours,
    pub local_time: String,
    pub notifications_paused: bool,
    pub tray_available: bool,
}

impl Default for NotificationContext {
    fn default() -> Self {
        Self {
            rule: NotificationRule::Always,
            quiet_hours: QuietHours::default(),
            local_time: "12:00".to_string(),
            notifications_paused: false,
            tray_available: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotificationAction {
    FireToast,
    BufferForEndOfInterval,
    BufferForQuietHoursEnd,
    InboxOnly { suppression_reason: Option<String> },
}

pub fn select_notifier_backend(
    system: &str,
    toasted_available: bool,
    desktop_notifier_available: bool,
) -> NotifierBackend {
    match system {
        "Windows" if toasted_available => NotifierBackend::Toasted,
        "Darwin" if desktop_notifier_available => NotifierBackend::DesktopNotifier,
        _ => NotifierBackend::Wx,
    }
}

pub fn classify_notification(context: NotificationContext) -> NotificationAction {
    if context.notifications_paused {
        return NotificationAction::InboxOnly {
            suppression_reason: Some("paused".to_string()),
        };
    }
    match context.rule {
        NotificationRule::Never => NotificationAction::InboxOnly {
            suppression_reason: Some("rule_never".to_string()),
        },
        NotificationRule::Digest => NotificationAction::BufferForEndOfInterval,
        NotificationRule::Always if in_quiet_hours(&context.quiet_hours, &context.local_time) => {
            NotificationAction::BufferForQuietHoursEnd
        }
        NotificationRule::Always if !context.tray_available => NotificationAction::InboxOnly {
            suppression_reason: Some("tray_disabled".to_string()),
        },
        NotificationRule::Always => NotificationAction::FireToast,
    }
}

fn in_quiet_hours(quiet_hours: &QuietHours, local_time: &str) -> bool {
    if !quiet_hours.enabled {
        return false;
    }
    let Some(now) = minutes(local_time) else {
        return false;
    };
    let Some(start) = minutes(&quiet_hours.start) else {
        return false;
    };
    let Some(end) = minutes(&quiet_hours.end) else {
        return false;
    };
    if start == end {
        return true;
    }
    if start < end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

fn minutes(value: &str) -> Option<i32> {
    let (hour, minute) = value.split_once(':')?;
    let hour = hour.parse::<i32>().ok()?;
    let minute = minute.parse::<i32>().ok()?;
    if (0..24).contains(&hour) && (0..60).contains(&minute) {
        Some(hour * 60 + minute)
    } else {
        None
    }
}
use serde::{Deserialize, Serialize};
