use mockall::predicate::eq;
use mockall::*;
use temps_core::*;

// ===== Parsing Tests =====

#[test]
fn test_parse_with_different_languages() {
    // Test English
    let result = parse("now", Language::English).unwrap();
    assert_eq!(result, TimeExpression::Now);

    // Test German
    let result = parse("jetzt", Language::German).unwrap();
    assert_eq!(result, TimeExpression::Now);
}

#[test]
fn test_time_units() {
    let test_cases = vec![
        ("in 5 seconds", TimeUnit::Second, 5, Direction::Future),
        ("2 minutes ago", TimeUnit::Minute, 2, Direction::Past),
        ("in 3 hours", TimeUnit::Hour, 3, Direction::Future),
        ("4 days ago", TimeUnit::Day, 4, Direction::Past),
        ("in 2 weeks", TimeUnit::Week, 2, Direction::Future),
        ("3 months ago", TimeUnit::Month, 3, Direction::Past),
        ("in 1 year", TimeUnit::Year, 1, Direction::Future),
    ];

    for (input, expected_unit, expected_amount, expected_direction) in test_cases {
        let result = parse(input, Language::English).unwrap();
        match result {
            TimeExpression::Relative(rel) => {
                assert_eq!(rel.unit, expected_unit);
                assert_eq!(rel.amount, expected_amount);
                assert_eq!(rel.direction, expected_direction);
            }
            _ => panic!("Expected relative time expression"),
        }
    }
}

#[test]
fn test_language_specific_expressions() {
    // Test some common expressions
    let test_cases = vec![
        (
            "1 day ago",
            Language::English,
            TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit: TimeUnit::Day,
                direction: Direction::Past,
            }),
        ),
        (
            "in 1 day",
            Language::English,
            TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit: TimeUnit::Day,
                direction: Direction::Future,
            }),
        ),
        (
            "vor 1 Tag",
            Language::German,
            TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit: TimeUnit::Day,
                direction: Direction::Past,
            }),
        ),
        (
            "in 1 Tag",
            Language::German,
            TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit: TimeUnit::Day,
                direction: Direction::Future,
            }),
        ),
    ];

    for (input, lang, expected) in test_cases {
        let result = parse(input, lang).unwrap();
        assert_eq!(result, expected);
    }
}

#[test]
fn test_invalid_language_expressions() {
    // Try parsing German with English parser
    let result = parse("jetzt", Language::English);
    assert!(result.is_err());

    // Try parsing English with German parser
    let result = parse("now", Language::German);
    assert!(result.is_err());
}

// ===== Number Parsing Tests =====

