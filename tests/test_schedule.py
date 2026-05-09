import datetime as dt

from station_scout.models import TuneTimer
from station_scout.schedule import due_timers


def test_due_timers_returns_enabled_timers_once_per_day() -> None:
    timers = [
        TuneTimer("abc", "Example", "07:30"),
        TuneTimer("def", "Disabled", "07:30", enabled=False),
        TuneTimer("ghi", "Later", "08:00"),
    ]
    fired = {("2026-05-09", "already", "07:30")}

    due = due_timers(timers, dt.datetime(2026, 5, 9, 7, 30), fired)

    assert due == [timers[0]]
    assert fired == {
        ("2026-05-09", "already", "07:30"),
        ("2026-05-09", "abc", "07:30"),
    }

