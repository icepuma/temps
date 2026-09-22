//! Integration tests for the jiff backend.
//!
//! Every test here drives the real [`JiffProvider`]. Time-dependent behaviour is
//! made reproducible with `JiffProvider::at(fixed)` rather than by
//! reimplementing the provider against a mock clock — a mock inevitably drifts
//! from production and hides the very bugs these tests exist to catch.

use jiff::{
    Span, Zoned,
    civil::{DateTime, date},
    tz::TimeZone,
};
use temps_core::*;
use temps_jiff::*;

/// A `Zoned` in an explicitly named IANA zone.
///
/// Zone-specific behaviour is pinned this way instead of via the `TZ`
/// environment variable, which is process-wide and would make the suite
/// order-dependent under a parallel test runner.
fn at_zone(zone: &str, y: i16, m: i8, d: i8, hour: i8, minute: i8) -> Zoned {
    date(y, m, d)
        .at(hour, minute, 0, 0)
        .in_tz(zone)
        .unwrap_or_else(|e| {
            panic!("{zone} {y}-{m}-{d} {hour}:{minute} is not a valid instant: {e}")
        })
}

/// A `Zoned` in UTC, for tests that only care about arithmetic.
fn utc(y: i16, m: i8, d: i8, hour: i8, minute: i8) -> Zoned {
    date(y, m, d)
        .at(hour, minute, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap()
}

fn resolve(provider: &JiffProvider, input: &str, language: Language) -> Zoned {
    let expr = parse(input, language).unwrap_or_else(|e| panic!("failed to parse {input:?}: {e}"));
    provider
        .parse_expression(expr)
        .unwrap_or_else(|e| panic!("failed to resolve {input:?}: {e}"))
}

const ALL_UNITS: [TimeUnit; 7] = [
    TimeUnit::Second,
    TimeUnit::Minute,
    TimeUnit::Hour,
    TimeUnit::Day,
    TimeUnit::Week,
    TimeUnit::Month,
    TimeUnit::Year,
];

const BOTH_DIRECTIONS: [Direction; 2] = [Direction::Past, Direction::Future];

// ===== Provider plumbing =====

#[test]
fn system_clock_provider_returns_a_plausible_now() {
    let provider = JiffProvider::new();
    let now = provider.now();
    assert!(now > Zoned::default());
}

#[test]
fn now_expression_returns_the_pinned_instant_exactly() {
    let fixed = utc(2024, 3, 15, 10, 30);
    let provider = JiffProvider::at(fixed.clone());

    assert_eq!(provider.now(), fixed);
    assert_eq!(resolve(&provider, "now", Language::English), fixed);
    assert_eq!(resolve(&provider, "jetzt", Language::German), fixed);
}

#[test]
fn pinned_provider_keeps_the_zone_of_the_instant_it_was_given() {
    let provider = JiffProvider::at(at_zone("America/New_York", 2024, 6, 15, 9, 0));
    let today = resolve(&provider, "today", Language::English);

    assert_eq!(today.time_zone().iana_name(), Some("America/New_York"));
    assert_eq!(today.date().to_string(), "2024-06-15");
    assert_eq!(today.hour(), 0);
}

// ===== Relative arithmetic =====

#[test]
fn english_relative_expressions_resolve_against_the_pinned_instant() {
    let base = utc(2024, 3, 15, 10, 30);
    let provider = JiffProvider::at(base.clone());

    let cases = vec![
        ("in 30 seconds", Span::new().seconds(30)),
        ("45 seconds ago", Span::new().seconds(-45)),
        ("in a second", Span::new().seconds(1)),
        ("in 5 minutes", Span::new().minutes(5)),
        ("10 minutes ago", Span::new().minutes(-10)),
        ("in a minute", Span::new().minutes(1)),
        ("a minute ago", Span::new().minutes(-1)),
        ("in 2 hours", Span::new().hours(2)),
        ("3 hours ago", Span::new().hours(-3)),
        ("in an hour", Span::new().hours(1)),
        ("an hour ago", Span::new().hours(-1)),
        ("in 1 day", Span::new().days(1)),
        ("2 days ago", Span::new().days(-2)),
        ("in a day", Span::new().days(1)),
        ("in one day", Span::new().days(1)),
        ("in 1 week", Span::new().weeks(1)),
        ("2 weeks ago", Span::new().weeks(-2)),
        ("in a week", Span::new().weeks(1)),
        ("one week ago", Span::new().weeks(-1)),
        ("in 1 month", Span::new().months(1)),
        ("1 month ago", Span::new().months(-1)),
        ("in 3 months", Span::new().months(3)),
        ("in 1 year", Span::new().years(1)),
        ("1 year ago", Span::new().years(-1)),
        ("in 2 years", Span::new().years(2)),
    ];

    for (input, span) in cases {
        let expected = base.checked_add(span).unwrap();
        assert_eq!(
            resolve(&provider, input, Language::English),
            expected,
            "wrong result for {input:?}"
        );
    }
}

#[test]
fn german_relative_expressions_resolve_against_the_pinned_instant() {
    let base = utc(2024, 3, 15, 10, 30);
    let provider = JiffProvider::at(base.clone());

    let cases = vec![
        ("in 30 Sekunden", Span::new().seconds(30)),
        ("vor 45 Sekunden", Span::new().seconds(-45)),
        ("in einer Sekunde", Span::new().seconds(1)),
        ("vor einer Sekunde", Span::new().seconds(-1)),
        ("in 5 Minuten", Span::new().minutes(5)),
        ("vor 10 Minuten", Span::new().minutes(-10)),
        ("in einer Minute", Span::new().minutes(1)),
        ("in 2 Stunden", Span::new().hours(2)),
        ("vor 3 Stunden", Span::new().hours(-3)),
        ("in einer Stunde", Span::new().hours(1)),
        ("vor einer Stunde", Span::new().hours(-1)),
        ("in 1 Tag", Span::new().days(1)),
        ("vor 2 Tagen", Span::new().days(-2)),
        ("in einem Tag", Span::new().days(1)),
        ("vor einem Tag", Span::new().days(-1)),
        ("in 1 Woche", Span::new().weeks(1)),
        ("vor 2 Wochen", Span::new().weeks(-2)),
        ("in einer Woche", Span::new().weeks(1)),
        ("in einem Monat", Span::new().months(1)),
        ("vor einem Monat", Span::new().months(-1)),
        ("in einem Jahr", Span::new().years(1)),
        ("vor einem Jahr", Span::new().years(-1)),
    ];

    for (input, span) in cases {
        let expected = base.checked_add(span).unwrap();
        assert_eq!(
            resolve(&provider, input, Language::German),
            expected,
            "wrong result for {input:?}"
        );
    }
}

#[test]
fn zero_day_offset_is_an_exact_identity() {
    // "in 0 days" must not drift: no rounding to midnight, no DST nudge, and
    // the identity has to hold in a zone that is mid-transition on the day.
    for base in [
        utc(2024, 3, 15, 10, 30),
        at_zone("America/New_York", 2024, 3, 9, 23, 30),
        at_zone("America/New_York", 2024, 11, 3, 12, 0),
    ] {
        let provider = JiffProvider::at(base.clone());

        for input in ["in 0 days", "in 0 seconds", "in 0 months", "in 0 years"] {
            assert_eq!(
                resolve(&provider, input, Language::English),
                base,
                "{input:?} moved the instant in {}",
                base.time_zone().iana_name().unwrap_or("UTC")
            );
        }

        assert_eq!(resolve(&provider, "0 days ago", Language::English), base);
    }
}

// ===== Calendar-aware month and year arithmetic =====

#[test]
fn adding_a_month_clamps_to_the_end_of_a_shorter_month() {
    let leap = JiffProvider::at(utc(2024, 1, 31, 10, 0));
    let result = resolve(&leap, "in 1 month", Language::English);
    assert_eq!(result.date().to_string(), "2024-02-29");

    let non_leap = JiffProvider::at(utc(2023, 1, 31, 10, 0));
    let result = resolve(&non_leap, "in einem Monat", Language::German);
    assert_eq!(result.date().to_string(), "2023-02-28");
}

#[test]
fn adding_a_year_to_a_leap_day_lands_on_february_28() {
    let provider = JiffProvider::at(utc(2024, 2, 29, 12, 0));
    let result = resolve(&provider, "in 1 year", Language::English);
    assert_eq!(result.date().to_string(), "2025-02-28");
}

#[test]
fn month_arithmetic_crosses_the_year_boundary() {
    let provider = JiffProvider::at(utc(2023, 10, 15, 9, 0));
    let result = resolve(&provider, "in 6 months", Language::English);
    assert_eq!(result.date().to_string(), "2024-04-15");

    let result = resolve(&provider, "6 months ago", Language::English);
    assert_eq!(result.date().to_string(), "2023-04-15");

    let result = resolve(&provider, "in 18 months", Language::English);
    assert_eq!(result.date().to_string(), "2025-04-15");
}

#[test]
fn relative_expressions_move_in_the_requested_direction() {
    let base = utc(2024, 3, 15, 10, 30);
    let provider = JiffProvider::at(base.clone());

    for unit in ALL_UNITS {
        let future = provider
            .parse_expression(TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit,
                direction: Direction::Future,
            }))
            .unwrap();
        let past = provider
            .parse_expression(TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit,
                direction: Direction::Past,
            }))
            .unwrap();

        assert!(future > base, "1 {unit:?} into the future did not advance");
        assert!(past < base, "1 {unit:?} into the past did not go back");
    }
}