#[test]
fn test_english_one_variations() {
    let test_cases = vec![
        "one second ago",
        "a second ago",
        "an hour ago",
        "one hour ago",
    ];

    for input in test_cases {
        let result = parse(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let expr = result.unwrap();
        if let TimeExpression::Relative(rel) = expr {
            assert_eq!(rel.amount, 1, "Expected amount 1 for: {input}");
        } else {
            panic!("Expected relative time expression for: {input}");
        }
    }
}

#[test]
fn test_german_one_variations() {
    let test_cases = vec![
        ("vor einem Tag", TimeUnit::Day),
        ("vor einer Woche", TimeUnit::Week),
        ("vor einem Monat", TimeUnit::Month),
        ("vor einem Jahr", TimeUnit::Year),
        ("vor einer Stunde", TimeUnit::Hour),
        ("vor einer Minute", TimeUnit::Minute),
        ("vor einer Sekunde", TimeUnit::Second),
    ];

    for (input, expected_unit) in test_cases {
        let result = parse(input, Language::German);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let expr = result.unwrap();
        if let TimeExpression::Relative(rel) = expr {
            assert_eq!(rel.amount, 1, "Expected amount 1 for: {input}");
            assert_eq!(rel.unit, expected_unit, "Wrong unit for: {input}");
            assert_eq!(rel.direction, Direction::Past);
        } else {
            panic!("Expected relative time expression for: {input}");
        }
    }
}

#[test]
fn test_mixed_case() {
    let test_cases = vec![
        ("NOW", Language::English),
        ("Now", Language::English),
        ("JETZT", Language::German),
        ("Jetzt", Language::German),
    ];

    for (input, lang) in test_cases {
        let result = parse(input, lang);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let expr = result.unwrap();
        assert_eq!(expr, TimeExpression::Now);
    }
}

// ===== Mock Tests =====

// Define a comprehensive mock provider
#[automock]
trait MockableTimeParser: Send + Sync {
    fn get_current_time(&self) -> i64;
    fn parse_time_expression(&self, expr: &str, lang: Language) -> temps_core::Result<i64>;
}

// A service that uses our mockable provider
struct TimeService<T: MockableTimeParser> {
    provider: T,
}

impl<T: MockableTimeParser> TimeService<T> {
    fn new(provider: T) -> Self {
        Self { provider }
    }

    fn parse_and_calculate_offset(&self, expr: &str, lang: Language) -> temps_core::Result<i64> {
        let parsed_time = self.provider.parse_time_expression(expr, lang)?;
        let current_time = self.provider.get_current_time();
        Ok(parsed_time - current_time)
    }

    fn is_in_past(&self, expr: &str, lang: Language) -> temps_core::Result<bool> {
        let offset = self.parse_and_calculate_offset(expr, lang)?;
        Ok(offset < 0)
    }

    fn is_in_future(&self, expr: &str, lang: Language) -> temps_core::Result<bool> {
        let offset = self.parse_and_calculate_offset(expr, lang)?;
        Ok(offset > 0)
    }
}

#[test]
fn test_service_with_mock_provider() {
    let mut mock = MockMockableTimeParser::new();

    // Set up expectations
    mock.expect_get_current_time()
        .times(1)
        .return_const(1000i64);

    mock.expect_parse_time_expression()
        .with(eq("in 5 hours"), eq(Language::English))
        .times(1)
        .return_const(Ok(19000i64)); // 1000 + 5*3600

    let service = TimeService::new(mock);
    let offset = service
        .parse_and_calculate_offset("in 5 hours", Language::English)
        .unwrap();
    assert_eq!(offset, 18000); // 5 hours in seconds
}

#[test]
fn test_is_in_past_with_mock() {
    let mut mock = MockMockableTimeParser::new();

    mock.expect_get_current_time()
        .times(1)
        .return_const(2000i64);

    mock.expect_parse_time_expression()
        .with(eq("2 hours ago"), eq(Language::English))
        .times(1)
        .return_const(Ok(-5200i64)); // 2000 - 2*3600

    let service = TimeService::new(mock);
    let is_past = service
        .is_in_past("2 hours ago", Language::English)
        .unwrap();
    assert!(is_past);
}

#[test]
fn test_is_in_future_with_mock() {
    let mut mock = MockMockableTimeParser::new();

    mock.expect_get_current_time()
        .times(1)
        .return_const(3000i64);

    mock.expect_parse_time_expression()
        .with(eq("in 3 days"), eq(Language::English))
        .times(1)
        .return_const(Ok(262200i64)); // 3000 + 3*24*3600

    let service = TimeService::new(mock);
    let is_future = service
        .is_in_future("in 3 days", Language::English)
        .unwrap();
    assert!(is_future);
}

#[test]
fn test_error_handling_with_mock() {
    let mut mock = MockMockableTimeParser::new();

    mock.expect_parse_time_expression()
        .with(eq("invalid expression"), eq(Language::English))
        .times(1)
        .return_const(Err(temps_core::TempsError::parse_error(
            "Failed to parse expression",
            "invalid expression",
        )));

    let service = TimeService::new(mock);
    let result = service.parse_and_calculate_offset("invalid expression", Language::English);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        temps_core::TempsError::parse_error("Failed to parse expression", "invalid expression")
    );
}

// Define a trait that can be mocked for date/time operations
#[automock]
trait DateTimeParser {
    fn current_year(&self) -> i32;
    fn current_month(&self) -> u32;
    fn current_day(&self) -> u32;
}

// A function that uses the trait
fn format_current_date(provider: &dyn DateTimeParser) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        provider.current_year(),
        provider.current_month(),
        provider.current_day()
    )
}

