use chumsky::{error::Rich, prelude::*};

use crate::{
    DayReference, DayTime, Direction, LanguageParser, RelativeTime, Result, StandardDate, Time,
    TimeExpression, TimeUnit, Weekday, WeekdayModifier,
    common::{
        ParserError, digit_number, four_digit_number, iso_datetime, keyword_ci, keywords,
        keywords_ci, longest, two_digit_number,
    },
    error::rich_errors_to_temps_error,
    time_utils,
};

/// Parser for German natural language time expressions.
///
/// German nouns (e.g., "Sekunden", "Minuten") are matched case-sensitively
/// to follow German orthographic rules, while abbreviations (e.g., "sek", "min")
/// are matched case-insensitively for convenience.
pub struct GermanParser;

fn whitespace_required<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    one_of(" \t\n\r")
        .labelled("whitespace")
        .repeated()
        .at_least(1)
        .ignored()
}

fn number<'a>() -> impl Parser<'a, &'a str, i64, ParserError<'a>> + Clone {
    // Source order is irrelevant: `keywords` tries the longest keyword first.
    choice((
        digit_number(),
        keywords([
            ("ein", 1i64),
            ("eine", 1),
            ("einem", 1),
            ("einen", 1),
            ("einer", 1),
            ("zwei", 2),
            ("drei", 3),
            ("vier", 4),
            ("fünf", 5),
            ("sechs", 6),
            ("sieben", 7),
            ("acht", 8),
            ("neun", 9),
            ("zehn", 10),
        ]),
    ))
    .labelled("Zahl")
}

fn time_unit<'a>() -> impl Parser<'a, &'a str, TimeUnit, ParserError<'a>> + Clone {
    choice((
        // Nouns keep their capitalisation; abbreviations stay case-insensitive.
        keywords([
            ("Sekunde", TimeUnit::Second),
            ("Sekunden", TimeUnit::Second),
            ("Minute", TimeUnit::Minute),
            ("Minuten", TimeUnit::Minute),
            ("Stunde", TimeUnit::Hour),
            ("Stunden", TimeUnit::Hour),
            ("Tag", TimeUnit::Day),
            ("Tage", TimeUnit::Day),
            ("Tagen", TimeUnit::Day),
            ("Woche", TimeUnit::Week),
            ("Wochen", TimeUnit::Week),
            ("Monat", TimeUnit::Month),
            ("Monate", TimeUnit::Month),
            ("Monaten", TimeUnit::Month),
            ("Jahr", TimeUnit::Year),
            ("Jahre", TimeUnit::Year),
            ("Jahren", TimeUnit::Year),
        ]),
        keywords_ci([
            ("sek", TimeUnit::Second),
            ("min", TimeUnit::Minute),
            ("std", TimeUnit::Hour),
        ]),
    ))
    .labelled("Zeiteinheit")
}

fn weekday<'a>() -> impl Parser<'a, &'a str, Weekday, ParserError<'a>> + Clone {
    choice((
        keywords([
            ("Montag", Weekday::Monday),
            ("Dienstag", Weekday::Tuesday),
            ("Mittwoch", Weekday::Wednesday),
            ("Donnerstag", Weekday::Thursday),
            ("Freitag", Weekday::Friday),
            ("Samstag", Weekday::Saturday),
            ("Sonntag", Weekday::Sunday),
        ]),
        keywords_ci([
            ("mo", Weekday::Monday),
            ("di", Weekday::Tuesday),
            ("mi", Weekday::Wednesday),
            ("do", Weekday::Thursday),
            ("fr", Weekday::Friday),
            ("sa", Weekday::Saturday),
            ("so", Weekday::Sunday),
        ]),
    ))
    .labelled("Wochentag")
}

fn day_shortcuts<'a>() -> impl Parser<'a, &'a str, DayReference, ParserError<'a>> + Clone {
    keywords_ci([
        ("heute", DayReference::Today),
        ("gestern", DayReference::Yesterday),
        ("morgen", DayReference::Tomorrow),
    ])
}

fn weekday_modifier<'a>() -> impl Parser<'a, &'a str, WeekdayModifier, ParserError<'a>> + Clone {
    choice((
        keyword_ci("letzten").to(WeekdayModifier::Last),
        keyword_ci("letzte").to(WeekdayModifier::Last),
        keyword_ci("nächsten").to(WeekdayModifier::Next),
        keyword_ci("nächste").to(WeekdayModifier::Next),
    ))
}

