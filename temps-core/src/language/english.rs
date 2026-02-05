use winnow::{
    Parser,
    ascii::{Caseless, multispace0, multispace1},
    combinator::{alt, delimited, opt, preceded},
};

use crate::{
    DayReference, DayTime, Direction, LanguageParser, Meridiem, RelativeTime, Result, StandardDate,
    Time, TimeExpression, TimeUnit, Weekday, WeekdayModifier, common, error::ParseErrorExt,
};

/// Parser for English natural language time expressions.
pub struct EnglishParser;

impl EnglishParser {
    fn parse_number(input: &mut &str) -> winnow::Result<i64> {
        alt((
            common::parse_digit_number,
            alt((
                Caseless("an").value(1),
                Caseless("a").value(1),
                Caseless("one").value(1),
            )),
            Caseless("two").value(2),
            Caseless("three").value(3),
            Caseless("four").value(4),
            Caseless("five").value(5),
            Caseless("six").value(6),
            Caseless("seven").value(7),
            Caseless("eight").value(8),
            Caseless("nine").value(9),
            Caseless("ten").value(10),
        ))
        .parse_next(input)
    }

    fn parse_time_unit(input: &mut &str) -> winnow::Result<TimeUnit> {
        alt((
            alt((
                Caseless("seconds").value(TimeUnit::Second),
                Caseless("second").value(TimeUnit::Second),
                Caseless("secs").value(TimeUnit::Second),
                Caseless("sec").value(TimeUnit::Second),
                Caseless("s").value(TimeUnit::Second),
            )),
            alt((
                Caseless("minutes").value(TimeUnit::Minute),
                Caseless("minute").value(TimeUnit::Minute),
                Caseless("mins").value(TimeUnit::Minute),
                Caseless("min").value(TimeUnit::Minute),
            )),
            alt((
                Caseless("hours").value(TimeUnit::Hour),
                Caseless("hour").value(TimeUnit::Hour),
                Caseless("hrs").value(TimeUnit::Hour),
                Caseless("hr").value(TimeUnit::Hour),
                Caseless("h").value(TimeUnit::Hour),
            )),
            alt((
                Caseless("days").value(TimeUnit::Day),
                Caseless("day").value(TimeUnit::Day),
                Caseless("d").value(TimeUnit::Day),
            )),
            alt((
                Caseless("weeks").value(TimeUnit::Week),
                Caseless("week").value(TimeUnit::Week),
                Caseless("wks").value(TimeUnit::Week),
                Caseless("wk").value(TimeUnit::Week),
                Caseless("w").value(TimeUnit::Week),
            )),
            alt((
                Caseless("months").value(TimeUnit::Month),
                Caseless("month").value(TimeUnit::Month),
                Caseless("mos").value(TimeUnit::Month),
                Caseless("mo").value(TimeUnit::Month),
            )),
            alt((
                Caseless("years").value(TimeUnit::Year),
                Caseless("year").value(TimeUnit::Year),
                Caseless("yrs").value(TimeUnit::Year),
                Caseless("yr").value(TimeUnit::Year),
                Caseless("y").value(TimeUnit::Year),
            )),
            // Single-letter abbreviations last to avoid ambiguity
            Caseless("m").value(TimeUnit::Minute),
        ))
        .parse_next(input)
    }

