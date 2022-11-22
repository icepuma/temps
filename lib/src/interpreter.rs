use chrono::DateTime;

use crate::Time;

pub(crate) fn interpret<Tz: chrono::TimeZone>(
    time: Time,
    now: DateTime<Tz>,
) -> chrono::DateTime<Tz> {
    match time {
        Time::Now => now,
    }
}
