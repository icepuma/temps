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
//! let provider = ChronoProvider::new();
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
//! - `ArithmeticOverflow`: Any result outside the range chrono can represent,
//!   about 262,000 years either side of year 0, whatever produced it: a
//!   relative amount of any unit (`in 300000 years`, `in 100000000 days`), a
//!   day reference past chrono's first or last date (`tomorrow` on the last
//!   one), or a time of day that is out of range on one of those dates. A
//!   provider pinned with [`ChronoProvider::at`] to an instant whose own local
//!   reading is out of range reports it for every expression that reads the
//!   pin.
//! - `DateCalculationError`: Date arithmetic that has no single result, such
//!   as a month or year offset that lands on a local time skipped or repeated
//!   by a daylight-saving transition, or a negative relative amount
//! - `AmbiguousTime`: Local times that cannot be resolved to an instant
//! - `InvalidDate`/`InvalidTime`: Components that are out of valid ranges

use chrono::{
    DateTime, Datelike, Days, Local, Months, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta,
    TimeZone, Utc,
};
use temps_core::{
    DayReference, Direction, Language, Result, TempsError, Time, TimeExpression, TimeParser,
    TimeUnit, Weekday,
    constants::{DAYS_PER_WEEK, MONTHS_PER_YEAR},
    errors::*,
    time_utils::{
        calculate_timezone_offset_seconds, calculate_weekday_offset, convert_12_to_24_hour,
        is_valid_time, is_valid_timezone_offset,
    },
};

/// Message for a pinned instant whose own local reading is outside chrono's
/// range, so that nothing relative to it can be resolved.
const ERR_NOW_OUT_OF_RANGE: &str =
    "The reference time's local reading is outside the range chrono can represent";

/// A result outside chrono's range that no relative amount produced.
fn result_out_of_range() -> TempsError {
    TempsError::arithmetic_overflow(ERR_RESULT_OUT_OF_RANGE)
}

/// A relative amount that moves the result outside chrono's range.
fn amount_out_of_range() -> TempsError {
    TempsError::arithmetic_overflow(ERR_AMOUNT_OUT_OF_RANGE)
}

/// Report a range failure met while resolving a relative expression as the
/// relative amount's doing, which it is; other errors pass through.
fn blame_amount(error: TempsError) -> TempsError {
    match error {
        TempsError::ArithmeticOverflow { .. } => amount_out_of_range(),
        other => other,
    }
}

/// `dt`, provided its local wall-clock reading is within chrono's range.
///
/// chrono range-checks only the UTC instant when it shifts a `DateTime`, so
/// within one zone offset of either end of its range it hands back instants
/// whose local reading is past `NaiveDateTime::MAX` or before
/// `NaiveDateTime::MIN`. `date_naive()` and `naive_local()` panic on such a
/// value, so none may escape as a result.
fn local_in_range(dt: DateTime<Local>) -> Option<DateTime<Local>> {
    dt.naive_utc().checked_add_offset(*dt.offset()).map(|_| dt)
}

/// The last instant chrono can represent in the local zone.
///
/// East of UTC that is where the wall clock reaches `NaiveDateTime::MAX`;
/// west of it, UTC runs out first, while the wall clock still reads the same
/// last date.
fn last_local_instant() -> Result<DateTime<Local>> {
    resolve_local(NaiveDateTime::MAX).or_else(|_| {
        local_in_range(DateTime::<Utc>::MAX_UTC.with_timezone(&Local))
            .ok_or_else(result_out_of_range)
    })
}

