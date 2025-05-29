//! Common test helpers for chrono-based tests

use chrono::{DateTime, Local, TimeZone};
use mockall::automock;

/// Common trait for mocking time sources in chrono tests
#[automock]
pub trait TimeSource: Send + Sync {
    fn now(&self) -> DateTime<Local>;
}

/// Create a fixed DateTime for testing
pub fn fixed_datetime(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, hour, minute, second)
        .unwrap()
}

/// Common test dates
pub mod test_dates {
    use super::*;

    pub fn jan_31_2024() -> DateTime<Local> {
        fixed_datetime(2024, 1, 31, 10, 0, 0)
    }

    pub fn feb_29_2024() -> DateTime<Local> {
        fixed_datetime(2024, 2, 29, 10, 0, 0)
    }

    pub fn june_15_2023() -> DateTime<Local> {
        fixed_datetime(2023, 6, 15, 14, 30, 0)
    }
}
