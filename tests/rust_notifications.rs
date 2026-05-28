use station_scout::notifications::{
    AccessibleAnnouncer, NotificationAction, NotificationContext, NotificationRule,
    NotifierBackend, QuietHours, allowed_announcement_events, classify_notification,
    select_notifier_backend,
};

#[test]
fn windows_uses_toasted_when_available_and_wx_when_missing() {
    assert_eq!(
        select_notifier_backend("Windows", true, false),
        NotifierBackend::Toasted
    );
    assert_eq!(
        select_notifier_backend("Windows", false, false),
        NotifierBackend::Wx
    );
}

#[test]
fn macos_uses_desktop_notifier_when_available_and_wx_when_missing() {
    assert_eq!(
        select_notifier_backend("Darwin", false, true),
        NotifierBackend::DesktopNotifier
    );
    assert_eq!(
        select_notifier_backend("Darwin", false, false),
        NotifierBackend::Wx
    );
}

#[test]
fn linux_uses_wx_notifier() {
    assert_eq!(
        select_notifier_backend("Linux", true, true),
        NotifierBackend::Wx
    );
}

#[test]
fn podcast_notification_classifier_honors_per_show_rules_and_pause_state() {
    assert_eq!(
        classify_notification(NotificationContext {
            rule: NotificationRule::Never,
            ..NotificationContext::default()
        }),
        NotificationAction::InboxOnly {
            suppression_reason: Some("rule_never".to_string())
        }
    );
    assert_eq!(
        classify_notification(NotificationContext {
            rule: NotificationRule::Digest,
            ..NotificationContext::default()
        }),
        NotificationAction::BufferForEndOfInterval
    );
    assert_eq!(
        classify_notification(NotificationContext {
            notifications_paused: true,
            ..NotificationContext::default()
        }),
        NotificationAction::InboxOnly {
            suppression_reason: Some("paused".to_string())
        }
    );
}

#[test]
fn podcast_notification_classifier_buffers_during_quiet_hours() {
    assert_eq!(
        classify_notification(NotificationContext {
            local_time: "23:15".to_string(),
            quiet_hours: QuietHours {
                enabled: true,
                start: "22:00".to_string(),
                end: "07:00".to_string(),
            },
            ..NotificationContext::default()
        }),
        NotificationAction::BufferForQuietHoursEnd
    );
    assert_eq!(
        classify_notification(NotificationContext {
            local_time: "12:00".to_string(),
            quiet_hours: QuietHours {
                enabled: true,
                start: "22:00".to_string(),
                end: "07:00".to_string(),
            },
            ..NotificationContext::default()
        }),
        NotificationAction::FireToast
    );
}

#[test]
fn accessible_announcer_accepts_only_discrete_whitelisted_events() {
    let mut announcer = AccessibleAnnouncer::default();

    assert!(announcer.announce_event("not-a-real-event", &[]).is_err());
    for event in [
        "play",
        "pause",
        "chapter_change",
        "new_toast",
        "notifications_paused",
        "error",
        "opml_import_complete",
    ] {
        announcer
            .announce_event(
                event,
                &[("chapter_title", "Intro"), ("detail", "Network down")],
            )
            .expect("whitelisted event should announce");
    }
    announcer.announce_position_on_demand("00:05:12");

    assert_eq!(
        announcer.spoken(),
        &[
            "Playing",
            "Paused",
            "Chapter: Intro",
            "New episode",
            "Notifications paused",
            "Error: Network down",
            "OPML import complete: 0 added, 0 skipped.",
            "00:05:12",
        ]
    );
}

#[test]
fn accessible_announcer_has_no_periodic_position_events() {
    let events = allowed_announcement_events();

    assert!(!events.contains(&"position_tick"));
    assert!(!events.contains(&"tick"));
    assert!(!events.contains(&"announce_every"));
}