/// Resolve a naive local datetime to a concrete instant, matching the jiff
/// backend's default `compatible` disambiguation.
///
/// Every `LocalResult` branch needs care, including `Single`:
///
/// * **`Single`** is normally an unremarkable local time, and re-normalising it
///   through UTC is a no-op. It is not always unremarkable: a nonexistent local
///   time is not reliably reported as `None`. When a spring-forward gap begins
///   exactly at midnight, chrono returns `Single` carrying the offset from
///   *before* the gap — in `America/Havana`, `2024-03-10 00:00:00` comes back as
///   `Single` at `-05:00`, a wall-clock reading that never occurred, while
///   `00:30:00` on the same date is `None`. Converting to UTC and back
///   (`naive_utc`, then `with_timezone(&Local)`) reinterprets that instant under
///   the offset actually in force and yields `2024-03-10 01:00:00-04:00`, the
///   same forward shift jiff performs.
/// * An **`Ambiguous`** local time (a DST fall-back fold) maps to two instants.
///   chrono's `LocalResult::Ambiguous` is not ordered by instant — `.earliest()`
///   can hand back the *later* one — so choose explicitly by comparison.
/// * **`None`** is the ordinary report for a nonexistent local time (a
///   spring-forward gap). jiff shifts such a time forward by the gap's own
///   width; interpreting the civil time with the offset in force *before* the
///   gap does exactly that, and works for any width — including whole days
///   skipped at the date line, where a fixed-size probe would give up.
///
///   chrono answers `None` as well for a local time whose instant would lie
///   outside its range, within one zone offset of `NaiveDateTime::MAX` or
///   `MIN`. There is no gap there to shift across, so that is reported as
///   [`TempsError::ArithmeticOverflow`], like every other result chrono cannot
///   represent, and [`TempsError::AmbiguousTime`] is kept for a local time
///   that genuinely cannot be resolved.
///
/// Every instant returned has a local reading within chrono's range, so
/// `date_naive()` on it cannot panic.
fn resolve_local(naive: NaiveDateTime) -> Result<DateTime<Local>> {
    use chrono::offset::LocalResult;

    let resolved = match naive.and_local_timezone(Local) {
        LocalResult::Single(dt) => Utc.from_utc_datetime(&dt.naive_utc()).with_timezone(&Local),
        LocalResult::Ambiguous(a, b) => {
            if a <= b {
                a
            } else {
                b
            }
        }
        LocalResult::None => {
            let mut pre_gap_offset = None;
            for days in 1..=3 {
                // Looking back past the first date chrono can name means the
                // time is at the start of its range, not in a gap.
                let probe = naive
                    .checked_sub_days(Days::new(days))
                    .ok_or_else(result_out_of_range)?;
                if let Some(dt) = probe.and_local_timezone(Local).earliest() {
                    pre_gap_offset = Some(*dt.offset());
                    break;
                }
            }
            let pre_gap_offset =
                pre_gap_offset.ok_or_else(|| TempsError::ambiguous_time(ERR_AMBIGUOUS_TIME))?;
            // With the offset known, only the end of the range can stop this.
            let utc = naive
                .checked_sub_offset(pre_gap_offset)
                .ok_or_else(result_out_of_range)?;
            Utc.from_utc_datetime(&utc).with_timezone(&Local)
        }
    };
    local_in_range(resolved).ok_or_else(result_out_of_range)
}

/// The civil date `day_ref` names, counted in calendar days from `today`.
///
/// Only the date is computed; turning it into an instant is left to the
/// caller, so a day-at-time expression never resolves a midnight it was not
/// asked for. On chrono's first date, east of UTC, that midnight is before the
/// first UTC instant even though the afternoon is not.
fn day_reference_date(today: NaiveDate, day_ref: DayReference) -> Result<NaiveDate> {
    let days = match day_ref {
        DayReference::Today => 0,
        DayReference::Yesterday => -1,
        DayReference::Tomorrow => 1,
        DayReference::DayBeforeYesterday => -2,
        DayReference::DayAfterTomorrow => 2,
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

            let current_offset = i64::from(today.weekday().num_days_from_monday());
            let target_offset = i64::from(target_weekday.num_days_from_monday());
            calculate_weekday_offset(current_offset, target_offset, modifier)
        }
    };

    let step = Days::new(days.unsigned_abs());
    if days >= 0 {
        today.checked_add_days(step)
    } else {
        today.checked_sub_days(step)
    }
    // Past the first or last date chrono can name.
    .ok_or_else(result_out_of_range)
}

/// The wall-clock time `time` names, on the 24-hour clock.
fn wall_time(time: &Time) -> Result<NaiveTime> {
    let invalid = || TempsError::invalid_time(time.hour, time.minute, time.second);
    if !is_valid_time(time.hour, time.minute, time.second, time.meridiem) {
        return Err(invalid());
    }

    let hour = convert_12_to_24_hour(time.hour, time.meridiem.as_ref());
    NaiveTime::from_hms_opt(hour.into(), time.minute.into(), time.second.into()).ok_or_else(invalid)
}