// ===== Overflow: parseable input must never panic =====

#[test]
fn huge_amounts_overflow_instead_of_panicking_for_every_unit_and_direction() {
    // `Span`'s plain setters panic outside jiff's per-unit range, and the
    // grammar accepts any digit run up to i64::MAX, so an unchecked builder
    // here would abort the process from an API that returns `Result`.
    let provider = JiffProvider::at(utc(2024, 3, 15, 10, 30));

    for unit in ALL_UNITS {
        for direction in BOTH_DIRECTIONS {
            for amount in [i64::MAX, i64::MAX - 1, 999_999_999_999] {
                let result = provider.parse_expression(TimeExpression::Relative(RelativeTime {
                    amount,
                    unit,
                    direction,
                }));

                assert!(
                    matches!(result, Err(TempsError::ArithmeticOverflow { .. })),
                    "{amount} {unit:?} {direction:?} should overflow cleanly, got {result:?}"
                );
            }
        }
    }
}

#[test]
fn huge_amounts_from_parsed_text_overflow_instead_of_panicking() {
    let provider = JiffProvider::at(utc(2024, 3, 15, 10, 30));

    let inputs = [
        ("999999999999 days ago", Language::English),
        ("in 999999999999 days", Language::English),
        ("9223372036854775807 seconds ago", Language::English),
        ("in 9223372036854775807 years", Language::English),
        ("vor 999999999999 Tagen", Language::German),
        ("in 999999999999 Wochen", Language::German),
    ];

    for (input, language) in inputs {
        let expr = parse(input, language).unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
        let result = provider.parse_expression(expr);
        assert!(
            matches!(result, Err(TempsError::ArithmeticOverflow { .. })),
            "{input:?} should overflow cleanly, got {result:?}"
        );
    }
}

