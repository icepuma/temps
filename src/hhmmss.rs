pub trait Hhmmss {
    /// Get seconds and milliseconds from a Duration
    fn seconds_milliseconds(&self) -> (i64, i128);

    /// Convert a
    /// * std::time::Duration
    /// * chrono::Duration
    /// * time::Duration
    /// to a "hh:mm:ss" formatted string (hh = hours, mm = minutes, ss = seconds)
    fn hhmmss(&self) -> String {
        let (seconds, _) = self.seconds_milliseconds();

        seconds_to_hhmmss(seconds)
    }

    /// Convert a
    /// * std::time::Duration
    /// * chrono::Duration
    /// * time::Duration
    /// to a "hh:mm:ss.xxx" formatted string (hh = hours, mm = minutes, ss = seconds, xxx = milliseconds)
    fn hhmmssxxx(&self) -> String {
        let (seconds, milliseconds) = self.seconds_milliseconds();

        seconds_milliseconds_to_hhmmssxxxx(seconds, milliseconds)
    }
}

/// Convert seconds to "hh:mm:ss"
fn seconds_to_hhmmss(seconds: i64) -> String {
    let (seconds, prefix) = if seconds < 0 {
        (-seconds, "-")
    } else {
        (seconds, "")
    };

    let (hours, seconds) = (seconds / 3_600, seconds % 3_600);
    let (minutes, seconds) = (seconds / 60, seconds % 60);

    format!("{prefix}{hours:02}:{minutes:02}:{seconds:02}")
}

/// Convert seconds to "hh:mm:ss.xxx"
fn seconds_milliseconds_to_hhmmssxxxx(seconds: i64, milliseconds: i128) -> String {
    let (seconds, milliseconds, prefix) = if seconds < 0 {
        (-seconds, -milliseconds, "-")
    } else {
        (seconds, milliseconds, "")
    };

    let (hours, seconds) = (seconds / 3_600, seconds % 3_600);
    let (minutes, seconds) = (seconds / 60, seconds % 60);

    format!("{prefix}{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}")
}

impl Hhmmss for std::time::Duration {
    fn seconds_milliseconds(&self) -> (i64, i128) {
        let seconds = self.as_secs();
        let milliseconds = self.subsec_millis();

        (seconds as i64, milliseconds as i128)
    }
}

#[cfg(feature = "chrono")]
impl Hhmmss for chrono::Duration {
    fn seconds_milliseconds(&self) -> (i64, i128) {
        let seconds = self.num_seconds();
        let milliseconds = self.num_milliseconds() as i128 - (1_000 * seconds) as i128;

        (seconds, milliseconds)
    }
}

#[cfg(feature = "time")]
impl Hhmmss for time::Duration {
    fn seconds_milliseconds(&self) -> (i64, i128) {
        let seconds = self.whole_seconds();
        let milliseconds = self.whole_milliseconds() - (1_000 * seconds) as i128;

        (seconds, milliseconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_std_time_duration() {
        let duration = std::time::Duration::new(5_000, 300_000_000);

        assert_eq!("01:23:20", duration.hhmmss());
        assert_eq!("01:23:20.300", duration.hhmmssxxx());

        let duration = std::time::Duration::new(2_000, 300_000_000)
            - std::time::Duration::new(1_000, 100_000_000);

        assert_eq!("00:16:40", duration.hhmmss());
        assert_eq!("00:16:40.200", duration.hhmmssxxx());

        let duration = std::time::Duration::new(0, 0);

        assert_eq!("00:00:00", duration.hhmmss());
        assert_eq!("00:00:00.000", duration.hhmmssxxx());
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn test_chrono_duration() {
        let duration = chrono::Duration::hours(2)
            + chrono::Duration::minutes(2)
            + chrono::Duration::seconds(200);

        assert_eq!("02:05:20", duration.hhmmss());
        assert_eq!("02:05:20.000", duration.hhmmssxxx());

        let duration = chrono::Duration::hours(2) + chrono::Duration::milliseconds(200);

        assert_eq!("02:00:00", duration.hhmmss());
        assert_eq!("02:00:00.200", duration.hhmmssxxx());

        let duration = chrono::Duration::hours(1)
            - chrono::Duration::hours(2)
            - chrono::Duration::milliseconds(333);

        assert_eq!("-01:00:00", duration.hhmmss());
        assert_eq!("-01:00:00.333", duration.hhmmssxxx());
    }

    #[cfg(feature = "time")]
    #[test]
    fn test_time_duration() {
        let duration =
            time::Duration::hours(2) + time::Duration::minutes(2) + time::Duration::seconds(200);

        assert_eq!("02:05:20", duration.hhmmss());
        assert_eq!("02:05:20.000", duration.hhmmssxxx());

        let duration = time::Duration::hours(2) + time::Duration::milliseconds(200);

        assert_eq!("02:00:00", duration.hhmmss());
        assert_eq!("02:00:00.200", duration.hhmmssxxx());

        let duration =
            time::Duration::hours(1) - time::Duration::hours(2) - time::Duration::milliseconds(333);

        assert_eq!("-01:00:00", duration.hhmmss());
        assert_eq!("-01:00:00.333", duration.hhmmssxxx());
    }
}
