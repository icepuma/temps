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
//! let provider = JiffProvider::new();
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
//! - `DateCalculationError`: A relative expression or a day reference, with or
//!   without a time, whose result is outside jiff's range (the error's context
//!   carries jiff's reason), or a negative relative amount
//! - `ArithmeticOverflow`: A relative amount too large for a `jiff::Span`
//! - `InvalidDate`/`InvalidTime`: Components that are out of valid ranges
//! - `BackendError`: Errors from the jiff library, including an absolute
//!   expression, a calendar date or a bare time of day outside jiff's range
//!
//! [`JiffProvider`]'s range limits say which error each kind of expression
//! gets at the edges of jiff's range, and how that differs from chrono.

use jiff::{Span, Zoned};
use temps_core::{
    DayReference, Direction, Language, Result, TempsError, TimeExpression, TimeParser, TimeUnit,
    Weekday,
    errors::*,
    time_utils::{
        calculate_timezone_offset_seconds, calculate_weekday_offset, convert_12_to_24_hour,
        is_valid_time, is_valid_timezone_offset,
    },
};

/// Jiff-based implementation of the TimeParser trait.
///
/// This provider uses jiff's `Zoned` as its datetime type, providing
/// high-precision time calculations and comprehensive timezone support.
///
/// ## Range limits
///
/// jiff's civil dates span the years `-9999..=9999`, and its instants run from
/// `jiff::Timestamp::MIN` (`-009999-01-02T01:59:59Z`) to `jiff::Timestamp::MAX`
/// (`9999-12-30T22:00:00.999999999Z`). Both instant limits fall inside the civil
/// range, so the first hours of year -9999 and the last hours of year 9999 are
/// out of reach. A civil datetime resolves only when its instant lands within
/// those limits, so the extreme accepted local datetimes depend on the zone's
/// offset.
///
/// **Upper edge.** The last accepted local datetime is `9999-12-30T22:00:00` in
/// UTC, `9999-12-30T17:00:00` at `-05:00` and `9999-12-31T07:00:00` at `+09:00`.
/// A date-only expression resolves to local midnight, so `9999-12-31` fails in
/// UTC but succeeds in `Asia/Tokyo`. Past that point, absolute expressions,
/// calendar dates (`31/12/9999`) and bare times of day (`22:30`) fail with
/// `TempsError::BackendError`. Relative expressions (`in 1 day` from
/// `9999-12-30T12:00Z`, say) and day references with or without a time
/// (`tomorrow`, `tomorrow at 10:00`, `tonight`) fail with
/// `TempsError::DateCalculationError`. A day with a time fails that way even
/// when the day's midnight is in range and only the time is not: pinned at
/// `9999-12-30T12:00Z`, `today at 22:00:01` is a
/// `TempsError::DateCalculationError`, while the bare time `22:00:01` is a
/// `TempsError::BackendError`.
///
/// `later today` resolves at the upper edge, too. On a day whose next midnight
/// jiff cannot represent (all of `9999-12-30` in UTC, `9999-12-31` in
/// `Asia/Tokyo`), it resolves to two hours on while that is in range and
/// otherwise stops at `jiff::Timestamp::MAX`: pinned at `9999-12-30T12:00Z` it
/// resolves to `9999-12-30T14:00Z`, and pinned at `9999-12-30T21:00Z` to
/// `9999-12-30T22:00:00.999999999Z`. It fails, with
/// `TempsError::DateCalculationError`, only in a zone whose clocks go back
/// across midnight shortly before `Timestamp::MAX`, so that the last instant
/// reads as an earlier date than the clock and today has no end to stop at.
///
/// **Lower edge.** The first accepted local datetime is `-009999-01-02T01:59:59`
/// in UTC, `-009999-01-01T20:59:59` at `-05:00` and `-009999-01-02T10:59:59` at
/// `+09:00`, so even `-9999-01-01T00:00` fails in UTC. Absolute expressions and
/// calendar dates cannot get there, because their years are unsigned. Only
/// relative expressions (`12100 years ago`) and expressions resolved against a
/// clock pinned near the edge can. Those fail with
/// `TempsError::DateCalculationError`, or with `TempsError::BackendError` for a
/// bare time of day that falls before the limit. Calendar arithmetic that
/// lands a fraction of a second before the limit fails with
/// `TempsError::DateCalculationError` too, although jiff itself lets it through
/// (see the known upstream issues below). A time on the first representable day
/// still resolves when the time itself is in range: pinned at
/// `-009999-01-02T02:30Z`, `today` fails because that day's midnight is out of
/// range, but `today at 22:00` succeeds.
///
/// **Which error.** An out-of-range result is reported by what produced it:
///
/// - `TempsError::DateCalculationError` for a relative expression, a day
///   reference with or without a time, and the rare `later today` failure
///   above. Its message says what fell outside the range, and its `context`
///   carries jiff's reason.
/// - `TempsError::ArithmeticOverflow` for a relative amount too large for a
///   `jiff::Span` to hold at all, such as more than 19,998 years or 7,304,484
///   days. That fails from any clock: `in 19999 years` is an
///   `ArithmeticOverflow`, while `in 19998 years` from 2024 is a
///   `DateCalculationError`.
/// - `TempsError::BackendError` for an absolute expression, a calendar date or
///   a bare time of day, carrying jiff's message.
///
/// These are range limits of the underlying library, not defects.
/// `ChronoProvider` reaches far beyond both edges and accepts the same
/// expressions. Where chrono's own, much wider range runs out, it reports every
/// out-of-range result as `TempsError::ArithmeticOverflow`, so code that
/// matches on the variant sees different ones from the two backends for the
/// same kind of failure.
///
/// ## Known upstream issues
///
/// **Offsets of pre-1970 sub-second instants.** jiff 0.2.37 finds a zone's
/// offset by the instant's Unix second, which it truncates toward zero instead
/// of flooring. For an instant before 1970 that falls within the last second
/// before a transition and has a sub-second part, that lookup lands on the
/// transition itself, so the `Zoned` carries the offset that applies *after*
/// the transition. The instant is right, but the civil fields (date and wall
/// clock) are wrong. For example, `1961-10-29T05:59:59.5Z` in
/// `America/New_York` reads as `00:59:59.5-05:00` instead of
/// `01:59:59.5-04:00`.
///
/// temps derives day references, times and calendar arithmetic from the pinned
/// instant's civil date, so a clock pinned in such a window carries the error
/// into the result. `later today` can also hit it from a whole-second clock,
/// because it clamps to one nanosecond before the next local midnight. When that
/// midnight is a pre-1970 transition, the result can read as the next day.
/// Instants from 1970 onward are not affected, and neither are whole-second
/// instants in other expressions. The ignored `upstream_jiff_bug_pre_1970_*`
/// and `upstream_jiff_bug_later_today_*` tests in this crate cover these cases
/// and should pass once jiff floors the second.
///
/// **The last second before `Timestamp::MIN`.** When jiff 0.2.37 converts a
/// civil datetime to an instant, it range-checks only the whole second. A civil
/// datetime with a sub-second part in the last second before
/// `jiff::Timestamp::MIN` passes that check. Debug builds then hit an assertion
/// inside jiff, and release builds get a `Zoned` earlier than
/// `Timestamp::MIN`. Calendar arithmetic (`1 day ago`, `12023 years ago`)
/// produces such a datetime whenever the clock has a sub-second part and the
/// result lands in that second, and the system clock can do that. temps checks
/// the whole-second part of the result first and returns
/// `TempsError::DateCalculationError`, so it neither panics nor returns an
/// out-of-range `Zoned`. The ignored
/// `upstream_jiff_bug_civil_datetime_just_before_timestamp_min_*` test should
/// pass once jiff rejects these datetimes itself, and the check can go then.
///
/// ## Example
///
/// ```
/// use temps_jiff::JiffProvider;
/// use temps_core::{TimeParser, parse, Language};
///
/// let provider = JiffProvider::new();
/// let expr = parse("next Monday", Language::English).unwrap();
/// let datetime = provider.parse_expression(expr).unwrap();
/// ```
#[derive(Debug, Clone, Default)]
pub struct JiffProvider {
    /// Fixed instant to resolve against, or `None` to read the system clock.
    now: Option<Zoned>,
}

