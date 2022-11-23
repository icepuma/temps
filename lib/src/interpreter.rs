use chrono::{DateTime, LocalResult, TimeZone, Utc};

use crate::{TempsError, Time};

pub(crate) fn interpret<Tz: chrono::TimeZone>(
    time: Time,
    now: DateTime<Tz>,
) -> Result<chrono::DateTime<Tz>, TempsError> {
    match time {
        Time::Now => Ok(now),
        Time::Date { day, month, year } => {
            let utc = Utc.with_ymd_and_hms(year, month, day, 0, 0, 0);

            if let LocalResult::Single(utc) = utc {
                Ok(utc.with_timezone(&now.timezone()))
            } else {
                Err(TempsError::ChronoError)
            }
        }
    }
}
