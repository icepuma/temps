//! # temps-core
//!
//! Core functionality for parsing human-readable time expressions.
//!
//! This crate provides the fundamental types and traits for parsing natural language
//! time expressions like "in 5 minutes", "yesterday at 3pm", or "next Monday".
//! It is designed to be backend-agnostic, allowing different datetime libraries
//! (chrono, jiff, etc.) to implement the parsing logic.
//!
//! ## Overview
//!
//! The crate consists of several key components:
//!
//! - **Types**: Core data structures representing different time expressions
//! - **Traits**: Interfaces for implementing time parsing with different backends
//! - **Parsers**: Language-specific parsers (English and German)
//! - **Utilities**: Helper functions for time calculations and conversions
//!
//! ## Example
//!
//! ```
//! use temps_core::{parse, Language, TimeExpression};
//!
//! // Parse a relative time expression
//! let expr = parse("in 5 minutes", Language::English).unwrap();
//! match expr {
//!     TimeExpression::Relative(rel) => {
//!         println!("Amount: {}, Unit: {:?}", rel.amount, rel.unit);
//!     }
//!     _ => {}
//! }
//!
//! // Parse with German language
//! let expr = parse("in 5 Minuten", Language::German).unwrap();
//! ```
//!
//! ## Supported Languages
//!
//! - English
//! - German
//!
//! ## Error Handling
//!
//! All parsing operations return a `Result<T, TempsError>` where `TempsError`
//! provides detailed information about what went wrong during parsing or
//! date calculations.

use winnow::{
    ascii::digit1,
    combinator::{alt, opt},
    prelude::*,
    token::{one_of, take_while},
};

// ===== Error Module =====
pub mod error;
pub use error::{Result, TempsError};

// ===== Core Types =====

/// Represents a parsed time expression.
///
/// This is the main output type of the parsing functions. It can represent
/// various forms of time expressions from natural language input.
///
/// # Examples
///
/// ```
/// use temps_core::{parse, Language, TimeExpression};
///
/// // "now" -> TimeExpression::Now
/// // "in 5 minutes" -> TimeExpression::Relative(...)
/// // "2024-01-15T14:30:00Z" -> TimeExpression::Absolute(...)
/// // "tomorrow" -> TimeExpression::Day(...)
/// // "3:30 pm" -> TimeExpression::Time(...)
/// // "tomorrow at 3:30 pm" -> TimeExpression::DayTime(...)
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum TimeExpression {
    /// The current moment in time (e.g., "now", "jetzt")
    Now,
    /// A time relative to now (e.g., "in 5 minutes", "3 days ago")
    Relative(RelativeTime),
    /// An absolute date/time (e.g., "2024-01-15T14:30:00Z")
    Absolute(AbsoluteTime),
    /// A day reference (e.g., "tomorrow", "next Monday")
    Day(DayReference),
    /// A time of day (e.g., "3:30 pm", "14:30")
    Time(Time),
    /// A calendar date (e.g., "15/03/2024", "31-12-2025")
    Date(StandardDate),
    /// A day with a specific time (e.g., "tomorrow at 3:30 pm")
    DayTime(DayTime),
}

/// Represents a time relative to the current moment.
///
/// # Examples
///
/// ```
/// use temps_core::{RelativeTime, TimeUnit, Direction};
///
/// // "in 5 minutes"
/// let future = RelativeTime {
///     amount: 5,
///     unit: TimeUnit::Minute,
///     direction: Direction::Future,
/// };
///
/// // "3 days ago"
/// let past = RelativeTime {
///     amount: 3,
///     unit: TimeUnit::Day,
///     direction: Direction::Past,
/// };
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct RelativeTime {
    /// The numeric amount (e.g., 5 in "5 minutes")
    pub amount: i64,
    /// The time unit (second, minute, hour, etc.)
    pub unit: TimeUnit,
    /// Whether this is in the past or future
    pub direction: Direction,
}

