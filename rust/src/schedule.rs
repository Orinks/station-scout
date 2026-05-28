use crate::models::TuneTimer;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

pub type FiredTimerKey = (String, String, String);

pub fn due_timers(
    timers: &[TuneTimer],
    now: DateTime<Utc>,
    fired_today: &mut HashSet<FiredTimerKey>,
) -> Vec<TuneTimer> {
    let today = now.format("%Y-%m-%d").to_string();
    let current_time = now.format("%H:%M").to_string();
    let mut due = Vec::new();
    for timer in timers {
        let key = (today.clone(), timer.stationuuid.clone(), timer.time.clone());
        if timer.enabled && timer.time == current_time && !fired_today.contains(&key) {
            fired_today.insert(key);
            due.push(timer.clone());
        }
    }
    due
}