fn modified_weekday<'a>() -> impl Parser<'a, &'a str, DayReference, ParserError<'a>> + Clone {
    weekday_modifier()
        .then_ignore(whitespace_required())
        .then(weekday())
        .map(|(modifier, day)| DayReference::Weekday {
            day,
            modifier: Some(modifier),
        })
}

fn simple_weekday<'a>() -> impl Parser<'a, &'a str, DayReference, ParserError<'a>> + Clone {
    weekday().map(|day| DayReference::Weekday {
        day,
        modifier: None,
    })
}

fn day_reference<'a>() -> impl Parser<'a, &'a str, DayReference, ParserError<'a>> + Clone {
    longest(vec![
        day_shortcuts().boxed(),
        modified_weekday().boxed(),
        simple_weekday().boxed(),
    ])
}

fn time_digits<'a>() -> impl Parser<'a, &'a str, (u8, u8, u8), ParserError<'a>> + Clone {
    two_digit_number()
        .then_ignore(just(':'))
        .then(two_digit_number())
        .then(just(':').ignore_then(two_digit_number()).or_not())
        .try_map(|((hour, minute), second), span| {
            let second = second.unwrap_or(0);
            if time_utils::is_valid_24_hour_time(hour, minute, second) {
                Ok((hour, minute, second))
            } else {
                Err(Rich::custom(span, "invalid time"))
            }
        })
}

fn time_expr<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    time_digits()
        .then_ignore(
            whitespace_required()
                .ignore_then(keyword_ci("uhr"))
                .or_not(),
        )
        .map(|(hour, minute, second)| {
            TimeExpression::Time(Time {
                hour,
                minute,
                second,
                meridiem: None,
            })
        })
}

fn day_at_time<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    day_reference()
        .then_ignore(whitespace_required())
        .then_ignore(keyword_ci("um"))
        .then_ignore(whitespace_required())
        .then(time_digits())
        .then_ignore(
            whitespace_required()
                .ignore_then(keyword_ci("uhr"))
                .or_not(),
        )
        .map(|(day, (hour, minute, second))| {
            TimeExpression::DayTime(DayTime {
                day,
                time: Time {
                    hour,
                    minute,
                    second,
                    meridiem: None,
                },
            })
        })
}

fn relative_past<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    keyword_ci("vor")
        .ignore_then(whitespace_required())
        .ignore_then(number())
        .then_ignore(whitespace_required())
        .then(time_unit())
        .map(|(amount, unit)| {
            TimeExpression::Relative(RelativeTime {
                amount,
                unit,
                direction: Direction::Past,
            })
        })
}

fn relative_future<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    keyword_ci("in")
        .ignore_then(whitespace_required())
        .ignore_then(number())
        .then_ignore(whitespace_required())
        .then(time_unit())
        .map(|(amount, unit)| {
            TimeExpression::Relative(RelativeTime {
                amount,
                unit,
                direction: Direction::Future,
            })
        })
}

fn now_expr<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    keyword_ci("jetzt").to(TimeExpression::Now)
}

fn date_format<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    two_digit_number()
        .then_ignore(just('.'))
        .then(two_digit_number())
        .then_ignore(just('.'))
        .then(four_digit_number())
        .try_map(|((day, month), year), span| {
            if time_utils::is_valid_calendar_date(year, month, day) {
                Ok(TimeExpression::Date(StandardDate { day, month, year }))
            } else {
                Err(Rich::custom(span, "invalid calendar date"))
            }
        })
}

fn parser<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> {
    // Longest-match: see the English parser for why order must not matter.
    longest(vec![
        iso_datetime().labelled("ISO 8601 datetime").boxed(),
        date_format().labelled("Datum (TT.MM.JJJJ)").boxed(),
        day_at_time().labelled("Tag mit Uhrzeit").boxed(),
        now_expr().labelled("`jetzt`").boxed(),
        day_reference()
            .map(TimeExpression::Day)
            .labelled("Tagesangabe")
            .boxed(),
        time_expr().labelled("Uhrzeit").boxed(),
        relative_past().labelled("`vor <n> <Einheit>`").boxed(),
        relative_future().labelled("`in <n> <Einheit>`").boxed(),
    ])
    .padded()
    .then_ignore(end())
}

impl LanguageParser for GermanParser {
    fn parse(&self, input: &str) -> Result<TimeExpression> {
        parser()
            .parse(input)
            .into_result()
            .map_err(|errs| rich_errors_to_temps_error(input, errs))
    }
}
