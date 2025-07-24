//! # temps-jiff
//!
//! Jiff integration for the temps time expression parser.
//!
//! This crate provides a `JiffProvider` that implements the `TimeParser` trait
//! using the jiff datetime library. It enables parsing natural language time
//! expressions into jiff's `Zoned` type.
//!
//! ## Features
//!
//! - Full implementation of the temps `TimeParser` trait
//! - Support for all time expression types
//! - Proper handling of month/year arithmetic using jiff's `Span`
//! - Timezone support (UTC and fixed offsets)
//! - Precise time calculations with nanosecond precision
//!
//! ## Example
//!
//! ```
//! use temps_jiff::{JiffProvider, parse_to_zoned};
//! use temps_core::{Language, TimeParser};
//!
//! // Parse using the convenience function
//! let datetime = parse_to_zoned("in 5 minutes", Language::English).unwrap();
//! println!("In 5 minutes: {}", datetime);
//!
//! // Or use the provider directly
//! let provider = JiffProvider;
//! let expr = temps_core::parse("tomorrow at 3:30 pm", Language::English).unwrap();
//! let datetime = provider.parse_expression(expr).unwrap();
//! ```
//!
//! ## Month and Year Arithmetic
//!
//! This implementation uses jiff's `Span` type for date arithmetic, which
//! provides correct handling of edge cases:
//!
//! - January 31 + 1 month = February 29 (leap year) or February 28 (non-leap year)
//! - February 29, 2024 + 1 year = February 28, 2025
//!
//! ## Error Handling
//!
//! All parsing operations return `Result<Zoned, TempsError>`. Common errors include:
//!
//! - `ParseError`: Invalid input that cannot be parsed
//! - `DateCalculationError`: Date arithmetic that results in invalid dates
//! - `InvalidDate`/`InvalidTime`: Components that are out of valid ranges
//! - `BackendError`: Errors from the jiff library

use jiff::{Span, Zoned};
use temps_core::{
    DayReference, Direction, Language, Result, TempsError, TimeExpression, TimeParser, TimeUnit,
    Weekday,
    time_utils::{
        calculate_timezone_offset_seconds, calculate_weekday_offset, convert_12_to_24_hour,
    },
};

/// Jiff-based implementation of the TimeParser trait.
///
/// This provider uses jiff's `Zoned` as its datetime type, providing
/// high-precision time calculations and comprehensive timezone support.
///
/// ## Example
///
/// ```
/// use temps_jiff::JiffProvider;
/// use temps_core::{TimeParser, parse, Language};
///
/// let provider = JiffProvider;
/// let expr = parse("next Monday", Language::English).unwrap();
/// let datetime = provider.parse_expression(expr).unwrap();
/// ```
pub struct JiffProvider;

impl TimeParser for JiffProvider {
    type DateTime = Zoned;

    fn now(&self) -> Self::DateTime {
        Zoned::now()
    }

