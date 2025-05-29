//! Common test helpers for jiff-based tests

use jiff::{Zoned, civil::date};
use mockall::automock;

/// Common trait for mocking time sources in jiff tests
#[automock]
pub trait TimeSource: Send + Sync {
    fn now(&self) -> Zoned;
}

/// Common test dates
pub mod test_dates {
    use super::*;

    pub fn jan_31_2024() -> Zoned {
        date(2024, 1, 31)
            .at(10, 0, 0, 0)
            .to_zoned(jiff::tz::TimeZone::system())
            .unwrap()
    }

    pub fn feb_29_2024() -> Zoned {
        date(2024, 2, 29)
            .at(10, 0, 0, 0)
            .to_zoned(jiff::tz::TimeZone::system())
            .unwrap()
    }

    pub fn june_15_2023() -> Zoned {
        date(2023, 6, 15)
            .at(14, 30, 0, 0)
            .to_zoned(jiff::tz::TimeZone::system())
            .unwrap()
    }
}
