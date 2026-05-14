use chumsky::{error::Rich, prelude::*, text};

use crate::{
    DayReference, DayTime, Direction, LanguageParser, Meridiem, RelativeTime, Result, StandardDate,
    Time, TimeExpression, TimeUnit, Weekday, WeekdayModifier,
    common::{
        ParserError, digit_number, four_digit_number, iso_datetime, keyword_ci, two_digit_number,
    },
    error::rich_errors_to_temps_error,
    time_utils,
};

/// Parser for English natural language time expressions.
pub struct EnglishParser;

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
        keyword_ci("an").to(1),
        keyword_ci("one").to(1),
        keyword_ci("a").to(1),
        keyword_ci("two").to(2),
        keyword_ci("three").to(3),
        keyword_ci("four").to(4),
        keyword_ci("five").to(5),
        keyword_ci("six").to(6),
        keyword_ci("seven").to(7),
        keyword_ci("eight").to(8),
        keyword_ci("nine").to(9),
        keyword_ci("ten").to(10),
    ))
    .labelled("number")
}

fn time_unit<'a>() -> impl Parser<'a, &'a str, TimeUnit, ParserError<'a>> + Clone {
    choice((
        choice((
            keyword_ci("seconds").to(TimeUnit::Second),
            keyword_ci("second").to(TimeUnit::Second),
            keyword_ci("secs").to(TimeUnit::Second),
            keyword_ci("sec").to(TimeUnit::Second),
        )),
        choice((
            keyword_ci("minutes").to(TimeUnit::Minute),
            keyword_ci("minute").to(TimeUnit::Minute),
            keyword_ci("mins").to(TimeUnit::Minute),
            keyword_ci("min").to(TimeUnit::Minute),
        )),
        choice((
            keyword_ci("hours").to(TimeUnit::Hour),
            keyword_ci("hour").to(TimeUnit::Hour),
            keyword_ci("hrs").to(TimeUnit::Hour),
            keyword_ci("hr").to(TimeUnit::Hour),
        )),
        choice((
            keyword_ci("days").to(TimeUnit::Day),
            keyword_ci("day").to(TimeUnit::Day),
        )),
        choice((
            keyword_ci("weeks").to(TimeUnit::Week),
            keyword_ci("week").to(TimeUnit::Week),
            keyword_ci("wks").to(TimeUnit::Week),
            keyword_ci("wk").to(TimeUnit::Week),
        )),
        choice((
            keyword_ci("months").to(TimeUnit::Month),
            keyword_ci("month").to(TimeUnit::Month),
            keyword_ci("mos").to(TimeUnit::Month),
            keyword_ci("mo").to(TimeUnit::Month),
        )),
        choice((
            keyword_ci("years").to(TimeUnit::Year),
            keyword_ci("year").to(TimeUnit::Year),
            keyword_ci("yrs").to(TimeUnit::Year),
            keyword_ci("yr").to(TimeUnit::Year),
        )),
        choice((
            keyword_ci("s").to(TimeUnit::Second),
            keyword_ci("h").to(TimeUnit::Hour),
            keyword_ci("d").to(TimeUnit::Day),
            keyword_ci("w").to(TimeUnit::Week),
            keyword_ci("y").to(TimeUnit::Year),
            keyword_ci("m").to(TimeUnit::Minute),
        )),
    ))
    .labelled("time unit")
}

fn weekday<'a>() -> impl Parser<'a, &'a str, Weekday, ParserError<'a>> + Clone {
    choice((
        choice((
            keyword_ci("monday").to(Weekday::Monday),
            keyword_ci("mon").to(Weekday::Monday),
        )),
        choice((
            keyword_ci("tuesday").to(Weekday::Tuesday),
            keyword_ci("tue").to(Weekday::Tuesday),
        )),
        choice((
            keyword_ci("wednesday").to(Weekday::Wednesday),
            keyword_ci("wed").to(Weekday::Wednesday),
        )),
        choice((
            keyword_ci("thursday").to(Weekday::Thursday),
            keyword_ci("thu").to(Weekday::Thursday),
        )),
        choice((
            keyword_ci("friday").to(Weekday::Friday),
            keyword_ci("fri").to(Weekday::Friday),
        )),
        choice((
            keyword_ci("saturday").to(Weekday::Saturday),
            keyword_ci("sat").to(Weekday::Saturday),
        )),
        choice((
            keyword_ci("sunday").to(Weekday::Sunday),
            keyword_ci("sun").to(Weekday::Sunday),
        )),
    ))
    .labelled("weekday")
}

