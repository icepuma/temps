//! Core test helpers that are shared across multiple test suites

use temps_core::{Direction, Language, RelativeTime, TimeExpression, TimeUnit};

/// Common test cases for relative time expressions in English
pub fn english_relative_time_test_cases() -> Vec<(&'static str, TimeExpression)> {
    vec![
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
    ]
}

/// Common test cases for relative time expressions in German
pub fn german_relative_time_test_cases() -> Vec<(&'static str, TimeExpression)> {
    vec![
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
    ]
}

/// Test cases for language-specific expressions
pub fn language_specific_test_cases() -> Vec<(&'static str, Language, TimeExpression)> {
    vec![
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
    ]
}