/// Represents an absolute date and time.
///
/// This type can represent various levels of precision, from just a date
/// to a full timestamp with timezone and nanosecond precision.
///
/// # Examples
///
/// ```
/// use temps_core::{AbsoluteTime, Timezone};
///
/// // Date only: "2024-01-15"
/// let date_only = AbsoluteTime {
///     year: 2024,
///     month: 1,
///     day: 15,
///     hour: None,
///     minute: None,
///     second: None,
///     nanosecond: None,
///     timezone: None,
/// };
///
/// // Full timestamp: "2024-01-15T14:30:00Z"
/// let full_timestamp = AbsoluteTime {
///     year: 2024,
///     month: 1,
///     day: 15,
///     hour: Some(14),
///     minute: Some(30),
///     second: Some(0),
///     nanosecond: None,
///     timezone: Some(Timezone::Utc),
/// };
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct AbsoluteTime {
    /// The year (e.g., 2024)
    pub year: u16,
    /// The month (1-12)
    pub month: u8,
    /// The day of month (1-31)
    pub day: u8,
    /// The hour (0-23), if specified
    pub hour: Option<u8>,
    /// The minute (0-59), if specified
    pub minute: Option<u8>,
    /// The second (0-59), if specified
    pub second: Option<u8>,
    /// The nanosecond (0-999999999), if specified
    pub nanosecond: Option<u32>,
    /// The timezone, if specified
    pub timezone: Option<Timezone>,
}

/// Represents a timezone specification.
///
/// # Examples
///
/// ```
/// use temps_core::Timezone;
///
/// // UTC timezone ("Z")
/// let utc = Timezone::Utc;
///
/// // Offset timezone ("+02:00")
/// let offset = Timezone::Offset { hours: 2, minutes: 0 };
///
/// // Negative offset ("-05:30")
/// let negative = Timezone::Offset { hours: -5, minutes: 30 };
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Timezone {
    /// UTC timezone (represented as "Z" in ISO format)
    Utc,
    /// Timezone offset from UTC
    Offset {
        /// Hours offset (-12 to +14)
        hours: i8,
        /// Minutes offset (0-59)
        minutes: u8,
    },
}

/// Represents a reference to a specific day.
///
/// # Examples
///
/// ```
/// use temps_core::{DayReference, Weekday, WeekdayModifier};
///
/// // "today"
/// let today = DayReference::Today;
///
/// // "next Monday"
/// let next_monday = DayReference::Weekday {
///     day: Weekday::Monday,
///     modifier: Some(WeekdayModifier::Next),
/// };
///
/// // "Friday" (upcoming Friday)
/// let friday = DayReference::Weekday {
///     day: Weekday::Friday,
///     modifier: None,
/// };
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DayReference {
    /// Today's date
    Today,
    /// Yesterday's date
    Yesterday,
    /// Tomorrow's date
    Tomorrow,
    /// A specific weekday
    Weekday {
        /// The day of the week
        day: Weekday,
        /// Optional modifier (next/last)
        modifier: Option<WeekdayModifier>,
    },
}

/// Represents a time of day.
///
/// Can represent both 12-hour (with AM/PM) and 24-hour formats.
///
/// # Examples
///
/// ```
/// use temps_core::{Time, Meridiem};
///
/// // "3:30 PM"
/// let afternoon = Time {
///     hour: 3,
///     minute: 30,
///     second: 0,
///     meridiem: Some(Meridiem::PM),
/// };
///
/// // "14:30" (24-hour format)
/// let military = Time {
///     hour: 14,
///     minute: 30,
///     second: 0,
///     meridiem: None,
/// };
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Time {
    /// Hour (0-23 for 24-hour format, 1-12 for 12-hour format)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Second (0-59)
    pub second: u8,
    /// AM/PM indicator for 12-hour format
    pub meridiem: Option<Meridiem>,
}

/// Represents a calendar date.
///
/// Used for parsing date formats like "15/03/2024" or "31-12-2025".
///
/// # Examples
///
/// ```
/// use temps_core::StandardDate;
///
/// // "15/03/2024"
/// let date = StandardDate {
///     day: 15,
///     month: 3,
///     year: 2024,
/// };
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct StandardDate {
    /// Day of month (1-31)
    pub day: u8,
    /// Month (1-12)
    pub month: u8,
    /// Year (e.g., 2024)
    pub year: u16,
}

/// Represents a combination of a day reference and a specific time.
///
/// Used for expressions like "tomorrow at 3:30 pm" or "next Monday at 9:00 am".
///
/// # Examples
///
/// ```
/// use temps_core::{DayTime, DayReference, Time, Meridiem};
///
/// // "tomorrow at 3:30 pm"
/// let tomorrow_afternoon = DayTime {
///     day: DayReference::Tomorrow,
///     time: Time {
///         hour: 3,
///         minute: 30,
///         second: 0,
///         meridiem: Some(Meridiem::PM),
///     },
/// };
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct DayTime {
    /// The day reference
    pub day: DayReference,
    /// The specific time on that day
    pub time: Time,
}

