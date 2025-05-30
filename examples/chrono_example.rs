//! Example demonstrating the temps library with chrono integration
//!
//! This example shows how to parse various time expressions and work with them
//! using the chrono backend.

use chrono::Local;
use temps::chrono::*;

fn main() {
    println!("=== temps with chrono - Comprehensive Examples ===\n");

    // Example 1: Parsing relative times
    println!("1. Relative Time Expressions:");
    println!("------------------------------");

    // Future relative times
    let examples = vec![
        "in 5 minutes",
        "in 2 hours",
        "in 3 days",
        "in 1 week",
        "in 2 months",
        "in 1 year",
    ];

    for expr in examples {
        match parse_to_datetime(expr, Language::English) {
            Ok(dt) => println!("  '{}' => {}", expr, dt.format("%Y-%m-%d %H:%M:%S %Z")),
            Err(e) => println!("  '{}' => Error: {}", expr, e),
        }
    }

    // Past relative times
    println!("\n  Past times:");
    let past_examples = vec![
        "5 minutes ago",
        "2 hours ago",
        "3 days ago",
        "1 week ago",
        "2 months ago",
        "1 year ago",
    ];

    for expr in past_examples {
        match parse_to_datetime(expr, Language::English) {
            Ok(dt) => println!("  '{}' => {}", expr, dt.format("%Y-%m-%d %H:%M:%S %Z")),
            Err(e) => println!("  '{}' => Error: {}", expr, e),
        }
    }

    // Example 2: Parsing absolute dates and times
    println!("\n2. Absolute Date/Time Expressions:");
    println!("-----------------------------------");

    let absolute_examples = vec![
        "2024-12-25",
        "2024-12-25T15:30:00",
        "2024-12-25T15:30:00Z",
        "2024-12-25T15:30:00+05:30",
        "2024-12-25T15:30:00.123456789Z",
    ];

    for expr in absolute_examples {
        match parse_to_datetime(expr, Language::English) {
            Ok(dt) => {
                println!("  '{}' =>", expr);
                println!("    Formatted: {}", dt.format("%Y-%m-%d %H:%M:%S%.9f %Z"));
                println!("    Unix timestamp: {}", dt.timestamp());
            }
            Err(e) => println!("  '{}' => Error: {}", expr, e),
        }
    }

    // Example 3: Day references
    println!("\n3. Day References:");
    println!("------------------");

    let day_examples = vec![
        "today",
        "yesterday",
        "tomorrow",
        "monday",
        "next friday",
        "last sunday",
    ];

    for expr in day_examples {
        match parse_to_datetime(expr, Language::English) {
            Ok(dt) => {
                println!(
                    "  '{}' => {} ({})",
                    expr,
                    dt.format("%Y-%m-%d"),
                    dt.format("%A")
                );
            }
            Err(e) => println!("  '{}' => Error: {}", expr, e),
        }
    }

    // Example 4: Time expressions
    println!("\n4. Time Expressions:");
    println!("--------------------");

    let time_examples = vec!["3:30 PM", "15:30", "9:15:30 AM", "23:59:59"];

    for expr in time_examples {
        match parse_to_datetime(expr, Language::English) {
            Ok(dt) => {
                println!("  '{}' => Today at {}", expr, dt.format("%H:%M:%S"));
            }
            Err(e) => println!("  '{}' => Error: {}", expr, e),
        }
    }

    // Example 5: Combined day and time
    println!("\n5. Day + Time Combinations:");
    println!("----------------------------");

    let combined_examples = vec![
        "tomorrow at 3:30 PM",
        "yesterday at 9:00 AM",
        "next monday at 14:30",
    ];

    for expr in combined_examples {
        match parse_to_datetime(expr, Language::English) {
            Ok(dt) => {
                println!("  '{}' => {}", expr, dt.format("%Y-%m-%d %H:%M:%S (%A)"));
            }
            Err(e) => println!("  '{}' => Error: {}", expr, e),
        }
    }

    // Example 6: Standard date formats
    println!("\n6. Standard Date Formats:");
    println!("-------------------------");

    let date_examples = vec!["25/12/2024", "12/25/2024", "31.12.2024"];

    for expr in date_examples {
        match parse_to_datetime(expr, Language::English) {
            Ok(dt) => {
                println!("  '{}' => {}", expr, dt.format("%Y-%m-%d"));
            }
            Err(e) => println!("  '{}' => Error: {}", expr, e),
        }
    }

    // Example 7: Using the ChronoProvider directly
    println!("\n7. Using ChronoProvider Directly:");
    println!("---------------------------------");

    let provider = ChronoProvider;
    let now = provider.now();
    println!("  Current time: {}", now.format("%Y-%m-%d %H:%M:%S %Z"));

    // Create a relative time expression manually
    let expr = TimeExpression::Relative(RelativeTime {
        amount: 30,
        unit: TimeUnit::Day,
        direction: Direction::Future,
    });

    match provider.parse_expression(expr) {
        Ok(dt) => {
            println!("  30 days from now: {}", dt.format("%Y-%m-%d %H:%M:%S"));
            println!("  That's a {}", dt.format("%A"));
        }
        Err(e) => println!("  Error: {}", e),
    }

    // Example 8: Working with different languages
    println!("\n8. Multi-language Support:");
    println!("--------------------------");

    let german_examples = vec![
        ("vor 5 Minuten", "5 minutes ago"),
        ("in 2 Stunden", "in 2 hours"),
        ("gestern", "yesterday"),
        ("morgen um 15:30", "tomorrow at 15:30"),
    ];

    println!("  German examples:");
    for (german, english) in german_examples {
        match parse_to_datetime(german, Language::German) {
            Ok(dt) => println!(
                "    '{}' ({}) => {}",
                german,
                english,
                dt.format("%Y-%m-%d %H:%M:%S")
            ),
            Err(e) => println!("    '{}' => Error: {}", german, e),
        }
    }

    // Example 9: Error handling
    println!("\n9. Error Handling:");
    println!("------------------");

    let invalid_examples = vec![
        "invalid input",
        "32/13/2024", // Invalid date
        "25:00:00",   // Invalid time
    ];

    for expr in invalid_examples {
        match parse_to_datetime(expr, Language::English) {
            Ok(_) => println!("  '{}' => Unexpectedly succeeded!", expr),
            Err(e) => println!("  '{}' => Error (expected): {}", expr, e),
        }
    }

    // Example 10: Practical use case - scheduling
    println!("\n10. Practical Example - Task Scheduling:");
    println!("----------------------------------------");

    let now = Local::now();
    println!("  Current time: {}", now.format("%Y-%m-%d %H:%M:%S"));

    let schedule = vec![
        ("in 15 minutes", "Quick sync meeting"),
        ("tomorrow at 9:00 AM", "Team standup"),
        ("next friday at 17:00", "Weekly review"),
        ("in 1 month", "Project deadline"),
    ];

    println!("\n  Upcoming tasks:");
    for (time_expr, task) in schedule {
        match parse_to_datetime(time_expr, Language::English) {
            Ok(dt) => {
                let duration = dt.signed_duration_since(now);
                let days = duration.num_days();
                let hours = duration.num_hours() % 24;
                let minutes = duration.num_minutes() % 60;

                println!("    • {} - {}", task, dt.format("%Y-%m-%d %H:%M"));
                if days > 0 {
                    println!(
                        "      (in {} days, {} hours, {} minutes)",
                        days,
                        hours.abs(),
                        minutes.abs()
                    );
                } else if hours > 0 {
                    println!("      (in {} hours, {} minutes)", hours, minutes.abs());
                } else {
                    println!("      (in {} minutes)", minutes);
                }
            }
            Err(e) => println!("    • {} - Error: {}", task, e),
        }
    }
}