impl JiffProvider {
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
    /// use jiff::civil::date;
    /// use jiff::tz::TimeZone;
    /// use temps_jiff::JiffProvider;
    /// use temps_core::{Language, TimeParser, parse};
    ///
    /// let fixed = date(2024, 1, 31).at(10, 0, 0, 0).to_zoned(TimeZone::UTC).unwrap();
    /// let provider = JiffProvider::at(fixed);
    /// let expr = parse("tomorrow", Language::English).unwrap();
    /// let resolved = provider.parse_expression(expr).unwrap();
    /// assert_eq!(resolved.date().to_string(), "2024-02-01");
    /// ```
    #[must_use]
    pub fn at(now: Zoned) -> Self {
        Self { now: Some(now) }
    }
}

fn jiff_date_components(year: u16, month: u8, day: u8) -> Result<(i16, i8, i8)> {
    Ok((
        i16::try_from(year).map_err(|_| TempsError::invalid_date(year, month, day))?,
        i8::try_from(month).map_err(|_| TempsError::invalid_date(year, month, day))?,
        i8::try_from(day).map_err(|_| TempsError::invalid_date(year, month, day))?,
    ))
}

fn jiff_time_components(
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
) -> Result<(i8, i8, i8, i32)> {
    Ok((
        i8::try_from(hour).map_err(|_| TempsError::invalid_time(hour, minute, second))?,
        i8::try_from(minute).map_err(|_| TempsError::invalid_time(hour, minute, second))?,
        i8::try_from(second).map_err(|_| TempsError::invalid_time(hour, minute, second))?,
        i32::try_from(nanosecond)
            .map_err(|_| TempsError::backend_error("Invalid nanosecond component", "jiff"))?,
    ))
}

