//! # temps-chrono
//!
//! Chrono integration for the temps time expression parser.
//!
//! This crate provides a `ChronoProvider` that implements the `TimeParser` trait
//! using the chrono datetime library. It enables parsing natural language time
//! expressions into chrono's `DateTime<Local>` type.
//!
//! ## Features
//!
//! - Full implementation of the temps `TimeParser` trait
//! - Support for all time expression types
//! - Proper handling of month/year arithmetic
//! - Timezone support (UTC and fixed offsets)
//! - DST-aware local time handling
//!
//! ## Example
//!
//! ```
//! use temps_chrono::{ChronoProvider, parse_to_datetime};
//! use temps_core::{Language, TimeParser};
//!
//! // Parse using the convenience function
//! let datetime = parse_to_datetime("in 5 minutes", Language::English).unwrap();
//! println!("In 5 minutes: {}", datetime);
//!
//! // Or use the provider directly
//! let provider = ChronoProvider;
//! let expr = temps_core::parse("tomorrow at 3:30 pm", Language::English).unwrap();
//! let datetime = provider.parse_expression(expr).unwrap();
//! ```
//!
//! ## Month and Year Arithmetic
//!
//! This implementation uses chrono's `checked_add_months` and `checked_sub_months`
//! for proper month/year arithmetic. This handles edge cases correctly:
//!
//! - January 31 + 1 month = February 29 (leap year) or February 28 (non-leap year)
//! - February 29, 2024 + 1 year = February 28, 2025
//!
//! ## Error Handling
//!
//! All parsing operations return `Result<DateTime<Local>, TempsError>`. Common errors include:
//!
//! - `ParseError`: Invalid input that cannot be parsed
//! - `DateCalculationError`: Date arithmetic that results in invalid dates
//! - `AmbiguousTime`: Local times that are ambiguous due to DST transitions
//! - `InvalidDate`/`InvalidTime`: Components that are out of valid ranges

use chrono::{DateTime, Datelike, Duration, Local, Months};
use temps_core::{
    DayReference, Direction, Language, Result, TempsError, TimeExpression, TimeParser, TimeUnit,
    Weekday,
    constants::MONTHS_PER_YEAR,
    time_utils::{
        calculate_timezone_offset_seconds, calculate_weekday_offset, convert_12_to_24_hour,
    },
};

/// Chrono-based implementation of the TimeParser trait.
///
/// This provider uses chrono's `DateTime<Local>` as its datetime type,
/// providing full support for timezones, DST, and proper date arithmetic.
///
/// ## Example
///
/// ```
/// use temps_chrono::ChronoProvider;
/// use temps_core::{TimeParser, parse, Language};
///
/// let provider = ChronoProvider;
/// let expr = parse("next Monday", Language::English).unwrap();
/// let datetime = provider.parse_expression(expr).unwrap();
/// ```
pub struct ChronoProvider;

impl TimeParser for ChronoProvider {
    type DateTime = DateTime<Local>;

    fn now(&self) -> Self::DateTime {
        Local::now()
    }