    fn parse_relative_past(input: &mut &str) -> winnow::Result<TimeExpression> {
        (
            Self::parse_number,
            multispace1,
            Self::parse_time_unit,
            multispace1,
            Caseless("ago"),
        )
            .map(|(amount, _, unit, _, _)| {
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
            (Caseless("in"), multispace1),
            (Self::parse_number, multispace1, Self::parse_time_unit),
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
        Caseless("now").value(TimeExpression::Now).parse_next(input)
    }

    fn parse_iso_datetime(input: &mut &str) -> winnow::Result<TimeExpression> {
        common::parse_iso_datetime(input)
    }

    fn parse_weekday(input: &mut &str) -> winnow::Result<Weekday> {
        alt((
            alt((
                Caseless("monday").value(Weekday::Monday),
                Caseless("mon").value(Weekday::Monday),
            )),
            alt((
                Caseless("tuesday").value(Weekday::Tuesday),
                Caseless("tue").value(Weekday::Tuesday),
            )),
            alt((
                Caseless("wednesday").value(Weekday::Wednesday),
                Caseless("wed").value(Weekday::Wednesday),
            )),
            alt((
                Caseless("thursday").value(Weekday::Thursday),
                Caseless("thu").value(Weekday::Thursday),
            )),
            alt((
                Caseless("friday").value(Weekday::Friday),
                Caseless("fri").value(Weekday::Friday),
            )),
            alt((
                Caseless("saturday").value(Weekday::Saturday),
                Caseless("sat").value(Weekday::Saturday),
            )),
            alt((
                Caseless("sunday").value(Weekday::Sunday),
                Caseless("sun").value(Weekday::Sunday),
            )),
        ))
        .parse_next(input)
    }

    fn parse_day_shortcuts(input: &mut &str) -> winnow::Result<DayReference> {
        alt((
            Caseless("today").value(DayReference::Today),
            Caseless("yesterday").value(DayReference::Yesterday),
            Caseless("tomorrow").value(DayReference::Tomorrow),
        ))
        .parse_next(input)
    }

    fn parse_weekday_modifier(input: &mut &str) -> winnow::Result<WeekdayModifier> {
        alt((
            Caseless("last").value(WeekdayModifier::Last),
            Caseless("next").value(WeekdayModifier::Next),
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

    fn parse_meridiem(input: &mut &str) -> winnow::Result<Meridiem> {
        alt((
            alt((
                Caseless("am").value(Meridiem::AM),
                Caseless("a.m.").value(Meridiem::AM),
            )),
            alt((
                Caseless("pm").value(Meridiem::PM),
                Caseless("p.m.").value(Meridiem::PM),
            )),
        ))
        .parse_next(input)
    }

    fn parse_time_digits(input: &mut &str) -> winnow::Result<(u8, u8, u8, Option<Meridiem>)> {
        let hour = common::parse_two_digit_number(input)?;
        ':'.parse_next(input)?;
        let minute = common::parse_two_digit_number(input)?;
        let second = opt(preceded(':', common::parse_two_digit_number))
            .parse_next(input)?
            .unwrap_or(0);
        let meridiem = opt(preceded(multispace1, Self::parse_meridiem)).parse_next(input)?;

        Ok((hour, minute, second, meridiem))
    }

    fn parse_time(input: &mut &str) -> winnow::Result<TimeExpression> {
        Self::parse_time_digits
            .map(|(hour, minute, second, meridiem)| {
                TimeExpression::Time(Time {
                    hour,
                    minute,
                    second,
                    meridiem,
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
            preceded(
                multispace1,
                preceded(
                    Caseless("at"),
                    preceded(multispace1, Self::parse_time_digits),
                ),
            ),
        )
            .map(|(day, (hour, minute, second, meridiem))| {
                TimeExpression::DayTime(DayTime {
                    day,
                    time: Time {
                        hour,
                        minute,
                        second,
                        meridiem,
                    },
                })
            })
            .parse_next(input)
    }

    fn parse_date_format(input: &mut &str) -> winnow::Result<TimeExpression> {
        alt((
            // YYYY-MM-DD
            (
                common::parse_four_digit_number,
                '-',
                common::parse_two_digit_number,
                '-',
                common::parse_two_digit_number,
            )
                .map(|(year, _, month, _, day)| {
                    TimeExpression::Date(StandardDate { day, month, year })
                }),
            // DD/MM/YYYY or DD-MM-YYYY (International format)
            (
                common::parse_two_digit_number,
                alt(('/', '-')),
                common::parse_two_digit_number,
                alt(('/', '-')),
                common::parse_four_digit_number,
            )
                .map(|(day, _, month, _, year)| {
                    TimeExpression::Date(StandardDate { day, month, year })
                }),
        ))
        .parse_next(input)
    }
}

impl LanguageParser for EnglishParser {
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