/// `now` moved by `months` calendar months in `direction`, keeping the wall
/// clock.
fn shift_months(
    now: DateTime<Local>,
    months: Months,
    direction: Direction,
) -> Result<DateTime<Local>> {
    let shifted = match direction {
        Direction::Past => now.checked_sub_months(months),
        Direction::Future => now.checked_add_months(months),
    };
    shifted.ok_or_else(|| {
        // chrono answers `None` both for a result outside its range and for a
        // wall-clock time on the target date that a daylight-saving transition
        // skips or repeats. Only the first is an overflow; tell them apart by
        // redoing the arithmetic on the naive local reading.
        let naive = match direction {
            Direction::Past => now.naive_local().checked_sub_months(months),
            Direction::Future => now.naive_local().checked_add_months(months),
        };
        match naive.map(resolve_local) {
            None | Some(Err(TempsError::ArithmeticOverflow { .. })) => amount_out_of_range(),
            Some(_) => TempsError::date_calculation(ERR_DATE_CALC_INVALID),
        }
    })
}

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
/// let provider = ChronoProvider::new();
/// let expr = parse("next Monday", Language::English).unwrap();
/// let datetime = provider.parse_expression(expr).unwrap();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ChronoProvider {
    /// Fixed instant to resolve against, or `None` to read the system clock.
    now: Option<DateTime<Local>>,
}

impl ChronoProvider {
    /// A provider that resolves expressions against the system clock.
    #[must_use]
    pub fn new() -> Self {
        Self { now: None }
    }

    /// A provider pinned to a fixed instant.
    ///
    /// Resolution of "now", "tomorrow" and every relative expression is
    /// relative to this instant, which makes results reproducible — including
    /// around daylight-saving transitions, where behaviour otherwise depends on
    /// the day the code happens to run.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{Local, TimeZone};
    /// use temps_chrono::ChronoProvider;
    /// use temps_core::{Language, TimeParser, parse};
    ///
    /// let fixed = Local.with_ymd_and_hms(2024, 1, 31, 10, 0, 0).unwrap();
    /// let provider = ChronoProvider::at(fixed);
    /// let expr = parse("tomorrow", Language::English).unwrap();
    /// let resolved = provider.parse_expression(expr).unwrap();
    /// assert_eq!(resolved.date_naive().to_string(), "2024-02-01");
    /// ```
    ///
    /// chrono can hold instants within one zone offset of either end of its
    /// range whose local reading is past `NaiveDateTime::MAX` or before
    /// `NaiveDateTime::MIN`. Pinned to such an instant, every expression that
    /// reads the pin, `now` included, fails with
    /// [`TempsError::ArithmeticOverflow`]; absolute dates and times, which do
    /// not read it, still resolve.
    #[must_use]
    pub fn at(now: DateTime<Local>) -> Self {
        Self { now: Some(now) }
    }

    /// The instant to resolve against, provided its local reading is within
    /// chrono's range.
    ///
    /// Every arm that reads the clock goes through this, so none can reach
    /// `date_naive()` or `naive_local()` on a pin that would make them panic.
    fn local_now(&self) -> Result<DateTime<Local>> {
        local_in_range(self.now())
            .ok_or_else(|| TempsError::arithmetic_overflow(ERR_NOW_OUT_OF_RANGE))
    }
}

impl TimeParser for ChronoProvider {
    type DateTime = DateTime<Local>;

    fn now(&self) -> Self::DateTime {
        self.now.unwrap_or_else(Local::now)
    }

