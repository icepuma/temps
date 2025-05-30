//! Example demonstrating the temps library with jiff integration
//!
//! This example shows how to parse various time expressions and work with them
//! using the jiff backend.

use jiff::Zoned;
use temps::jiff::*;

fn main() {
    println!("=== temps with jiff - Comprehensive Examples ===\n");

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
        match parse_to_zoned(expr, Language::English) {
            Ok(dt) => println!("  '{}' => {}", expr, dt),
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
        match parse_to_zoned(expr, Language::English) {
            Ok(dt) => println!("  '{}' => {}", expr, dt),
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
        match parse_to_zoned(expr, Language::English) {
            Ok(dt) => {
                println!("  '{}' =>", expr);
                println!("    Formatted: {}", dt);
                println!("    Unix timestamp: {}", dt.timestamp().as_second());
                println!("    Timezone: {:?}", dt.time_zone());
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
        match parse_to_zoned(expr, Language::English) {
            Ok(dt) => {
                println!("  '{}' => {} ({:?})", expr, dt.date(), dt.weekday());
            }
            Err(e) => println!("  '{}' => Error: {}", expr, e),
        }
    }

    // Example 4: Time expressions
    println!("\n4. Time Expressions:");
    println!("--------------------");

    let time_examples = vec!["3:30 PM", "15:30", "9:15:30 AM", "23:59:59"];

    for expr in time_examples {
        match parse_to_zoned(expr, Language::English) {
            Ok(dt) => {
                println!("  '{}' => Today at {}", expr, dt.time());
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
        match parse_to_zoned(expr, Language::English) {
            Ok(dt) => {
                println!(
                    "  '{}' => {} {} ({:?})",
                    expr,
                    dt.date(),
                    dt.time(),
                    dt.weekday()
                );
            }
            Err(e) => println!("  '{}' => Error: {}", expr, e),
        }
    }

    // Example 6: Standard date formats
    println!("\n6. Standard Date Formats:");
    println!("-------------------------");

    let date_examples = vec!["25/12/2024", "12/25/2024", "31.12.2024"];

    for expr in date_examples {
        match parse_to_zoned(expr, Language::English) {
            Ok(dt) => {
                println!("  '{}' => {}", expr, dt.date());
            }
            Err(e) => println!("  '{}' => Error: {}", expr, e),
        }
    }

    // Example 7: Using the JiffProvider directly
    println!("\n7. Using JiffProvider Directly:");
    println!("---------------------------------");

    let provider = JiffProvider;
    let now = provider.now();
    println!("  Current time: {}", now);

    // Create a relative time expression manually
    let expr = TimeExpression::Relative(RelativeTime {
        amount: 30,
        unit: TimeUnit::Day,
        direction: Direction::Future,
    });

    match provider.parse_expression(expr) {
        Ok(dt) => {
            println!("  30 days from now: {}", dt);
            println!("  That's a {:?}", dt.weekday());
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
        match parse_to_zoned(german, Language::German) {
            Ok(dt) => println!("    '{}' ({}) => {}", german, english, dt),
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
        match parse_to_zoned(expr, Language::English) {
            Ok(_) => println!("  '{}' => Unexpectedly succeeded!", expr),
            Err(e) => println!("  '{}' => Error (expected): {}", expr, e),
        }
    }

    // Example 10: Practical use case - scheduling with jiff's powerful span arithmetic
    println!("\n10. Practical Example - Task Scheduling:");
    println!("----------------------------------------");

    let now = Zoned::now();
    println!("  Current time: {}", now);

    let schedule = vec![
        ("in 15 minutes", "Quick sync meeting"),
        ("tomorrow at 9:00 AM", "Team standup"),
        ("next friday at 17:00", "Weekly review"),
        ("in 1 month", "Project deadline"),
    ];

    println!("\n  Upcoming tasks:");
    for (time_expr, task) in schedule {
        match parse_to_zoned(time_expr, Language::English) {
            Ok(dt) => {
                // Calculate the span between now and the scheduled time
                match now.until(&dt) {
                    Ok(span) => {
                        println!("    • {} - {}", task, dt);

                        // Format the duration in a human-readable way
                        if span.get_days() > 0 {
                            println!(
                                "      (in {} days, {} hours, {} minutes)",
                                span.get_days(),
                                span.get_hours(),
                                span.get_minutes()
                            );
                        } else if span.get_hours() > 0 {
                            println!(
                                "      (in {} hours, {} minutes)",
                                span.get_hours(),
                                span.get_minutes()
                            );
                        } else {
                            println!("      (in {} minutes)", span.get_minutes());
                        }
                    }
                    Err(e) => println!("    • {} - Error calculating duration: {}", task, e),
                }
            }
            Err(e) => println!("    • {} - Error: {}", task, e),
        }
    }

    // Example 11: Advanced jiff features - timezone handling
    println!("\n11. Advanced Timezone Handling:");
    println!("--------------------------------");

    match parse_to_zoned("2024-12-25T15:30:00+05:30", Language::English) {
        Ok(dt) => {
            println!("  Original time: {}", dt);

            // Convert to different timezones using jiff's timezone support
            use jiff::tz::TimeZone;

            if let Ok(utc_tz) = TimeZone::get("UTC") {
                let utc_time = dt.with_time_zone(utc_tz);
                println!("  In UTC: {}", utc_time);
            }

            if let Ok(ny_tz) = TimeZone::get("America/New_York") {
                let ny_time = dt.with_time_zone(ny_tz);
                println!("  In New York: {}", ny_time);
            }

            if let Ok(tokyo_tz) = TimeZone::get("Asia/Tokyo") {
                let tokyo_time = dt.with_time_zone(tokyo_tz);
                println!("  In Tokyo: {}", tokyo_time);
            }
        }
        Err(e) => println!("  Error: {}", e),
    }

    // Example 12: Working with spans and date arithmetic
    println!("\n12. Date Arithmetic with Jiff:");
    println!("-------------------------------");

    match parse_to_zoned("2024-01-31", Language::English) {
        Ok(dt) => {
            println!("  Starting date: {}", dt);

            // Add various spans to see jiff's smart date handling
            use jiff::Span;

            // Adding months handles month-end correctly
            if let Ok(plus_1_month) = dt.checked_add(Span::new().months(1)) {
                println!("  + 1 month: {} (handles month-end)", plus_1_month.date());
            }

            // Complex span arithmetic
            if let Ok(complex) =
                dt.checked_add(Span::new().years(1).months(2).days(15).hours(3).minutes(30))
            {
                println!(
                    "  + 1 year, 2 months, 15 days, 3 hours, 30 minutes: {}",
                    complex
                );
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
}
