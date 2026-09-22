//! Integration tests for the chrono backend.
//!
//! Every test drives the real [`ChronoProvider`]. Time-dependent behaviour is
//! pinned with `ChronoProvider::at(...)` instead of being reimplemented against
//! a mock: a mock that duplicates the provider's arithmetic can only ever
//! assert that the duplicate is self-consistent, and drifts away from the code
//! it is supposed to guard.

use chrono::{
    DateTime, Datelike, Days, Duration, Local, NaiveDate, NaiveDateTime, Offset, TimeZone,
    Timelike, Utc,
};
use temps_chrono::{ChronoProvider, parse_to_datetime};
use temps_core::*;
use temps_testhelpers::chrono::{fixed_datetime, test_dates};

// ===== Helpers =====

/// Parse `input` and resolve it against `provider`, panicking with the input on
/// failure so a broken case names itself.
fn resolve(provider: &ChronoProvider, input: &str, language: Language) -> DateTime<Local> {
    let expr =
        parse(input, language).unwrap_or_else(|error| panic!("failed to parse {input:?}: {error}"));
    provider
        .parse_expression(expr)
        .unwrap_or_else(|error| panic!("failed to resolve {input:?}: {error}"))
}

/// Resolve `input` without unwrapping, for cases expected to fail.
fn try_resolve(
    provider: &ChronoProvider,
    input: &str,
    language: Language,
) -> Result<DateTime<Local>> {
    provider.parse_expression(parse(input, language)?)
}

/// Assert that `input` is rejected as an arithmetic overflow: the error every
/// result outside chrono's range is reported as.
fn assert_overflow(provider: &ChronoProvider, input: &str, language: Language) {
    let result = try_resolve(provider, input, language);
    assert!(
        matches!(result, Err(TempsError::ArithmeticOverflow { .. })),
        "expected overflow for {input:?}, got {result:?}"
    );
}

/// Assert that `input` is rejected as an arithmetic overflow whose message is
/// `expected`, so the error names the real cause.
fn assert_overflow_saying(
    provider: &ChronoProvider,
    input: &str,
    language: Language,
    expected: &str,
) {
    match try_resolve(provider, input, language) {
        Err(TempsError::ArithmeticOverflow { operation }) => {
            assert_eq!(operation, expected, "wrong overflow message for {input:?}")
        }
        other => panic!("expected overflow for {input:?}, got {other:?}"),
    }
}

/// A wall-clock time on chrono's last date that names exactly one instant in
/// every zone: the far future runs on today's offsets, which span UTC-12 to
/// UTC+14, so 10:00 is between 20:00 UTC the day before and 22:00 UTC.
fn on_the_last_date() -> DateTime<Local> {
    local_instant(NaiveDate::MAX.and_hms_opt(10, 0, 0).expect("valid time"))
}

/// A wall-clock time on chrono's first date that names exactly one instant in
/// every zone. The far past runs on local mean time, whose offsets span
/// -15:56 (Asia/Manila) to +15:13:42 (America/Metlakatla), so 16:00 is after
/// the first UTC instant everywhere.
fn on_the_first_date() -> DateTime<Local> {
    local_instant(NaiveDate::MIN.and_hms_opt(16, 0, 0).expect("valid time"))
}

/// The local instant with the given wall-clock reading, for readings that are
/// neither skipped nor repeated in the zone the test runs in.
fn local_instant(naive: NaiveDateTime) -> DateTime<Local> {
    Local
        .from_local_datetime(&naive)
        .single()
        .unwrap_or_else(|| panic!("{naive} should name exactly one local instant"))
}

/// The local instant whose UTC reading is the given clock time.
///
/// Naming an instant by its UTC reading is unambiguous in every zone, which
/// matters where the local wall clock is ambiguous or skipped.
fn instant_at_utc(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
    Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
        .expect("UTC clock reading is always unambiguous")
        .with_timezone(&Local)
}

/// A mid-June instant, far from any zone's daylight-saving transition, for
/// tests whose expectations must hold in whatever zone the suite runs in.
fn quiet_instant() -> DateTime<Local> {
    fixed_datetime(2024, 6, 12, 10, 30, 15)
}

fn every_unit() -> [TimeUnit; 7] {
    [
        TimeUnit::Second,
        TimeUnit::Minute,
        TimeUnit::Hour,
        TimeUnit::Day,
        TimeUnit::Week,
        TimeUnit::Month,
        TimeUnit::Year,
    ]
}

// ===== Provider basics =====

#[test]
fn default_provider_reads_the_system_clock() {
    let provider = ChronoProvider::new();
    let before = Local::now();
    let now = provider.now();
    let after = Local::now();

    assert!(now >= before && now <= after, "now() must read the clock");

    let parsed = provider.parse_expression(TimeExpression::Now).unwrap();
    assert!(
        parsed.signed_duration_since(now).num_seconds().abs() < 1,
        "'now' should resolve within a second of the clock"
    );
}

#[test]
fn pinned_provider_resolves_now_to_its_own_instant() {
    let base = quiet_instant();
    let provider = ChronoProvider::at(base);

    assert_eq!(provider.now(), base);
    assert_eq!(
        provider.parse_expression(TimeExpression::Now).unwrap(),
        base
    );
    assert_eq!(resolve(&provider, "now", Language::English), base);
    assert_eq!(resolve(&provider, "jetzt", Language::German), base);
}

// ===== Relative expressions =====

#[test]
fn english_sub_day_offsets_shift_by_an_exact_duration() {
    let base = quiet_instant();
    let provider = ChronoProvider::at(base);

    let cases = [
        ("in 30 seconds", Duration::seconds(30)),
        ("45 seconds ago", Duration::seconds(-45)),
        ("in 5 minutes", Duration::minutes(5)),
        ("10 minutes ago", Duration::minutes(-10)),
        ("in a minute", Duration::minutes(1)),
        ("in 2 hours", Duration::hours(2)),
        ("3 hours ago", Duration::hours(-3)),
        ("an hour ago", Duration::hours(-1)),
        ("in a second", Duration::seconds(1)),
        ("in an hour", Duration::hours(1)),
        ("a minute ago", Duration::minutes(-1)),
    ];

    for (input, offset) in cases {
        assert_eq!(
            resolve(&provider, input, Language::English),
            base + offset,
            "wrong result for {input:?}"
        );
    }
}

#[test]
fn german_sub_day_offsets_shift_by_an_exact_duration() {
    let base = quiet_instant();
    let provider = ChronoProvider::at(base);

    let cases = [
        ("in 30 Sekunden", Duration::seconds(30)),
        ("vor 45 Sekunden", Duration::seconds(-45)),
        ("in 5 Minuten", Duration::minutes(5)),
        ("vor 10 Minuten", Duration::minutes(-10)),
        ("in einer Minute", Duration::minutes(1)),
        ("in 2 Stunden", Duration::hours(2)),
        ("vor 3 Stunden", Duration::hours(-3)),
        ("in einer Sekunde", Duration::seconds(1)),
        ("in einer Stunde", Duration::hours(1)),
        ("vor einer Sekunde", Duration::seconds(-1)),
    ];

    for (input, offset) in cases {
        assert_eq!(
            resolve(&provider, input, Language::German),
            base + offset,
            "wrong result for {input:?}"
        );
    }
}