    fn parse_expression(&self, expr: TimeExpression) -> Result<Self::DateTime> {
        match expr {
            TimeExpression::Now => Ok(self.now()),
            TimeExpression::Relative(rel) => {
                let now = self.now();

                // Create a span based on the time unit
                let span = match rel.unit {
                    TimeUnit::Second => Span::new().seconds(rel.amount),
                    TimeUnit::Minute => Span::new().minutes(rel.amount),
                    TimeUnit::Hour => Span::new().hours(rel.amount),
                    TimeUnit::Day => Span::new().days(rel.amount),
                    TimeUnit::Week => Span::new().weeks(rel.amount),
                    TimeUnit::Month => Span::new().months(rel.amount),
                    TimeUnit::Year => Span::new().years(rel.amount),
                };

                // Apply the span in the correct direction
                match rel.direction {
                    Direction::Past => now.checked_sub(span).map_err(|e| {
                        TempsError::date_calculation_with_source(
                            "Date calculation error",
                            e.to_string(),
                        )
                    }),
                    Direction::Future => now.checked_add(span).map_err(|e| {
                        TempsError::date_calculation_with_source(
                            "Date calculation error",
                            e.to_string(),
                        )
                    }),
                }
            }
            TimeExpression::Absolute(abs) => {
                use jiff::civil::{Date, DateTime, Time};
                use jiff::tz::{Offset, TimeZone};

                let date = Date::new(abs.year as i16, abs.month as i8, abs.day as i8)
                    .map_err(|e| TempsError::backend_error(e.to_string(), "jiff"))?;

                if let (Some(hour), Some(minute)) = (abs.hour, abs.minute) {
                    // Validate hour is in valid range (0-23)
                    if hour > 23 {
                        return Err(TempsError::invalid_time(
                            hour,
                            minute,
                            abs.second.unwrap_or(0),
                        ));
                    }
                    // Validate minute is in valid range (0-59)
                    if minute > 59 {
                        return Err(TempsError::invalid_time(
                            hour,
                            minute,
                            abs.second.unwrap_or(0),
                        ));
                    }
                    // Validate second is in valid range (0-59)
                    if let Some(second) = abs.second {
                        if second > 59 {
                            return Err(TempsError::invalid_time(hour, minute, second));
                        }
                    }

                    let time = Time::new(
                        hour as i8,
                        minute as i8,
                        abs.second.unwrap_or(0) as i8,
                        abs.nanosecond.unwrap_or(0) as i32,
                    )
                    .map_err(|e| TempsError::backend_error(e.to_string(), "jiff"))?;

                    let datetime = DateTime::from_parts(date, time);

                    match &abs.timezone {
                        Some(temps_core::Timezone::Utc) => datetime
                            .to_zoned(TimeZone::UTC)
                            .map(|z| z.with_time_zone(TimeZone::system()))
                            .map_err(|e| {
                                TempsError::backend_error(
                                    format!("Timezone conversion error: {e}"),
                                    "jiff",
                                )
                            }),
                        Some(temps_core::Timezone::Offset { hours, minutes }) => {
                            let total_seconds = calculate_timezone_offset_seconds(*hours, *minutes);
                            let offset = Offset::from_seconds(total_seconds).map_err(|_| {
                                TempsError::invalid_timezone_offset(*hours, *minutes)
                            })?;

                            datetime
                                .to_zoned(TimeZone::fixed(offset))
                                .map(|z| z.with_time_zone(TimeZone::system()))
                                .map_err(|e| {
                                    TempsError::backend_error(
                                        format!("Timezone conversion error: {e}"),
                                        "jiff",
                                    )
                                })
                        }
                        None => {
                            // No timezone specified, treat as system timezone
                            datetime.to_zoned(TimeZone::system()).map_err(|e| {
                                TempsError::backend_error(
                                    format!("Timezone conversion error: {e}"),
                                    "jiff",
                                )
                            })
                        }
                    }
                } else {
                    // Date only, set time to midnight
                    let datetime = date.at(0, 0, 0, 0);
                    datetime.to_zoned(TimeZone::system()).map_err(|e| {
                        TempsError::backend_error(format!("Timezone conversion error: {e}"), "jiff")
                    })
                }
            }
            TimeExpression::Day(day_ref) => {
                let now = self.now();
                match day_ref {
                    DayReference::Today => {
                        let date = now.date();
                        date.at(0, 0, 0, 0)
                            .to_zoned(now.time_zone().clone())
                            .map_err(|e| {
                                TempsError::date_calculation_with_source(
                                    "Failed to create today's date",
                                    e.to_string(),
                                )
                            })
                    }
                    DayReference::Yesterday => {
                        let yesterday = now.checked_sub(Span::new().days(1)).map_err(|e| {
                            TempsError::date_calculation_with_source(
                                "Failed to calculate yesterday",
                                e.to_string(),
                            )
                        })?;
                        let date = yesterday.date();
                        date.at(0, 0, 0, 0)
                            .to_zoned(now.time_zone().clone())
                            .map_err(|e| {
                                TempsError::date_calculation_with_source(
                                    "Failed to create yesterday's date",
                                    e.to_string(),
                                )
                            })
                    }
                    DayReference::Tomorrow => {
                        let tomorrow = now.checked_add(Span::new().days(1)).map_err(|e| {
                            TempsError::date_calculation_with_source(
                                "Failed to calculate tomorrow",
                                e.to_string(),
                            )
                        })?;
                        let date = tomorrow.date();
                        date.at(0, 0, 0, 0)
                            .to_zoned(now.time_zone().clone())
                            .map_err(|e| {
                                TempsError::date_calculation_with_source(
                                    "Failed to create tomorrow's date",
                                    e.to_string(),
                                )
                            })
                    }
                    DayReference::Weekday { day, modifier } => {
                        let target_weekday = match day {
                            Weekday::Monday => jiff::civil::Weekday::Monday,
                            Weekday::Tuesday => jiff::civil::Weekday::Tuesday,
                            Weekday::Wednesday => jiff::civil::Weekday::Wednesday,
                            Weekday::Thursday => jiff::civil::Weekday::Thursday,
                            Weekday::Friday => jiff::civil::Weekday::Friday,
                            Weekday::Saturday => jiff::civil::Weekday::Saturday,
                            Weekday::Sunday => jiff::civil::Weekday::Sunday,
                        };

                        let current_weekday = now.weekday();
                        let current_offset = current_weekday.to_monday_zero_offset() as i64;
                        let target_offset = target_weekday.to_monday_zero_offset() as i64;

                        let days_to_add =
                            calculate_weekday_offset(current_offset, target_offset, modifier);
                        let target_date = now.checked_add(Span::new().days(days_to_add));

                        let target = target_date.map_err(|e| {
                            TempsError::date_calculation_with_source(
                                "Failed to calculate weekday",
                                e.to_string(),
                            )
                        })?;
                        let date = target.date();
                        date.at(0, 0, 0, 0)
                            .to_zoned(now.time_zone().clone())
                            .map_err(|e| {
                                TempsError::date_calculation_with_source(
                                    "Failed to create weekday date",
                                    e.to_string(),
                                )
                            })
                    }
                }
            }
            TimeExpression::Time(time) => {
                let now = self.now();
                let date = now.date();
                let hour = convert_12_to_24_hour(time.hour, time.meridiem.as_ref());

                // Validate time components
                if hour > 23 || time.minute > 59 || time.second > 59 {
                    return Err(TempsError::invalid_time(hour, time.minute, time.second));
                }

                date.at(hour as i8, time.minute as i8, time.second as i8, 0)
                    .to_zoned(now.time_zone().clone())
                    .map_err(|e| {
                        TempsError::backend_error(format!("Failed to create time: {e}"), "jiff")
                    })
            }
            TimeExpression::DayTime(day_time) => {
                // First get the day
                let day_result =
                    self.parse_expression(TimeExpression::Day(day_time.day.clone()))?;
                let date = day_result.date();

                let hour =
                    convert_12_to_24_hour(day_time.time.hour, day_time.time.meridiem.as_ref());

                // Validate time components
                if hour > 23 || day_time.time.minute > 59 || day_time.time.second > 59 {
                    return Err(TempsError::invalid_time(
                        hour,
                        day_time.time.minute,
                        day_time.time.second,
                    ));
                }

                date.at(
                    hour as i8,
                    day_time.time.minute as i8,
                    day_time.time.second as i8,
                    0,
                )
                .to_zoned(day_result.time_zone().clone())
                .map_err(|e| {
                    TempsError::backend_error(format!("Failed to create day time: {e}"), "jiff")
                })
            }
            TimeExpression::Date(date) => {
                use jiff::civil::Date;

                let jiff_date = Date::new(date.year as i16, date.month as i8, date.day as i8)
                    .map_err(|_| TempsError::invalid_date(date.year, date.month, date.day))?;

                jiff_date
                    .at(0, 0, 0, 0)
                    .to_zoned(jiff::tz::TimeZone::system())
                    .map_err(|e| {
                        TempsError::backend_error(format!("Failed to create date: {e}"), "jiff")
                    })
            }
        }
    }
}

