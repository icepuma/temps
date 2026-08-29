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
    /// A short way into the future, clamped so it cannot leave today
    /// (e.g., "later today"). Resolves to `now + 2h`, or the last second of
    /// today if that would cross midnight.
    LaterToday,
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
/// let offset = Timezone::Offset { total_minutes: 120 };
///
/// // Negative offset ("-05:30")
/// let negative = Timezone::Offset { total_minutes: -330 };
///
/// // Negative sub-hour offset ("-00:30")
/// let half_hour_west = Timezone::Offset { total_minutes: -30 };
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Timezone {
    /// UTC timezone (represented as "Z" in ISO format)
    Utc,
    /// Timezone offset from UTC, in minutes east of UTC.
    ///
    /// A single signed field so that negative sub-hour offsets such as
    /// `-00:30` are representable; a split hour/minute pair cannot carry the
    /// sign when the hour component is zero.
    Offset {
        /// Offset from UTC in minutes, from -720 (-12:00) to +840 (+14:00)
        total_minutes: i16,
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
    /// The day before yesterday's date
    DayBeforeYesterday,
    /// Day after tomorrow's date
    DayAfterTomorrow,
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
    /// The occurrence within the current Monday-to-Sunday week, which may be
    /// in the past (e.g. "this weekend" asked on a Sunday).
    This,
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

    /// Error message for a relative amount too large for the backend to represent
    pub const ERR_AMOUNT_OUT_OF_RANGE: &str = "Relative amount is too large to represent as a date";

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

    /// Error message for negative relative amounts
    pub const ERR_RELATIVE_AMOUNT_NON_NEGATIVE: &str = "Relative amount must be non-negative";

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
    pub fn format_invalid_timezone_offset(total_minutes: i16) -> String {
        let sign = if total_minutes < 0 { '-' } else { '+' };
        let magnitude = total_minutes.unsigned_abs();
        format!(
            "Invalid timezone offset: {sign}{:02}:{:02}",
            magnitude / 60,
            magnitude % 60
        )
    }
}

// ===== Time Utils Module =====

pub mod time_utils {
    //! Time conversion and calculation utilities

    use crate::{Meridiem, Timezone, WeekdayModifier, constants::SECONDS_PER_MINUTE};

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
                if hour >= 12 {
                    // 12 PM is noon; anything above 12 is not a valid 12-hour
                    // clock hour, so pass it through rather than overflowing.
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
    pub fn calculate_timezone_offset_seconds(total_minutes: i16) -> i32 {
        i32::from(total_minutes).saturating_mul(SECONDS_PER_MINUTE)
    }

    /// Check whether the date components form a real calendar date.
    #[must_use]
    pub fn is_valid_calendar_date(year: u16, month: u8, day: u8) -> bool {
        let days_in_month = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_leap_year(year) => 29,
            2 => 28,
            _ => return false,
        };

        (1..=days_in_month).contains(&day)
    }

    /// Check whether the time components form a valid 24-hour clock time.
    #[must_use]
    pub fn is_valid_24_hour_time(hour: u8, minute: u8, second: u8) -> bool {
        hour <= 23 && minute <= 59 && second <= 59
    }

    /// Check whether time components are valid for either 24-hour or AM/PM notation.
    #[must_use]
    pub fn is_valid_time(hour: u8, minute: u8, second: u8, meridiem: Option<Meridiem>) -> bool {
        match meridiem {
            Some(_) => (1..=12).contains(&hour) && minute <= 59 && second <= 59,
            None => is_valid_24_hour_time(hour, minute, second),
        }
    }

    /// Check whether a timezone offset is in the supported UTC-12:00..=UTC+14:00 range.
    #[must_use]
    pub fn is_valid_timezone_offset(offset: Timezone) -> bool {
        match offset {
            Timezone::Utc => true,
            Timezone::Offset { total_minutes } => (-720..=840).contains(&total_minutes),
        }
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
            Some(WeekdayModifier::This) => {
                // Same Monday-to-Sunday week, looking backwards if already passed
                days_diff
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

    #[must_use]
    fn is_leap_year(year: u16) -> bool {
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
    }
}

// ===== Common Parsing Module =====

/// Common parsing utilities shared across language implementations.
///
/// This module contains parser building blocks that are shared between
/// different language implementations, such as ISO datetime parsing,
/// number parsing, and the case-insensitive keyword helper.
pub mod common {
    use super::{AbsoluteTime, TimeExpression, Timezone, time_utils};
    use chumsky::{error::Rich, extra, prelude::*, text};

    /// The error type used throughout the parsers.
    pub type ParserError<'a> = extra::Err<Rich<'a, char>>;

    /// Match an ASCII keyword case-insensitively.
    ///
    /// Used for English keywords ("now", "ago", "in") and German
    /// abbreviations ("sek", "min", "uhr"). Non-ASCII characters in
    /// `target` are compared exactly. The parser is internally a chain
    /// of single-character matchers so error messages mention the
    /// keyword's first expected character; the whole branch is then
    /// labelled with `target` so callers see "expected `now`" etc.
    pub fn keyword_ci<'a>(
        target: &'static str,
    ) -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
        let mut chars = target.chars();
        let first = chars.next().expect("keyword must be non-empty");
        let mut parser: chumsky::Boxed<'a, 'a, &'a str, (), ParserError<'a>> =
            char_ci(first).ignored().boxed();
        for c in chars {
            parser = parser.then_ignore(char_ci(c)).boxed();
        }

        // Require a word boundary after keywords that end in a word character,
        // so `keyword_ci("day")` cannot match inside "days" and `keyword_ci("m")`
        // cannot match inside "min". Without this, matching is pure prefix
        // matching and every `choice` has to be hand-ordered longest-first --
        // a convention that fails silently when it is broken.
        //
        // Keywords ending in punctuation (`a.m.`) need no such check.
        if target.chars().last().is_some_and(is_word_char) {
            parser = parser.then_ignore(word_boundary()).boxed();
        }

        parser.labelled(target)
    }

    /// Characters that may not directly follow a word-like keyword.
    fn is_word_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    /// Succeeds without consuming input when the next character cannot continue
    /// a word, or at end of input.
    fn word_boundary<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
        any()
            .filter(|c: &char| !is_word_char(*c))
            .ignored()
            .rewind()
            .or(end())
    }

    /// Try every alternative from the same input position and keep whichever
    /// consumed the most.
    ///
    /// `choice` commits to the first alternative that succeeds and never
    /// re-enters it, so a shorter match shadows a longer one and a later
    /// failure cannot recover. Longest-match makes source order irrelevant, and
    /// unlike sorting a keyword list it works between alternatives of any
    /// shape, not just plain keywords.
    ///
    /// Every alternative is run, so reserve this for alternations where the
    /// shadowing risk is real rather than using it as a blanket `choice`.
    ///
    /// # Panics
    ///
    /// Panics if `parsers` is empty.
    pub fn longest<'a, T>(
        parsers: Vec<chumsky::Boxed<'a, 'a, &'a str, T, ParserError<'a>>>,
    ) -> impl Parser<'a, &'a str, T, ParserError<'a>> + Clone
    where
        T: 'a,
    {
        assert!(
            !parsers.is_empty(),
            "longest() needs at least one alternative"
        );
        let parsers = std::rc::Rc::new(parsers);

        custom(move |inp| {
            let start = inp.cursor();
            let mut best: Option<(usize, usize)> = None;
            let mut first_err: Option<Rich<'a, char>> = None;

            for (index, parser) in parsers.iter().enumerate() {
                let checkpoint = inp.save();
                match inp.parse(parser.clone()) {
                    Ok(_) => {
                        let consumed = inp.slice_since(&start..).len();
                        if best.is_none_or(|(best_len, _)| consumed > best_len) {
                            best = Some((consumed, index));
                        }
                    }
                    Err(err) => {
                        if first_err.is_none() {
                            first_err = Some(err);
                        }
                    }
                }
                // Always return to the start so the next alternative sees the
                // same input.
                inp.rewind(checkpoint);
            }

            match best {
                // Re-run the winner to actually consume its input. Parsers are
                // pure, so this yields the same value.
                Some((_, index)) => inp.parse(parsers[index].clone()),
                None => Err(first_err.expect("a non-empty alternation always produces an error")),
            }
        })
    }

    /// Case-**sensitive** keyword with the same word-boundary rule as
    /// [`keyword_ci`], for languages where capitalisation is meaningful.
    pub fn keyword_cs<'a>(
        target: &'static str,
    ) -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
        let mut parser: chumsky::Boxed<'a, 'a, &'a str, (), ParserError<'a>> =
            just(target).ignored().boxed();
        if target.chars().last().is_some_and(is_word_char) {
            parser = parser.then_ignore(word_boundary()).boxed();
        }
        parser.labelled(target)
    }

    /// Case-sensitive counterpart of [`keywords_ci`], longest keyword first.
    pub fn keywords<'a, T>(
        pairs: impl IntoIterator<Item = (&'static str, T)>,
    ) -> impl Parser<'a, &'a str, T, ParserError<'a>> + Clone
    where
        T: Clone + 'a,
    {
        let mut pairs: Vec<(&'static str, T)> = pairs.into_iter().collect();
        pairs.sort_by_key(|(kw, _)| std::cmp::Reverse(kw.chars().count()));

        let mut iter = pairs.into_iter();
        let (first_kw, first_val) = iter.next().expect("keyword set must be non-empty");
        let mut parser: chumsky::Boxed<'a, 'a, &'a str, T, ParserError<'a>> =
            keyword_cs(first_kw).to(first_val).boxed();
        for (kw, val) in iter {
            parser = parser.or(keyword_cs(kw).to(val)).boxed();
        }
        parser
    }

    /// Build a case-insensitive alternation over `(keyword, value)` pairs,
    /// trying the longest keyword first.
    ///
    /// `choice` commits to the first alternative that succeeds and never
    /// re-enters it, so a shorter keyword listed before a longer one that
    /// shares its prefix silently shadows it — `"a couple"` swallowing the
    /// start of `"a couple of"` leaves a remainder nothing can parse. Sorting
    /// here makes the source order irrelevant instead of load-bearing.
    pub fn keywords_ci<'a, T>(
        pairs: impl IntoIterator<Item = (&'static str, T)>,
    ) -> impl Parser<'a, &'a str, T, ParserError<'a>> + Clone
    where
        T: Clone + 'a,
    {
        let mut pairs: Vec<(&'static str, T)> = pairs.into_iter().collect();
        pairs.sort_by_key(|(kw, _)| std::cmp::Reverse(kw.chars().count()));

        let mut iter = pairs.into_iter();
        let (first_kw, first_val) = iter.next().expect("keyword set must be non-empty");
        let mut parser: chumsky::Boxed<'a, 'a, &'a str, T, ParserError<'a>> =
            keyword_ci(first_kw).to(first_val).boxed();
        for (kw, val) in iter {
            parser = parser.or(keyword_ci(kw).to(val)).boxed();
        }
        parser
    }

    fn char_ci<'a>(target: char) -> chumsky::Boxed<'a, 'a, &'a str, char, ParserError<'a>> {
        let lower = target.to_lowercase().next().unwrap_or(target);
        let upper = target.to_uppercase().next().unwrap_or(target);
        if lower == upper {
            just(target).boxed()
        } else {
            one_of([lower, upper]).boxed()
        }
    }

    fn ascii_digit<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
        one_of('0'..='9').labelled("digit")
    }

    /// Parse a sequence of digits as an `i64`.
    pub fn digit_number<'a>() -> impl Parser<'a, &'a str, i64, ParserError<'a>> + Clone {
        ascii_digit()
            .repeated()
            .at_least(1)
            .to_slice()
            .try_map(|s: &str, span| {
                s.parse::<i64>()
                    .map_err(|e| Rich::custom(span, e.to_string()))
            })
            .labelled("number")
    }

    /// Parse a 1 or 2 digit number as `u8`.
    pub fn two_digit_number<'a>() -> impl Parser<'a, &'a str, u8, ParserError<'a>> + Clone {
        ascii_digit()
            .repeated()
            .at_least(1)
            .at_most(2)
            .to_slice()
            .try_map(|s: &str, span| {
                s.parse::<u8>()
                    .map_err(|e| Rich::custom(span, e.to_string()))
            })
    }

    /// Parse a 4-digit number as `u16`.
    pub fn four_digit_number<'a>() -> impl Parser<'a, &'a str, u16, ParserError<'a>> + Clone {
        ascii_digit()
            .repeated()
            .exactly(4)
            .to_slice()
            .try_map(|s: &str, span| {
                s.parse::<u16>()
                    .map_err(|e| Rich::custom(span, e.to_string()))
            })
            .labelled("4-digit year")
    }

    fn offset_timezone<'a>() -> impl Parser<'a, &'a str, Timezone, ParserError<'a>> + Clone {
        one_of(['+', '-'])
            .then(two_digit_number())
            .then(just(':').ignore_then(two_digit_number()).or_not())
            .try_map(|((sign, hours), minutes), span| {
                let minutes = minutes.unwrap_or(0);
                if minutes > 59 {
                    return Err(Rich::custom(span, "timezone minute offset out of range"));
                }
                let magnitude = i16::from(hours)
                    .checked_mul(60)
                    .and_then(|h| h.checked_add(i16::from(minutes)))
                    .ok_or_else(|| Rich::custom(span, "timezone hour offset out of range"))?;
                let total_minutes = if sign == '+' { magnitude } else { -magnitude };
                let offset = Timezone::Offset { total_minutes };

                if time_utils::is_valid_timezone_offset(offset) {
                    Ok(offset)
                } else {
                    Err(Rich::custom(span, "invalid timezone offset"))
                }
            })
    }

    fn timezone<'a>() -> impl Parser<'a, &'a str, Timezone, ParserError<'a>> + Clone {
        choice((just('Z').to(Timezone::Utc), offset_timezone()))
    }

    fn fractional_seconds<'a>() -> impl Parser<'a, &'a str, u32, ParserError<'a>> + Clone {
        text::digits(10).to_slice().try_map(|s: &str, span| {
            let fraction = if s.len() > 9 { &s[..9] } else { s };
            let parsed: u32 = fraction
                .parse()
                .map_err(|e: std::num::ParseIntError| Rich::custom(span, e.to_string()))?;
            let fraction_len =
                u32::try_from(fraction.len()).expect("fraction length is capped at 9 digits");
            Ok(parsed * 10_u32.pow(9 - fraction_len))
        })
    }

    /// Parse ISO 8601 datetime format.
    ///
    /// Supports:
    /// - Date only: `2024-01-15`
    /// - Date and time: `2024-01-15T14:30:00`
    /// - With timezone: `2024-01-15T14:30:00Z`
    /// - With offset: `2024-01-15T14:30:00+02:00`
    /// - With fractional seconds: `2024-01-15T14:30:00.123Z`
    pub fn iso_datetime<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
        let date = four_digit_number()
            .then_ignore(just('-'))
            .then(two_digit_number())
            .then_ignore(just('-'))
            .then(two_digit_number())
            .try_map(|((year, month), day), span| {
                if time_utils::is_valid_calendar_date(year, month, day) {
                    Ok((year, month, day))
                } else {
                    Err(Rich::custom(span, "invalid calendar date"))
                }
            });

        let time = one_of(['T', ' '])
            .ignore_then(two_digit_number())
            .then_ignore(just(':'))
            .then(two_digit_number())
            .then(
                just(':')
                    .ignore_then(two_digit_number())
                    .then(just('.').ignore_then(fractional_seconds()).or_not())
                    .or_not(),
            )
            .then(timezone().or_not())
            .try_map(|(((hour, minute), sec_part), tz), span| {
                let second = sec_part.as_ref().map_or(0, |(s, _)| *s);
                if time_utils::is_valid_24_hour_time(hour, minute, second) {
                    Ok((hour, minute, sec_part, tz))
                } else {
                    Err(Rich::custom(span, "invalid time"))
                }
            });

        date.then(time.or_not())
            .map(|((year, month, day), time_opt)| match time_opt {
                Some((h, m, sec_part, tz)) => {
                    let (second, nanosecond) = match sec_part {
                        Some((s, frac)) => (Some(s), frac),
                        None => (None, None),
                    };
                    TimeExpression::Absolute(AbsoluteTime {
                        year,
                        month,
                        day,
                        hour: Some(h),
                        minute: Some(m),
                        second,
                        nanosecond,
                        timezone: tz,
                    })
                }
                None => TimeExpression::Absolute(AbsoluteTime {
                    year,
                    month,
                    day,
                    hour: None,
                    minute: None,
                    second: None,
                    nanosecond: None,
                    timezone: None,
                }),
            })
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