#[test]
fn test_format_date_with_mock() {
    let mut mock = MockDateTimeParser::new();

    // Set expectations
    mock.expect_current_year().times(1).return_const(2023);
    mock.expect_current_month().times(1).return_const(6u32);
    mock.expect_current_day().times(1).return_const(15u32);

    // Use the mock
    let result = format_current_date(&mock);
    assert_eq!(result, "2023-06-15");
}

// ===== Comprehensive Expression Parsing Tests =====

#[test]
fn test_parsing_english_expressions() {
    // Test that all English expressions parse correctly
    let test_cases = vec![
        ("now", TimeExpression::Now),
        (
            "in 5 seconds",
            TimeExpression::Relative(RelativeTime {
                amount: 5,
                unit: TimeUnit::Second,
                direction: Direction::Future,
            }),
        ),
        (
            "2 minutes ago",
            TimeExpression::Relative(RelativeTime {
                amount: 2,
                unit: TimeUnit::Minute,
                direction: Direction::Past,
            }),
        ),
        (
            "in an hour",
            TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit: TimeUnit::Hour,
                direction: Direction::Future,
            }),
        ),
        (
            "a day ago",
            TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit: TimeUnit::Day,
                direction: Direction::Past,
            }),
        ),
        (
            "in one week",
            TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit: TimeUnit::Week,
                direction: Direction::Future,
            }),
        ),
    ];

    for (input, expected) in test_cases {
        let result = parse(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        assert_eq!(parsed, expected, "Mismatch for input: {input}");
    }
}

#[test]
fn test_parsing_german_expressions() {
    // Test that all German expressions parse correctly
    let test_cases = vec![
        ("jetzt", TimeExpression::Now),
        (
            "in 5 Sekunden",
            TimeExpression::Relative(RelativeTime {
                amount: 5,
                unit: TimeUnit::Second,
                direction: Direction::Future,
            }),
        ),
        (
            "vor 2 Minuten",
            TimeExpression::Relative(RelativeTime {
                amount: 2,
                unit: TimeUnit::Minute,
                direction: Direction::Past,
            }),
        ),
        (
            "in einer Stunde",
            TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit: TimeUnit::Hour,
                direction: Direction::Future,
            }),
        ),
        (
            "vor einem Tag",
            TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit: TimeUnit::Day,
                direction: Direction::Past,
            }),
        ),
        (
            "in einer Woche",
            TimeExpression::Relative(RelativeTime {
                amount: 1,
                unit: TimeUnit::Week,
                direction: Direction::Future,
            }),
        ),
    ];

    for (input, expected) in test_cases {
        let result = parse(input, Language::German);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        assert_eq!(parsed, expected, "Mismatch for input: {input}");
    }
}

