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
        // Longer patterns must come before shorter ones to prevent partial matches
        keyword_ci("a couple").to(2),
        keyword_ci("a few").to(3),
        keyword_ci("couple of").to(2),
        keyword_ci("a dozen").to(12),
        keyword_ci("an").to(1),
        keyword_ci("a").to(1),
        keyword_ci("one").to(1),
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
        keyword_ci("day after tomorrow").to(DayReference::DayAfterTomorrow),
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
    choice((
        the_day_after_tomorrow(),
        day_shortcuts(),
        modified_weekday(),
        this_weekday(),
        weekend_ref(),
        simple_weekday(),
    ))
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

/// Parse a raw hour (number or named time like "noon") for use in fractional expressions.
fn raw_hour<'a>() -> impl Parser<'a, &'a str, u8, ParserError<'a>> + Clone {
    choice((
        two_digit_number().try_map(|h, span| {
            if h <= 23 {
                Ok(h)
            } else {
                Err(Rich::custom(span, "hour must be 0-23"))
            }
        }),
        keyword_ci("noon").to(12u8),
        keyword_ci("midnight").to(0u8),
    ))
}

/// Parse fractional time: "half past X", "quarter past X", "quarter to X".
fn fractional_time<'a>()
-> impl Parser<'a, &'a str, (u8, u8, u8, Option<Meridiem>), ParserError<'a>> + Clone {
    let half_past = keyword_ci("half past")
        .ignore_then(whitespace_required())
        .ignore_then(raw_hour())
        .map(|h| (h, 30u8, 0u8, None::<Meridiem>));

    let quarter_past = keyword_ci("quarter past")
        .ignore_then(whitespace_required())
        .ignore_then(raw_hour())
        .map(|h| (h, 15u8, 0u8, None::<Meridiem>));

    let quarter_to = keyword_ci("quarter to")
        .ignore_then(whitespace_required())
        .ignore_then(raw_hour())
        .map(|h| {
            if h == 0 {
                (23u8, 45u8, 0u8, None::<Meridiem>)
            } else {
                (h - 1, 45u8, 0u8, None::<Meridiem>)
            }
        });

    choice((half_past, quarter_past, quarter_to)).try_map(|(hour, minute, second, mer), span| {
        if time_utils::is_valid_time(hour, minute, second, mer) {
            Ok((hour, minute, second, mer))
        } else {
            Err(Rich::custom(span, "invalid time"))
        }
    })
}

fn time_expr<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    choice((
        fractional_time().map(|(hour, minute, second, meridiem)| {
            TimeExpression::Time(Time {
                hour,
                minute,
                second,
                meridiem,
            })
        }),
        time_digits().map(|(hour, minute, second, meridiem)| {
            TimeExpression::Time(Time {
                hour,
                minute,
                second,
                meridiem,
            })
        }),
    ))
}

fn named_time<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    choice((
        keyword_ci("noon").to(TimeExpression::Time(Time {
            hour: 12,
            minute: 0,
            second: 0,
            meridiem: None,
        })),
        keyword_ci("midnight").to(TimeExpression::Time(Time {
            hour: 0,
            minute: 0,
            second: 0,
            meridiem: None,
        })),
        keyword_ci("teatime").to(TimeExpression::Time(Time {
            hour: 16,
            minute: 0,
            second: 0,
            meridiem: None,
        })),
    ))
}

/// Parse part-of-day: "morning", "afternoon", "evening", "night".
/// Returns a Time with a default hour.
fn part_of_day<'a>() -> impl Parser<'a, &'a str, Time, ParserError<'a>> + Clone {
    choice((
        keyword_ci("morning").to(Time {
            hour: 8,
            minute: 0,
            second: 0,
            meridiem: None,
        }),
        keyword_ci("afternoon").to(Time {
            hour: 13,
            minute: 0,
            second: 0,
            meridiem: None,
        }),
        keyword_ci("evening").to(Time {
            hour: 18,
            minute: 0,
            second: 0,
            meridiem: None,
        }),
        keyword_ci("night").to(Time {
            hour: 20,
            minute: 0,
            second: 0,
            meridiem: None,
        }),
    ))
}