#[test]
fn amounts_within_span_range_but_beyond_the_calendar_fail_without_panicking() {
    // Large-but-representable spans get past the `Span` builder and have to be
    // rejected by the arithmetic instead.
    let provider = JiffProvider::at(utc(2024, 3, 15, 10, 30));

    for (amount, direction) in [(9_000, Direction::Future), (19_998, Direction::Past)] {
        assert!(
            Span::new().try_years(amount).is_ok(),
            "{amount} years must be a valid span, or this test never reaches the arithmetic"
        );

        let result = provider.parse_expression(TimeExpression::Relative(RelativeTime {
            amount,
            unit: TimeUnit::Year,
            direction,
        }));
        assert!(
            matches!(result, Err(TempsError::DateCalculationError { .. })),
            "{amount} years {direction:?} should be rejected by the arithmetic, got {result:?}"
        );
    }
}

#[test]
fn arithmetic_beyond_the_calendar_says_what_failed() {
    // The message must describe the failure rather than repeat the variant's
    // own "Date calculation error" prefix, and jiff's reason must be kept.
    let provider = JiffProvider::at(utc(2024, 3, 15, 10, 30));
    let near_the_end = JiffProvider::at(utc(9999, 12, 30, 21, 0));

    let cases = [
        (&provider, "in 9000 years", Language::English),
        (&provider, "12100 years ago", Language::English),
        (&provider, "in 3000000 days", Language::English),
        (&provider, "in 10000 Jahren", Language::German),
        (&near_the_end, "later today", Language::English),
    ];

    for (provider, input, language) in cases {
        let expr = parse(input, language).unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
        let error = provider
            .parse_expression(expr)
            .expect_err("the result is outside jiff's range");

        let TempsError::DateCalculationError { message, context } = &error else {
            panic!("{input:?} should be a date calculation error, got {error:?}");
        };
        assert_ne!(
            message,
            temps_core::errors::ERR_DATE_CALC_ERROR,
            "{input:?} repeats the variant name instead of saying what failed"
        );
        assert!(
            message.contains("outside the supported range"),
            "{input:?} has an unhelpful message: {message:?}"
        );
        assert!(
            context.as_deref().is_some_and(|c| !c.is_empty()),
            "{input:?} dropped jiff's reason: {error:?}"
        );
        assert!(
            !error
                .to_string()
                .contains("Date calculation error: Date calculation error"),
            "{input:?} renders tautologically: {error}"
        );
    }
}

// ===== Range limits =====

/// A clock `minutes` after the first instant jiff can represent,
/// `-009999-01-02T01:59:59Z`, in the given zone.
fn just_after_timestamp_min(minutes: i64, zone: TimeZone) -> Zoned {
    jiff::Timestamp::MIN
        .checked_add(Span::new().minutes(minutes))
        .unwrap()
        .to_zoned(zone)
}

#[test]
fn day_at_time_on_the_first_representable_day_matches_the_bare_time() {
    // The local midnight of jiff's first day precedes `Timestamp::MIN`, but its
    // evening does not. A day-at-time expression has to convert only the wall
    // time it asks for, exactly as the bare time does.
    let utc_cases: &[_] = &[
        ("today at 22:00", "22:00", Language::English),
        ("today at 3 pm", "3 pm", Language::English),
        ("tonight", "20:00", Language::English),
        ("this evening", "18:00", Language::English),
        ("heute um 22:00", "22:00", Language::German),
    ];
    // At -05:00 the first representable local instant is 20:59:59 on
    // -9999-01-01, so only a late evening is in range on that local day.
    let minus_five_cases: &[_] = &[
        ("today at 22:00", "22:00", Language::English),
        ("heute um 22:00", "22:00", Language::German),
    ];

    for (zone, cases) in [
        (TimeZone::UTC, utc_cases),
        (TimeZone::fixed(jiff::tz::offset(-5)), minus_five_cases),
    ] {
        let provider = JiffProvider::at(just_after_timestamp_min(30, zone.clone()));

        assert!(
            provider
                .parse_expression(TimeExpression::Day(DayReference::Today))
                .is_err(),
            "the day's own midnight is out of range, so the premise does not hold"
        );

        for &(day_time, bare_time, language) in cases {
            assert_eq!(
                resolve(&provider, day_time, language),
                resolve(&provider, bare_time, language),
                "{day_time:?} disagrees with {bare_time:?} at {zone:?}"
            );
        }
    }
}

