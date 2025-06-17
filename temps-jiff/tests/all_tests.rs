use jiff::{Span, Zoned, civil::DateTime};
use temps_core::*;
use temps_jiff::*;
use temps_testhelpers::jiff::{MockTimeSource, TimeSource};

// ===== Integration Tests =====

#[test]
fn test_time_provider_trait() {
    let provider = JiffProvider;
    let now = provider.now();
    // Basic test that we can create a provider and get current time
    assert!(now > Zoned::default());
}

#[test]
fn test_jiff_provider_consistency() {
    let provider = JiffProvider;

    // Test that parsing "now" returns the current time (approximately)
    let now = provider.now();
    let parsed_now = provider.parse_expression(TimeExpression::Now).unwrap();

    // They should be very close (within a second)
    let diff = parsed_now
        .timestamp()
        .as_second()
        .saturating_sub(now.timestamp().as_second())
        .abs();
    assert!(
        diff < 1,
        "Parsed 'now' should be within 1 second of actual now"
    );
}

// ===== Date Arithmetic Tests =====

#[test]
fn test_month_arithmetic_edge_cases() {
    // Test that parsing "in 1 month" works
    let provider = JiffProvider;
    let expr = parse("in 1 month", Language::English).unwrap();
    let result = provider.parse_expression(expr);
    assert!(result.is_ok());
}