/// Units of time used in relative expressions.
///
/// # Examples
///
/// ```
/// use temps_core::TimeUnit;
///
/// // Used in expressions like:
/// // "5 seconds", "10 minutes", "2 hours", "3 days",
/// // "1 week", "6 months", "2 years"
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum TimeUnit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

/// Direction of time relative to now.
///
/// # Examples
///
/// ```
/// use temps_core::Direction;
///
/// // "5 minutes ago" -> Direction::Past
/// // "in 5 minutes" -> Direction::Future
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Direction {
    Past,
    Future,
}

/// Days of the week.
///
/// Used in expressions like "next Monday" or "last Friday".
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// Modifiers for weekday references.
///
/// # Examples
///
/// ```
/// use temps_core::WeekdayModifier;
///
/// // "last Monday" -> WeekdayModifier::Last
/// // "next Friday" -> WeekdayModifier::Next
/// // "Monday" (no modifier) -> finds the next occurrence
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum WeekdayModifier {
    Last,
    Next,
}

/// AM/PM indicator for 12-hour time format.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Meridiem {
    AM,
    PM,
}

/// Supported languages for parsing time expressions.
///
/// # Examples
///
/// ```
/// use temps_core::{parse, Language};
///
/// // Parse English
/// let expr = parse("in 5 minutes", Language::English);
///
/// // Parse German
/// let expr = parse("in 5 Minuten", Language::German);
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Language {
    English,
    German,
}

// ===== Traits =====

/// Trait for implementing time parsing with a specific datetime backend.
///
/// This trait should be implemented by datetime libraries (chrono, jiff, etc.)
/// to provide the actual time calculation logic.
///
/// # Examples
///
/// ```
/// use temps_core::{TimeParser, TimeExpression, Result};
///
/// struct MyTimeParser;
///
/// impl TimeParser for MyTimeParser {
///     type DateTime = String; // Your datetime type
///
///     fn now(&self) -> Self::DateTime {
///         "2024-01-15T14:30:00Z".to_string()
///     }
///
///     fn parse_expression(&self, expr: TimeExpression) -> Result<Self::DateTime> {
///         // Implementation here
///         Ok(self.now())
///     }
/// }
/// ```
pub trait TimeParser {
    /// The datetime type used by this implementation
    type DateTime;

    /// Get the current date and time
    fn now(&self) -> Self::DateTime;

    /// Parse a time expression into a concrete datetime
    ///
    /// # Errors
    ///
    /// Returns `TempsError` if:
    /// - Date calculation results in an invalid date
    /// - Arithmetic overflow occurs
    /// - The backend library returns an error
    fn parse_expression(&self, expr: TimeExpression) -> Result<Self::DateTime>;
}

/// Trait for implementing language-specific parsers.
///
/// This trait is implemented by language modules to provide
/// natural language parsing for different languages.
///
/// # Examples
///
/// ```
/// use temps_core::{LanguageParser, TimeExpression, Result};
///
/// struct MyLanguageParser;
///
/// impl LanguageParser for MyLanguageParser {
///     fn parse(&self, input: &str) -> Result<TimeExpression> {
///         // Parse language-specific input
///         Ok(TimeExpression::Now)
///     }
/// }
/// ```
pub trait LanguageParser {
    /// Parse a natural language time expression
    ///
    /// # Errors
    ///
    /// Returns `TempsError::ParseError` if the input cannot be parsed
    fn parse(&self, input: &str) -> Result<TimeExpression>;
}

// ===== Constants Module =====

pub mod constants {
    //! Common constants used across the temps library

    /// Number of seconds in one hour
    pub const SECONDS_PER_HOUR: i32 = 3600;

    /// Number of seconds in one minute  
    pub const SECONDS_PER_MINUTE: i32 = 60;

    /// Number of minutes in one hour
    pub const MINUTES_PER_HOUR: i32 = 60;

    /// Number of hours in one day
    pub const HOURS_PER_DAY: i32 = 24;

    /// Number of days in one week
    pub const DAYS_PER_WEEK: i32 = 7;

    /// Number of months in one year
    pub const MONTHS_PER_YEAR: i32 = 12;
}