/// Parse a natural language time expression into a jiff `Zoned` datetime.
///
/// This is a convenience function that combines parsing and time calculation
/// in a single call.
///
/// # Arguments
///
/// * `input` - The natural language time expression to parse
/// * `language` - The language to use for parsing
///
/// # Returns
///
/// Returns `Ok(Zoned)` if parsing succeeds, or `Err(TempsError)`
/// if the input cannot be parsed or the date calculation fails.
///
/// # Examples
///
/// ```
/// use temps_jiff::parse_to_zoned;
/// use temps_core::Language;
///
/// // Parse English expressions
/// let dt = parse_to_zoned("in 30 minutes", Language::English).unwrap();
/// let dt = parse_to_zoned("tomorrow at 12:00", Language::English).unwrap();
/// let dt = parse_to_zoned("last Monday", Language::English).unwrap();
///
/// // Parse German expressions  
/// let dt = parse_to_zoned("in 30 Minuten", Language::German).unwrap();
/// let dt = parse_to_zoned("morgen um 15:30", Language::German).unwrap();
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - The input cannot be parsed as a valid time expression
/// - Date calculation results in an invalid date
/// - Components are out of valid ranges (e.g., month 13)
/// - The jiff library returns an error during calculations
pub fn parse_to_zoned(input: &str, language: Language) -> Result<Zoned> {
    let expr = temps_core::parse(input, language)?;
    JiffProvider.parse_expression(expr)
}