#[test]
fn test_leap_year_handling() {
    // Test that February 29, 2024 + 1 year = February 28, 2025
    // We can't test exact dates without mocking, but we can test that the parsing works
    let provider = JiffProvider;

    let expr = parse("in 1 year", Language::English).unwrap();
    let result = provider.parse_expression(expr);
    assert!(result.is_ok());

    let expr = parse("1 year ago", Language::English).unwrap();
    let result = provider.parse_expression(expr);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_years() {
    // Test multiple year arithmetic
    let provider = JiffProvider;

    let expr = parse("in 5 years", Language::English).unwrap();
    let result = provider.parse_expression(expr);
    assert!(result.is_ok());

    let expr = parse("10 years ago", Language::English).unwrap();
    let result = provider.parse_expression(expr);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_months() {
    // Test multiple month arithmetic
    let provider = JiffProvider;

    let expr = parse("in 18 months", Language::English).unwrap();
    let result = provider.parse_expression(expr);
    assert!(result.is_ok());

    let expr = parse("6 months ago", Language::English).unwrap();
    let result = provider.parse_expression(expr);
    assert!(result.is_ok());
}

#[test]
fn test_date_arithmetic_consistency() {
    // Test that going forward and then backward by the same amount doesn't always
    // return to the exact same date (due to month length differences)
    // This is expected behavior

    let provider = JiffProvider;

    // Test month arithmetic
    let forward_month = TimeExpression::Relative(RelativeTime {
        amount: 1,
        unit: TimeUnit::Month,
        direction: Direction::Future,
    });

    let backward_month = TimeExpression::Relative(RelativeTime {
        amount: 1,
        unit: TimeUnit::Month,
        direction: Direction::Past,
    });

    let now = provider.now();
    let forward_result = provider.parse_expression(forward_month.clone());
    assert!(forward_result.is_ok());

    let backward_result = provider.parse_expression(backward_month);
    assert!(backward_result.is_ok());

    // Verify the results are different from now
    if let (Ok(forward_time), Ok(backward_time)) = (forward_result, backward_result) {
        assert!(forward_time > now);
        assert!(backward_time < now);
    }

    // Year arithmetic should be more consistent
    let forward_year = TimeExpression::Relative(RelativeTime {
        amount: 1,
        unit: TimeUnit::Year,
        direction: Direction::Future,
    });

    let year_result = provider.parse_expression(forward_year);
    assert!(year_result.is_ok());
}

// ===== Mock Tests =====

// Testable wrapper around JiffProvider
struct TestableJiffProvider<T: TimeSource> {
    time_source: T,
}

impl<T: TimeSource> TestableJiffProvider<T> {
    fn new(time_source: T) -> Self {
        Self { time_source }
    }
}

impl<T: TimeSource> TimeParser for TestableJiffProvider<T> {
    type DateTime = Zoned;

    fn now(&self) -> Self::DateTime {
        self.time_source.now()
    }

    fn parse_expression(&self, expr: TimeExpression) -> temps_core::Result<Self::DateTime> {
        let now = self.now();
        match expr {
            TimeExpression::Now => Ok(now),
            TimeExpression::Relative(rel) => {
                // Use JiffProvider's logic but with our mocked now
                let span = match rel.unit {
                    TimeUnit::Second => Span::new().seconds(rel.amount),
                    TimeUnit::Minute => Span::new().minutes(rel.amount),
                    TimeUnit::Hour => Span::new().hours(rel.amount),
                    TimeUnit::Day => Span::new().days(rel.amount),
                    TimeUnit::Week => Span::new().weeks(rel.amount),
                    TimeUnit::Month => Span::new().months(rel.amount),
                    TimeUnit::Year => Span::new().years(rel.amount),
                };

                match rel.direction {
                    Direction::Past => now.checked_sub(span).map_err(|e| {
                        temps_core::TempsError::date_calculation_with_source(
                            "Failed to subtract time",
                            e.to_string(),
                        )
                    }),
                    Direction::Future => now.checked_add(span).map_err(|e| {
                        temps_core::TempsError::date_calculation_with_source(
                            "Failed to add time",
                            e.to_string(),
                        )
                    }),
                }
            }
            TimeExpression::Absolute(abs) => {
                JiffProvider.parse_expression(TimeExpression::Absolute(abs))
            }
            TimeExpression::Day(day_ref) => {
                JiffProvider.parse_expression(TimeExpression::Day(day_ref))
            }
            TimeExpression::Time(time) => JiffProvider.parse_expression(TimeExpression::Time(time)),
            TimeExpression::DayTime(day_time) => {
                JiffProvider.parse_expression(TimeExpression::DayTime(day_time))
            }
            TimeExpression::Date(date) => JiffProvider.parse_expression(TimeExpression::Date(date)),
        }
    }
}

#[test]
fn test_english_time_expressions_with_mock() {
    let mut mock = MockTimeSource::new();
    let base_time = DateTime::constant(2024, 3, 15, 10, 30, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .unwrap();
    mock.expect_now().return_const(base_time.clone());

    let provider = TestableJiffProvider::new(mock);

    // Test various English natural language expressions
    let test_cases = vec![
        // Seconds
        (
            "in 30 seconds",
            base_time.checked_add(Span::new().seconds(30)).unwrap(),
        ),
        (
            "45 seconds ago",
            base_time.checked_sub(Span::new().seconds(45)).unwrap(),
        ),
        // Minutes
        (
            "in 5 minutes",
            base_time.checked_add(Span::new().minutes(5)).unwrap(),
        ),
        (
            "10 minutes ago",
            base_time.checked_sub(Span::new().minutes(10)).unwrap(),
        ),
        (
            "in a minute",
            base_time.checked_add(Span::new().minutes(1)).unwrap(),
        ),
        (
            "an hour ago",
            base_time.checked_sub(Span::new().hours(1)).unwrap(),
        ),
        // Hours
        (
            "in 2 hours",
            base_time.checked_add(Span::new().hours(2)).unwrap(),
        ),
        (
            "3 hours ago",
            base_time.checked_sub(Span::new().hours(3)).unwrap(),
        ),
        // Days
        (
            "in 1 day",
            base_time.checked_add(Span::new().days(1)).unwrap(),
        ),
        (
            "2 days ago",
            base_time.checked_sub(Span::new().days(2)).unwrap(),
        ),
        (
            "in a day",
            base_time.checked_add(Span::new().days(1)).unwrap(),
        ),
        // Weeks
        (
            "in 1 week",
            base_time.checked_add(Span::new().weeks(1)).unwrap(),
        ),
        (
            "2 weeks ago",
            base_time.checked_sub(Span::new().weeks(2)).unwrap(),
        ),
        (
            "in a week",
            base_time.checked_add(Span::new().weeks(1)).unwrap(),
        ),
    ];

    for (input, expected) in test_cases {
        let expr = parse(input, Language::English).unwrap();
        let result = provider.parse_expression(expr).unwrap();
        assert_eq!(result, expected, "Failed for input: {}", input);
    }
}

#[test]
fn test_german_time_expressions_with_mock() {
    let mut mock = MockTimeSource::new();
    let base_time = DateTime::constant(2024, 3, 15, 10, 30, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .unwrap();
    mock.expect_now().return_const(base_time.clone());

    let provider = TestableJiffProvider::new(mock);

    // Test various German natural language expressions
    let test_cases = vec![
        // Seconds
        (
            "in 30 Sekunden",
            base_time.checked_add(Span::new().seconds(30)).unwrap(),
        ),
        (
            "vor 45 Sekunden",
            base_time.checked_sub(Span::new().seconds(45)).unwrap(),
        ),
        // Minutes
        (
            "in 5 Minuten",
            base_time.checked_add(Span::new().minutes(5)).unwrap(),
        ),
        (
            "vor 10 Minuten",
            base_time.checked_sub(Span::new().minutes(10)).unwrap(),
        ),
        (
            "in einer Minute",
            base_time.checked_add(Span::new().minutes(1)).unwrap(),
        ),
        (
            "vor einer Stunde",
            base_time.checked_sub(Span::new().hours(1)).unwrap(),
        ),
        // Hours
        (
            "in 2 Stunden",
            base_time.checked_add(Span::new().hours(2)).unwrap(),
        ),
        (
            "vor 3 Stunden",
            base_time.checked_sub(Span::new().hours(3)).unwrap(),
        ),
        // Days
        (
            "in 1 Tag",
            base_time.checked_add(Span::new().days(1)).unwrap(),
        ),
        (
            "vor 2 Tagen",
            base_time.checked_sub(Span::new().days(2)).unwrap(),
        ),
        (
            "in einem Tag",
            base_time.checked_add(Span::new().days(1)).unwrap(),
        ),
        // Weeks
        (
            "in 1 Woche",
            base_time.checked_add(Span::new().weeks(1)).unwrap(),
        ),
        (
            "vor 2 Wochen",
            base_time.checked_sub(Span::new().weeks(2)).unwrap(),
        ),
        (
            "in einer Woche",
            base_time.checked_add(Span::new().weeks(1)).unwrap(),
        ),
    ];

    for (input, expected) in test_cases {
        let expr = parse(input, Language::German).unwrap();
        let result = provider.parse_expression(expr).unwrap();
        assert_eq!(result, expected, "Failed for input: {}", input);
    }
}

#[test]
fn test_month_arithmetic_edge_cases_with_mock() {
    let mut mock = MockTimeSource::new();

    // Test case 1: January 31 + 1 month = February 29 (leap year)
    let jan_31_2024 = DateTime::constant(2024, 1, 31, 10, 0, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .unwrap();
    mock.expect_now().return_const(jan_31_2024);

    let provider = TestableJiffProvider::new(mock);
    let expr = parse("in 1 month", Language::English).unwrap();
    let result = provider.parse_expression(expr).unwrap();

    assert_eq!(result.month(), 2);
    assert_eq!(result.day(), 29);
    assert_eq!(result.year(), 2024);
}

#[test]
fn test_month_arithmetic_non_leap_year() {
    let mut mock = MockTimeSource::new();

    // Test case 2: January 31 + 1 month = February 28 (non-leap year)
    let jan_31_2023 = DateTime::constant(2023, 1, 31, 10, 0, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .unwrap();
    mock.expect_now().return_const(jan_31_2023);

    let provider = TestableJiffProvider::new(mock);
    let expr = parse("in einem Monat", Language::German).unwrap();
    let result = provider.parse_expression(expr).unwrap();

    assert_eq!(result.month(), 2);
    assert_eq!(result.day(), 28);
    assert_eq!(result.year(), 2023);
}

#[test]
fn test_year_arithmetic_with_leap_day() {
    let mut mock = MockTimeSource::new();

    // Test case: February 29, 2024 + 1 year = February 28, 2025
    let feb_29 = DateTime::constant(2024, 2, 29, 12, 0, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .unwrap();
    mock.expect_now().return_const(feb_29);

    let provider = TestableJiffProvider::new(mock);
    let expr = parse("in 1 year", Language::English).unwrap();
    let result = provider.parse_expression(expr).unwrap();

    assert_eq!(result.month(), 2);
    assert_eq!(result.day(), 28);
    assert_eq!(result.year(), 2025);
}

#[test]
fn test_cross_year_boundary_calculations() {
    let mut mock = MockTimeSource::new();

    // Test: October 15, 2023 + 6 months = April 15, 2024
    let oct_15 = DateTime::constant(2023, 10, 15, 9, 0, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .unwrap();
    mock.expect_now().return_const(oct_15);

    let provider = TestableJiffProvider::new(mock);
    let expr = parse("in 6 months", Language::English).unwrap();
    let result = provider.parse_expression(expr).unwrap();

    assert_eq!(result.year(), 2024);
    assert_eq!(result.month(), 4);
    assert_eq!(result.day(), 15);
}

#[test]
fn test_iso_datetime_absolute_time() {
    let provider = JiffProvider;

    let test_cases = vec![
        // Basic RFC3339 dates
        "2024-01-15T14:30:00Z",
        "2024-01-15T14:30:00+02:00",
        "2024-01-15T14:30:00.123Z",
    ];

    for input in test_cases {
        let expr = parse(input, Language::English).unwrap();
        let result = provider.parse_expression(expr);
        assert!(
            result.is_ok(),
            "Failed to parse absolute time: {} - Error: {:?}",
            input,
            result.err()
        );

        // Verify it's a valid datetime
        let datetime = result.unwrap();
        assert!(datetime.year() >= 2024, "Year should be 2024 or later");
    }
}

#[test]
fn test_complex_expressions_with_articles() {
    let mut mock = MockTimeSource::new();
    let base_time = DateTime::constant(2024, 3, 15, 10, 30, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .unwrap();
    mock.expect_now().return_const(base_time.clone());

    let provider = TestableJiffProvider::new(mock);

    // Test English articles: "a", "an", "one"
    let english_cases = vec![
        (
            "in a second",
            base_time.checked_add(Span::new().seconds(1)).unwrap(),
        ),
        (
            "in an hour",
            base_time.checked_add(Span::new().hours(1)).unwrap(),
        ),
        (
            "in one day",
            base_time.checked_add(Span::new().days(1)).unwrap(),
        ),
        (
            "a minute ago",
            base_time.checked_sub(Span::new().minutes(1)).unwrap(),
        ),
        (
            "an hour ago",
            base_time.checked_sub(Span::new().hours(1)).unwrap(),
        ),
        (
            "one week ago",
            base_time.checked_sub(Span::new().weeks(1)).unwrap(),
        ),
    ];

    for (input, expected) in english_cases {
        let expr = parse(input, Language::English).unwrap();
        let result = provider.parse_expression(expr).unwrap();
        assert_eq!(result, expected, "Failed for input: {}", input);
    }
}

#[test]
fn test_german_articles_with_time_calculations() {
    let mut mock = MockTimeSource::new();
    let base_time = DateTime::constant(2024, 3, 15, 10, 30, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .unwrap();
    mock.expect_now().return_const(base_time.clone());

    let provider = TestableJiffProvider::new(mock);

    // Test German articles: "ein", "eine", "einem", "einer", etc.
    let german_cases = vec![
        (
            "in einer Sekunde",
            base_time.checked_add(Span::new().seconds(1)).unwrap(),
        ),
        (
            "in einer Minute",
            base_time.checked_add(Span::new().minutes(1)).unwrap(),
        ),
        (
            "in einer Stunde",
            base_time.checked_add(Span::new().hours(1)).unwrap(),
        ),
        (
            "in einem Tag",
            base_time.checked_add(Span::new().days(1)).unwrap(),
        ),
        (
            "in einer Woche",
            base_time.checked_add(Span::new().weeks(1)).unwrap(),
        ),
        (
            "vor einer Sekunde",
            base_time.checked_sub(Span::new().seconds(1)).unwrap(),
        ),
        (
            "vor einem Tag",
            base_time.checked_sub(Span::new().days(1)).unwrap(),
        ),
    ];

    for (input, expected) in german_cases {
        let expr = parse(input, Language::German).unwrap();
        let result = provider.parse_expression(expr).unwrap();
        assert_eq!(result, expected, "Failed for input: {}", input);
    }
}

#[test]
fn test_now_expression_with_mock() {
    let mut mock = MockTimeSource::new();
    let current_time = DateTime::constant(2024, 3, 15, 10, 30, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .unwrap();
    mock.expect_now().return_const(current_time.clone());

    let provider = TestableJiffProvider::new(mock);

    // Test "now" in English
    let expr = parse("now", Language::English).unwrap();
    let result = provider.parse_expression(expr).unwrap();
    assert_eq!(result, current_time);

    // Test "jetzt" in German
    let expr = parse("jetzt", Language::German).unwrap();
    let result = provider.parse_expression(expr).unwrap();
    assert_eq!(result, current_time);
}

#[test]
fn test_month_year_calculations_with_spans() {
    let mut mock = MockTimeSource::new();
    let base_time = DateTime::constant(2024, 1, 15, 12, 0, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .unwrap();
    mock.expect_now().return_const(base_time.clone());

    let provider = TestableJiffProvider::new(mock);

    // Test month calculations
    let month_cases = vec![
        (
            "in 1 month",
            base_time.checked_add(Span::new().months(1)).unwrap(),
        ),
        (
            "in 3 months",
            base_time.checked_add(Span::new().months(3)).unwrap(),
        ),
        (
            "1 month ago",
            base_time.checked_sub(Span::new().months(1)).unwrap(),
        ),
        (
            "in einem Monat",
            base_time.checked_add(Span::new().months(1)).unwrap(),
        ),
        (
            "vor einem Monat",
            base_time.checked_sub(Span::new().months(1)).unwrap(),
        ),
    ];

    for (input, expected) in month_cases {
        let lang = if input.contains("vor") || input.contains("einem") {
            Language::German
        } else {
            Language::English
        };
        let expr = parse(input, lang).unwrap();
        let result = provider.parse_expression(expr).unwrap();
        assert_eq!(result, expected, "Failed for input: {}", input);
    }

    // Test year calculations
    let year_cases = vec![
        (
            "in 1 year",
            base_time.checked_add(Span::new().years(1)).unwrap(),
        ),
        (
            "in 2 years",
            base_time.checked_add(Span::new().years(2)).unwrap(),
        ),
        (
            "1 year ago",
            base_time.checked_sub(Span::new().years(1)).unwrap(),
        ),
        (
            "in einem Jahr",
            base_time.checked_add(Span::new().years(1)).unwrap(),
        ),
        (
            "vor einem Jahr",
            base_time.checked_sub(Span::new().years(1)).unwrap(),
        ),
    ];

    for (input, expected) in year_cases {
        let lang = if input.contains("vor") || input.contains("einem") {
            Language::German
        } else {
            Language::English
        };
        let expr = parse(input, lang).unwrap();
        let result = provider.parse_expression(expr).unwrap();
        assert_eq!(result, expected, "Failed for input: {}", input);
    }
}

#[test]
fn test_day_references_with_jiff() {
    let test_cases = vec![
        ("today", Language::English),
        ("yesterday", Language::English),
        ("tomorrow", Language::English),
        ("heute", Language::German),
        ("gestern", Language::German),
        ("morgen", Language::German),
    ];

    for (input, lang) in test_cases {
        let result = parse_to_zoned(input, lang);
        assert!(result.is_ok(), "Failed to parse: {}", input);
        let datetime = result.unwrap();
        // Should return midnight of the respective day
        assert_eq!(datetime.hour(), 0);
        assert_eq!(datetime.minute(), 0);
        assert_eq!(datetime.second(), 0);
    }
}

#[test]
fn test_weekday_parsing_with_jiff() {
    let test_cases = vec![
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
        "mon",
        "tue",
        "wed",
        "thu",
        "fri",
        "sat",
        "sun",
    ];

    for input in test_cases {
        let result = parse_to_zoned(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {}", input);
        let datetime = result.unwrap();
        // Should return midnight
        assert_eq!(datetime.hour(), 0);
        assert_eq!(datetime.minute(), 0);
        assert_eq!(datetime.second(), 0);
    }
}

#[test]
fn test_time_parsing_with_jiff() {
    let test_cases = vec![
        ("3:30 pm", 15, 30),
        ("10:15 am", 10, 15),
        ("14:30", 14, 30),
        ("9:00 PM", 21, 0),
        ("12:00 PM", 12, 0),
        ("12:00 AM", 0, 0),
    ];

    for (input, expected_hour, expected_minute) in test_cases {
        let result = parse_to_zoned(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {}", input);
        let datetime = result.unwrap();
        assert_eq!(datetime.hour(), expected_hour);
        assert_eq!(datetime.minute(), expected_minute);
    }
}

#[test]
fn test_day_at_time_with_jiff() {
    let result = parse_to_zoned("tomorrow at 3:30 pm", Language::English);
    assert!(result.is_ok());
    let datetime = result.unwrap();
    assert_eq!(datetime.hour(), 15);
    assert_eq!(datetime.minute(), 30);

    let result = parse_to_zoned("next monday at 9:00 am", Language::English);
    assert!(result.is_ok());
    let datetime = result.unwrap();
    assert_eq!(datetime.hour(), 9);
    assert_eq!(datetime.minute(), 0);
}

#[test]
fn test_date_parsing_with_jiff() {
    let test_cases = vec![
        ("15/03/2024", 2024, 3, 15),
        ("31-12-2025", 2025, 12, 31),
        ("01/01/2023", 2023, 1, 1),
    ];

    for (input, year, month, day) in test_cases {
        let result = parse_to_zoned(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {}", input);
        let datetime = result.unwrap();
        assert_eq!(datetime.year(), year);
        assert_eq!(datetime.month(), month);
        assert_eq!(datetime.day(), day);
        assert_eq!(datetime.hour(), 0); // Should be midnight
    }
}