/// Day and week offsets are calendar arithmetic, not fixed multiples of 24
/// hours: the wall-clock time is carried over to the target date.
#[test]
fn day_and_week_offsets_move_the_calendar_date_and_keep_the_wall_clock() {
    let base = quiet_instant();
    let provider = ChronoProvider::at(base);

    let cases = [
        ("in 1 day", Language::English, 1_i64),
        ("2 days ago", Language::English, -2),
        ("in a day", Language::English, 1),
        ("in one day", Language::English, 1),
        ("a day ago", Language::English, -1),
        ("a couple of days ago", Language::English, -2),
        ("in 1 week", Language::English, 7),
        ("2 weeks ago", Language::English, -14),
        ("in a week", Language::English, 7),
        ("one week ago", Language::English, -7),
        ("in 1 Tag", Language::German, 1),
        ("vor 2 Tagen", Language::German, -2),
        ("in einem Tag", Language::German, 1),
        ("vor einem Tag", Language::German, -1),
        ("in 1 Woche", Language::German, 7),
        ("vor 2 Wochen", Language::German, -14),
        ("in einer Woche", Language::German, 7),
    ];

    for (input, language, days) in cases {
        let result = resolve(&provider, input, language);
        let expected_date = if days >= 0 {
            base.date_naive().checked_add_days(Days::new(days as u64))
        } else {
            base.date_naive()
                .checked_sub_days(Days::new(days.unsigned_abs()))
        }
        .expect("date within range");

        assert_eq!(
            result.date_naive(),
            expected_date,
            "wrong date for {input:?}"
        );
        assert_eq!(result.time(), base.time(), "wall clock moved for {input:?}");
    }
}

#[test]
fn month_arithmetic_clamps_to_the_end_of_a_shorter_month() {
    // January 31 + 1 month lands on the last day of February.
    let provider = ChronoProvider::at(test_dates::jan_31_2024());
    let result = resolve(&provider, "in 1 month", Language::English);
    assert_eq!(
        (result.year(), result.month(), result.day()),
        (2024, 2, 29),
        "2024 is a leap year"
    );

    let provider = ChronoProvider::at(fixed_datetime(2023, 1, 31, 10, 0, 0));
    let result = resolve(&provider, "in einem Monat", Language::German);
    assert_eq!((result.year(), result.month(), result.day()), (2023, 2, 28));
}

#[test]
fn year_arithmetic_clamps_a_leap_day() {
    let provider = ChronoProvider::at(test_dates::feb_29_2024());
    let result = resolve(&provider, "in 1 year", Language::English);
    assert_eq!((result.year(), result.month(), result.day()), (2025, 2, 28));

    let result = resolve(&provider, "1 year ago", Language::English);
    assert_eq!((result.year(), result.month(), result.day()), (2023, 2, 28));
}

#[test]
fn month_arithmetic_crosses_the_year_boundary() {
    let provider = ChronoProvider::at(fixed_datetime(2023, 10, 15, 9, 0, 0));

    let result = resolve(&provider, "in 6 months", Language::English);
    assert_eq!((result.year(), result.month(), result.day()), (2024, 4, 15));

    let result = resolve(&provider, "in 18 months", Language::English);
    assert_eq!((result.year(), result.month(), result.day()), (2025, 4, 15));

    let result = resolve(&provider, "6 months ago", Language::English);
    assert_eq!((result.year(), result.month(), result.day()), (2023, 4, 15));

    let result = resolve(&provider, "in 5 years", Language::English);
    assert_eq!(
        (result.year(), result.month(), result.day()),
        (2028, 10, 15)
    );

    let result = resolve(&provider, "10 years ago", Language::English);
    assert_eq!(
        (result.year(), result.month(), result.day()),
        (2013, 10, 15)
    );
}

/// A zero amount is the identity, down to the nanosecond — no rounding to the
/// second, no trip through a date and back.
#[test]
fn a_zero_amount_resolves_to_exactly_now() {
    let base = quiet_instant()
        .with_nanosecond(123_456_789)
        .expect("valid nanosecond");
    let provider = ChronoProvider::at(base);

    for input in [
        "in 0 seconds",
        "0 seconds ago",
        "in 0 minutes",
        "in 0 hours",
        "in 0 days",
        "0 days ago",
        "in 0 weeks",
        "in 0 months",
        "in 0 years",
    ] {
        let result = resolve(&provider, input, Language::English);
        assert_eq!(result, base, "{input:?} should be the identity");
        assert_eq!(
            result.nanosecond(),
            base.nanosecond(),
            "{input:?} dropped sub-second precision"
        );
    }
}

// ===== Overflow =====

/// `digit_number` accepts any run of digits, so an amount far outside chrono's
/// range reaches the provider. It must come back as an error: this is a
/// `Result`-returning API and used to abort the process instead.
#[test]
fn huge_amounts_report_overflow_for_every_unit_and_direction() {
    let provider = ChronoProvider::at(quiet_instant());

    for unit in every_unit() {
        for direction in [Direction::Past, Direction::Future] {
            let expr = TimeExpression::Relative(RelativeTime {
                amount: i64::MAX,
                unit,
                direction,
            });

            let result = provider.parse_expression(expr);
            assert!(
                matches!(result, Err(TempsError::ArithmeticOverflow { .. })),
                "expected overflow for i64::MAX {unit:?} {direction:?}, got {result:?}"
            );
        }
    }
}

#[test]
fn huge_amounts_in_parsed_input_report_overflow_rather_than_panicking() {
    let provider = ChronoProvider::at(quiet_instant());

    let cases = [
        ("999999999999 days ago", Language::English),
        ("in 999999999999 days", Language::English),
        ("999999999999 weeks ago", Language::English),
        ("in 999999999999 weeks", Language::English),
        ("999999999999 months ago", Language::English),
        ("in 999999999999 months", Language::English),
        ("999999999999 years ago", Language::English),
        ("in 999999999999 years", Language::English),
        ("999999999999 hours ago", Language::English),
        ("in 999999999999 hours", Language::English),
        ("999999999999 minutes ago", Language::English),
        ("999999999999999 seconds ago", Language::English),
        ("in 999999999999999 seconds", Language::English),
        ("in 9223372036854775807 days", Language::English),
        ("vor 999999999999 Tagen", Language::German),
        ("in 999999999999 Wochen", Language::German),
    ];

    for (input, language) in cases {
        let result = try_resolve(&provider, input, language);
        assert!(
            matches!(result, Err(TempsError::ArithmeticOverflow { .. })),
            "expected overflow for {input:?}, got {result:?}"
        );
    }

    // The same must hold for the convenience entry point, which resolves
    // against the system clock.
    assert!(matches!(
        parse_to_datetime("999999999999 days ago", Language::English),
        Err(TempsError::ArithmeticOverflow { .. })
    ));
}