#[test]
fn test_articles_parsing() {
    // Test that "a", "an", "one" all parse to 1
    let articles = vec!["a", "an", "one"];

    for article in articles {
        let input = format!("in {article} hour");
        let result = parse(&input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {input}");

        let expr = result.unwrap();
        if let TimeExpression::Relative(rel) = expr {
            assert_eq!(
                rel.amount, 1,
                "Article '{article}' should parse to amount 1"
            );
        } else {
            panic!("Expected relative time expression for: {input}");
        }
    }
}

#[test]
fn test_iso_datetime_parsing() {
    let test_cases = vec![
        // Basic date
        "2024-01-15",
        // Date with time
        "2024-01-15T14:30:00",
        // Date with time and timezone
        "2024-01-15T14:30:00Z",
        "2024-01-15T14:30:00+02:00",
        "2024-01-15T14:30:00-05:00",
        // With fractional seconds
        "2024-01-15T14:30:00.123Z",
        // Space separator instead of T
        "2024-01-15 14:30:00",
    ];

    for input in test_cases {
        // Test English parser
        let result = parse(input, Language::English);
        assert!(
            result.is_ok(),
            "Failed to parse '{input}' with English parser"
        );
        let expr = result.unwrap();

        if let TimeExpression::Absolute(abs) = expr {
            // Verify the parsed components match the input
            match input {
                "2024-01-15" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, None);
                }
                "2024-01-15T14:30:00" | "2024-01-15 14:30:00" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, Some(14));
                    assert_eq!(abs.minute, Some(30));
                    assert_eq!(abs.second, Some(0));
                }
                "2024-01-15T14:30:00Z" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, Some(14));
                    assert_eq!(abs.minute, Some(30));
                    assert_eq!(abs.second, Some(0));
                    assert_eq!(abs.timezone, Some(crate::Timezone::Utc));
                }
                "2024-01-15T14:30:00+02:00" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, Some(14));
                    assert_eq!(abs.minute, Some(30));
                    assert_eq!(abs.second, Some(0));
                    assert_eq!(
                        abs.timezone,
                        Some(crate::Timezone::Offset {
                            hours: 2,
                            minutes: 0
                        })
                    );
                }
                "2024-01-15T14:30:00-05:00" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, Some(14));
                    assert_eq!(abs.minute, Some(30));
                    assert_eq!(abs.second, Some(0));
                    assert_eq!(
                        abs.timezone,
                        Some(crate::Timezone::Offset {
                            hours: -5,
                            minutes: 0
                        })
                    );
                }
                "2024-01-15T14:30:00.123Z" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, Some(14));
                    assert_eq!(abs.minute, Some(30));
                    assert_eq!(abs.second, Some(0));
                    assert_eq!(abs.nanosecond, Some(123000000)); // .123 seconds = 123 million nanoseconds
                    assert_eq!(abs.timezone, Some(crate::Timezone::Utc));
                }
                _ => panic!("Unhandled test case: {input}"),
            }
        } else {
            panic!("Expected absolute time expression for: {input}");
        }

        // Test German parser
        let result = parse(input, Language::German);
        assert!(
            result.is_ok(),
            "Failed to parse '{input}' with German parser"
        );
        let expr = result.unwrap();

        if let TimeExpression::Absolute(abs) = expr {
            // Verify the parsed components match the input
            match input {
                "2024-01-15" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, None);
                }
                "2024-01-15T14:30:00" | "2024-01-15 14:30:00" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, Some(14));
                    assert_eq!(abs.minute, Some(30));
                    assert_eq!(abs.second, Some(0));
                }
                "2024-01-15T14:30:00Z" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, Some(14));
                    assert_eq!(abs.minute, Some(30));
                    assert_eq!(abs.second, Some(0));
                    assert_eq!(abs.timezone, Some(crate::Timezone::Utc));
                }
                "2024-01-15T14:30:00+02:00" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, Some(14));
                    assert_eq!(abs.minute, Some(30));
                    assert_eq!(abs.second, Some(0));
                    assert_eq!(
                        abs.timezone,
                        Some(crate::Timezone::Offset {
                            hours: 2,
                            minutes: 0
                        })
                    );
                }
                "2024-01-15T14:30:00-05:00" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, Some(14));
                    assert_eq!(abs.minute, Some(30));
                    assert_eq!(abs.second, Some(0));
                    assert_eq!(
                        abs.timezone,
                        Some(crate::Timezone::Offset {
                            hours: -5,
                            minutes: 0
                        })
                    );
                }
                "2024-01-15T14:30:00.123Z" => {
                    assert_eq!(abs.year, 2024);
                    assert_eq!(abs.month, 1);
                    assert_eq!(abs.day, 15);
                    assert_eq!(abs.hour, Some(14));
                    assert_eq!(abs.minute, Some(30));
                    assert_eq!(abs.second, Some(0));
                    assert_eq!(abs.nanosecond, Some(123000000)); // .123 seconds = 123 million nanoseconds
                    assert_eq!(abs.timezone, Some(crate::Timezone::Utc));
                }
                _ => panic!("Unhandled test case: {input}"),
            }
        } else {
            panic!("Expected absolute time expression for: {input}");
        }
    }
}