    fn parse_expression(&self, expr: TimeExpression) -> Result<Self::DateTime> {
        match expr {
            TimeExpression::Now => self.local_now(),
            TimeExpression::Relative(rel) => {
                if rel.amount < 0 {
                    return Err(TempsError::date_calculation(
                        ERR_RELATIVE_AMOUNT_NON_NEGATIVE,
                    ));
                }

                let now = self.local_now()?;

                if rel.amount == 0 {
                    return Ok(now);
                }

                // Handle months and years separately for proper date arithmetic
                match rel.unit {
                    TimeUnit::Month => {
                        let months =
                            Months::new(rel.amount.try_into().map_err(|_| amount_out_of_range())?);
                        shift_months(now, months, rel.direction)
                    }
                    TimeUnit::Year => {
                        // Convert years to months for proper arithmetic
                        let months_count = rel
                            .amount
                            .checked_mul(MONTHS_PER_YEAR as i64)
                            .ok_or_else(|| TempsError::arithmetic_overflow(ERR_YEAR_OVERFLOW))?;
                        let months = Months::new(
                            months_count.try_into().map_err(|_| amount_out_of_range())?,
                        );
                        shift_months(now, months, rel.direction)
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
                        .ok_or_else(amount_out_of_range)?;

                        let date = match rel.direction {
                            Direction::Past => now.date_naive().checked_sub_days(Days::new(days)),
                            Direction::Future => now.date_naive().checked_add_days(Days::new(days)),
                        }
                        .ok_or_else(amount_out_of_range)?;

                        resolve_local(date.and_time(now.time())).map_err(blame_amount)
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
                        .ok_or_else(amount_out_of_range)?;

                        let shifted = match rel.direction {
                            Direction::Past => now.checked_sub_signed(duration),
                            Direction::Future => now.checked_add_signed(duration),
                        };
                        // chrono checks only that the UTC instant is in range;
                        // the local wall clock can run out before it does.
                        shifted
                            .and_then(local_in_range)
                            .ok_or_else(amount_out_of_range)
                    }
                }
            }
            TimeExpression::Absolute(abs) => {
                use chrono::FixedOffset;

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
                            resolve_local(naive_dt)?
                        }
                    }
                } else {
                    // Date only, set time to midnight
                    let midnight = date
                        .and_hms_opt(0, 0, 0)
                        .ok_or_else(|| TempsError::date_calculation(ERR_MIDNIGHT_FAILED))?;
                    resolve_local(midnight)?
                };

                Ok(datetime)
            }
            TimeExpression::Day(day_ref) => {
                let date = day_reference_date(self.local_now()?.date_naive(), day_ref)?;
                resolve_local(date.and_time(NaiveTime::MIN))
            }
            TimeExpression::Time(time) => {
                let wall = wall_time(&time)?;
                resolve_local(self.local_now()?.date_naive().and_time(wall))
            }
            TimeExpression::DayTime(day_time) => {
                // Only the requested wall time is resolved to an instant. Going
                // through the day's midnight first would fail on a day whose
                // midnight is out of range even though the time itself is not.
                let date = day_reference_date(self.local_now()?.date_naive(), day_time.day)?;
                resolve_local(date.and_time(wall_time(&day_time.time)?))
            }
            TimeExpression::LaterToday => {
                let now = self.local_now()?;
                // `None` when two hours on is past the end of chrono's range;
                // the clamp below then applies.
                let later = now
                    .checked_add_signed(TimeDelta::hours(2))
                    .and_then(local_in_range);

                // Clamp against the true end of the local day. A fixed 23:59:59
                // is wrong in zones where the day is cut short by a transition,
                // and would drop sub-second precision.
                let last_today = match now.date_naive().succ_opt() {
                    Some(tomorrow) => resolve_local(tomorrow.and_time(NaiveTime::MIN))?
                        .checked_sub_signed(TimeDelta::nanoseconds(1))
                        .ok_or_else(result_out_of_range)?,
                    // chrono cannot name tomorrow, so today ends with its range.
                    None => last_local_instant()?,
                };

                // Compare instants, not dates: `later` may be on a date chrono
                // cannot name.
                let clamped = later.map_or(last_today, |later| later.min(last_today));
                // Never resolve into the past.
                Ok(clamped.max(now))
            }
            TimeExpression::Date(date) => {
                NaiveDate::from_ymd_opt(date.year as i32, date.month as u32, date.day as u32)
                    .ok_or_else(|| TempsError::invalid_date(date.year, date.month, date.day))?
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| TempsError::date_calculation(ERR_MIDNIGHT_FAILED))
                    .and_then(resolve_local)
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
///   ([`TempsError::ParseError`])
/// - The result is outside the range chrono can represent, whatever produced
///   it: a relative amount of any unit, a day reference or a time of day
///   ([`TempsError::ArithmeticOverflow`])
/// - Date arithmetic has no single result, such as a month or year offset
///   that lands on a local time a daylight-saving transition skips or repeats
///   ([`TempsError::DateCalculationError`])
/// - The resulting local time cannot be resolved to an instant
///   ([`TempsError::AmbiguousTime`])
pub fn parse_to_datetime(input: &str, language: Language) -> Result<DateTime<Local>> {
    let expr = temps_core::parse(input, language)?;
    ChronoProvider::new().parse_expression(expr)
}