#[test]
fn day_at_time_past_either_range_edge_is_a_date_calculation_error() {
    // A day with a time fails like the day reference alone, not like a bare
    // time: callers matching on `DateCalculationError` for `tomorrow` get the
    // same variant for `tomorrow at 10:00`.
    let near_the_end = JiffProvider::at(utc(9999, 12, 30, 12, 0));
    let near_the_start = JiffProvider::at(just_after_timestamp_min(30, TimeZone::UTC));
    let near_the_end_in_new_york =
        JiffProvider::at(at_zone("America/New_York", 9999, 12, 30, 12, 0));

    // The last and first whole seconds jiff can represent still resolve...
    assert_eq!(
        resolve(&near_the_end, "today at 22:00:00", Language::English),
        utc(9999, 12, 30, 22, 0)
    );
    assert_eq!(
        resolve(&near_the_start, "today at 01:59:59", Language::English).timestamp(),
        jiff::Timestamp::MIN
    );

    // ...and one second beyond them does not.
    let cases = [
        (&near_the_end, "today at 22:00:01", Language::English),
        (&near_the_end, "tomorrow at 1:00", Language::English),
        (&near_the_end, "next friday at 10:00", Language::English),
        (&near_the_end, "tomorrow evening", Language::English),
        (&near_the_end, "morgen um 10:00", Language::German),
        (
            &near_the_end_in_new_york,
            "tomorrow at 10:00",
            Language::English,
        ),
        (&near_the_start, "today at 01:59:58", Language::English),
        (&near_the_start, "heute um 01:00", Language::German),
    ];
    for (provider, input, language) in cases {
        let expr = parse(input, language).unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
        let result = provider.parse_expression(expr);
        assert!(
            matches!(result, Err(TempsError::DateCalculationError { .. })),
            "{input:?} should be a date calculation error, got {result:?}"
        );
    }
}

#[test]
fn calendar_arithmetic_into_the_second_before_the_first_instant_fails_cleanly() {
    // jiff 0.2.37 range-checks only the whole second when it turns the shifted
    // civil datetime back into an instant. A result a fraction of a second
    // before `Timestamp::MIN` tripped a debug assertion inside jiff, and a
    // release build returned a `Zoned` earlier than `Timestamp::MIN`.
    let utc_at = |dt: DateTime| dt.to_zoned(TimeZone::UTC).unwrap();

    let cases = [
        (
            utc_at(date(-9999, 1, 3).at(1, 59, 58, 700_000_000)),
            "1 day ago",
            Language::English,
        ),
        (
            utc_at(date(-9999, 1, 9).at(1, 59, 58, 700_000_000)),
            "1 week ago",
            Language::English,
        ),
        (
            utc_at(date(-9999, 2, 2).at(1, 59, 58, 700_000_000)),
            "1 month ago",
            Language::English,
        ),
        (
            utc_at(date(-9998, 1, 2).at(1, 59, 58, 700_000_000)),
            "1 year ago",
            Language::English,
        ),
        (
            date(-9999, 1, 2)
                .at(20, 59, 58, 700_000_000)
                .to_zoned(TimeZone::fixed(jiff::tz::offset(-5)))
                .unwrap(),
            "vor 1 Tag",
            Language::German,
        ),
        // At +23:00 the limit reads as -9999-01-03T00:59:59 locally.
        (
            date(-9999, 1, 4)
                .at(0, 59, 58, 700_000_000)
                .to_zoned(TimeZone::fixed(jiff::tz::offset(23)))
                .unwrap(),
            "1 day ago",
            Language::English,
        ),
        // Reachable from an everyday clock, too.
        (
            utc_at(date(2024, 1, 2).at(1, 59, 58, 500_000_000)),
            "12023 years ago",
            Language::English,
        ),
    ];
    for (now, input, language) in cases {
        let provider = JiffProvider::at(now.clone());
        let expr = parse(input, language).unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
        let result = provider.parse_expression(expr);
        assert!(
            matches!(result, Err(TempsError::DateCalculationError { .. })),
            "{input:?} from {now} lands before Timestamp::MIN, got {result:?}"
        );
    }

    // Half a second later, the same expressions land just inside the range.
    let first_instant_and_a_bit = |millis| {
        jiff::Timestamp::MIN
            .checked_add(jiff::SignedDuration::from_millis(millis))
            .unwrap()
    };
    let cases = [
        (
            utc_at(date(-9999, 1, 3).at(1, 59, 59, 200_000_000)),
            "1 day ago",
            200,
        ),
        (
            utc_at(date(-9999, 2, 2).at(1, 59, 59, 200_000_000)),
            "1 month ago",
            200,
        ),
        (
            utc_at(date(2024, 1, 2).at(1, 59, 59, 500_000_000)),
            "12023 years ago",
            500,
        ),
    ];
    for (now, input, millis) in cases {
        let provider = JiffProvider::at(now);
        assert_eq!(
            resolve(&provider, input, Language::English).timestamp(),
            first_instant_and_a_bit(millis),
            "{input:?}"
        );
    }
}