/// Month and year amounts small enough to count, but whose result is past
/// the ±262,143 years chrono can represent, are range overflows like every
/// other unit — not a "date calculation" failure.
#[test]
fn month_and_year_offsets_past_the_range_report_overflow() {
    let provider = ChronoProvider::at(quiet_instant());

    for (input, language) in [
        ("in 300000 years", Language::English),
        ("300000 years ago", Language::English),
        ("in 3600000 months", Language::English),
        ("3600000 months ago", Language::English),
        ("in 300000 Jahren", Language::German),
        ("vor 300000 Jahren", Language::German),
        ("in 3600000 Monaten", Language::German),
    ] {
        assert_overflow_saying(&provider, input, language, errors::ERR_AMOUNT_OUT_OF_RANGE);
    }

    assert!(matches!(
        parse_to_datetime("in 300000 years", Language::English),
        Err(TempsError::ArithmeticOverflow { .. })
    ));

    // One month past either end of the range, from its last and first dates.
    let last = ChronoProvider::at(on_the_last_date());
    for input in ["in 1 month", "in 1 year", "in 12 months"] {
        assert_overflow_saying(
            &last,
            input,
            Language::English,
            errors::ERR_AMOUNT_OUT_OF_RANGE,
        );
    }
    let first = ChronoProvider::at(on_the_first_date());
    for input in ["1 month ago", "1 year ago"] {
        assert_overflow_saying(
            &first,
            input,
            Language::English,
            errors::ERR_AMOUNT_OUT_OF_RANGE,
        );
    }
    assert_overflow_saying(
        &first,
        "vor einem Monat",
        Language::German,
        errors::ERR_AMOUNT_OUT_OF_RANGE,
    );
}

/// A day reference that runs off either end of chrono's calendar is a range
/// overflow, and its message does not blame a relative amount the input does
/// not contain.
#[test]
fn day_references_off_the_calendar_report_overflow() {
    let last = ChronoProvider::at(on_the_last_date());
    for (input, language) in [
        ("tomorrow", Language::English),
        ("day after tomorrow", Language::English),
        ("tomorrow at 10:00", Language::English),
        ("tomorrow morning", Language::English),
        ("next monday", Language::English),
        ("next sunday at 9:00 am", Language::English),
        ("morgen", Language::German),
        ("übermorgen", Language::German),
        ("morgen um 10:00", Language::German),
    ] {
        assert_overflow_saying(&last, input, language, errors::ERR_RESULT_OUT_OF_RANGE);
    }
    // Still today: nothing to overflow.
    assert_eq!(
        resolve(&last, "today at 11:00", Language::English).date_naive(),
        NaiveDate::MAX
    );

    let first = ChronoProvider::at(on_the_first_date());
    for (input, language) in [
        ("yesterday", Language::English),
        ("the day before yesterday", Language::English),
        ("yesterday at 3 pm", Language::English),
        ("last monday", Language::English),
        ("last friday evening", Language::English),
        ("gestern", Language::German),
        ("vorgestern", Language::German),
        ("gestern um 15:00", Language::German),
    ] {
        assert_overflow_saying(&first, input, language, errors::ERR_RESULT_OUT_OF_RANGE);
    }
    assert_eq!(
        resolve(&first, "today at 5 pm", Language::English).date_naive(),
        NaiveDate::MIN
    );
}

// ===== Day references =====

#[test]
fn day_references_resolve_to_midnight_of_the_right_date() {
    let base = quiet_instant();
    let provider = ChronoProvider::at(base);

    let cases = [
        ("today", Language::English, 0_i64),
        ("yesterday", Language::English, -1),
        ("tomorrow", Language::English, 1),
        ("day after tomorrow", Language::English, 2),
        ("the day before yesterday", Language::English, -2),
        ("heute", Language::German, 0),
        ("gestern", Language::German, -1),
        ("morgen", Language::German, 1),
        ("übermorgen", Language::German, 2),
        ("vorgestern", Language::German, -2),
    ];

    for (input, language, days) in cases {
        let result = resolve(&provider, input, language);
        let expected_date = if days >= 0 {
            base.date_naive().checked_add_days(Days::new(days as u64))
        } else {
            base.date_naive()
                .checked_sub_days(Days::new(days.unsigned_abs()))
        }
        .expect("date within range");

        assert_eq!(
            result.date_naive(),
            expected_date,
            "wrong date for {input:?}"
        );
        assert_eq!(
            (result.hour(), result.minute(), result.second()),
            (0, 0, 0),
            "{input:?} should be midnight"
        );
    }
}

#[test]
fn weekday_references_resolve_to_the_expected_dates() {
    // 2024-06-12 is a Wednesday.
    let provider = ChronoProvider::at(quiet_instant());

    let cases = [
        ("monday", 17),
        ("tuesday", 18),
        ("wednesday", 12),
        ("thursday", 13),
        ("friday", 14),
        ("saturday", 15),
        ("sunday", 16),
        ("mon", 17),
        ("tue", 18),
        ("wed", 12),
        ("thu", 13),
        ("fri", 14),
        ("sat", 15),
        ("sun", 16),
        ("next wednesday", 19),
        ("last wednesday", 5),
        ("this wednesday", 12),
        ("last monday", 10),
    ];

    for (input, day) in cases {
        let result = resolve(&provider, input, Language::English);
        assert_eq!(
            (result.year(), result.month(), result.day()),
            (2024, 6, day),
            "wrong date for {input:?}"
        );
        assert_eq!(
            (result.hour(), result.minute(), result.second()),
            (0, 0, 0),
            "{input:?} should be midnight"
        );
    }
}

/// German writes modifiers capitalised at the start of a sentence, and the
/// umlaut in "nächsten" must survive the case-insensitive match.
#[test]
fn capitalised_german_weekday_modifiers_are_accepted() {
    let provider = ChronoProvider::at(quiet_instant());

    for (input, day) in [
        ("nächsten Montag", 17),
        ("Nächsten Montag", 17),
        ("letzten Montag", 10),
        ("Letzten Montag", 10),
    ] {
        let result = resolve(&provider, input, Language::German);
        assert_eq!(
            (result.year(), result.month(), result.day()),
            (2024, 6, day),
            "wrong date for {input:?}"
        );
    }
}

#[test]
fn a_day_reference_can_be_combined_with_a_time() {
    let provider = ChronoProvider::at(quiet_instant());

    let result = resolve(&provider, "tomorrow at 3:30 pm", Language::English);
    assert_eq!((result.year(), result.month(), result.day()), (2024, 6, 13));
    assert_eq!((result.hour(), result.minute()), (15, 30));

    let result = resolve(&provider, "next monday at 9:00 am", Language::English);
    assert_eq!((result.year(), result.month(), result.day()), (2024, 6, 17));
    assert_eq!((result.hour(), result.minute()), (9, 0));

    let result = resolve(
        &provider,
        "day after tomorrow at 5:00 pm",
        Language::English,
    );
    assert_eq!((result.year(), result.month(), result.day()), (2024, 6, 14));
    assert_eq!((result.hour(), result.minute()), (17, 0));
}

// ===== Times of day =====

#[test]
fn times_resolve_on_the_pinned_day() {
    let base = quiet_instant();
    let provider = ChronoProvider::at(base);

    let cases = [
        ("3:30 pm", 15, 30),
        ("10:15 am", 10, 15),
        ("14:30", 14, 30),
        ("9:00 PM", 21, 0),
        ("12:00 PM", 12, 0),
        ("12:00 AM", 0, 0),
        ("noon", 12, 0),
        ("midnight", 0, 0),
    ];

    for (input, hour, minute) in cases {
        let result = resolve(&provider, input, Language::English);
        assert_eq!(result.date_naive(), base.date_naive(), "{input:?}");
        assert_eq!(
            (result.hour(), result.minute()),
            (hour, minute),
            "{input:?}"
        );
        assert_eq!(result.second(), 0, "{input:?}");
    }
}