// ===== Errors Module =====

pub mod errors {
    //! Common error messages and error handling utilities

    /// Error message for when month amount must be positive
    pub const ERR_MONTH_POSITIVE: &str = "Month amount must be a positive number";

    /// Error message for when year amount must be positive
    pub const ERR_YEAR_POSITIVE: &str = "Year amount must be a positive number";

    /// Error message for invalid date calculation
    pub const ERR_DATE_CALC_INVALID: &str = "Date calculation resulted in invalid date";

    /// Error message for year calculation overflow
    pub const ERR_YEAR_OVERFLOW: &str = "Year calculation overflow";

    /// Error message for invalid date
    pub const ERR_INVALID_DATE: &str = "Invalid date";

    /// Error message for invalid time
    pub const ERR_INVALID_TIME: &str = "Invalid time";

    /// Error message for ambiguous local time
    pub const ERR_AMBIGUOUS_TIME: &str = "Ambiguous or invalid local time";

    /// Error message for failed midnight time creation
    pub const ERR_MIDNIGHT_FAILED: &str = "Failed to create midnight time";

    /// Error message for date calculation errors
    pub const ERR_DATE_CALC_ERROR: &str = "Date calculation error";

    /// Error message for timezone conversion errors
    pub const ERR_TIMEZONE_CONVERSION: &str = "Timezone conversion error";

    /// Format error message for invalid date with components
    #[must_use]
    pub fn format_invalid_date(year: u16, month: u8, day: u8) -> String {
        format!("Invalid date: {year}-{month}-{day}")
    }

    /// Format error message for invalid time with components
    #[must_use]
    pub fn format_invalid_time(hour: u8, minute: u8, second: u8) -> String {
        format!("Invalid time: {hour}:{minute}:{second}")
    }

    /// Format error message for invalid timezone offset
    #[must_use]
    pub fn format_invalid_timezone_offset(hours: i8, minutes: u8) -> String {
        format!("Invalid timezone offset: {hours}:{minutes}")
    }
}

// ===== Time Utils Module =====

pub mod time_utils {
    //! Time conversion and calculation utilities

    use crate::{
        Meridiem, WeekdayModifier,
        constants::{SECONDS_PER_HOUR, SECONDS_PER_MINUTE},
    };

    /// Convert 12-hour time format to 24-hour format
    ///
    /// # Examples
    /// ```
    /// use temps_core::{Meridiem, time_utils::convert_12_to_24_hour};
    ///
    /// assert_eq!(convert_12_to_24_hour(12, Some(&Meridiem::AM)), 0);  // 12 AM -> 0
    /// assert_eq!(convert_12_to_24_hour(12, Some(&Meridiem::PM)), 12); // 12 PM -> 12
    /// assert_eq!(convert_12_to_24_hour(3, Some(&Meridiem::PM)), 15);  // 3 PM -> 15
    /// assert_eq!(convert_12_to_24_hour(14, None), 14);                // 24-hour format
    /// ```
    #[must_use]
    pub fn convert_12_to_24_hour(hour: u8, meridiem: Option<&Meridiem>) -> u8 {
        match meridiem {
            Some(Meridiem::AM) => {
                if hour == 12 {
                    0
                } else {
                    hour
                }
            }
            Some(Meridiem::PM) => {
                if hour == 12 {
                    hour
                } else {
                    hour + 12
                }
            }
            None => hour,
        }
    }

    /// Calculate total seconds for a timezone offset
    ///
    /// Uses saturating arithmetic to prevent overflow
    #[must_use]
    pub fn calculate_timezone_offset_seconds(hours: i8, minutes: u8) -> i32 {
        let hour_seconds = (hours as i32).saturating_mul(SECONDS_PER_HOUR);
        let minute_seconds = (minutes as i32).saturating_mul(SECONDS_PER_MINUTE);
        hour_seconds.saturating_add(minute_seconds)
    }