// Messages for arithmetic that leaves jiff's supported range. jiff's own
// explanation travels alongside as the error's context.
const ERR_RELATIVE_OUT_OF_RANGE: &str =
    "Relative amount moves the date outside the supported range";
const ERR_TOMORROW_START_OUT_OF_RANGE: &str =
    "The start of tomorrow is outside the supported range";
const ERR_TODAY_END_OUT_OF_RANGE: &str = "The last instant of today is outside the supported range";
const ERR_DAY_MIDNIGHT_OUT_OF_RANGE: &str =
    "Midnight of the requested day is outside the supported range";
const ERR_DAY_TIME_OUT_OF_RANGE: &str =
    "The time on the requested day is outside the supported range";

/// Fails when shifting `now` by the calendar span `span` lands in the fraction
/// of a second before `jiff::Timestamp::MIN`. Call it before `now.checked_add`.
///
/// Calendar arithmetic (days and longer) moves the civil datetime and then
/// converts it back to an instant in `now`'s zone. jiff 0.2.37 range-checks
/// only the whole second of that conversion. A result with a sub-second part
/// in the last second before `Timestamp::MIN` gets through: it trips a debug
/// assertion inside jiff, and a release build returns a `Zoned` earlier than
/// `Timestamp::MIN`. The same civil datetime without its sub-second part is
/// checked correctly, and it is out of range exactly when the full result is.
fn reject_calendar_shift_before_timestamp_min(
    now: &Zoned,
    span: Span,
) -> std::result::Result<(), jiff::Error> {
    // If the civil arithmetic itself fails, `checked_add` fails the same way.
    let Ok(shifted) = now.datetime().checked_add(span) else {
        return Ok(());
    };
    // A whole-second result is range-checked correctly. Offsets stay under 26
    // hours, so a civil date after -9999-01-03 is hours clear of the limit.
    if shifted.subsec_nanosecond() == 0 || shifted.date() > jiff::civil::date(-9999, 1, 3) {
        return Ok(());
    }
    shifted
        .with()
        .subsec_nanosecond(0)
        .build()?
        .to_zoned(now.time_zone().clone())
        .map(drop)
}

/// The civil date `day_ref` names, counted in calendar days from `now`'s local
/// date.
///
/// Only the date is computed; turning it into an instant is left to the
/// caller, so a day-at-time expression never has to materialise a midnight it
/// was not asked for.
fn day_reference_date(now: &Zoned, day_ref: DayReference) -> Result<jiff::civil::Date> {
    let today = now.date();
    let (days, description) = match day_ref {
        DayReference::Today => return Ok(today),
        DayReference::Yesterday => (-1, "yesterday"),
        DayReference::Tomorrow => (1, "tomorrow"),
        DayReference::DayBeforeYesterday => (-2, "day before yesterday"),
        DayReference::DayAfterTomorrow => (2, "day after tomorrow"),
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

            let current_offset = today.weekday().to_monday_zero_offset() as i64;
            let target_offset = target_weekday.to_monday_zero_offset() as i64;

            (
                calculate_weekday_offset(current_offset, target_offset, modifier),
                "weekday",
            )
        }
    };

    today.checked_add(Span::new().days(days)).map_err(|e| {
        TempsError::date_calculation_with_source(
            format!("Failed to calculate {description}"),
            e.to_string(),
        )
    })
}

impl TimeParser for JiffProvider {
    type DateTime = Zoned;