/// "later today" means later *today*: it may be clamped, but it never lands on
/// another date and never resolves into the past.
#[test]
fn later_today_stays_within_the_current_day() {
    let base = fixed_datetime(2024, 6, 12, 10, 30, 0);
    let provider = ChronoProvider::at(base);
    let result = resolve(&provider, "later today", Language::English);
    assert_eq!(result, base + Duration::hours(2));

    let late = fixed_datetime(2024, 6, 12, 23, 30, 0);
    let provider = ChronoProvider::at(late);
    let result = resolve(&provider, "later today", Language::English);
    assert_eq!(
        result.date_naive(),
        late.date_naive(),
        "later today must not spill into tomorrow"
    );
    assert!(result >= late, "later today must not resolve into the past");
}

/// chrono cannot name the day after `NaiveDate::MAX`, but that is no reason to
/// fail when two hours on is still today: nothing needs clamping.
///
/// The pin is 09:00 so that the premise holds in every zone: even at UTC-12,
/// the westernmost offset in use, 11:00 local is 23:00 UTC, still within
/// chrono's range.
#[test]
fn later_today_on_the_last_date_chrono_can_name_is_two_hours_on() {
    let base = local_instant(
        NaiveDate::MAX
            .and_hms_opt(9, 0, 0)
            .expect("valid time of day"),
    );
    let two_hours_on = base
        .checked_add_signed(Duration::hours(2))
        .expect("two hours after 09:00 on the last date is in range at every offset");
    assert_eq!(two_hours_on.date_naive(), NaiveDate::MAX);
    let provider = ChronoProvider::at(base);

    assert_eq!(
        resolve(&provider, "later today", Language::English),
        two_hours_on
    );
    assert_eq!(
        provider
            .parse_expression(TimeExpression::LaterToday)
            .unwrap(),
        two_hours_on
    );
}

// ===== Absolute dates and times =====

#[test]
fn iso_datetimes_resolve_to_the_instant_they_name() {
    let provider = ChronoProvider::at(quiet_instant());

    let cases = [
        (
            "2024-01-15T14:30:00Z",
            Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap(),
        ),
        (
            "2024-01-15T14:30:00+02:00",
            Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 0).unwrap(),
        ),
        (
            "2024-01-15T14:30:00-05:00",
            Utc.with_ymd_and_hms(2024, 1, 15, 19, 30, 0).unwrap(),
        ),
        // A negative offset smaller than an hour: the sign belongs to the whole
        // offset, not to its hour component.
        (
            "2024-01-15T14:30:00-00:30",
            Utc.with_ymd_and_hms(2024, 1, 15, 15, 0, 0).unwrap(),
        ),
        (
            "2024-01-15T14:30:00+00:30",
            Utc.with_ymd_and_hms(2024, 1, 15, 14, 0, 0).unwrap(),
        ),
    ];

    for (input, expected) in cases {
        let result = resolve(&provider, input, Language::English);
        assert_eq!(result, expected, "wrong instant for {input:?}");
    }

    let result = resolve(&provider, "2024-01-15T14:30:00.123Z", Language::English);
    assert_eq!(result.nanosecond(), 123_000_000);
}

#[test]
fn dates_resolve_to_local_midnight() {
    let provider = ChronoProvider::at(quiet_instant());

    let cases = [
        ("15/03/2024", 2024, 3, 15),
        ("31-12-2025", 2025, 12, 31),
        ("01/01/2023", 2023, 1, 1),
    ];

    for (input, year, month, day) in cases {
        let result = resolve(&provider, input, Language::English);
        assert_eq!(
            (result.year(), result.month(), result.day()),
            (year, month, day),
            "wrong date for {input:?}"
        );
        assert_eq!(
            (result.hour(), result.minute(), result.second()),
            (0, 0, 0),
            "{input:?} should be midnight"
        );
    }
}

#[test]
fn invalid_programmatic_inputs_are_rejected() {
    let provider = ChronoProvider::at(quiet_instant());

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

    // A minute without an hour is not a time that can be honoured.
    let minute_without_hour = TimeExpression::Absolute(AbsoluteTime {
        year: 2024,
        month: 1,
        day: 15,
        hour: None,
        minute: Some(30),
        second: None,
        nanosecond: None,
        timezone: None,
    });
    assert!(matches!(
        provider.parse_expression(minute_without_hour),
        Err(TempsError::InvalidTime { .. })
    ));
}

// ===== Zone-dependent behaviour =====
//
// `TZ` is process-wide, so a test that sets it would corrupt every test running
// beside it. The tests in `zone_pinned` are therefore `#[ignore]`d and re-run
// below in a child process that owns its own environment; each `*_suite` test
// is the runner for one zone.