    /// Calculate the day offset for weekday calculations
    ///
    /// Returns the number of days to add/subtract to reach the target weekday
    ///
    /// # Arguments
    /// * `current_day_offset` - Current weekday as offset from Monday (0-6)
    /// * `target_day_offset` - Target weekday as offset from Monday (0-6)
    /// * `modifier` - Whether to get next, last, or closest occurrence
    #[must_use]
    pub fn calculate_weekday_offset(
        current_day_offset: i64,
        target_day_offset: i64,
        modifier: Option<WeekdayModifier>,
    ) -> i64 {
        let days_diff = target_day_offset - current_day_offset;

        match modifier {
            None => {
                // Get the next occurrence (including today if it matches)
                if days_diff >= 0 {
                    days_diff
                } else {
                    7 + days_diff
                }
            }
            Some(WeekdayModifier::Next) => {
                // Next occurrence (not including today)
                if days_diff > 0 {
                    days_diff
                } else {
                    7 + days_diff
                }
            }
            Some(WeekdayModifier::Last) => {
                // Previous occurrence (not including today)
                if days_diff < 0 {
                    days_diff
                } else {
                    days_diff - 7
                }
            }
        }
    }
}

// ===== Common Parsing Module =====

/// Common parsing utilities shared across language implementations.
///
/// This module contains parser functions that are shared between
/// different language implementations, such as ISO datetime parsing
/// and number parsing.
pub mod common {

    use super::*;

    /// Parse a sequence of digits as an i64.
    ///
    /// Used for parsing numeric amounts in time expressions.
    ///
    /// # Examples
    ///
    /// This parses "123" -> 123, "5" -> 5, etc.
    pub fn parse_digit_number(input: &mut &str) -> winnow::Result<i64> {
        digit1.try_map(|s: &str| s.parse::<i64>()).parse_next(input)
    }

    /// Parse ISO 8601 datetime format.
    ///
    /// Supports various ISO datetime formats:
    /// - Date only: `2024-01-15`
    /// - Date and time: `2024-01-15T14:30:00`
    /// - With timezone: `2024-01-15T14:30:00Z`
    /// - With offset: `2024-01-15T14:30:00+02:00`
    /// - With fractional seconds: `2024-01-15T14:30:00.123Z`
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Parses: "2024-01-15T14:30:00Z"
    /// // Into: TimeExpression::Absolute(AbsoluteTime { ... })
    /// ```
    pub fn parse_iso_datetime(input: &mut &str) -> winnow::Result<TimeExpression> {
        // Parse date components
        let year = parse_four_digit_number.parse_next(input)?;
        '-'.parse_next(input)?;
        let month = parse_two_digit_number.parse_next(input)?;
        '-'.parse_next(input)?;
        let day = parse_two_digit_number.parse_next(input)?;

        // Parse optional time components
        let time_part = opt((
            one_of(['T', ' ']),
            parse_two_digit_number, // hour
            ':',
            parse_two_digit_number, // minute
            opt((
                ':',
                parse_two_digit_number, // second
                opt((
                    '.',
                    digit1.try_map(|s: &str| {
                        // Convert fractional seconds to nanoseconds
                        let fraction = if s.len() > 9 { &s[..9] } else { s };

                        // Parse the fraction and multiply by appropriate power of 10
                        let parsed = fraction.parse::<u32>()?;
                        let multiplier = 10_u32.pow(9 - fraction.len() as u32);
                        Ok::<u32, std::num::ParseIntError>(parsed * multiplier)
                    }),
                )),
            )),
            opt(parse_timezone),
        ))
        .parse_next(input)?;

        let (hour, minute, second, nanosecond, timezone) =
            if let Some((_, h, _, m, sec_part, tz)) = time_part {
                // We have time components
                let hour = Some(h);
                let minute = Some(m);

                // Extract seconds and fractional seconds if present
                let (second, nanosecond) = if let Some((_, s, frac)) = sec_part {
                    (Some(s), frac.map(|(_, n)| n))
                } else {
                    (None, None)
                };

                (hour, minute, second, nanosecond, tz)
            } else {
                // Date only, no time components
                (None, None, None, None, None)
            };

        Ok(TimeExpression::Absolute(AbsoluteTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanosecond,
            timezone,
        }))
    }

    /// Parse timezone specification.
    ///
    /// Supports:
    /// - `Z` for UTC
    /// - `+HH:MM` or `-HH:MM` for offsets
    /// - `+HH` or `-HH` (minutes optional)
    fn parse_timezone(input: &mut &str) -> winnow::Result<Timezone> {
        alt(("Z".map(|_| Timezone::Utc), parse_offset_timezone)).parse_next(input)
    }

    /// Parse timezone offset in +/-HH:MM format.
    ///
    /// Examples: `+02:00`, `-05:30`, `+09`
    fn parse_offset_timezone(input: &mut &str) -> winnow::Result<Timezone> {
        let sign = one_of(['+', '-']).parse_next(input)?;
        let hours = parse_two_digit_number.parse_next(input)?;
        let minutes = opt((':', parse_two_digit_number))
            .parse_next(input)?
            .map(|(_, m)| m)
            .unwrap_or(0);

        let hours = if sign == '+' {
            hours as i8
        } else {
            -(hours as i8)
        };

        Ok(Timezone::Offset { hours, minutes })
    }

    /// Parse a two-digit number as u8.
    ///
    /// Used for parsing hours, minutes, days, months.
    /// Accepts 1 or 2 digits (e.g., "5" or "05").
    pub fn parse_two_digit_number(input: &mut &str) -> winnow::Result<u8> {
        take_while(1..=2, |c: char| c.is_ascii_digit())
            .try_map(|s: &str| s.parse::<u8>())
            .parse_next(input)
    }

    /// Parse a four-digit number as u16.
    ///
    /// Used for parsing years.
    /// Requires exactly 4 digits.
    pub fn parse_four_digit_number(input: &mut &str) -> winnow::Result<u16> {
        take_while(4..=4, |c: char| c.is_ascii_digit())
            .try_map(|s: &str| s.parse::<u16>())
            .parse_next(input)
    }
}

