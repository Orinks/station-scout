from __future__ import annotations

import datetime as dt

from station_scout.models import TuneTimer

FiredTimerKey = tuple[str, str, str]


def due_timers(
    timers: list[TuneTimer],
    now: dt.datetime,
    fired_today: set[FiredTimerKey],
) -> list[TuneTimer]:
    today = now.date().isoformat()
    current_time = now.strftime("%H:%M")
    due: list[TuneTimer] = []
    for timer in timers:
        key = (today, timer.stationuuid, timer.time)
        if timer.enabled and timer.time == current_time and key not in fired_today:
            fired_today.add(key)
            due.append(timer)
    return due

