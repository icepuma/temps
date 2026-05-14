use chumsky::{error::Rich, prelude::*};

use crate::{
    DayReference, DayTime, Direction, LanguageParser, RelativeTime, Result, StandardDate, Time,
    TimeExpression, TimeUnit, Weekday, WeekdayModifier,
    common::{
        ParserError, digit_number, four_digit_number, iso_datetime, keyword_ci, two_digit_number,
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
    choice((
        digit_number(),
        choice((
            just("einem").to(1),
            just("einer").to(1),
            just("einen").to(1),
            just("eine").to(1),
            just("ein").to(1),
        )),
        choice((
            just("zwei").to(2),
            just("drei").to(3),
            just("vier").to(4),
            just("fünf").to(5),
            just("sechs").to(6),
        )),
        choice((
            just("sieben").to(7),
            just("acht").to(8),
            just("neun").to(9),
            just("zehn").to(10),
        )),
    ))
    .labelled("Zahl")
}

fn time_unit<'a>() -> impl Parser<'a, &'a str, TimeUnit, ParserError<'a>> + Clone {
    choice((
        choice((
            just("Sekunden").to(TimeUnit::Second),
            just("Sekunde").to(TimeUnit::Second),
            keyword_ci("sek").to(TimeUnit::Second),
        )),
        choice((
            just("Minuten").to(TimeUnit::Minute),
            just("Minute").to(TimeUnit::Minute),
            keyword_ci("min").to(TimeUnit::Minute),
        )),
        choice((
            just("Stunden").to(TimeUnit::Hour),
            just("Stunde").to(TimeUnit::Hour),
            keyword_ci("std").to(TimeUnit::Hour),
        )),
        choice((
            just("Tagen").to(TimeUnit::Day),
            just("Tage").to(TimeUnit::Day),
            just("Tag").to(TimeUnit::Day),
        )),
        choice((
            just("Wochen").to(TimeUnit::Week),
            just("Woche").to(TimeUnit::Week),
        )),
        choice((
            just("Monaten").to(TimeUnit::Month),
            just("Monate").to(TimeUnit::Month),
            just("Monat").to(TimeUnit::Month),
        )),
        choice((
            just("Jahren").to(TimeUnit::Year),
            just("Jahre").to(TimeUnit::Year),
            just("Jahr").to(TimeUnit::Year),
        )),
    ))
    .labelled("Zeiteinheit")
}

fn weekday<'a>() -> impl Parser<'a, &'a str, Weekday, ParserError<'a>> + Clone {
    choice((
        choice((
            just("Montag").to(Weekday::Monday),
            keyword_ci("mo").to(Weekday::Monday),
        )),
        choice((
            just("Dienstag").to(Weekday::Tuesday),
            keyword_ci("di").to(Weekday::Tuesday),
        )),
        choice((
            just("Mittwoch").to(Weekday::Wednesday),
            keyword_ci("mi").to(Weekday::Wednesday),
        )),
        choice((
            just("Donnerstag").to(Weekday::Thursday),
            keyword_ci("do").to(Weekday::Thursday),
        )),
        choice((
            just("Freitag").to(Weekday::Friday),
            keyword_ci("fr").to(Weekday::Friday),
        )),
        choice((
            just("Samstag").to(Weekday::Saturday),
            keyword_ci("sa").to(Weekday::Saturday),
        )),
        choice((
            just("Sonntag").to(Weekday::Sunday),
            keyword_ci("so").to(Weekday::Sunday),
        )),
    ))
    .labelled("Wochentag")
}

fn day_shortcuts<'a>() -> impl Parser<'a, &'a str, DayReference, ParserError<'a>> + Clone {
    choice((
        keyword_ci("heute").to(DayReference::Today),
        keyword_ci("gestern").to(DayReference::Yesterday),
        keyword_ci("morgen").to(DayReference::Tomorrow),
    ))
}

fn weekday_modifier<'a>() -> impl Parser<'a, &'a str, WeekdayModifier, ParserError<'a>> + Clone {
    choice((
        just("letzten").to(WeekdayModifier::Last),
        just("letzte").to(WeekdayModifier::Last),
        just("nächsten").to(WeekdayModifier::Next),
        just("nächste").to(WeekdayModifier::Next),
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
    choice((day_shortcuts(), modified_weekday(), simple_weekday()))
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
    choice((
        iso_datetime().labelled("ISO 8601 datetime"),
        date_format().labelled("Datum (TT.MM.JJJJ)"),
        day_at_time().labelled("Tag mit Uhrzeit"),
        now_expr().labelled("`jetzt`"),
        day_reference()
            .map(TimeExpression::Day)
            .labelled("Tagesangabe"),
        time_expr().labelled("Uhrzeit"),
        relative_past().labelled("`vor <n> <Einheit>`"),
        relative_future().labelled("`in <n> <Einheit>`"),
    ))
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
