use winnow::{
    Parser,
    ascii::{Caseless, multispace0, multispace1},
    combinator::{alt, delimited, opt, preceded},
};

use crate::{
    DayReference, DayTime, Direction, LanguageParser, RelativeTime, Result, StandardDate, Time,
    TimeExpression, TimeUnit, Weekday, WeekdayModifier, common, error::ParseErrorExt, time_utils,
};

/// Parser for German natural language time expressions.
///
/// German nouns (e.g., "Sekunden", "Minuten") are matched case-sensitively
/// to follow German orthographic rules, while abbreviations (e.g., "sek", "min")
/// are matched case-insensitively for convenience.
pub struct GermanParser;

impl GermanParser {
    fn parse_number(input: &mut &str) -> winnow::Result<i64> {
        alt((
            common::parse_digit_number,
            alt((
                "einem".value(1),
                "einer".value(1),
                "einen".value(1),
                "eine".value(1),
                "ein".value(1),
            )),
            alt((
                "zwei".value(2),
                "drei".value(3),
                "vier".value(4),
                "fünf".value(5),
                "sechs".value(6),
            )),
            alt((
                "sieben".value(7),
                "acht".value(8),
                "neun".value(9),
                "zehn".value(10),
            )),
        ))
        .parse_next(input)
    }

    fn parse_time_unit(input: &mut &str) -> winnow::Result<TimeUnit> {
        alt((
            alt((
                "Sekunden".value(TimeUnit::Second),
                "Sekunde".value(TimeUnit::Second),
                Caseless("sek").value(TimeUnit::Second), // Abbreviations can be case-insensitive
            )),
            alt((
                "Minuten".value(TimeUnit::Minute),
                "Minute".value(TimeUnit::Minute),
                Caseless("min").value(TimeUnit::Minute), // Abbreviations can be case-insensitive
            )),
            alt((
                "Stunden".value(TimeUnit::Hour),
                "Stunde".value(TimeUnit::Hour),
                Caseless("std").value(TimeUnit::Hour), // Abbreviations can be case-insensitive
            )),
            alt((
                "Tagen".value(TimeUnit::Day),
                "Tage".value(TimeUnit::Day),
                "Tag".value(TimeUnit::Day),
            )),
            alt((
                "Wochen".value(TimeUnit::Week),
                "Woche".value(TimeUnit::Week),
            )),
            alt((
                "Monaten".value(TimeUnit::Month),
                "Monate".value(TimeUnit::Month),
                "Monat".value(TimeUnit::Month),
            )),
            alt((
                "Jahren".value(TimeUnit::Year),
                "Jahre".value(TimeUnit::Year),
                "Jahr".value(TimeUnit::Year),
            )),
        ))
        .parse_next(input)
    }

    fn parse_relative_past(input: &mut &str) -> winnow::Result<TimeExpression> {
        preceded(
            Caseless("vor"),
            preceded(
                multispace1,
                (Self::parse_number, multispace1, Self::parse_time_unit),
            ),
        )
        .map(|(amount, _, unit)| {
            TimeExpression::Relative(RelativeTime {
                amount,
                unit,
                direction: Direction::Past,
            })
        })
        .parse_next(input)
    }

    fn parse_relative_future(input: &mut &str) -> winnow::Result<TimeExpression> {
        preceded(
            Caseless("in"),
            preceded(
                multispace1,
                (Self::parse_number, multispace1, Self::parse_time_unit),
            ),
        )
        .map(|(amount, _, unit)| {
            TimeExpression::Relative(RelativeTime {
                amount,
                unit,
                direction: Direction::Future,
            })
        })
        .parse_next(input)
    }

    fn parse_now(input: &mut &str) -> winnow::Result<TimeExpression> {
        Caseless("jetzt")
            .value(TimeExpression::Now)
            .parse_next(input)
    }

    fn parse_iso_datetime(input: &mut &str) -> winnow::Result<TimeExpression> {
        common::parse_iso_datetime(input)
    }

    fn parse_weekday(input: &mut &str) -> winnow::Result<Weekday> {
        alt((
            alt((
                "Montag".value(Weekday::Monday),
                Caseless("mo").value(Weekday::Monday), // Abbreviations can be case-insensitive
            )),
            alt((
                "Dienstag".value(Weekday::Tuesday),
                Caseless("di").value(Weekday::Tuesday), // Abbreviations can be case-insensitive
            )),
            alt((
                "Mittwoch".value(Weekday::Wednesday),
                Caseless("mi").value(Weekday::Wednesday), // Abbreviations can be case-insensitive
            )),
            alt((
                "Donnerstag".value(Weekday::Thursday),
                Caseless("do").value(Weekday::Thursday), // Abbreviations can be case-insensitive
            )),
            alt((
                "Freitag".value(Weekday::Friday),
                Caseless("fr").value(Weekday::Friday), // Abbreviations can be case-insensitive
            )),
            alt((
                "Samstag".value(Weekday::Saturday),
                Caseless("sa").value(Weekday::Saturday), // Abbreviations can be case-insensitive
            )),
            alt((
                "Sonntag".value(Weekday::Sunday),
                Caseless("so").value(Weekday::Sunday), // Abbreviations can be case-insensitive
            )),
        ))
        .parse_next(input)
    }