// ===== Language Support =====

/// Language-specific parser implementations.
///
/// Each submodule contains a parser for a specific language.
/// All parsers implement the `LanguageParser` trait.
pub mod language {
    /// English language parser.
    ///
    /// Supports expressions like:
    /// - "in 5 minutes", "3 days ago"
    /// - "tomorrow at 3:30 pm"
    /// - "next Monday", "last Friday"
    pub mod english;

    /// German language parser.
    ///
    /// Supports expressions like:
    /// - "in 5 Minuten", "vor 3 Tagen"
    /// - "morgen um 15:30"
    /// - "nächsten Montag", "letzten Freitag"
    pub mod german;
}

// ===== Main Parsing Function =====

/// Parse a natural language time expression.
///
/// This is the main entry point for parsing time expressions. It takes
/// a string input and a language, and returns a parsed `TimeExpression`.
///
/// # Arguments
///
/// * `input` - The natural language time expression to parse
/// * `language` - The language to use for parsing
///
/// # Returns
///
/// Returns `Ok(TimeExpression)` if parsing succeeds, or `Err(TempsError)`
/// if the input cannot be parsed.
///
/// # Examples
///
/// ```
/// use temps_core::{parse, Language, TimeExpression};
///
/// // Parse English expressions
/// let expr = parse("in 5 minutes", Language::English).unwrap();
/// let expr = parse("tomorrow at 3:30 pm", Language::English).unwrap();
/// let expr = parse("next Monday", Language::English).unwrap();
///
/// // Parse German expressions
/// let expr = parse("in 5 Minuten", Language::German).unwrap();
/// let expr = parse("morgen um 15:30", Language::German).unwrap();
/// let expr = parse("nächsten Montag", Language::German).unwrap();
///
/// // Parse ISO datetime (works in any language)
/// let expr = parse("2024-01-15T14:30:00Z", Language::English).unwrap();
/// ```
///
/// # Supported Formats
///
/// ## Relative Time
/// - "in 5 minutes", "5 minutes ago"
/// - "in 2 hours", "an hour ago"
/// - "in 3 days", "2 days ago"
/// - "in a week", "2 weeks ago"
/// - "in 6 months", "a month ago"
/// - "in 2 years", "a year ago"
///
/// ## Day References
/// - "today", "yesterday", "tomorrow"
/// - "Monday", "Tuesday", etc.
/// - "next Monday", "last Friday"
///
/// ## Times
/// - "3:30 pm", "10:15 am"
/// - "14:30", "09:00"
///
/// ## Dates
/// - "15/03/2024", "31-12-2025"
///
/// ## Combined
/// - "tomorrow at 3:30 pm"
/// - "next Monday at 9:00 am"
///
/// ## ISO Format
/// - "2024-01-15T14:30:00Z"
/// - "2024-01-15T14:30:00+02:00"
/// - "2024-01-15T14:30:00.123Z"
pub fn parse(input: &str, language: Language) -> Result<TimeExpression> {
    match language {
        Language::English => language::english::EnglishParser.parse(input),
        Language::German => language::german::GermanParser.parse(input),
    }
}