#[test]
fn day_at_time_can_reach_back_to_the_first_representable_day() {
    let first_day_at_22 = date(-9999, 1, 2)
        .at(22, 0, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap();

    // One day after the first representable day.
    let provider = JiffProvider::at(utc(-9999, 1, 3, 10, 0));
    assert_eq!(
        resolve(&provider, "yesterday at 22:00", Language::English),
        first_day_at_22
    );
    assert_eq!(
        resolve(&provider, "gestern um 22:00", Language::German),
        first_day_at_22
    );

    // -9999-01-02 is a Tuesday; the pinned day is the following Monday.
    let provider = JiffProvider::at(utc(-9999, 1, 8, 8, 0));
    assert_eq!(
        resolve(&provider, "last tuesday at 22:00", Language::English),
        first_day_at_22
    );
}

// ===== Absolute times =====

#[test]
fn an_hour_without_a_minute_is_honoured_rather_than_collapsing_to_midnight() {
    let provider = JiffProvider::new();

    let result = provider
        .parse_expression(TimeExpression::Absolute(AbsoluteTime {
            year: 2024,
            month: 6,
            day: 15,
            hour: Some(14),
            minute: None,
            second: None,
            nanosecond: None,
            timezone: None,
        }))
        .expect("an absolute time with only an hour should resolve");

    assert_eq!(result.date().to_string(), "2024-06-15");
    assert_eq!(result.hour(), 14, "the supplied hour was dropped");
    assert_eq!(result.minute(), 0);
    assert_eq!(result.second(), 0);
}

#[test]
fn an_hour_without_a_minute_is_honoured_in_an_explicit_utc_offset() {
    let provider = JiffProvider::new();

    let result = provider
        .parse_expression(TimeExpression::Absolute(AbsoluteTime {
            year: 2024,
            month: 6,
            day: 15,
            hour: Some(14),
            minute: None,
            second: None,
            nanosecond: None,
            timezone: Some(Timezone::Utc),
        }))
        .expect("an absolute UTC time with only an hour should resolve");

    let expected = utc(2024, 6, 15, 14, 0);
    assert_eq!(
        result.timestamp(),
        expected.timestamp(),
        "the supplied hour was dropped"
    );
}

#[test]
fn a_minute_without_an_hour_is_an_error() {
    let provider = JiffProvider::new();

    let result = provider.parse_expression(TimeExpression::Absolute(AbsoluteTime {
        year: 2024,
        month: 6,
        day: 15,
        hour: None,
        minute: Some(30),
        second: None,
        nanosecond: None,
        timezone: None,
    }));

    assert!(
        matches!(result, Err(TempsError::InvalidTime { minute: 30, .. })),
        "a minute with no hour should be rejected, got {result:?}"
    );
}

#[test]
fn a_date_only_absolute_time_is_local_midnight() {
    let provider = JiffProvider::new();

    let result = provider
        .parse_expression(TimeExpression::Absolute(AbsoluteTime {
            year: 2024,
            month: 6,
            day: 15,
            hour: None,
            minute: None,
            second: None,
            nanosecond: None,
            timezone: None,
        }))
        .unwrap();

    assert_eq!(result.date().to_string(), "2024-06-15");
    assert_eq!(result.hour(), 0);
    assert_eq!(result.minute(), 0);
}

#[test]
fn rfc3339_input_round_trips_to_the_same_instant() {
    let provider = JiffProvider::new();

    let cases = [
        ("2024-01-15T14:30:00Z", "2024-01-15T14:30:00Z"),
        ("2024-01-15T14:30:00+02:00", "2024-01-15T12:30:00Z"),
        ("2024-01-15T14:30:00.123Z", "2024-01-15T14:30:00.123Z"),
    ];

    for (input, expected) in cases {
        let resolved = resolve(&provider, input, Language::English);
        let expected: jiff::Timestamp = expected.parse().unwrap();
        assert_eq!(
            resolved.timestamp(),
            expected,
            "wrong instant for {input:?}"
        );
    }
}

// ===== Day references around DST transitions =====

#[test]
fn day_references_use_calendar_days_across_spring_forward() {
    // 23:30 the evening before a spring-forward: a fixed 24-hour step would
    // skip a day, because the following local day is only 23 hours long.
    let provider = JiffProvider::at(at_zone("America/New_York", 2024, 3, 9, 23, 30));

    let cases = [
        ("today", "2024-03-09"),
        ("tomorrow", "2024-03-10"),
        ("yesterday", "2024-03-08"),
        ("the day after tomorrow", "2024-03-11"),
        ("the day before yesterday", "2024-03-07"),
    ];

    for (input, expected) in cases {
        let resolved = resolve(&provider, input, Language::English);
        assert_eq!(
            resolved.date().to_string(),
            expected,
            "wrong date for {input:?}"
        );
        assert_eq!(resolved.hour(), 0, "{input:?} should be local midnight");
        assert_eq!(resolved.time_zone().iana_name(), Some("America/New_York"));
    }
}

#[test]
fn day_references_use_calendar_days_across_fall_back() {
    // The local day 2024-11-03 in New York is 25 hours long.
    let provider = JiffProvider::at(at_zone("America/New_York", 2024, 11, 3, 23, 30));

    let cases = [
        ("today", "2024-11-03"),
        ("tomorrow", "2024-11-04"),
        ("yesterday", "2024-11-02"),
        ("the day after tomorrow", "2024-11-05"),
        ("the day before yesterday", "2024-11-01"),
    ];

    for (input, expected) in cases {
        let resolved = resolve(&provider, input, Language::English);
        assert_eq!(
            resolved.date().to_string(),
            expected,
            "wrong date for {input:?}"
        );
        assert_eq!(resolved.hour(), 0, "{input:?} should be local midnight");
    }
}

#[test]
fn german_day_references_use_calendar_days_across_the_eu_transition() {
    // Europe/Berlin springs forward on 2024-03-31 at 02:00 local.
    let provider = JiffProvider::at(at_zone("Europe/Berlin", 2024, 3, 30, 23, 30));

    let cases = [
        ("heute", "2024-03-30"),
        ("morgen", "2024-03-31"),
        ("gestern", "2024-03-29"),
        ("übermorgen", "2024-04-01"),
        ("Übermorgen", "2024-04-01"),
        ("vorgestern", "2024-03-28"),
    ];

    for (input, expected) in cases {
        let resolved = resolve(&provider, input, Language::German);
        assert_eq!(
            resolved.date().to_string(),
            expected,
            "wrong date for {input:?}"
        );
        assert_eq!(resolved.hour(), 0, "{input:?} should be local midnight");
    }
}

#[test]
fn a_nonexistent_local_midnight_shifts_forward_by_the_gap() {
    // Cuba springs forward at midnight: 2024-03-10 00:00 does not exist and
    // 01:00 is the first instant of the day.
    let provider = JiffProvider::at(at_zone("America/Havana", 2024, 3, 10, 12, 0));

    let today = resolve(&provider, "today", Language::English);
    assert_eq!(today.date().to_string(), "2024-03-10");
    assert_eq!(
        today.hour(),
        1,
        "a nonexistent midnight should shift forward by the gap"
    );

    // Reaching the same day from the day before must agree.
    let eve = JiffProvider::at(at_zone("America/Havana", 2024, 3, 9, 22, 0));
    assert_eq!(resolve(&eve, "tomorrow", Language::English), today);
}

#[test]
fn an_ambiguous_local_midnight_resolves_to_the_earlier_instant() {
    // Cuba falls back at 01:00 on 2024-11-03, so local midnight occurs twice.
    let provider = JiffProvider::at(at_zone("America/Havana", 2024, 11, 3, 12, 0));

    let today = resolve(&provider, "today", Language::English);
    assert_eq!(today.date().to_string(), "2024-11-03");
    assert_eq!(today.hour(), 0);
    assert_eq!(
        today.offset(),
        jiff::tz::offset(-4),
        "an ambiguous midnight should pick the earlier (pre-transition) instant"
    );
}

#[test]
fn a_day_skipped_at_the_date_line_does_not_error() {
    // Pacific/Apia jumped straight from 2011-12-29 to 2011-12-31; the whole
    // local day 2011-12-30 is missing.
    let provider = JiffProvider::at(at_zone("Pacific/Apia", 2011, 12, 29, 12, 0));

    let tomorrow = provider
        .parse_expression(TimeExpression::Day(DayReference::Tomorrow))
        .expect("a skipped calendar day must not be an error");

    assert_eq!(
        tomorrow.date().to_string(),
        "2011-12-31",
        "the whole skipped day should shift forward by the gap"
    );

    // A time on the skipped day shifts forward by the same gap.
    let afternoon = resolve(&provider, "tomorrow at 15:00", Language::English);
    assert_eq!(afternoon.date().to_string(), "2011-12-31");
    assert_eq!(afternoon.hour(), 15);
}

// ===== Weekdays =====

#[test]
fn weekday_references_resolve_relative_to_the_pinned_day() {
    // 2024-03-15 is a Friday.
    let provider = JiffProvider::at(utc(2024, 3, 15, 10, 30));

    let cases = [
        ("monday", "2024-03-18"),
        ("next monday", "2024-03-18"),
        ("last monday", "2024-03-11"),
        ("friday", "2024-03-15"),
        ("next friday", "2024-03-22"),
        ("last friday", "2024-03-08"),
        ("sunday", "2024-03-17"),
    ];

    for (input, expected) in cases {
        let resolved = resolve(&provider, input, Language::English);
        assert_eq!(
            resolved.date().to_string(),
            expected,
            "wrong date for {input:?}"
        );
        assert_eq!(resolved.hour(), 0);
    }
}

#[test]
fn weekday_references_stay_on_calendar_days_across_a_transition() {
    // Friday 2024-03-08 in New York; the following Sunday is the short day.
    let provider = JiffProvider::at(at_zone("America/New_York", 2024, 3, 8, 20, 0));

    let cases = [
        ("sunday", "2024-03-10"),
        ("monday", "2024-03-11"),
        ("next friday", "2024-03-15"),
    ];

    for (input, expected) in cases {
        let resolved = resolve(&provider, input, Language::English);
        assert_eq!(
            resolved.date().to_string(),
            expected,
            "wrong date for {input:?}"
        );
        assert_eq!(resolved.hour(), 0);
    }
}

// ===== Times and day-at-time =====

#[test]
fn times_resolve_on_the_pinned_day() {
    let provider = JiffProvider::at(utc(2024, 3, 15, 10, 30));

    let cases = [
        ("3:30 pm", 15, 30),
        ("10:15 am", 10, 15),
        ("14:30", 14, 30),
        ("9:00 PM", 21, 0),
        ("12:00 PM", 12, 0),
        ("12:00 AM", 0, 0),
    ];

    for (input, hour, minute) in cases {
        let resolved = resolve(&provider, input, Language::English);
        assert_eq!(resolved.date().to_string(), "2024-03-15");
        assert_eq!(resolved.hour(), hour, "wrong hour for {input:?}");
        assert_eq!(resolved.minute(), minute, "wrong minute for {input:?}");
    }
}

#[test]
fn day_at_time_combines_the_calendar_day_with_the_time() {
    let provider = JiffProvider::at(utc(2024, 3, 15, 10, 30));

    let resolved = resolve(&provider, "tomorrow at 3:30 pm", Language::English);
    assert_eq!(resolved.date().to_string(), "2024-03-16");
    assert_eq!(resolved.hour(), 15);
    assert_eq!(resolved.minute(), 30);

    let resolved = resolve(&provider, "next monday at 9:00 am", Language::English);
    assert_eq!(resolved.date().to_string(), "2024-03-18");
    assert_eq!(resolved.hour(), 9);
    assert_eq!(resolved.minute(), 0);
}

#[test]
fn day_at_time_across_spring_forward_keeps_the_calendar_day() {
    let provider = JiffProvider::at(at_zone("America/New_York", 2024, 3, 9, 23, 30));

    let resolved = resolve(&provider, "tomorrow at 3:30 pm", Language::English);
    assert_eq!(resolved.date().to_string(), "2024-03-10");
    assert_eq!(resolved.hour(), 15);
    assert_eq!(resolved.minute(), 30);
    assert_eq!(resolved.offset(), jiff::tz::offset(-4), "should be on DST");
}

// ===== later today =====

#[test]
fn later_today_advances_two_hours_when_that_stays_on_the_same_day() {
    let base = utc(2024, 3, 15, 10, 30);
    let provider = JiffProvider::at(base.clone());

    let resolved = resolve(&provider, "later today", Language::English);
    assert_eq!(resolved, base.checked_add(Span::new().hours(2)).unwrap());
}

#[test]
fn later_today_never_leaves_today_and_never_goes_backwards() {
    let base = utc(2024, 3, 15, 23, 30);
    let provider = JiffProvider::at(base.clone());

    let resolved = resolve(&provider, "later today", Language::English);
    assert!(resolved >= base, "later today must not move into the past");
    assert_eq!(
        resolved.date().to_string(),
        "2024-03-15",
        "later today must not cross midnight"
    );
}

// ===== Calendar dates =====

#[test]
fn calendar_dates_resolve_to_local_midnight() {
    let provider = JiffProvider::new();

    let cases = [
        ("15/03/2024", "2024-03-15"),
        ("31-12-2025", "2025-12-31"),
        ("01/01/2023", "2023-01-01"),
    ];

    for (input, expected) in cases {
        let resolved = resolve(&provider, input, Language::English);
        assert_eq!(resolved.date().to_string(), expected);
        assert_eq!(resolved.hour(), 0);
        assert_eq!(resolved.minute(), 0);
    }
}

// ===== Programmatic rejection =====

#[test]
fn invalid_programmatic_inputs_are_rejected() {
    let provider = JiffProvider::new();

    let invalid_time = TimeExpression::Time(Time {
        hour: 0,
        minute: 30,
        second: 0,
        meridiem: Some(Meridiem::PM),
    });
    assert!(matches!(
        provider.parse_expression(invalid_time),
        Err(TempsError::InvalidTime { hour: 0, .. })
    ));

    let invalid_timezone = TimeExpression::Absolute(AbsoluteTime {
        year: 2024,
        month: 1,
        day: 15,
        hour: Some(12),
        minute: Some(0),
        second: Some(0),
        nanosecond: None,
        timezone: Some(Timezone::Offset {
            total_minutes: -750,
        }),
    });
    assert!(matches!(
        provider.parse_expression(invalid_timezone),
        Err(TempsError::InvalidTimezoneOffset {
            total_minutes: -750
        })
    ));

    let negative_relative = TimeExpression::Relative(RelativeTime {
        amount: -1,
        unit: TimeUnit::Hour,
        direction: Direction::Future,
    });
    assert!(matches!(
        provider.parse_expression(negative_relative),
        Err(TempsError::DateCalculationError { .. })
    ));
}

#[test]
fn the_convenience_function_uses_the_system_clock() {
    let before = Zoned::now();
    let resolved = parse_to_zoned("now", Language::English).unwrap();
    let after = Zoned::now();

    assert!(resolved >= before && resolved <= after);

    // And it still resolves the whole grammar.
    for (input, language) in [
        ("tomorrow", Language::English),
        ("in 5 minutes", Language::English),
        ("morgen um 15:30", Language::German),
    ] {
        assert!(
            parse_to_zoned(input, language).is_ok(),
            "{input:?} should resolve"
        );
    }
}

#[test]
fn civil_datetime_input_is_accepted_by_the_pinned_provider() {
    // Guards the `DateTime` -> `Zoned` construction used to pin the provider.
    let fixed = DateTime::constant(2024, 3, 15, 10, 30, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap();
    let provider = JiffProvider::at(fixed.clone());
    assert_eq!(provider.now(), fixed);
}

// ===== Known upstream issues =====
//
// jiff 0.2.37 truncates a negative Unix timestamp toward zero when it looks up
// a zone's offset, so a pre-1970 instant in the last second before a transition
// is given the offset that applies after the transition. It also range-checks
// only the whole second when it converts a civil datetime to an instant. These
// tests state the correct behaviour and should pass once jiff fixes each bug.

#[test]
#[ignore = "upstream jiff bug: TZif lookup truncates negative sub-second timestamps"]
fn upstream_jiff_bug_pre_1970_sub_second_instant_gets_the_offset_in_force() {
    // New York fell back at 1961-10-29T06:00:00Z (02:00 EDT became 01:00 EST),
    // so half a second earlier it was still EDT.
    let new_york = TimeZone::get("America/New_York").unwrap();
    let instant: jiff::Timestamp = "1961-10-29T05:59:59.5Z".parse().unwrap();
    assert_eq!(new_york.to_offset(instant), jiff::tz::offset(-4));

    // Havana sprang forward at 1965-06-01T05:00:00Z (00:00 CST became 01:00
    // CDT). Half a second earlier the local time was 23:59:59.5 on May 31, so
    // "today" is that day's midnight, which is never after now.
    let havana = TimeZone::get("America/Havana").unwrap();
    let now = "1965-06-01T04:59:59.5Z"
        .parse::<jiff::Timestamp>()
        .unwrap()
        .to_zoned(havana);
    let provider = JiffProvider::at(now.clone());

    let today = resolve(&provider, "today", Language::English);
    assert_eq!(today.date().to_string(), "1965-05-31");
    assert!(today <= now, "today ({today}) is after now ({now})");
}

#[test]
#[ignore = "upstream jiff bug: TZif lookup truncates negative sub-second timestamps"]
fn upstream_jiff_bug_later_today_before_a_pre_1970_midnight_transition_stays_today() {
    // Tokyo sprang forward at local midnight on 1950-05-07 (00:00 became 01:00).
    // "later today" clamps to one nanosecond before that midnight, which is
    // still 23:59:59.999999999+09:00 on May 6.
    let provider = JiffProvider::at(at_zone("Asia/Tokyo", 1950, 5, 6, 23, 0));

    let later = resolve(&provider, "later today", Language::English);
    assert_eq!(later.date().to_string(), "1950-05-06");
    assert_eq!(later.offset(), jiff::tz::offset(9));
}

#[test]
#[ignore = "upstream jiff bug: civil-to-instant conversion range-checks only the whole second"]
fn upstream_jiff_bug_civil_datetime_just_before_timestamp_min_is_rejected() {
    // One nanosecond before `Timestamp::MIN` (`-009999-01-02T01:59:59Z`). jiff
    // checks only the whole second, which is in range, so a debug build panics
    // in an assertion and a release build accepts the datetime. temps guards
    // against this in its calendar arithmetic; see
    // calendar_arithmetic_into_the_second_before_the_first_instant_fails_cleanly.
    let just_before = date(-9999, 1, 2).at(1, 59, 58, 999_999_999);
    assert!(just_before.to_zoned(TimeZone::UTC).is_err());

    // Calendar arithmetic goes through the same conversion.
    let a_day_later = date(-9999, 1, 3)
        .at(1, 59, 58, 999_999_999)
        .to_zoned(TimeZone::UTC)
        .unwrap();
    assert!(a_day_later.checked_sub(Span::new().days(1)).is_err());
}