#[test]
fn test_german_articles_parsing() {
    // Test German articles
    let test_cases = vec![
        ("in einem Tag", TimeUnit::Day),
        ("in einer Woche", TimeUnit::Week),
        ("in einem Monat", TimeUnit::Month),
        ("in einem Jahr", TimeUnit::Year),
    ];

    for (input, expected_unit) in test_cases {
        let result = parse(input, Language::German);
        assert!(result.is_ok(), "Failed to parse: {input}");

        let expr = result.unwrap();
        if let TimeExpression::Relative(rel) = expr {
            assert_eq!(rel.amount, 1, "German article should parse to amount 1");
            assert_eq!(rel.unit, expected_unit, "Unexpected unit for: {input}");
        } else {
            panic!("Expected relative time expression for: {input}");
        }
    }
}

#[test]
fn test_complex_mock_sequence() {
    let mut mock = MockMockableTimeParser::new();
    let mut seq = mockall::Sequence::new();

    // Parse "now"
    mock.expect_parse_time_expression()
        .with(eq("now"), eq(Language::English))
        .times(1)
        .in_sequence(&mut seq)
        .return_const(Ok(1000i64));

    // First call returns current time
    mock.expect_get_current_time()
        .times(1)
        .in_sequence(&mut seq)
        .return_const(1000i64);

    // Parse "in 1 hour"
    mock.expect_parse_time_expression()
        .with(eq("in 1 hour"), eq(Language::English))
        .times(1)
        .in_sequence(&mut seq)
        .return_const(Ok(5600i64)); // 2000 + 3600

    // Second call returns a later time
    mock.expect_get_current_time()
        .times(1)
        .in_sequence(&mut seq)
        .return_const(2000i64);

    let service = TimeService::new(mock);

    // First calculation
    let offset1 = service
        .parse_and_calculate_offset("now", Language::English)
        .unwrap();
    assert_eq!(offset1, 0);

    // Second calculation
    let offset2 = service
        .parse_and_calculate_offset("in 1 hour", Language::English)
        .unwrap();
    assert_eq!(offset2, 3600);
}

#[test]
fn test_parsing_with_specific_context() {
    // This test shows that parsing is deterministic and doesn't need time mocking
    let test_cases = vec![
        ("now", Language::English, TimeExpression::Now),
        ("jetzt", Language::German, TimeExpression::Now),
    ];

    for (input, lang, expected) in test_cases {
        let result = parse(input, lang).unwrap();
        assert_eq!(result, expected);
    }
}