    fn parse_expression(&self, expr: TimeExpression) -> Result<Self::DateTime> {
        match expr {
            TimeExpression::Now => Ok(self.now()),
            TimeExpression::Relative(rel) => {
                let now = self.now();

                // Handle months and years separately for proper date arithmetic
                match rel.unit {
                    TimeUnit::Month => {
                        let months = Months::new(rel.amount.try_into().map_err(|_| {
                            TempsError::date_calculation("Month amount must be a positive number")
                        })?);

                        match rel.direction {
                            Direction::Past => now.checked_sub_months(months).ok_or_else(|| {
                                TempsError::date_calculation(
                                    "Date calculation resulted in invalid date",
                                )
                            }),
                            Direction::Future => now.checked_add_months(months).ok_or_else(|| {
                                TempsError::date_calculation(
                                    "Date calculation resulted in invalid date",
                                )
                            }),
                        }
                    }
                    TimeUnit::Year => {
                        // Convert years to months for proper arithmetic
                        let months_count = rel
                            .amount
                            .checked_mul(MONTHS_PER_YEAR as i64)
                            .ok_or_else(|| {
                                TempsError::arithmetic_overflow("Year calculation overflow")
                            })?;
                        let months = Months::new(months_count.try_into().map_err(|_| {
                            TempsError::date_calculation("Year amount must be a positive number")
                        })?);

                        match rel.direction {
                            Direction::Past => now.checked_sub_months(months).ok_or_else(|| {
                                TempsError::date_calculation(
                                    "Date calculation resulted in invalid date",
                                )
                            }),
                            Direction::Future => now.checked_add_months(months).ok_or_else(|| {
                                TempsError::date_calculation(
                                    "Date calculation resulted in invalid date",
                                )
                            }),
                        }
                    }
                    _ => {
                        // Use Duration for time units that have fixed lengths
                        let duration = match rel.unit {
                            TimeUnit::Second => Duration::seconds(rel.amount),
                            TimeUnit::Minute => Duration::minutes(rel.amount),
                            TimeUnit::Hour => Duration::hours(rel.amount),
                            TimeUnit::Day => Duration::days(rel.amount),
                            TimeUnit::Week => Duration::weeks(rel.amount),
                            _ => unreachable!(), // Month and Year handled above
                        };

                        match rel.direction {
                            Direction::Past => Ok(now - duration),
                            Direction::Future => Ok(now + duration),
                        }
                    }
                }
            }
            TimeExpression::Absolute(abs) => {
                use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

                let date =
                    NaiveDate::from_ymd_opt(abs.year as i32, abs.month as u32, abs.day as u32)
                        .ok_or_else(|| TempsError::invalid_date(abs.year, abs.month, abs.day))?;

                let datetime = if let (Some(hour), Some(minute)) = (abs.hour, abs.minute) {
                    let time = NaiveTime::from_hms_nano_opt(
                        hour as u32,
                        minute as u32,
                        abs.second.unwrap_or(0) as u32,
                        abs.nanosecond.unwrap_or(0),
                    )
                    .ok_or_else(|| {
                        TempsError::invalid_time(hour, minute, abs.second.unwrap_or(0))
                    })?;

                    let naive_dt = NaiveDateTime::new(date, time);

                    match &abs.timezone {
                        Some(temps_core::Timezone::Utc) => {
                            Utc.from_utc_datetime(&naive_dt).with_timezone(&Local)
                        }
                        Some(temps_core::Timezone::Offset { hours, minutes }) => {
                            let offset_seconds =
                                calculate_timezone_offset_seconds(*hours, *minutes);
                            let offset =
                                FixedOffset::east_opt(offset_seconds).ok_or_else(|| {
                                    TempsError::invalid_timezone_offset(*hours, *minutes)
                                })?;
                            offset
                                .from_local_datetime(&naive_dt)
                                .single()
                                .ok_or_else(|| {
                                    TempsError::ambiguous_time("Ambiguous or invalid local time")
                                })?
                                .with_timezone(&Local)
                        }
                        None => {
                            // No timezone specified, treat as local time
                            Local
                                .from_local_datetime(&naive_dt)
                                .single()
                                .ok_or_else(|| {
                                    TempsError::ambiguous_time("Ambiguous or invalid local time")
                                })?
                        }
                    }
                } else {
                    // Date only, set time to midnight
                    let midnight = date.and_hms_opt(0, 0, 0).ok_or_else(|| {
                        TempsError::date_calculation("Failed to create midnight time")
                    })?;
                    Local
                        .from_local_datetime(&midnight)
                        .single()
                        .ok_or_else(|| {
                            TempsError::ambiguous_time("Ambiguous or invalid local time")
                        })?
                };

                Ok(datetime)
            }
            TimeExpression::Day(day_ref) => {
                let now = self.now();
                match day_ref {
                    DayReference::Today => {
                        let midnight = now.date_naive().and_hms_opt(0, 0, 0).ok_or_else(|| {
                            TempsError::date_calculation("Failed to create midnight time")
                        })?;
                        midnight.and_local_timezone(Local).single().ok_or_else(|| {
                            TempsError::ambiguous_time("Ambiguous or invalid local time")
                        })
                    }
                    DayReference::Yesterday => {
                        let yesterday = now - Duration::days(1);
                        let midnight =
                            yesterday.date_naive().and_hms_opt(0, 0, 0).ok_or_else(|| {
                                TempsError::date_calculation("Failed to create midnight time")
                            })?;
                        midnight.and_local_timezone(Local).single().ok_or_else(|| {
                            TempsError::ambiguous_time("Ambiguous or invalid local time")
                        })
                    }
                    DayReference::Tomorrow => {
                        let tomorrow = now + Duration::days(1);
                        let midnight =
                            tomorrow.date_naive().and_hms_opt(0, 0, 0).ok_or_else(|| {
                                TempsError::date_calculation("Failed to create midnight time")
                            })?;
                        midnight.and_local_timezone(Local).single().ok_or_else(|| {
                            TempsError::ambiguous_time("Ambiguous or invalid local time")
                        })
                    }
                    DayReference::Weekday { day, modifier } => {
                        let target_weekday = match day {
                            Weekday::Monday => chrono::Weekday::Mon,
                            Weekday::Tuesday => chrono::Weekday::Tue,
                            Weekday::Wednesday => chrono::Weekday::Wed,
                            Weekday::Thursday => chrono::Weekday::Thu,
                            Weekday::Friday => chrono::Weekday::Fri,
                            Weekday::Saturday => chrono::Weekday::Sat,
                            Weekday::Sunday => chrono::Weekday::Sun,
                        };

                        let current_weekday = now.weekday();
                        let current_offset = current_weekday.num_days_from_monday() as i64;
                        let target_offset = target_weekday.num_days_from_monday() as i64;

                        let days_to_add =
                            calculate_weekday_offset(current_offset, target_offset, modifier);
                        let target_date = now + Duration::days(days_to_add);

                        let midnight =
                            target_date
                                .date_naive()
                                .and_hms_opt(0, 0, 0)
                                .ok_or_else(|| {
                                    TempsError::date_calculation("Failed to create midnight time")
                                })?;
                        midnight.and_local_timezone(Local).single().ok_or_else(|| {
                            TempsError::ambiguous_time("Ambiguous or invalid local time")
                        })
                    }
                }
            }
            TimeExpression::Time(time) => {
                let now = self.now();
                let hour = convert_12_to_24_hour(time.hour, time.meridiem.as_ref()) as u32;

                Ok(now
                    .date_naive()
                    .and_hms_opt(hour, time.minute as u32, time.second as u32)
                    .ok_or_else(|| TempsError::invalid_time(time.hour, time.minute, time.second))?
                    .and_local_timezone(Local)
                    .single()
                    .ok_or_else(|| TempsError::ambiguous_time("Ambiguous local time"))?)
            }
            TimeExpression::DayTime(day_time) => {
                // First get the day
                let day_result =
                    self.parse_expression(TimeExpression::Day(day_time.day.clone()))?;
                let date = day_result.date_naive();

                let hour =
                    convert_12_to_24_hour(day_time.time.hour, day_time.time.meridiem.as_ref())
                        as u32;

                Ok(date
                    .and_hms_opt(
                        hour,
                        day_time.time.minute as u32,
                        day_time.time.second as u32,
                    )
                    .ok_or_else(|| {
                        TempsError::invalid_time(
                            day_time.time.hour,
                            day_time.time.minute,
                            day_time.time.second,
                        )
                    })?
                    .and_local_timezone(Local)
                    .single()
                    .ok_or_else(|| TempsError::ambiguous_time("Ambiguous local time"))?)
            }
            TimeExpression::Date(date) => {
                use chrono::NaiveDate;

                NaiveDate::from_ymd_opt(date.year as i32, date.month as u32, date.day as u32)
                    .ok_or_else(|| TempsError::invalid_date(date.year, date.month, date.day))?
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| TempsError::date_calculation("Failed to create midnight time"))?
                    .and_local_timezone(Local)
                    .single()
                    .ok_or_else(|| TempsError::ambiguous_time("Ambiguous local time"))
            }
        }
    }
}

/// Parse a natural language time expression into a chrono `DateTime<Local>`.
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
/// Returns `Ok(DateTime<Local>)` if parsing succeeds, or `Err(TempsError)`
/// if the input cannot be parsed or the date calculation fails.
///
/// # Examples
///
/// ```
/// use temps_chrono::parse_to_datetime;
/// use temps_core::Language;
///
/// // Parse English expressions
/// let dt = parse_to_datetime("in 30 minutes", Language::English).unwrap();
/// let dt = parse_to_datetime("tomorrow at 12:00", Language::English).unwrap();
/// let dt = parse_to_datetime("last Monday", Language::English).unwrap();
///
/// // Parse German expressions  
/// let dt = parse_to_datetime("in 30 Minuten", Language::German).unwrap();
/// let dt = parse_to_datetime("morgen um 15:30", Language::German).unwrap();
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - The input cannot be parsed as a valid time expression
/// - Date calculation results in an invalid date
/// - The resulting time is ambiguous due to DST transitions
pub fn parse_to_datetime(input: &str, language: Language) -> Result<DateTime<Local>> {
    let expr = temps_core::parse(input, language)?;
    ChronoProvider.parse_expression(expr)
}