/// Day reference followed by a part of day: "tomorrow morning", "today afternoon", etc.
fn day_with_part_of_day<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    day_reference()
        .then_ignore(whitespace_required())
        .then(part_of_day())
        .map(|(day, time)| TimeExpression::DayTime(DayTime { day, time }))
}

/// "this" + day-like expression: "this morning", "this afternoon", "this evening".
fn this_part_of_day<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    keyword_ci("this")
        .ignore_then(whitespace_required())
        .ignore_then(part_of_day())
        .map(|time| {
            TimeExpression::DayTime(DayTime {
                day: DayReference::Today,
                time,
            })
        })
}

/// "this" + weekday: "this Monday", "this Friday".
fn this_weekday<'a>() -> impl Parser<'a, &'a str, DayReference, ParserError<'a>> + Clone {
    keyword_ci("this")
        .ignore_then(whitespace_required())
        .ignore_then(weekday())
        .map(|day| DayReference::Weekday {
            day,
            modifier: None,
        })
}

/// "this weekend" / "next weekend".
fn weekend_ref<'a>() -> impl Parser<'a, &'a str, DayReference, ParserError<'a>> + Clone {
    choice((
        keyword_ci("this weekend").to(DayReference::Weekday {
            day: Weekday::Saturday,
            modifier: None,
        }),
        keyword_ci("next weekend").to(DayReference::Weekday {
            day: Weekday::Saturday,
            modifier: Some(WeekdayModifier::Next),
        }),
    ))
}

/// Standalone expressions that map to DayTime.
fn standalone_daytime<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    choice((
        keyword_ci("tonight").to(TimeExpression::DayTime(DayTime {
            day: DayReference::Today,
            time: Time {
                hour: 20,
                minute: 0,
                second: 0,
                meridiem: None,
            },
        })),
        choice((
            keyword_ci("eod"),
            keyword_ci("end of day"),
            keyword_ci("end of the day"),
        ))
        .to(TimeExpression::DayTime(DayTime {
            day: DayReference::Today,
            time: Time {
                hour: 17,
                minute: 0,
                second: 0,
                meridiem: None,
            },
        })),
    ))
}

/// "the day after tomorrow" — synonym with "the" prefix.
fn the_day_after_tomorrow<'a>() -> impl Parser<'a, &'a str, DayReference, ParserError<'a>> + Clone {
    keyword_ci("the day after tomorrow").to(DayReference::DayAfterTomorrow)
}

/// "the day before yesterday" — two days ago.
fn the_day_before_yesterday<'a>()
-> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    keyword_ci("the day before yesterday").to(TimeExpression::Relative(RelativeTime {
        amount: 2,
        unit: TimeUnit::Day,
        direction: Direction::Past,
    }))
}

/// "fortnight" = 2 weeks (future direction assumed for scheduling).
fn fortnight<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    keyword_ci("fortnight").to(TimeExpression::Relative(RelativeTime {
        amount: 2,
        unit: TimeUnit::Week,
        direction: Direction::Future,
    }))
}

/// "later" / "later today" — vague future (~2 hours).
fn later_expr<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    choice((keyword_ci("later today"), keyword_ci("later"))).to(TimeExpression::Relative(
        RelativeTime {
            amount: 2,
            unit: TimeUnit::Hour,
            direction: Direction::Future,
        },
    ))
}

/// "a week from now" / "a week from today".
fn week_from_now<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> + Clone {
    choice((
        keyword_ci("a week from today"),
        keyword_ci("a week from now"),
    ))
    .to(TimeExpression::Relative(RelativeTime {
        amount: 1,
        unit: TimeUnit::Week,
        direction: Direction::Future,
    }))
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
        day_with_part_of_day().labelled("day with part of day"),
        now_expr().labelled("`now`"),
        standalone_daytime().labelled("standalone (tonight, EOD)"),
        this_part_of_day().labelled("this morning/afternoon/evening"),
        day_reference()
            .map(TimeExpression::Day)
            .labelled("day reference"),
        named_time().labelled("named time (noon, midnight, teatime)"),
        time_expr().labelled("time of day"),
        relative_past().labelled("`<n> <unit> ago`"),
        relative_future().labelled("`in <n> <unit>`"),
        week_from_now().labelled("a week from now/today"),
        fortnight().labelled("fortnight"),
        the_day_before_yesterday().labelled("the day before yesterday"),
        later_expr().labelled("later/later today"),
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