#[test]
fn test_time_parsing_english() {
    let test_cases = vec![
        (
            "3:30 pm",
            Time {
                hour: 3,
                minute: 30,
                second: 0,
                meridiem: Some(Meridiem::PM),
            },
        ),
        (
            "10:15 am",
            Time {
                hour: 10,
                minute: 15,
                second: 0,
                meridiem: Some(Meridiem::AM),
            },
        ),
        (
            "12:00 pm",
            Time {
                hour: 12,
                minute: 0,
                second: 0,
                meridiem: Some(Meridiem::PM),
            },
        ),
        (
            "12:00 am",
            Time {
                hour: 12,
                minute: 0,
                second: 0,
                meridiem: Some(Meridiem::AM),
            },
        ),
        (
            "14:30",
            Time {
                hour: 14,
                minute: 30,
                second: 0,
                meridiem: None,
            },
        ),
        (
            "09:45:30",
            Time {
                hour: 9,
                minute: 45,
                second: 30,
                meridiem: None,
            },
        ),
    ];

    for (input, expected_time) in test_cases {
        let result = parse(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        if let TimeExpression::Time(time) = parsed {
            assert_eq!(time, expected_time, "Mismatch for input: {input}");
        } else {
            panic!("Expected Time expression for: {input}");
        }
    }
}

#[test]
fn test_day_at_time_english() {
    let test_cases = vec![
        "today at 3:30 pm",
        "tomorrow at 10:00 am",
        "monday at 14:30",
        "next friday at 9:00 pm",
        "last tuesday at 8:15 am",
    ];

    for input in test_cases {
        let result = parse(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        assert!(
            matches!(parsed, TimeExpression::DayTime(_)),
            "Expected DayTime expression for: {input}"
        );
    }
}

#[test]
fn test_date_format_parsing() {
    let test_cases = vec![
        (
            "15/03/2024",
            StandardDate {
                day: 15,
                month: 3,
                year: 2024,
            },
        ),
        (
            "01-12-2023",
            StandardDate {
                day: 1,
                month: 12,
                year: 2023,
            },
        ),
        (
            "31/12/2025",
            StandardDate {
                day: 31,
                month: 12,
                year: 2025,
            },
        ),
    ];

    for (input, expected_date) in test_cases {
        let result = parse(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        if let TimeExpression::Date(date) = parsed {
            assert_eq!(date, expected_date, "Mismatch for input: {input}");
        } else {
            panic!("Expected Date expression for: {input}");
        }
    }
}

#[test]
fn test_time_parsing_german() {
    let test_cases = vec![
        (
            "14:30",
            Time {
                hour: 14,
                minute: 30,
                second: 0,
                meridiem: None,
            },
        ),
        (
            "09:45 Uhr",
            Time {
                hour: 9,
                minute: 45,
                second: 0,
                meridiem: None,
            },
        ),
        (
            "23:59",
            Time {
                hour: 23,
                minute: 59,
                second: 0,
                meridiem: None,
            },
        ),
    ];

    for (input, expected_time) in test_cases {
        let result = parse(input, Language::German);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        if let TimeExpression::Time(time) = parsed {
            assert_eq!(time, expected_time, "Mismatch for input: {input}");
        } else {
            panic!("Expected Time expression for: {input}");
        }
    }
}

#[test]
fn test_day_at_time_german() {
    let test_cases = vec![
        "heute um 14:30",
        "morgen um 10:00 Uhr",
        "Montag um 15:45",
        "nächsten Freitag um 21:00",
    ];

    for input in test_cases {
        let result = parse(input, Language::German);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        assert!(
            matches!(parsed, TimeExpression::DayTime(_)),
            "Expected DayTime expression for: {input}"
        );
    }
}

#[test]
fn test_date_format_parsing_german() {
    let test_cases = vec![
        (
            "15.03.2024",
            StandardDate {
                day: 15,
                month: 3,
                year: 2024,
            },
        ),
        (
            "01.12.2023",
            StandardDate {
                day: 1,
                month: 12,
                year: 2023,
            },
        ),
        (
            "31.12.2025",
            StandardDate {
                day: 31,
                month: 12,
                year: 2025,
            },
        ),
    ];

    for (input, expected_date) in test_cases {
        let result = parse(input, Language::German);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        if let TimeExpression::Date(date) = parsed {
            assert_eq!(date, expected_date, "Mismatch for input: {input}");
        } else {
            panic!("Expected Date expression for: {input}");
        }
    }
}

#[test]
fn test_day_shortcuts_english() {
    let test_cases = vec![
        ("today", TimeExpression::Day(DayReference::Today)),
        ("tomorrow", TimeExpression::Day(DayReference::Tomorrow)),
        ("yesterday", TimeExpression::Day(DayReference::Yesterday)),
        ("TODAY", TimeExpression::Day(DayReference::Today)),
        ("Tomorrow", TimeExpression::Day(DayReference::Tomorrow)),
        ("YESTERDAY", TimeExpression::Day(DayReference::Yesterday)),
    ];

    for (input, expected) in test_cases {
        let result = parse(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        assert_eq!(parsed, expected, "Mismatch for input: {input}");
    }
}

#[test]
fn test_day_shortcuts_german() {
    let test_cases = vec![
        ("heute", TimeExpression::Day(DayReference::Today)),
        ("morgen", TimeExpression::Day(DayReference::Tomorrow)),
        ("gestern", TimeExpression::Day(DayReference::Yesterday)),
        ("HEUTE", TimeExpression::Day(DayReference::Today)),
        ("Morgen", TimeExpression::Day(DayReference::Tomorrow)),
        ("GESTERN", TimeExpression::Day(DayReference::Yesterday)),
    ];

    for (input, expected) in test_cases {
        let result = parse(input, Language::German);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        assert_eq!(parsed, expected, "Mismatch for input: {input}");
    }
}

#[test]
fn test_weekday_parsing_english() {
    let test_cases = vec![
        (
            "monday",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Monday,
                modifier: None,
            }),
        ),
        (
            "tuesday",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Tuesday,
                modifier: None,
            }),
        ),
        (
            "wednesday",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Wednesday,
                modifier: None,
            }),
        ),
        (
            "thursday",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Thursday,
                modifier: None,
            }),
        ),
        (
            "friday",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Friday,
                modifier: None,
            }),
        ),
        (
            "saturday",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Saturday,
                modifier: None,
            }),
        ),
        (
            "sunday",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Sunday,
                modifier: None,
            }),
        ),
        // Abbreviations
        (
            "mon",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Monday,
                modifier: None,
            }),
        ),
        (
            "tue",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Tuesday,
                modifier: None,
            }),
        ),
        (
            "wed",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Wednesday,
                modifier: None,
            }),
        ),
        (
            "thu",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Thursday,
                modifier: None,
            }),
        ),
        (
            "fri",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Friday,
                modifier: None,
            }),
        ),
        (
            "sat",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Saturday,
                modifier: None,
            }),
        ),
        (
            "sun",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Sunday,
                modifier: None,
            }),
        ),
    ];

    for (input, expected) in test_cases {
        let result = parse(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        assert_eq!(parsed, expected, "Mismatch for input: {input}");
    }
}