/// Re-run this test binary with `TZ` set, executing only the ignored tests
/// whose name contains `filter`.
fn run_zone_suite(zone: &str, filter: &str) {
    use std::path::PathBuf;
    use std::process::Command;

    let tzdir = std::env::var("TZDIR").unwrap_or_else(|_| "/usr/share/zoneinfo".to_string());
    let zone_file = PathBuf::from(&tzdir).join(zone);
    if !zone_file.exists() {
        eprintln!("skipping {filter}: no time zone database entry at {zone_file:?}");
        return;
    }

    let exe = std::env::current_exe().expect("path of the running test binary");
    let output = Command::new(&exe)
        .env("TZ", zone)
        .args(["--ignored", "--test-threads=1", "--nocapture", filter])
        .output()
        .unwrap_or_else(|error| panic!("failed to re-run {exe:?} with TZ={zone}: {error}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "TZ={zone} {filter} failed\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    assert!(
        !matched_no_tests(&stdout),
        "TZ={zone} {filter} matched no tests; has one been renamed?\n{stdout}"
    );
}

/// Whether a libtest run's output says its filter selected no tests.
///
/// Read from the whole `running N tests` header line: a substring match on the
/// summary would also find `0 passed` inside `10 passed`.
fn matched_no_tests(stdout: &str) -> bool {
    stdout.lines().any(|line| line.trim() == "running 0 tests")
}

/// The guard must tell an empty run from one that passed 10, 20, … tests, whose
/// summary also contains the text `0 passed`.
#[test]
fn zone_suite_guard_only_flags_a_run_that_matched_nothing() {
    let summary = |count: usize| {
        format!(
            "\nrunning {count} tests\n\ntest result: ok. {count} passed; 0 failed; 0 ignored; \
             0 measured; 18 filtered out; finished in 0.00s\n\n"
        )
    };

    assert!(matched_no_tests(&summary(0)));
    for count in [1, 3, 10, 20, 100, 110] {
        assert!(
            !matched_no_tests(&summary(count)),
            "a run of {count} passing tests is not an empty run"
        );
    }
}

#[test]
fn us_eastern_suite() {
    run_zone_suite("America/New_York", "zone_pinned::us_eastern");
}

#[test]
fn havana_suite() {
    run_zone_suite("America/Havana", "zone_pinned::havana");
}

#[test]
fn apia_suite() {
    run_zone_suite("Pacific/Apia", "zone_pinned::apia");
}

#[test]
fn tokyo_suite() {
    run_zone_suite("Asia/Tokyo", "zone_pinned::tokyo");
}

/// Tests that only make sense in a specific time zone.
///
/// Each is `#[ignore]`d because it needs `TZ` set for the whole process; the
/// `*_suite` tests above run them in a child process. To run one group by hand:
///
/// ```text
/// TZ=America/New_York cargo test -p temps-chrono --test all_tests -- --ignored us_eastern
/// ```
mod zone_pinned {
    use super::*;
    use chrono::TimeDelta;

    /// Every instant that really renders as local midnight on the given date:
    /// empty where the civil time is skipped by a transition, two entries where
    /// it happens twice.
    ///
    /// chrono's `LocalResult` alone is not enough to tell those apart — for a
    /// nonexistent local time it can answer `Single` carrying the offset from
    /// the wrong side of the gap, so each candidate is round-tripped through
    /// UTC and kept only if it still reads back as the requested civil time.
    /// This reads chrono directly, so a failure here points at the time zone
    /// database rather than at temps.
    fn instants_at_local_midnight(year: i32, month: u32, day: u32) -> Vec<DateTime<Local>> {
        instants_at_local(civil(year, month, day, 0, 0))
    }

    /// Every instant that really renders as the civil time `naive`; see
    /// [`instants_at_local_midnight`].
    fn instants_at_local(naive: NaiveDateTime) -> Vec<DateTime<Local>> {
        let candidates = match naive.and_local_timezone(Local) {
            chrono::LocalResult::Single(dt) => vec![dt],
            chrono::LocalResult::Ambiguous(first, second) => vec![first, second],
            chrono::LocalResult::None => vec![],
        };

        candidates
            .into_iter()
            .map(|dt| Utc.from_utc_datetime(&dt.naive_utc()).with_timezone(&Local))
            .filter(|dt| dt.naive_local() == naive)
            .collect()
    }

    /// The civil datetime with the given calendar date and wall-clock time.
    fn civil(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|date| date.and_hms_opt(hour, minute, 0))
            .expect("valid civil datetime")
    }

    // --- America/New_York: spring forward 2024-03-10 02:00, fall back 2024-11-03 02:00 ---

    /// Day references are calendar arithmetic. Adding a fixed 24 hours to
    /// 2024-03-09 23:30 EST lands on 2024-03-11 — a day late — because the
    /// following day is only 23 hours long.
    #[test]
    #[ignore = "requires TZ=America/New_York; run via us_eastern_suite"]
    fn us_eastern_day_references_cross_spring_forward_by_calendar_days() {
        // 2024-03-09 23:30 EST
        let base = instant_at_utc(2024, 3, 10, 4, 30);
        assert_eq!(base.date_naive().to_string(), "2024-03-09");
        assert_eq!(base.hour(), 23, "expected 23:30 local in US Eastern");

        let provider = ChronoProvider::at(base);

        for (input, expected) in [
            ("today", "2024-03-09"),
            ("yesterday", "2024-03-08"),
            ("tomorrow", "2024-03-10"),
            ("day after tomorrow", "2024-03-11"),
            ("the day before yesterday", "2024-03-07"),
        ] {
            let result = resolve(&provider, input, Language::English);
            assert_eq!(
                result.date_naive().to_string(),
                expected,
                "wrong date for {input:?}"
            );
            assert_eq!(
                (result.hour(), result.minute(), result.second()),
                (0, 0, 0),
                "{input:?} should be midnight"
            );
        }
    }

    /// "in N days" keeps the wall-clock time across a transition, so the
    /// elapsed absolute time is 23 hours, not 24.
    #[test]
    #[ignore = "requires TZ=America/New_York; run via us_eastern_suite"]
    fn us_eastern_day_and_week_offsets_keep_the_wall_clock_across_spring_forward() {
        let base = instant_at_utc(2024, 3, 10, 4, 30); // 2024-03-09 23:30 EST
        let provider = ChronoProvider::at(base);

        let next_day = resolve(&provider, "in 1 day", Language::English);
        assert_eq!(next_day.date_naive().to_string(), "2024-03-10");
        assert_eq!(next_day.time(), base.time(), "wall clock should be kept");
        assert_ne!(
            next_day.offset().fix(),
            base.offset().fix(),
            "the transition should have been crossed"
        );
        assert_eq!(
            next_day.signed_duration_since(base),
            Duration::hours(23),
            "the day of a spring-forward is 23 hours long"
        );

        let next_week = resolve(&provider, "in 1 week", Language::English);
        assert_eq!(next_week.date_naive().to_string(), "2024-03-16");
        assert_eq!(next_week.time(), base.time());
        assert_eq!(
            next_week.signed_duration_since(base),
            Duration::hours(7 * 24 - 1)
        );
    }

    /// The mirror image: the day of a fall-back is 25 hours long, and the wall
    /// clock is still preserved.
    #[test]
    #[ignore = "requires TZ=America/New_York; run via us_eastern_suite"]
    fn us_eastern_day_offsets_keep_the_wall_clock_across_fall_back() {
        let base = instant_at_utc(2024, 11, 3, 3, 30); // 2024-11-02 23:30 EDT
        assert_eq!(base.date_naive().to_string(), "2024-11-02");
        let provider = ChronoProvider::at(base);

        let next_day = resolve(&provider, "in 1 day", Language::English);
        assert_eq!(next_day.date_naive().to_string(), "2024-11-03");
        assert_eq!(next_day.time(), base.time());
        assert_eq!(
            next_day.signed_duration_since(base),
            Duration::hours(25),
            "the day of a fall-back is 25 hours long"
        );

        let tomorrow = resolve(&provider, "tomorrow", Language::English);
        assert_eq!(tomorrow.date_naive().to_string(), "2024-11-03");
        let day_after = resolve(&provider, "day after tomorrow", Language::English);
        assert_eq!(day_after.date_naive().to_string(), "2024-11-04");
    }

    /// A month or year offset that lands on a wall-clock time the target date
    /// skips (spring forward) or repeats (fall back) is a
    /// `DateCalculationError`. chrono's month arithmetic answers `None` for
    /// such a time just as it does for a result past the end of its range, and
    /// only the latter is an overflow.
    ///
    /// This pins current behaviour, not the desired one. It is the still-open
    /// round-1 bug-hunt finding #1: month and year arithmetic does not go
    /// through `resolve_local`, so it does not shift a gap forward or pick the
    /// earlier instant of a fold the way every other expression does, and the
    /// jiff backend resolves these same inputs. When that is fixed, this test
    /// should expect those instants instead.
    #[test]
    #[ignore = "requires TZ=America/New_York; run via us_eastern_suite"]
    fn us_eastern_month_and_year_offsets_into_a_gap_or_fold_are_a_date_calculation_error() {
        let gap = civil(2024, 3, 10, 2, 30);
        let fold = civil(2024, 11, 3, 1, 30);
        assert!(instants_at_local(gap).is_empty(), "{gap} should be skipped");
        assert_eq!(instants_at_local(fold).len(), 2, "{fold} should repeat");

        let into_gap = [
            ("2024-02-10T02:30:00", "in 1 month", Language::English),
            ("2024-02-10T02:30:00", "in einem Monat", Language::German),
            ("2024-04-10T02:30:00", "1 month ago", Language::English),
            ("2023-03-10T02:30:00", "in 1 year", Language::English),
        ];
        let into_fold = [
            ("2024-10-03T01:30:00", "in 1 month", Language::English),
            ("2024-12-03T01:30:00", "1 month ago", Language::English),
            ("2025-11-03T01:30:00", "vor einem Jahr", Language::German),
        ];
        let cases = into_gap
            .map(|case| (case, gap))
            .into_iter()
            .chain(into_fold.map(|case| (case, fold)));

        for ((from, input, language), target) in cases {
            let from: NaiveDateTime = from.parse().expect("valid civil datetime");
            let provider = ChronoProvider::at(local_instant(from));
            let case = format!("{input:?} from {from}, landing on {target}");
            match try_resolve(&provider, input, language) {
                Err(TempsError::DateCalculationError { message, context }) => {
                    assert_eq!(message, errors::ERR_DATE_CALC_INVALID, "{case}");
                    assert_eq!(context, None, "{case}");
                }
                other => panic!("expected a date calculation error for {case}, got {other:?}"),
            }
        }
    }

    /// West of UTC, chrono's last date ends in UTC before it ends on the wall
    /// clock: 21:00 on it is already past the last UTC instant. Landing there
    /// is a range overflow, the same error one day further gives, not a DST
    /// ambiguity — there is no transition anywhere near.
    #[test]
    #[ignore = "requires TZ=America/New_York; run via us_eastern_suite"]
    fn us_eastern_day_offsets_past_the_utc_range_report_overflow() {
        let base = instant_at_utc(2024, 6, 13, 1, 0); // 2024-06-12 21:00 EDT
        assert_eq!(base.hour(), 21, "expected 21:00 local in US Eastern");
        let provider = ChronoProvider::at(base);
        let days_to_last_date = (NaiveDate::MAX - base.date_naive()).num_days();

        let day_before = resolve(
            &provider,
            &format!("in {} days", days_to_last_date - 1),
            Language::English,
        );
        assert_eq!(day_before.date_naive(), NaiveDate::MAX.pred_opt().unwrap());

        // Landing on the last date past its last UTC instant is the relative
        // amount's doing; one day more runs off the calendar itself.
        for amount in [days_to_last_date, days_to_last_date + 1] {
            for (input, language) in [
                (format!("in {amount} days"), Language::English),
                (format!("in {amount} Tagen"), Language::German),
            ] {
                assert_overflow_saying(
                    &provider,
                    &input,
                    language,
                    errors::ERR_AMOUNT_OUT_OF_RANGE,
                );
            }
        }

        // The same holds for a time of day named on that date, which contains
        // no relative amount to blame.
        let provider =
            ChronoProvider::at(local_instant(NaiveDate::MAX.and_hms_opt(12, 0, 0).unwrap()));
        assert_eq!(
            resolve(&provider, "3 pm", Language::English).hour(),
            15,
            "15:00 is still within range"
        );
        for input in ["23:00", "11 pm", "tonight", "today at 11 pm"] {
            assert_overflow_saying(
                &provider,
                input,
                Language::English,
                errors::ERR_RESULT_OUT_OF_RANGE,
            );
        }
    }

    /// A month offset whose calendar arithmetic succeeds can still land past
    /// the last UTC instant: 20:00 on chrono's last date is 01:00 UTC the next
    /// day in US Eastern. chrono answers `None` there just as it does for a
    /// daylight-saving gap, so this checks the out-of-range half of that split
    /// is still reported as the relative amount's overflow.
    #[test]
    #[ignore = "requires TZ=America/New_York; run via us_eastern_suite"]
    fn us_eastern_month_offsets_past_the_utc_range_report_overflow() {
        let from = NaiveDate::MAX - Days::new(61);
        assert_eq!((from.month(), from.day()), (10, 31));
        let provider = ChronoProvider::at(local_instant(from.and_hms_opt(20, 0, 0).unwrap()));

        // The naive arithmetic has a result; its instant does not.
        let target = NaiveDate::MAX.and_hms_opt(20, 0, 0).unwrap();
        assert!(
            instants_at_local(target).is_empty(),
            "{target} should be past the last UTC instant"
        );

        for (input, language) in [
            ("in 2 months", Language::English),
            ("in 2 Monaten", Language::German),
        ] {
            assert_overflow_saying(&provider, input, language, errors::ERR_AMOUNT_OUT_OF_RANGE);
        }

        // One month less stays in range.
        let result = resolve(&provider, "in 1 month", Language::English);
        assert_eq!(result.naive_local(), civil(262142, 11, 30, 20, 0));
    }

    /// West of UTC, going back to the start of chrono's range runs out of
    /// local wall clock first: an instant just after the first UTC instant
    /// reads as a local time before `NaiveDateTime::MIN`. Such a value used to
    /// come back as `Ok` and panic in `date_naive()`.
    #[test]
    #[ignore = "requires TZ=America/New_York; run via us_eastern_suite"]
    fn us_eastern_sub_day_offsets_before_the_local_range_report_overflow() {
        let base = instant_at_utc(2024, 6, 13, 1, 0); // 2024-06-12 21:00 EDT
        let provider = ChronoProvider::at(base);

        // Lands exactly on the first UTC instant, whose local reading does not
        // exist; one more hour is past the UTC range as well.
        let hours_to_first_instant = (base.naive_utc() - NaiveDateTime::MIN).num_hours();
        for amount in [hours_to_first_instant, hours_to_first_instant + 1] {
            assert_overflow(&provider, &format!("{amount} hours ago"), Language::English);
            assert_overflow(
                &provider,
                &format!("{} seconds ago", amount * 3600),
                Language::English,
            );
            assert_overflow(
                &provider,
                &format!("vor {amount} Stunden"),
                Language::German,
            );
        }

        let edge = ChronoProvider::at(local_instant(NaiveDateTime::MIN + TimeDelta::minutes(30)));
        for input in ["1 hour ago", "60 minutes ago", "3600 seconds ago"] {
            assert_overflow(&edge, input, Language::English);
        }
        let result = resolve(&edge, "29 minutes ago", Language::English);
        assert_eq!(result.date_naive(), NaiveDate::MIN);
    }

    /// Two hours after 18:00 on chrono's last date is past its last UTC
    /// instant, so "later today" stops there — still today on the wall clock.
    #[test]
    #[ignore = "requires TZ=America/New_York; run via us_eastern_suite"]
    fn us_eastern_later_today_clamps_to_the_last_utc_instant() {
        let base = local_instant(NaiveDate::MAX.and_hms_opt(18, 0, 0).unwrap());
        let provider = ChronoProvider::at(base);

        let result = resolve(&provider, "later today", Language::English);
        assert_eq!(result, DateTime::<Utc>::MAX_UTC);
        assert_eq!(result.date_naive(), NaiveDate::MAX);
        assert!(result > base);
    }

    /// West of UTC it is the start of the range where a pin can have no local
    /// reading: an hour after the first UTC instant reads as a local time
    /// before `NaiveDateTime::MIN`.
    #[test]
    #[ignore = "requires TZ=America/New_York; run via us_eastern_suite"]
    fn us_eastern_a_pin_with_no_local_reading_is_an_error_not_a_panic() {
        let pin = (DateTime::<Utc>::MIN_UTC + TimeDelta::hours(1)).with_timezone(&Local);
        assert!(
            !has_local_reading(pin),
            "the pin's local reading should be before NaiveDateTime::MIN"
        );
        assert!(
            has_local_reading(pin + TimeDelta::hours(10)),
            "ten hours on, the wall clock should be readable again"
        );
        assert_pin_is_rejected(&ChronoProvider::at(pin), &["in 10 hours"]);
    }

    // --- America/Havana: transitions at local midnight ---

    /// Cuba ends daylight saving at 01:00 local, so 2024-11-03 00:00–00:59
    /// happens twice. `.single()` returns `None` for such a fold, which used to
    /// make plain "today" fail; the earlier of the two instants is chosen.
    #[test]
    #[ignore = "requires TZ=America/Havana; run via havana_suite"]
    fn havana_today_resolves_when_local_midnight_is_ambiguous() {
        let instants = instants_at_local_midnight(2024, 11, 3);
        assert_eq!(
            instants.len(),
            2,
            "expected 2024-11-03 00:00 to happen twice in America/Havana, got {instants:?}"
        );
        let earlier = *instants.iter().min().expect("two instants");

        // Noon on the fold day.
        let provider = ChronoProvider::at(instant_at_utc(2024, 11, 3, 17, 0));

        let today = resolve(&provider, "today", Language::English);
        assert_eq!(
            today, earlier,
            "an ambiguous midnight resolves to the earlier instant"
        );
        assert_eq!(today.date_naive().to_string(), "2024-11-03");
        assert_eq!(today.hour(), 0);

        // The same civil midnight reached as "tomorrow" from the day before.
        let provider = ChronoProvider::at(instant_at_utc(2024, 11, 2, 16, 0));
        assert_eq!(resolve(&provider, "tomorrow", Language::English), earlier);
        assert_eq!(resolve(&provider, "midnight", Language::English).hour(), 0);
    }

    /// Cuba starts daylight saving at midnight, so 2024-03-10 00:00 does not
    /// exist at all. A nonexistent time is shifted forward by the width of the
    /// gap, matching jiff's `compatible` disambiguation.
    #[test]
    #[ignore = "requires TZ=America/Havana; run via havana_suite"]
    fn havana_day_references_resolve_when_local_midnight_does_not_exist() {
        assert!(
            instants_at_local_midnight(2024, 3, 10).is_empty(),
            "expected 2024-03-10 00:00 not to exist in America/Havana"
        );

        // Noon on the gap day.
        let provider = ChronoProvider::at(instant_at_utc(2024, 3, 10, 16, 0));
        let today = resolve(&provider, "today", Language::English);
        assert_eq!(today.date_naive().to_string(), "2024-03-10");
        assert_eq!(
            (today.hour(), today.minute()),
            (1, 0),
            "midnight is shifted forward by the one-hour gap"
        );

        // Reached as "tomorrow" from the previous day.
        let provider = ChronoProvider::at(instant_at_utc(2024, 3, 9, 16, 0));
        let tomorrow = resolve(&provider, "tomorrow", Language::English);
        assert_eq!(tomorrow.date_naive().to_string(), "2024-03-10");
        assert_eq!((tomorrow.hour(), tomorrow.minute()), (1, 0));
    }

    // --- Pacific/Apia: 2011-12-30 was skipped entirely ---

    /// Samoa moved across the date line at the end of 2011: 2011-12-30 has no
    /// instants whatsoever. The gap is 24 hours wide, which a fixed-size probe
    /// for the surrounding offset would miss.
    #[test]
    #[ignore = "requires TZ=Pacific/Apia; run via apia_suite"]
    fn apia_tomorrow_resolves_over_a_whole_skipped_day() {
        assert!(
            instants_at_local_midnight(2011, 12, 30).is_empty(),
            "expected 2011-12-30 not to exist in Pacific/Apia"
        );

        // 2011-12-29 12:00 local (UTC-10).
        let base = instant_at_utc(2011, 12, 29, 22, 0);
        assert_eq!(base.date_naive().to_string(), "2011-12-29");
        let provider = ChronoProvider::at(base);

        let tomorrow = resolve(&provider, "tomorrow", Language::English);
        assert_eq!(
            tomorrow.date_naive().to_string(),
            "2011-12-31",
            "the skipped day is shifted forward by the width of the gap"
        );
        assert_eq!((tomorrow.hour(), tomorrow.minute()), (0, 0));

        assert_eq!(
            resolve(&provider, "today", Language::English)
                .date_naive()
                .to_string(),
            "2011-12-29"
        );
    }

    // --- Asia/Tokyo: a fixed +09:00, east of UTC ---
    //
    // East of UTC the wall clock reaches `NaiveDateTime::MAX` nine hours before
    // UTC does, so there is a band of instants chrono can hold whose local
    // reading does not exist. `date_naive()` and `naive_local()` panic on them.

    /// Sub-day offsets into that band used to come back as `Ok`, and the
    /// caller's first `date_naive()` then panicked.
    #[test]
    #[ignore = "requires TZ=Asia/Tokyo; run via tokyo_suite"]
    fn tokyo_sub_day_offsets_past_the_local_range_report_overflow() {
        let base = instant_at_utc(2024, 6, 12, 12, 0); // 2024-06-12 21:00 JST
        assert_eq!(base.hour(), 21, "expected 21:00 local in Asia/Tokyo");
        let provider = ChronoProvider::at(base);
        let hours_to_last_hour = (NaiveDateTime::MAX - base.naive_local()).num_hours();

        let last = resolve(
            &provider,
            &format!("in {hours_to_last_hour} hours"),
            Language::English,
        );
        assert_eq!(
            last.naive_local(),
            NaiveDate::MAX.and_hms_opt(23, 0, 0).unwrap()
        );

        // The first nine hours past the wall clock's end are still within the
        // UTC range; the tenth is not.
        for amount in (1..=10).map(|hours| hours_to_last_hour + hours) {
            assert_overflow(&provider, &format!("in {amount} hours"), Language::English);
            assert_overflow(&provider, &format!("in {amount} Stunden"), Language::German);
            assert_overflow(
                &provider,
                &format!("in {} minutes", amount * 60),
                Language::English,
            );
            assert_overflow(
                &provider,
                &format!("in {} seconds", amount * 3600),
                Language::English,
            );
        }

        // Half an hour before the end, even the shortest step can overflow.
        let edge = ChronoProvider::at(local_instant(NaiveDateTime::MAX - TimeDelta::minutes(30)));
        for input in ["later", "in 1 hour", "in 60 minutes", "in 3600 seconds"] {
            assert_overflow(&edge, input, Language::English);
        }
        let result = resolve(&edge, "in 29 minutes", Language::English);
        assert_eq!(result.date_naive(), NaiveDate::MAX);
    }

    /// At the start of the range it is UTC that runs out first east of UTC:
    /// local midnight on `NaiveDate::MIN` is before the first UTC instant.
    /// That is a range overflow, not a DST ambiguity.
    #[test]
    #[ignore = "requires TZ=Asia/Tokyo; run via tokyo_suite"]
    fn tokyo_day_offsets_before_the_utc_range_report_overflow() {
        let base = instant_at_utc(2024, 6, 11, 18, 0); // 2024-06-12 03:00 JST
        assert_eq!(base.hour(), 3, "expected 03:00 local in Asia/Tokyo");
        let provider = ChronoProvider::at(base);
        let days_to_first_date = (base.date_naive() - NaiveDate::MIN).num_days();

        let day_after = resolve(
            &provider,
            &format!("{} days ago", days_to_first_date - 1),
            Language::English,
        );
        assert_eq!(day_after.date_naive(), NaiveDate::MIN.succ_opt().unwrap());

        for amount in [days_to_first_date, days_to_first_date + 1] {
            for (input, language) in [
                (format!("{amount} days ago"), Language::English),
                (format!("vor {amount} Tagen"), Language::German),
            ] {
                assert_overflow_saying(
                    &provider,
                    &input,
                    language,
                    errors::ERR_AMOUNT_OUT_OF_RANGE,
                );
            }
        }

        // Pinned at noon on that first date, its own midnight and early hours
        // are out of range too, and no relative amount is to blame.
        let provider =
            ChronoProvider::at(local_instant(NaiveDate::MIN.and_hms_opt(12, 0, 0).unwrap()));
        for input in ["today", "midnight", "3 am", "today at 3 am"] {
            assert_overflow_saying(
                &provider,
                input,
                Language::English,
                errors::ERR_RESULT_OUT_OF_RANGE,
            );
        }
        assert_eq!(resolve(&provider, "noon", Language::English).hour(), 12);
    }

    /// The mirror image of the US Eastern month overflow, at the start of the
    /// range: in Tokyo, 04:00 on chrono's first date is before the first UTC
    /// instant, although the calendar arithmetic that reaches it succeeds.
    #[test]
    #[ignore = "requires TZ=Asia/Tokyo; run via tokyo_suite"]
    fn tokyo_month_offsets_before_the_utc_range_report_overflow() {
        let from = NaiveDate::MIN + Days::new(59);
        assert_eq!((from.month(), from.day()), (3, 1));
        let provider = ChronoProvider::at(local_instant(from.and_hms_opt(4, 0, 0).unwrap()));

        // The naive arithmetic has a result; its instant does not.
        let target = NaiveDate::MIN.and_hms_opt(4, 0, 0).unwrap();
        assert!(
            instants_at_local(target).is_empty(),
            "{target} should be before the first UTC instant"
        );

        for (input, language) in [
            ("2 months ago", Language::English),
            ("vor 2 Monaten", Language::German),
        ] {
            assert_overflow_saying(&provider, input, language, errors::ERR_AMOUNT_OUT_OF_RANGE);
        }

        // One month less stays in range.
        let result = resolve(&provider, "1 month ago", Language::English);
        assert_eq!(result.naive_local(), civil(-262143, 2, 1, 4, 0));
    }

    /// A day-at-time expression resolves only the wall time it names. On the
    /// first date, midnight is before the first UTC instant, but 15:00 is not,
    /// so "today at 3 pm" must agree with a bare "3 pm" rather than fail on a
    /// midnight nobody asked for.
    #[test]
    #[ignore = "requires TZ=Asia/Tokyo; run via tokyo_suite"]
    fn tokyo_day_at_time_on_the_first_date_does_not_need_its_midnight() {
        let first =
            ChronoProvider::at(local_instant(NaiveDate::MIN.and_hms_opt(12, 0, 0).unwrap()));
        assert_overflow(&first, "today", Language::English);

        for (with_day, bare, language) in [
            ("today at 3 pm", "3 pm", Language::English),
            ("tonight", "20:00", Language::English),
            ("this evening", "18:00", Language::English),
            ("this afternoon", "13:00", Language::English),
            ("eod", "17:00", Language::English),
            ("heute um 15:00", "15:00", Language::German),
        ] {
            let expected = resolve(&first, bare, language);
            assert_eq!(expected.date_naive(), NaiveDate::MIN, "{bare:?}");
            assert_eq!(
                resolve(&first, with_day, language),
                expected,
                "{with_day:?} should agree with {bare:?}"
            );
        }

        // The same date reached from the day after it.
        let second = ChronoProvider::at(local_instant(
            NaiveDate::MIN
                .succ_opt()
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap(),
        ));
        assert_overflow(&second, "yesterday", Language::English);
        let three_pm = local_instant(NaiveDate::MIN.and_hms_opt(15, 0, 0).unwrap());
        for (input, language) in [
            ("yesterday at 3 pm", Language::English),
            (
                &*format!("last {} at 3 pm", NaiveDate::MIN.weekday()),
                Language::English,
            ),
            ("gestern um 15:00", Language::German),
        ] {
            assert_eq!(resolve(&second, input, language), three_pm, "{input:?}");
        }
    }

    /// A pin chrono can hold but whose local reading is past
    /// `NaiveDateTime::MAX`: every expression that reads the pin must fail
    /// with an error rather than panic in `date_naive()`, and an absolute
    /// time, which does not read it, still resolves.
    #[test]
    #[ignore = "requires TZ=Asia/Tokyo; run via tokyo_suite"]
    fn tokyo_a_pin_with_no_local_reading_is_an_error_not_a_panic() {
        let pin = (DateTime::<Utc>::MAX_UTC - TimeDelta::hours(1)).with_timezone(&Local);
        assert!(
            !has_local_reading(pin),
            "the pin's local reading should be past NaiveDateTime::MAX"
        );
        for hours in [10, 24] {
            assert!(
                has_local_reading(pin - TimeDelta::hours(hours)),
                "{hours} hours back, the wall clock should be readable again"
            );
        }
        assert_pin_is_rejected(&ChronoProvider::at(pin), &["10 hours ago", "24 hours ago"]);
    }

    /// Whether chrono can read `dt`'s local wall clock, that is, whether
    /// `date_naive()` and `naive_local()` on it would not panic.
    fn has_local_reading(dt: DateTime<Local>) -> bool {
        dt.naive_utc().checked_add_offset(*dt.offset()).is_some()
    }

    /// The overflow message for a pin whose own local reading is out of range.
    /// The crate keeps it private, so it is spelled out here.
    const ERR_PIN_OUT_OF_RANGE: &str =
        "The reference time's local reading is outside the range chrono can represent";

    /// Every expression that reads `provider`'s pin is an overflow that names
    /// the pin as its cause. That includes the relative expressions in
    /// `in_range`, whose results would themselves have a local reading chrono
    /// can represent: the guard is on the pin, not on the result. Absolute
    /// times, which do not read the pin, still resolve.
    fn assert_pin_is_rejected(provider: &ChronoProvider, in_range: &[&str]) {
        for input in in_range {
            assert_overflow_saying(provider, input, Language::English, ERR_PIN_OUT_OF_RANGE);
        }
        for (input, language) in [
            ("now", Language::English),
            ("later today", Language::English),
            ("later", Language::English),
            ("today", Language::English),
            ("tomorrow", Language::English),
            ("yesterday", Language::English),
            ("monday", Language::English),
            ("3 pm", Language::English),
            ("tonight", Language::English),
            ("tomorrow at 3 pm", Language::English),
            ("in 0 seconds", Language::English),
            ("in 0 days", Language::English),
            ("in 1 second", Language::English),
            ("1 second ago", Language::English),
            ("in 1 day", Language::English),
            ("1 day ago", Language::English),
            ("in 1 week", Language::English),
            ("in 1 month", Language::English),
            ("1 month ago", Language::English),
            ("in 1 year", Language::English),
            ("jetzt", Language::German),
            ("heute", Language::German),
            ("morgen um 15:00", Language::German),
        ] {
            assert_overflow_saying(provider, input, language, ERR_PIN_OUT_OF_RANGE);
        }
        for input in ["2024-01-15T10:00:00Z", "2024-01-15", "15/03/2024"] {
            let result = resolve(provider, input, Language::English);
            assert_eq!(result.year(), 2024, "{input:?}");
        }
    }

    /// Two hours after 23:00 on chrono's last date has no wall-clock reading,
    /// so "later today" stops at the last instant that has one.
    #[test]
    #[ignore = "requires TZ=Asia/Tokyo; run via tokyo_suite"]
    fn tokyo_later_today_clamps_to_the_last_local_instant() {
        let base = local_instant(NaiveDate::MAX.and_hms_opt(23, 0, 0).unwrap());
        let provider = ChronoProvider::at(base);

        let result = resolve(&provider, "later today", Language::English);
        assert_eq!(result.naive_local(), NaiveDateTime::MAX);
        assert!(result > base);
    }
}
