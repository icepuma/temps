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

use chrono::{
    DateTime, Datelike, Days, Duration, Local, Months, NaiveDateTime, TimeDelta, TimeZone, Utc,
};

/// Resolve a naive local datetime to a concrete instant, matching the jiff
/// backend's default `compatible` disambiguation.
///
/// Two cases need care:
///
/// * An **ambiguous** local time (a DST fall-back fold) maps to two instants.
///   chrono's `LocalResult::Ambiguous` is not ordered by instant — `.earliest()`
///   can hand back the *later* one — so choose explicitly by comparison.
/// * A **nonexistent** local time (a spring-forward gap) maps to none. jiff
///   shifts such a time forward by the gap's own width; interpreting the civil
///   time with the offset in force *before* the gap does exactly that, and works
///   for any width — including whole days skipped at the date line, where a
///   fixed-size probe would give up.
fn resolve_local(naive: NaiveDateTime) -> Option<DateTime<Local>> {
    use chrono::offset::LocalResult;
    match naive.and_local_timezone(Local) {
        LocalResult::Single(dt) => {
            Some(Utc.from_utc_datetime(&dt.naive_utc()).with_timezone(&Local))
        }
        LocalResult::Ambiguous(a, b) => Some(if a <= b { a } else { b }),
        LocalResult::None => {
            let pre_gap_offset = (1..=3).find_map(|days| {
                naive
                    .checked_sub_days(Days::new(days))?
                    .and_local_timezone(Local)
                    .earliest()
                    .map(|dt| *dt.offset())
            })?;
            let utc = naive.checked_sub_offset(pre_gap_offset)?;
            Some(Utc.from_utc_datetime(&utc).with_timezone(&Local))
        }
    }
}
use temps_core::{
    DayReference, Direction, Language, Result, TempsError, TimeExpression, TimeParser, TimeUnit,
    Weekday,
    constants::{DAYS_PER_WEEK, MONTHS_PER_YEAR},
    errors::*,
    time_utils::{
        calculate_timezone_offset_seconds, calculate_weekday_offset, convert_12_to_24_hour,
        is_valid_time, is_valid_timezone_offset,
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
                if rel.amount < 0 {
                    return Err(TempsError::date_calculation(
                        ERR_RELATIVE_AMOUNT_NON_NEGATIVE,
                    ));
                }

                let now = self.now();

                if rel.amount == 0 {
                    return Ok(now);
                }

                // Handle months and years separately for proper date arithmetic
                match rel.unit {
                    TimeUnit::Month => {
                        let months = Months::new(rel.amount.try_into().map_err(|_| {
                            TempsError::arithmetic_overflow(ERR_AMOUNT_OUT_OF_RANGE)
                        })?);

                        match rel.direction {
                            Direction::Past => now
                                .checked_sub_months(months)
                                .ok_or_else(|| TempsError::date_calculation(ERR_DATE_CALC_INVALID)),
                            Direction::Future => now
                                .checked_add_months(months)
                                .ok_or_else(|| TempsError::date_calculation(ERR_DATE_CALC_INVALID)),
                        }
                    }
                    TimeUnit::Year => {
                        // Convert years to months for proper arithmetic
                        let months_count = rel
                            .amount
                            .checked_mul(MONTHS_PER_YEAR as i64)
                            .ok_or_else(|| TempsError::arithmetic_overflow(ERR_YEAR_OVERFLOW))?;
                        let months = Months::new(months_count.try_into().map_err(|_| {
                            TempsError::arithmetic_overflow(ERR_AMOUNT_OUT_OF_RANGE)
                        })?);

                        match rel.direction {
                            Direction::Past => now
                                .checked_sub_months(months)
                                .ok_or_else(|| TempsError::date_calculation(ERR_DATE_CALC_INVALID)),
                            Direction::Future => now
                                .checked_add_months(months)
                                .ok_or_else(|| TempsError::date_calculation(ERR_DATE_CALC_INVALID)),
                        }
                    }
                    TimeUnit::Day | TimeUnit::Week => {
                        // Calendar-aware, matching the jiff backend: "in 3 days"
                        // keeps the wall-clock time across a DST transition.
                        let days = if matches!(rel.unit, TimeUnit::Week) {
                            rel.amount.checked_mul(i64::from(DAYS_PER_WEEK))
                        } else {
                            Some(rel.amount)
                        }
                        .and_then(|d| u64::try_from(d).ok())
                        .ok_or_else(|| TempsError::arithmetic_overflow(ERR_AMOUNT_OUT_OF_RANGE))?;

                        let date = match rel.direction {
                            Direction::Past => now.date_naive().checked_sub_days(Days::new(days)),
                            Direction::Future => now.date_naive().checked_add_days(Days::new(days)),
                        }
                        .ok_or_else(|| TempsError::arithmetic_overflow(ERR_AMOUNT_OUT_OF_RANGE))?;

                        let naive = date.and_time(now.time());
                        resolve_local(naive)
                            .ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))
                    }
                    _ => {
                        // Fixed-length units. Use the fallible constructors and a
                        // checked add: a large parsed amount must be an error, not
                        // a panic.
                        let duration = match rel.unit {
                            TimeUnit::Second => TimeDelta::try_seconds(rel.amount),
                            TimeUnit::Minute => TimeDelta::try_minutes(rel.amount),
                            TimeUnit::Hour => TimeDelta::try_hours(rel.amount),
                            _ => unreachable!(), // Day/Week/Month/Year handled above
                        }
                        .ok_or_else(|| TempsError::arithmetic_overflow(ERR_AMOUNT_OUT_OF_RANGE))?;

                        let shifted = match rel.direction {
                            Direction::Past => now.checked_sub_signed(duration),
                            Direction::Future => now.checked_add_signed(duration),
                        };
                        shifted
                            .ok_or_else(|| TempsError::arithmetic_overflow(ERR_AMOUNT_OUT_OF_RANGE))
                    }
                }
            }
            TimeExpression::Absolute(abs) => {
                use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

                let date =
                    NaiveDate::from_ymd_opt(abs.year as i32, abs.month as u32, abs.day as u32)
                        .ok_or_else(|| TempsError::invalid_date(abs.year, abs.month, abs.day))?;

                if abs.hour.is_none() && abs.minute.is_some() {
                    // A minute without an hour is not a time we can honour; say so
                    // rather than silently falling through to midnight.
                    return Err(TempsError::invalid_time(
                        0,
                        abs.minute.unwrap_or(0),
                        abs.second.unwrap_or(0),
                    ));
                }

                let datetime = if let Some(hour) = abs.hour {
                    // Default only the components below the one supplied.
                    let minute = abs.minute.unwrap_or(0);
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
                        Some(temps_core::Timezone::Offset { total_minutes }) => {
                            if !is_valid_timezone_offset(temps_core::Timezone::Offset {
                                total_minutes: *total_minutes,
                            }) {
                                return Err(TempsError::invalid_timezone_offset(*total_minutes));
                            }

                            let offset_seconds = calculate_timezone_offset_seconds(*total_minutes);
                            let offset =
                                FixedOffset::east_opt(offset_seconds).ok_or_else(|| {
                                    TempsError::invalid_timezone_offset(*total_minutes)
                                })?;
                            offset
                                .from_local_datetime(&naive_dt)
                                .single()
                                .ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))?
                                .with_timezone(&Local)
                        }
                        None => {
                            // No timezone specified, treat as local time
                            resolve_local(naive_dt)
                                .ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))?
                        }
                    }
                } else {
                    // Date only, set time to midnight
                    let midnight = date
                        .and_hms_opt(0, 0, 0)
                        .ok_or_else(|| TempsError::date_calculation(ERR_MIDNIGHT_FAILED))?;
                    resolve_local(midnight)
                        .ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))?
                };

                Ok(datetime)
            }
            TimeExpression::Day(day_ref) => {
                let now = self.now();
                match day_ref {
                    DayReference::Today => {
                        let midnight = now
                            .date_naive()
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| TempsError::date_calculation(ERR_MIDNIGHT_FAILED))?;
                        resolve_local(midnight)
                            .ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))
                    }
                    DayReference::Yesterday => {
                        let midnight = now
                            .date_naive()
                            .checked_sub_days(Days::new(1))
                            .ok_or_else(|| TempsError::date_calculation(ERR_DATE_CALC_INVALID))?
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| TempsError::date_calculation(ERR_MIDNIGHT_FAILED))?;
                        resolve_local(midnight)
                            .ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))
                    }
                    DayReference::Tomorrow => {
                        let midnight = now
                            .date_naive()
                            .checked_add_days(Days::new(1))
                            .ok_or_else(|| TempsError::date_calculation(ERR_DATE_CALC_INVALID))?
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| TempsError::date_calculation(ERR_MIDNIGHT_FAILED))?;
                        resolve_local(midnight)
                            .ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))
                    }
                    DayReference::DayBeforeYesterday => {
                        let midnight = now
                            .date_naive()
                            .checked_sub_days(Days::new(2))
                            .ok_or_else(|| TempsError::date_calculation(ERR_DATE_CALC_INVALID))?
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| TempsError::date_calculation(ERR_MIDNIGHT_FAILED))?;
                        resolve_local(midnight)
                            .ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))
                    }
                    DayReference::DayAfterTomorrow => {
                        let midnight = now
                            .date_naive()
                            .checked_add_days(Days::new(2))
                            .ok_or_else(|| TempsError::date_calculation(ERR_DATE_CALC_INVALID))?
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| TempsError::date_calculation(ERR_MIDNIGHT_FAILED))?;
                        resolve_local(midnight)
                            .ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))
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
                        let base = now.date_naive();
                        let target_date = if days_to_add >= 0 {
                            base.checked_add_days(Days::new(days_to_add.unsigned_abs()))
                        } else {
                            base.checked_sub_days(Days::new(days_to_add.unsigned_abs()))
                        }
                        .ok_or_else(|| TempsError::date_calculation(ERR_DATE_CALC_INVALID))?;

                        let midnight = target_date
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| TempsError::date_calculation(ERR_MIDNIGHT_FAILED))?;
                        resolve_local(midnight)
                            .ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))
                    }
                }
            }
            TimeExpression::Time(time) => {
                let now = self.now();
                if !is_valid_time(time.hour, time.minute, time.second, time.meridiem) {
                    return Err(TempsError::invalid_time(
                        time.hour,
                        time.minute,
                        time.second,
                    ));
                }

                let hour = convert_12_to_24_hour(time.hour, time.meridiem.as_ref()) as u32;

                let naive = now
                    .date_naive()
                    .and_hms_opt(hour, time.minute as u32, time.second as u32)
                    .ok_or_else(|| TempsError::invalid_time(time.hour, time.minute, time.second))?;
                Ok(resolve_local(naive)
                    .ok_or_else(|| TempsError::ambiguous_time("Ambiguous local time"))?)
            }
            TimeExpression::DayTime(day_time) => {
                // First get the day
                let day_result = self.parse_expression(TimeExpression::Day(day_time.day))?;
                let date = day_result.date_naive();

                if !is_valid_time(
                    day_time.time.hour,
                    day_time.time.minute,
                    day_time.time.second,
                    day_time.time.meridiem,
                ) {
                    return Err(TempsError::invalid_time(
                        day_time.time.hour,
                        day_time.time.minute,
                        day_time.time.second,
                    ));
                }

                let hour =
                    convert_12_to_24_hour(day_time.time.hour, day_time.time.meridiem.as_ref())
                        as u32;

                let naive = date
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
                    })?;
                Ok(resolve_local(naive)
                    .ok_or_else(|| TempsError::ambiguous_time("Ambiguous local time"))?)
            }
            TimeExpression::LaterToday => {
                let now = self.now();
                let later = now
                    .checked_add_signed(Duration::hours(2))
                    .ok_or_else(|| TempsError::arithmetic_overflow(ERR_AMOUNT_OUT_OF_RANGE))?;

                // Clamp against the true end of the local day. A fixed 23:59:59
                // is wrong in zones where the day is cut short by a transition,
                // and would drop sub-second precision.
                let tomorrow = now
                    .date_naive()
                    .checked_add_days(Days::new(1))
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .ok_or_else(|| TempsError::date_calculation(ERR_DATE_CALC_INVALID))?;
                let tomorrow_start = resolve_local(tomorrow)
                    .ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))?;

                if later < tomorrow_start {
                    return Ok(later);
                }
                let last_today = tomorrow_start
                    .checked_sub_signed(TimeDelta::nanoseconds(1))
                    .ok_or_else(|| TempsError::date_calculation(ERR_DATE_CALC_INVALID))?;
                // Never resolve into the past.
                Ok(if last_today < now { now } else { last_today })
            }
            TimeExpression::Date(date) => {
                use chrono::NaiveDate;

                NaiveDate::from_ymd_opt(date.year as i32, date.month as u32, date.day as u32)
                    .ok_or_else(|| TempsError::invalid_date(date.year, date.month, date.day))?
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| TempsError::date_calculation(ERR_MIDNIGHT_FAILED))
                    .and_then(|naive| {
                        resolve_local(naive)
                            .ok_or_else(|| TempsError::ambiguous_time("Ambiguous local time"))
                    })
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