#[test]
fn test_weekday_modifiers_english() {
    let test_cases = vec![
        (
            "next monday",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Monday,
                modifier: Some(WeekdayModifier::Next),
            }),
        ),
        (
            "last friday",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Friday,
                modifier: Some(WeekdayModifier::Last),
            }),
        ),
        (
            "next sun",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Sunday,
                modifier: Some(WeekdayModifier::Next),
            }),
        ),
        (
            "last wed",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Wednesday,
                modifier: Some(WeekdayModifier::Last),
            }),
        ),
    ];

    for (input, expected) in test_cases {
        let result = parse(input, Language::English);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        assert_eq!(parsed, expected, "Mismatch for input: {input}");
    }
}

#[test]
fn test_weekday_parsing_german() {
    let test_cases = vec![
        (
            "Montag",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Monday,
                modifier: None,
            }),
        ),
        (
            "Dienstag",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Tuesday,
                modifier: None,
            }),
        ),
        (
            "Mittwoch",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Wednesday,
                modifier: None,
            }),
        ),
        (
            "Donnerstag",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Thursday,
                modifier: None,
            }),
        ),
        (
            "Freitag",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Friday,
                modifier: None,
            }),
        ),
        (
            "Samstag",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Saturday,
                modifier: None,
            }),
        ),
        (
            "Sonntag",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Sunday,
                modifier: None,
            }),
        ),
        // Abbreviations
        (
            "mo",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Monday,
                modifier: None,
            }),
        ),
        (
            "di",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Tuesday,
                modifier: None,
            }),
        ),
        (
            "mi",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Wednesday,
                modifier: None,
            }),
        ),
        (
            "do",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Thursday,
                modifier: None,
            }),
        ),
        (
            "fr",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Friday,
                modifier: None,
            }),
        ),
        (
            "sa",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Saturday,
                modifier: None,
            }),
        ),
        (
            "so",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Sunday,
                modifier: None,
            }),
        ),
    ];

    for (input, expected) in test_cases {
        let result = parse(input, Language::German);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        assert_eq!(parsed, expected, "Mismatch for input: {input}");
    }
}

#[test]
fn test_weekday_modifiers_german() {
    let test_cases = vec![
        (
            "nächsten Montag",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Monday,
                modifier: Some(WeekdayModifier::Next),
            }),
        ),
        (
            "letzten Freitag",
            TimeExpression::Day(DayReference::Weekday {
                day: Weekday::Friday,
                modifier: Some(WeekdayModifier::Last),
            }),
        ),
    ];

    for (input, expected) in test_cases {
        let result = parse(input, Language::German);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let parsed = result.unwrap();
        assert_eq!(parsed, expected, "Mismatch for input: {input}");
    }
}