    fn parse_day_shortcuts(input: &mut &str) -> winnow::Result<DayReference> {
        alt((
            Caseless("heute").value(DayReference::Today),
            Caseless("gestern").value(DayReference::Yesterday),
            Caseless("morgen").value(DayReference::Tomorrow),
        ))
        .parse_next(input)
    }

    fn parse_weekday_modifier(input: &mut &str) -> winnow::Result<WeekdayModifier> {
        alt((
            alt((
                "letzten".value(WeekdayModifier::Last),
                "letzte".value(WeekdayModifier::Last),
            )),
            alt((
                "nächsten".value(WeekdayModifier::Next),
                "nächste".value(WeekdayModifier::Next),
            )),
        ))
        .parse_next(input)
    }

    fn parse_modified_weekday(input: &mut &str) -> winnow::Result<DayReference> {
        (
            Self::parse_weekday_modifier,
            multispace1,
            Self::parse_weekday,
        )
            .map(|(modifier, _, day)| DayReference::Weekday {
                day,
                modifier: Some(modifier),
            })
            .parse_next(input)
    }

    fn parse_simple_weekday(input: &mut &str) -> winnow::Result<DayReference> {
        Self::parse_weekday
            .map(|day| DayReference::Weekday {
                day,
                modifier: None,
            })
            .parse_next(input)
    }

    fn parse_day_reference(input: &mut &str) -> winnow::Result<TimeExpression> {
        alt((
            Self::parse_day_shortcuts,
            Self::parse_modified_weekday,
            Self::parse_simple_weekday,
        ))
        .map(TimeExpression::Day)
        .parse_next(input)
    }

    fn parse_time_digits(input: &mut &str) -> winnow::Result<(u8, u8, u8)> {
        (
            common::parse_two_digit_number,
            ':',
            common::parse_two_digit_number,
            opt(preceded(':', common::parse_two_digit_number)).map(|second| second.unwrap_or(0)),
        )
            .verify_map(|(hour, _, minute, second)| {
                time_utils::is_valid_24_hour_time(hour, minute, second)
                    .then_some((hour, minute, second))
            })
            .parse_next(input)
    }

    fn parse_time(input: &mut &str) -> winnow::Result<TimeExpression> {
        (
            Self::parse_time_digits,
            opt(preceded(multispace1, Caseless("uhr"))),
        )
            .map(|((hour, minute, second), _)| {
                TimeExpression::Time(Time {
                    hour,
                    minute,
                    second,
                    meridiem: None, // German typically uses 24-hour format
                })
            })
            .parse_next(input)
    }

    fn parse_day_at_time(input: &mut &str) -> winnow::Result<TimeExpression> {
        (
            alt((
                Self::parse_day_shortcuts,
                Self::parse_modified_weekday,
                Self::parse_simple_weekday,
            )),
            multispace1,
            Caseless("um"),
            multispace1,
            Self::parse_time_digits,
            opt(preceded(multispace1, Caseless("uhr"))),
        )
            .map(|(day, _, _, _, time_digits, _)| {
                TimeExpression::DayTime(DayTime {
                    day,
                    time: Time {
                        hour: time_digits.0,
                        minute: time_digits.1,
                        second: time_digits.2,
                        meridiem: None,
                    },
                })
            })
            .parse_next(input)
    }

    fn parse_date_format(input: &mut &str) -> winnow::Result<TimeExpression> {
        // DD.MM.YYYY (German format)
        (
            common::parse_two_digit_number,
            '.',
            common::parse_two_digit_number,
            '.',
            common::parse_four_digit_number,
        )
            .verify_map(|(day, _, month, _, year)| {
                time_utils::is_valid_calendar_date(year, month, day)
                    .then_some(TimeExpression::Date(StandardDate { day, month, year }))
            })
            .parse_next(input)
    }
}

impl LanguageParser for GermanParser {
    fn parse(&self, input: &str) -> Result<TimeExpression> {
        delimited(
            multispace0,
            alt((
                Self::parse_iso_datetime,
                Self::parse_date_format,
                Self::parse_day_at_time,
                Self::parse_now,
                Self::parse_day_reference,
                Self::parse_time,
                Self::parse_relative_past,
                Self::parse_relative_future,
            )),
            multispace0,
        )
        .parse(input)
        .map_err(|e| e.to_temps_error(input))
    }
}
