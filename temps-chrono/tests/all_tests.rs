use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use mockall::*;
use temps_chrono::*;
use temps_core::*;
use temps_testhelpers::chrono::{MockTimeSource, TimeSource, test_dates};

// ===== Integration Tests =====

#[test]
fn test_time_provider_trait() {
    let provider = ChronoProvider;
    let now = provider.now();
    // Basic test that we can create a provider and get current time
    assert!(now > DateTime::<Local>::default());
}

#[test]
fn test_chrono_provider_consistency() {
    let provider = ChronoProvider;

    // Test that parsing "now" returns the current time (approximately)
    let now = provider.now();
    let parsed_now = provider.parse_expression(TimeExpression::Now).unwrap();

    // They should be very close (within a second)
    let diff = parsed_now.signed_duration_since(now).num_seconds().abs();
    assert!(
        diff < 1,
        "Parsed 'now' should be within 1 second of actual now"
    );
}

// ===== Date Arithmetic Tests =====

#[test]
fn test_month_arithmetic_edge_cases() {
    // Test that parsing "in 1 month" works
    let result = parse_to_datetime("in 1 month", Language::English);
    assert!(result.is_ok());
}

#[test]
fn test_leap_year_handling() {
    // Test that February 29, 2024 + 1 year = February 28, 2025
    // We can't test exact dates without mocking, but we can test that the parsing works

    let result = parse_to_datetime("in 1 year", Language::English);
    assert!(result.is_ok());

    let result = parse_to_datetime("1 year ago", Language::English);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_years() {
    // Test multiple year arithmetic
    let result = parse_to_datetime("in 5 years", Language::English);
    assert!(result.is_ok());

    let result = parse_to_datetime("10 years ago", Language::English);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_months() {
    // Test multiple month arithmetic
    let result = parse_to_datetime("in 18 months", Language::English);
    assert!(result.is_ok());

    let result = parse_to_datetime("6 months ago", Language::English);
    assert!(result.is_ok());
}

#[test]
fn test_date_arithmetic_consistency() {
    // Test that going forward and then backward by the same amount doesn't always
    // return to the exact same date (due to month length differences)
    // This is expected behavior

    let provider = ChronoProvider;

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

// Example of basic mock usage
#[automock]
trait BasicTimeSource {
    fn now(&self) -> DateTime<Local>;
}

// A service that uses TimeSource
struct TimeCalculator<T: BasicTimeSource> {
    time_source: T,
}

impl<T: BasicTimeSource> TimeCalculator<T> {
    fn new(time_source: T) -> Self {
        Self { time_source }
    }

    fn hours_from_now(&self, hours: i64) -> DateTime<Local> {
        self.time_source.now() + chrono::Duration::hours(hours)
    }

    fn days_ago(&self, days: i64) -> DateTime<Local> {
        self.time_source.now() - chrono::Duration::days(days)
    }
}

#[test]
fn test_hours_from_now_with_mock() {
    let mut mock = MockBasicTimeSource::new();

    // Mock the time to be June 15, 2023 14:30:00
    let fixed_time = test_dates::june_15_2023();
    mock.expect_now().times(1).return_const(fixed_time);

    let calculator = TimeCalculator::new(mock);
    let result = calculator.hours_from_now(2);

    // Should be 16:30:00 on the same day
    assert_eq!(result.hour(), 16);
    assert_eq!(result.minute(), 30);
}

#[test]
fn test_days_ago_with_mock() {
    let mut mock = MockBasicTimeSource::new();

    // Mock the time
    let fixed_time = Local.with_ymd_and_hms(2023, 6, 15, 12, 0, 0).unwrap();
    mock.expect_now().times(1).return_const(fixed_time);

    let calculator = TimeCalculator::new(mock);
    let result = calculator.days_ago(5);

    // Should be June 10, 2023
    assert_eq!(result.day(), 10);
    assert_eq!(result.month(), 6);
}

#[test]
fn test_multiple_calls_with_sequence() {
    let mut mock = MockBasicTimeSource::new();

    // Set up a sequence of different times
    let mut seq = Sequence::new();

    let time1 = Local.with_ymd_and_hms(2023, 1, 1, 10, 0, 0).unwrap();
    let time2 = Local.with_ymd_and_hms(2023, 1, 1, 11, 0, 0).unwrap();

    mock.expect_now()
        .times(1)
        .in_sequence(&mut seq)
        .return_const(time1);

    mock.expect_now()
        .times(1)
        .in_sequence(&mut seq)
        .return_const(time2);

    let calculator = TimeCalculator::new(mock);

    // First call gets time1
    let result1 = calculator.hours_from_now(0);
    assert_eq!(result1.hour(), 10);

    // Second call gets time2
    let result2 = calculator.hours_from_now(0);
    assert_eq!(result2.hour(), 11);
}

// ===== Advanced Mock Tests with Expression Parsing =====

// Testable wrapper around ChronoProvider
struct TestableChronoProvider<T: TimeSource> {
    time_source: T,
}

impl<T: TimeSource> TestableChronoProvider<T> {
    fn new(time_source: T) -> Self {
        Self { time_source }
    }
}

impl<T: TimeSource> TimeParser for TestableChronoProvider<T> {
    type DateTime = DateTime<Local>;
    type Error = String;

    fn now(&self) -> Self::DateTime {
        self.time_source.now()
    }

    fn parse_expression(&self, expr: TimeExpression) -> Result<Self::DateTime, Self::Error> {
        let now = self.now();
        match expr {
            TimeExpression::Now => Ok(now),
            TimeExpression::Relative(rel) => {
                use chrono::{Duration, Months};

                match rel.unit {
                    TimeUnit::Second => {
                        let duration = Duration::seconds(rel.amount);
                        match rel.direction {
                            Direction::Past => Ok(now - duration),
                            Direction::Future => Ok(now + duration),
                        }
                    }
                    TimeUnit::Minute => {
                        let duration = Duration::minutes(rel.amount);
                        match rel.direction {
                            Direction::Past => Ok(now - duration),
                            Direction::Future => Ok(now + duration),
                        }
                    }
                    TimeUnit::Hour => {
                        let duration = Duration::hours(rel.amount);
                        match rel.direction {
                            Direction::Past => Ok(now - duration),
                            Direction::Future => Ok(now + duration),
                        }
                    }
                    TimeUnit::Day => {
                        let duration = Duration::days(rel.amount);
                        match rel.direction {
                            Direction::Past => Ok(now - duration),
                            Direction::Future => Ok(now + duration),
                        }
                    }
                    TimeUnit::Week => {
                        let duration = Duration::weeks(rel.amount);
                        match rel.direction {
                            Direction::Past => Ok(now - duration),
                            Direction::Future => Ok(now + duration),
                        }
                    }
                    TimeUnit::Month => {
                        let months =
                            Months::new(rel.amount.try_into().map_err(|_| {
                                "Month amount must be a positive number".to_string()
                            })?);

                        match rel.direction {
                            Direction::Past => now.checked_sub_months(months).ok_or_else(|| {
                                "Date calculation resulted in invalid date".to_string()
                            }),
                            Direction::Future => now.checked_add_months(months).ok_or_else(|| {
                                "Date calculation resulted in invalid date".to_string()
                            }),
                        }
                    }
                    TimeUnit::Year => {
                        let months =
                            Months::new((rel.amount * 12).try_into().map_err(|_| {
                                "Year amount must be a positive number".to_string()
                            })?);

                        match rel.direction {
                            Direction::Past => now.checked_sub_months(months).ok_or_else(|| {
                                "Date calculation resulted in invalid date".to_string()
                            }),
                            Direction::Future => now.checked_add_months(months).ok_or_else(|| {
                                "Date calculation resulted in invalid date".to_string()
                            }),
                        }
                    }
                }
            }
            TimeExpression::Absolute(abs) => {
                ChronoProvider.parse_expression(TimeExpression::Absolute(abs))
            }
            TimeExpression::Day(day_ref) => {
                ChronoProvider.parse_expression(TimeExpression::Day(day_ref))
            }
            TimeExpression::Time(time) => {
                ChronoProvider.parse_expression(TimeExpression::Time(time))
            }
            TimeExpression::DayTime(day_time) => {
                ChronoProvider.parse_expression(TimeExpression::DayTime(day_time))
            }
            TimeExpression::Date(date) => {
                ChronoProvider.parse_expression(TimeExpression::Date(date))
            }
        }
    }
}

#[test]
fn test_english_time_expressions_with_mock() {
    let mut mock = MockTimeSource::new();
    let base_time = Local.with_ymd_and_hms(2024, 3, 15, 10, 30, 0).unwrap();
    mock.expect_now().return_const(base_time);

    let provider = TestableChronoProvider::new(mock);

    // Test various English natural language expressions
    let test_cases = vec![
        // Seconds
        ("in 30 seconds", base_time + chrono::Duration::seconds(30)),
        ("45 seconds ago", base_time - chrono::Duration::seconds(45)),
        // Minutes
        ("in 5 minutes", base_time + chrono::Duration::minutes(5)),
        ("10 minutes ago", base_time - chrono::Duration::minutes(10)),
        ("in a minute", base_time + chrono::Duration::minutes(1)),
        ("an hour ago", base_time - chrono::Duration::hours(1)),
        // Hours
        ("in 2 hours", base_time + chrono::Duration::hours(2)),
        ("3 hours ago", base_time - chrono::Duration::hours(3)),
        // Days
        ("in 1 day", base_time + chrono::Duration::days(1)),
        ("2 days ago", base_time - chrono::Duration::days(2)),
        ("in a day", base_time + chrono::Duration::days(1)),
        // Weeks
        ("in 1 week", base_time + chrono::Duration::weeks(1)),
        ("2 weeks ago", base_time - chrono::Duration::weeks(2)),
        ("in a week", base_time + chrono::Duration::weeks(1)),
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
    let base_time = Local.with_ymd_and_hms(2024, 3, 15, 10, 30, 0).unwrap();
    mock.expect_now().return_const(base_time);

    let provider = TestableChronoProvider::new(mock);

    // Test various German natural language expressions
    let test_cases = vec![
        // Seconds
        ("in 30 Sekunden", base_time + chrono::Duration::seconds(30)),
        ("vor 45 Sekunden", base_time - chrono::Duration::seconds(45)),
        // Minutes
        ("in 5 Minuten", base_time + chrono::Duration::minutes(5)),
        ("vor 10 Minuten", base_time - chrono::Duration::minutes(10)),
        ("in einer Minute", base_time + chrono::Duration::minutes(1)),
        ("vor einer Stunde", base_time - chrono::Duration::hours(1)),
        // Hours
        ("in 2 Stunden", base_time + chrono::Duration::hours(2)),
        ("vor 3 Stunden", base_time - chrono::Duration::hours(3)),
        // Days
        ("in 1 Tag", base_time + chrono::Duration::days(1)),
        ("vor 2 Tagen", base_time - chrono::Duration::days(2)),
        ("in einem Tag", base_time + chrono::Duration::days(1)),
        // Weeks
        ("in 1 Woche", base_time + chrono::Duration::weeks(1)),
        ("vor 2 Wochen", base_time - chrono::Duration::weeks(2)),
        ("in einer Woche", base_time + chrono::Duration::weeks(1)),
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
    let jan_31_2024 = Local.with_ymd_and_hms(2024, 1, 31, 10, 0, 0).unwrap();
    mock.expect_now().return_const(jan_31_2024);

    let provider = TestableChronoProvider::new(mock);
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
    let jan_31_2023 = Local.with_ymd_and_hms(2023, 1, 31, 10, 0, 0).unwrap();
    mock.expect_now().return_const(jan_31_2023);

    let provider = TestableChronoProvider::new(mock);
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
    let feb_29 = Local.with_ymd_and_hms(2024, 2, 29, 12, 0, 0).unwrap();
    mock.expect_now().return_const(feb_29);

    let provider = TestableChronoProvider::new(mock);
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
    let oct_15 = Local.with_ymd_and_hms(2023, 10, 15, 9, 0, 0).unwrap();
    mock.expect_now().return_const(oct_15);

    let provider = TestableChronoProvider::new(mock);
    let expr = parse("in 6 months", Language::English).unwrap();
    let result = provider.parse_expression(expr).unwrap();

    assert_eq!(result.year(), 2024);
    assert_eq!(result.month(), 4);
    assert_eq!(result.day(), 15);
}

#[test]
fn test_complex_expressions_with_articles() {
    let mut mock = MockTimeSource::new();
    let base_time = Local.with_ymd_and_hms(2024, 3, 15, 10, 30, 0).unwrap();
    mock.expect_now().return_const(base_time);

    let provider = TestableChronoProvider::new(mock);

    // Test English articles: "a", "an", "one"
    let english_cases = vec![
        ("in a second", base_time + chrono::Duration::seconds(1)),
        ("in an hour", base_time + chrono::Duration::hours(1)),
        ("in one day", base_time + chrono::Duration::days(1)),
        ("a minute ago", base_time - chrono::Duration::minutes(1)),
        ("an hour ago", base_time - chrono::Duration::hours(1)),
        ("one week ago", base_time - chrono::Duration::weeks(1)),
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
    let base_time = Local.with_ymd_and_hms(2024, 3, 15, 10, 30, 0).unwrap();
    mock.expect_now().return_const(base_time);

    let provider = TestableChronoProvider::new(mock);

    // Test German articles: "ein", "eine", "einem", "einer", etc.
    let german_cases = vec![
        ("in einer Sekunde", base_time + chrono::Duration::seconds(1)),
        ("in einer Minute", base_time + chrono::Duration::minutes(1)),
        ("in einer Stunde", base_time + chrono::Duration::hours(1)),
        ("in einem Tag", base_time + chrono::Duration::days(1)),
        ("in einer Woche", base_time + chrono::Duration::weeks(1)),
        (
            "vor einer Sekunde",
            base_time - chrono::Duration::seconds(1),
        ),
        ("vor einem Tag", base_time - chrono::Duration::days(1)),
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
    let current_time = Local.with_ymd_and_hms(2024, 3, 15, 10, 30, 0).unwrap();
    mock.expect_now().return_const(current_time);

    let provider = TestableChronoProvider::new(mock);

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
fn test_iso_datetime_absolute_time() {
    let provider = ChronoProvider;

    let test_cases = vec![
        // Basic RFC3339 dates
        "2024-01-15T14:30:00Z",
        "2024-01-15T14:30:00+02:00",
        "2024-01-15T14:30:00.123Z",
    ];

    for input in test_cases {
        let expr = parse(input, Language::English).unwrap();
        let result = provider.parse_expression(expr);
        assert!(result.is_ok(), "Failed to parse absolute time: {}", input);

        // Verify it's a valid datetime with expected values
        let datetime = result.unwrap();
        assert_eq!(datetime.year(), 2024, "Year should be 2024");
        assert_eq!(datetime.month(), 1, "Month should be January");
        assert_eq!(datetime.day(), 15, "Day should be 15");
    }
}

#[test]
fn test_day_references_with_chrono() {
    let test_cases = vec![
        ("today", Language::English),
        ("yesterday", Language::English),
        ("tomorrow", Language::English),
        ("heute", Language::German),
        ("gestern", Language::German),
        ("morgen", Language::German),
    ];

    for (input, lang) in test_cases {
        let result = parse_to_datetime(input, lang);
        assert!(result.is_ok(), "Failed to parse: {}", input);
        let datetime = result.unwrap();
        // Should return midnight of the respective day
        assert_eq!(datetime.hour(), 0);
        assert_eq!(datetime.minute(), 0);
        assert_eq!(datetime.second(), 0);
    }
}

#[test]
fn test_weekday_parsing_with_chrono() {
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
        let result = parse_to_datetime(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {}", input);
        let datetime = result.unwrap();
        // Should return midnight
        assert_eq!(datetime.hour(), 0);
        assert_eq!(datetime.minute(), 0);
        assert_eq!(datetime.second(), 0);
    }
}

#[test]
fn test_time_parsing_with_chrono() {
    let test_cases = vec![
        ("3:30 pm", 15, 30),
        ("10:15 am", 10, 15),
        ("14:30", 14, 30),
        ("9:00 PM", 21, 0),
        ("12:00 PM", 12, 0),
        ("12:00 AM", 0, 0),
    ];

    for (input, expected_hour, expected_minute) in test_cases {
        let result = parse_to_datetime(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {}", input);
        let datetime = result.unwrap();
        assert_eq!(datetime.hour(), expected_hour);
        assert_eq!(datetime.minute(), expected_minute);
    }
}

#[test]
fn test_day_at_time_with_chrono() {
    let result = parse_to_datetime("tomorrow at 3:30 pm", Language::English);
    assert!(result.is_ok());
    let datetime = result.unwrap();
    assert_eq!(datetime.hour(), 15);
    assert_eq!(datetime.minute(), 30);

    let result = parse_to_datetime("next monday at 9:00 am", Language::English);
    assert!(result.is_ok());
    let datetime = result.unwrap();
    assert_eq!(datetime.hour(), 9);
    assert_eq!(datetime.minute(), 0);
}

#[test]
fn test_date_parsing_with_chrono() {
    let test_cases = vec![
        ("15/03/2024", 2024, 3, 15),
        ("31-12-2025", 2025, 12, 31),
        ("01/01/2023", 2023, 1, 1),
    ];

    for (input, year, month, day) in test_cases {
        let result = parse_to_datetime(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {}", input);
        let datetime = result.unwrap();
        assert_eq!(datetime.year(), year);
        assert_eq!(datetime.month(), month);
        assert_eq!(datetime.day(), day);
        assert_eq!(datetime.hour(), 0); // Should be midnight
    }
}