fn day_shortcuts<'a>() -> impl Parser<'a, &'a str, DayReference, ParserError<'a>> + Clone {
    choice((
        keyword_ci("today").to(DayReference::Today),
        keyword_ci("yesterday").to(DayReference::Yesterday),
        keyword_ci("tomorrow").to(DayReference::Tomorrow),
    ))
}

fn weekday_modifier<'a>() -> impl Parser<'a, &'a str, WeekdayModifier, ParserError<'a>> + Clone {
    choice((
        keyword_ci("last").to(WeekdayModifier::Last),
        keyword_ci("next").to(WeekdayModifier::Next),
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

fn meridiem<'a>() -> impl Parser<'a, &'a str, Meridiem, ParserError<'a>> + Clone {
    choice((
        keyword_ci("a.m.").to(Meridiem::AM),
        keyword_ci("p.m.").to(Meridiem::PM),
        keyword_ci("am").to(Meridiem::AM),
        keyword_ci("pm").to(Meridiem::PM),
    ))
    .labelled("am/pm")
}

fn time_with_minutes<'a>()
-> impl Parser<'a, &'a str, (u8, u8, u8, Option<Meridiem>), ParserError<'a>> + Clone {
    two_digit_number()
        .then_ignore(just(':'))
        .then(two_digit_number())
        .then(just(':').ignore_then(two_digit_number()).or_not())
        .then(text::whitespace().ignore_then(meridiem()).or_not())
        .try_map(|(((hour, minute), second), mer), span| {
            let second = second.unwrap_or(0);
            if time_utils::is_valid_time(hour, minute, second, mer) {
                Ok((hour, minute, second, mer))
            } else {
                Err(Rich::custom(span, "invalid time"))
            }
        })
}

fn hour_meridiem<'a>()
-> impl Parser<'a, &'a str, (u8, u8, u8, Option<Meridiem>), ParserError<'a>> + Clone {
    two_digit_number()
        .then(text::whitespace().ignore_then(meridiem()))
        .try_map(|(hour, mer), span| {
            if time_utils::is_valid_time(hour, 0, 0, Some(mer)) {
                Ok((hour, 0, 0, Some(mer)))
            } else {
                Err(Rich::custom(span, "invalid time"))
            }
        })
}

fn time_digits<'a>()
-> impl Parser<'a, &'a str, (u8, u8, u8, Option<Meridiem>), ParserError<'a>> + Clone {
    choice((time_with_minutes(), hour_meridiem()))
}

fn time_expr<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    time_digits().map(|(hour, minute, second, meridiem)| {
        TimeExpression::Time(Time {
            hour,
            minute,
            second,
            meridiem,
        })
    })
}

fn day_at_time<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    day_reference()
        .then_ignore(whitespace_required())
        .then_ignore(keyword_ci("at"))
        .then_ignore(whitespace_required())
        .then(time_digits())
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
}

fn relative_past<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    number()
        .then_ignore(whitespace_required())
        .then(time_unit())
        .then_ignore(whitespace_required())
        .then_ignore(keyword_ci("ago"))
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
    keyword_ci("now").to(TimeExpression::Now)
}

fn date_format<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    let iso_like = four_digit_number()
        .then_ignore(just('-'))
        .then(two_digit_number())
        .then_ignore(just('-'))
        .then(two_digit_number())
        .try_map(|((year, month), day), span| {
            if time_utils::is_valid_calendar_date(year, month, day) {
                Ok(TimeExpression::Date(StandardDate { day, month, year }))
            } else {
                Err(Rich::custom(span, "invalid calendar date"))
            }
        });

    let international = two_digit_number()
        .then(one_of(['/', '-']))
        .then(two_digit_number())
        .then(one_of(['/', '-']))
        .then(four_digit_number())
        .try_map(|((((day, first), month), second), year), span| {
            if first == second && time_utils::is_valid_calendar_date(year, month, day) {
                Ok(TimeExpression::Date(StandardDate { day, month, year }))
            } else {
                Err(Rich::custom(span, "invalid date"))
            }
        });

    choice((iso_like, international))
}

fn parser<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> {
    choice((
        iso_datetime().labelled("ISO 8601 datetime"),
        date_format().labelled("calendar date"),
        day_at_time().labelled("day with time"),
        now_expr().labelled("`now`"),
        day_reference()
            .map(TimeExpression::Day)
            .labelled("day reference"),
        time_expr().labelled("time of day"),
        relative_past().labelled("`<n> <unit> ago`"),
        relative_future().labelled("`in <n> <unit>`"),
    ))
    .padded()
    .then_ignore(end())
}

impl LanguageParser for EnglishParser {
    fn parse(&self, input: &str) -> Result<TimeExpression> {
        parser()
            .parse(input)
            .into_result()
            .map_err(|errs| rich_errors_to_temps_error(input, errs))
    }
}
