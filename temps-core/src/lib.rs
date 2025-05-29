use winnow::{
    ascii::digit1,
    combinator::{alt, opt},
    error::ContextError,
    prelude::*,
    token::{one_of, take_while},
};

// ===== Core Types =====

#[derive(Debug, PartialEq, Clone)]
pub enum TimeExpression {
    Now,
    Relative(RelativeTime),
    Absolute(AbsoluteTime),
    Day(DayReference),
    Time(Time),
    Date(StandardDate),
    DayTime(DayTime),
}

#[derive(Debug, PartialEq, Clone)]
pub struct RelativeTime {
    pub amount: i64,
    pub unit: TimeUnit,
    pub direction: Direction,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AbsoluteTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    pub second: Option<u8>,
    pub nanosecond: Option<u32>,
    pub timezone: Option<Timezone>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Timezone {
    Utc,
    Offset { hours: i8, minutes: u8 },
}

#[derive(Debug, PartialEq, Clone)]
pub enum DayReference {
    Today,
    Yesterday,
    Tomorrow,
    Weekday {
        day: Weekday,
        modifier: Option<WeekdayModifier>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub meridiem: Option<Meridiem>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct StandardDate {
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

#[derive(Debug, PartialEq, Clone)]
pub struct DayTime {
    pub day: DayReference,
    pub time: Time,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TimeUnit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Direction {
    Past,
    Future,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum WeekdayModifier {
    Last,
    Next,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Meridiem {
    AM,
    PM,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    English,
    German,
}

// ===== Traits =====

pub trait TimeParser {
    type DateTime;
    type Error;

    fn now(&self) -> Self::DateTime;
    fn parse_expression(&self, expr: TimeExpression) -> Result<Self::DateTime, Self::Error>;
}

pub trait LanguageParser {
    fn parse<'a>(
        &self,
        input: &'a str,
    ) -> Result<TimeExpression, winnow::error::ParseError<&'a str, ContextError>>;
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
    pub fn format_invalid_date(year: u16, month: u8, day: u8) -> String {
        format!("Invalid date: {}-{}-{}", year, month, day)
    }

    /// Format error message for invalid time with components
    pub fn format_invalid_time(hour: u8, minute: u8, second: u8) -> String {
        format!("Invalid time: {}:{}:{}", hour, minute, second)
    }

    /// Format error message for invalid timezone offset
    pub fn format_invalid_timezone_offset(hours: i8, minutes: u8) -> String {
        format!("Invalid timezone offset: {}:{}", hours, minutes)
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

pub mod common {
    use super::*;

    /// Parse digits as i64
    pub fn parse_digit_number(input: &mut &str) -> winnow::Result<i64> {
        digit1.try_map(|s: &str| s.parse::<i64>()).parse_next(input)
    }

    /// Parse ISO datetime format that's common across languages
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

    /// Parse timezone (Z or offset)
    fn parse_timezone(input: &mut &str) -> winnow::Result<Timezone> {
        alt(("Z".map(|_| Timezone::Utc), parse_offset_timezone)).parse_next(input)
    }

    /// Parse timezone offset (+/-HH:MM)
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

    /// Parse two digit number as u8
    pub fn parse_two_digit_number(input: &mut &str) -> winnow::Result<u8> {
        take_while(1..=2, |c: char| c.is_ascii_digit())
            .try_map(|s: &str| s.parse::<u8>())
            .parse_next(input)
    }

    /// Parse four digit number as u16
    pub fn parse_four_digit_number(input: &mut &str) -> winnow::Result<u16> {
        take_while(4..=4, |c: char| c.is_ascii_digit())
            .try_map(|s: &str| s.parse::<u16>())
            .parse_next(input)
    }
}

// ===== Language Support =====

pub mod language {
    pub mod english;
    pub mod german;
}

// ===== Main Parsing Function =====

pub fn parse(
    input: &str,
    language: Language,
) -> Result<TimeExpression, winnow::error::ParseError<&str, winnow::error::ContextError>> {
    match language {
        Language::English => language::english::EnglishParser.parse(input),
        Language::German => language::german::GermanParser.parse(input),
    }
}