    fn now(&self) -> Self::DateTime {
        self.now.clone().unwrap_or_else(Zoned::now)
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

                // Create a span based on the time unit
                // The `try_*` builders are essential: the plain setters panic when
                // the amount exceeds jiff's per-unit range, and `rel.amount` comes
                // straight from user input.
                let span = match rel.unit {
                    TimeUnit::Second => Span::new().try_seconds(rel.amount),
                    TimeUnit::Minute => Span::new().try_minutes(rel.amount),
                    TimeUnit::Hour => Span::new().try_hours(rel.amount),
                    TimeUnit::Day => Span::new().try_days(rel.amount),
                    TimeUnit::Week => Span::new().try_weeks(rel.amount),
                    TimeUnit::Month => Span::new().try_months(rel.amount),
                    TimeUnit::Year => Span::new().try_years(rel.amount),
                }
                .map_err(|_| TempsError::arithmetic_overflow(ERR_AMOUNT_OUT_OF_RANGE))?;

                // Apply the span in the correct direction. jiff's own
                // `checked_sub` is `checked_add` of the negated span.
                let span = match rel.direction {
                    Direction::Past => span.negate(),
                    Direction::Future => span,
                };
                let out_of_range = |e: jiff::Error| {
                    TempsError::date_calculation_with_source(
                        ERR_RELATIVE_OUT_OF_RANGE,
                        e.to_string(),
                    )
                };
                // Seconds, minutes and hours use timestamp arithmetic, which
                // jiff range-checks correctly.
                if matches!(
                    rel.unit,
                    TimeUnit::Day | TimeUnit::Week | TimeUnit::Month | TimeUnit::Year
                ) {
                    reject_calendar_shift_before_timestamp_min(&now, span).map_err(out_of_range)?;
                }
                now.checked_add(span).map_err(out_of_range)
            }
            TimeExpression::Absolute(abs) => {
                use jiff::civil::{Date, DateTime, Time};
                use jiff::tz::{Offset, TimeZone};

                let (year, month, day) = jiff_date_components(abs.year, abs.month, abs.day)?;
                let date = Date::new(year, month, day)
                    .map_err(|e| TempsError::backend_error(e.to_string(), "jiff"))?;

                if abs.hour.is_none() && abs.minute.is_some() {
                    // A minute without an hour is not a time we can honour; say so
                    // rather than silently falling through to midnight.
                    return Err(TempsError::invalid_time(
                        0,
                        abs.minute.unwrap_or(0),
                        abs.second.unwrap_or(0),
                    ));
                }

                if let Some(hour) = abs.hour {
                    // Default only the components below the one supplied; dropping a
                    // supplied hour would silently return local midnight instead.
                    let minute = abs.minute.unwrap_or(0);
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
                    if let Some(second) = abs.second
                        && second > 59
                    {
                        return Err(TempsError::invalid_time(hour, minute, second));
                    }

                    let second = abs.second.unwrap_or(0);
                    let nanosecond = abs.nanosecond.unwrap_or(0);
                    let (hour, minute, second, nanosecond) =
                        jiff_time_components(hour, minute, second, nanosecond)?;

                    let time = Time::new(hour, minute, second, nanosecond)
                        .map_err(|e| TempsError::backend_error(e.to_string(), "jiff"))?;

                    let datetime = DateTime::from_parts(date, time);

                    match &abs.timezone {
                        Some(temps_core::Timezone::Utc) => datetime
                            .to_zoned(TimeZone::UTC)
                            .map(|z| z.with_time_zone(TimeZone::system()))
                            .map_err(|e| {
                                TempsError::backend_error(
                                    format!("{ERR_TIMEZONE_CONVERSION}: {e}"),
                                    "jiff",
                                )
                            }),
                        Some(temps_core::Timezone::Offset { total_minutes }) => {
                            if !is_valid_timezone_offset(temps_core::Timezone::Offset {
                                total_minutes: *total_minutes,
                            }) {
                                return Err(TempsError::invalid_timezone_offset(*total_minutes));
                            }

                            let total_seconds = calculate_timezone_offset_seconds(*total_minutes);
                            let offset = Offset::from_seconds(total_seconds)
                                .map_err(|_| TempsError::invalid_timezone_offset(*total_minutes))?;

                            datetime
                                .to_zoned(TimeZone::fixed(offset))
                                .map(|z| z.with_time_zone(TimeZone::system()))
                                .map_err(|e| {
                                    TempsError::backend_error(
                                        format!("{ERR_TIMEZONE_CONVERSION}: {e}"),
                                        "jiff",
                                    )
                                })
                        }
                        None => {
                            // No timezone specified, treat as system timezone
                            datetime.to_zoned(TimeZone::system()).map_err(|e| {
                                TempsError::backend_error(
                                    format!("{ERR_TIMEZONE_CONVERSION}: {e}"),
                                    "jiff",
                                )
                            })
                        }
                    }
                } else {
                    // Date only, set time to midnight
                    let datetime = date.at(0, 0, 0, 0);
                    datetime.to_zoned(TimeZone::system()).map_err(|e| {
                        TempsError::backend_error(format!("{ERR_TIMEZONE_CONVERSION}: {e}"), "jiff")
                    })
                }
            }
            TimeExpression::Day(day_ref) => {
                let now = self.now();
                day_reference_date(&now, day_ref)?
                    .at(0, 0, 0, 0)
                    .to_zoned(now.time_zone().clone())
                    .map_err(|e| {
                        TempsError::date_calculation_with_source(
                            ERR_DAY_MIDNIGHT_OUT_OF_RANGE,
                            e.to_string(),
                        )
                    })
            }
            TimeExpression::Time(time) => {
                let now = self.now();
                let date = now.date();

                if !is_valid_time(time.hour, time.minute, time.second, time.meridiem) {
                    return Err(TempsError::invalid_time(
                        time.hour,
                        time.minute,
                        time.second,
                    ));
                }

                let hour = convert_12_to_24_hour(time.hour, time.meridiem.as_ref());

                let (hour, minute, second, nanosecond) =
                    jiff_time_components(hour, time.minute, time.second, 0)?;

                date.at(hour, minute, second, nanosecond)
                    .to_zoned(now.time_zone().clone())
                    .map_err(|e| {
                        TempsError::backend_error(format!("Failed to create time: {e}"), "jiff")
                    })
            }
            TimeExpression::DayTime(day_time) => {
                // Only the requested wall time is converted to an instant. Going
                // through the day's midnight first would fail on a day whose
                // midnight is out of range even though the time itself is not.
                let now = self.now();
                let date = day_reference_date(&now, day_time.day)?;

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
                    convert_12_to_24_hour(day_time.time.hour, day_time.time.meridiem.as_ref());

                let (hour, minute, second, nanosecond) =
                    jiff_time_components(hour, day_time.time.minute, day_time.time.second, 0)?;

                // Out of range here means the same thing as for the day's
                // midnight, so it is reported with the same variant.
                date.at(hour, minute, second, nanosecond)
                    .to_zoned(now.time_zone().clone())
                    .map_err(|e| {
                        TempsError::date_calculation_with_source(
                            ERR_DAY_TIME_OUT_OF_RANGE,
                            e.to_string(),
                        )
                    })
            }
            TimeExpression::LaterToday => {
                let now = self.now();
                // `None` when two hours on is past `Timestamp::MAX`; the clamp
                // below then applies.
                let later = now.checked_add(Span::new().hours(2)).ok();

                // Clamp against the true end of the local day rather than a fixed
                // 23:59:59, which need not exist and would drop sub-second precision.
                let tomorrow_start = now
                    .date()
                    .tomorrow()
                    .and_then(|tomorrow| tomorrow.at(0, 0, 0, 0).to_zoned(now.time_zone().clone()));
                let last_today = match tomorrow_start {
                    Ok(tomorrow_start) => tomorrow_start
                        .checked_sub(Span::new().nanoseconds(1))
                        .map_err(|e| {
                            TempsError::date_calculation_with_source(
                                ERR_TODAY_END_OUT_OF_RANGE,
                                e.to_string(),
                            )
                        })?,
                    // jiff cannot represent the start of tomorrow, either its
                    // civil date or its instant, so today ends with jiff's range.
                    Err(e) => {
                        let last_instant = jiff::Timestamp::MAX.to_zoned(now.time_zone().clone());
                        // Only a zone whose clocks go back across midnight can
                        // read that instant as another date than `now`.
                        if last_instant.date() != now.date() {
                            return Err(TempsError::date_calculation_with_source(
                                ERR_TOMORROW_START_OUT_OF_RANGE,
                                e.to_string(),
                            ));
                        }
                        last_instant
                    }
                };

                // Compare instants, not dates: whether `later` is still today
                // is whether it comes before the end of today.
                let clamped = match later {
                    Some(later) => later.min(last_today),
                    None => last_today,
                };
                // Never resolve into the past.
                Ok(clamped.max(now))
            }
            TimeExpression::Date(date) => {
                use jiff::civil::Date;

                let (year, month, day) = jiff_date_components(date.year, date.month, date.day)?;
                let jiff_date = Date::new(year, month, day)
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
    JiffProvider::new().parse_expression(expr)
}
