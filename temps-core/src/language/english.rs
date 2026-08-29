use chumsky::{error::Rich, prelude::*, text};

use crate::{
    DayReference, DayTime, Direction, LanguageParser, Meridiem, RelativeTime, Result, StandardDate,
    Time, TimeExpression, TimeUnit, Weekday, WeekdayModifier,
    common::{
        ParserError, digit_number, four_digit_number, iso_datetime, keyword_ci, keywords_ci,
        longest, two_digit_number,
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
    // Source order is irrelevant: `keywords_ci` tries the longest keyword first.
    choice((
        digit_number(),
        keywords_ci([
            ("a", 1i64),
            ("an", 1),
            ("one", 1),
            ("two", 2),
            ("three", 3),
            ("four", 4),
            ("five", 5),
            ("six", 6),
            ("seven", 7),
            ("eight", 8),
            ("nine", 9),
            ("ten", 10),
            ("a couple", 2),
            ("a couple of", 2),
            ("couple of", 2),
            ("a few", 3),
            ("a dozen", 12),
        ]),
    ))
    .labelled("number")
}

fn time_unit<'a>() -> impl Parser<'a, &'a str, TimeUnit, ParserError<'a>> + Clone {
    keywords_ci([
        ("second", TimeUnit::Second),
        ("seconds", TimeUnit::Second),
        ("sec", TimeUnit::Second),
        ("secs", TimeUnit::Second),
        ("s", TimeUnit::Second),
        ("minute", TimeUnit::Minute),
        ("minutes", TimeUnit::Minute),
        ("min", TimeUnit::Minute),
        ("mins", TimeUnit::Minute),
        ("m", TimeUnit::Minute),
        ("hour", TimeUnit::Hour),
        ("hours", TimeUnit::Hour),
        ("hr", TimeUnit::Hour),
        ("hrs", TimeUnit::Hour),
        ("h", TimeUnit::Hour),
        ("day", TimeUnit::Day),
        ("days", TimeUnit::Day),
        ("d", TimeUnit::Day),
        ("week", TimeUnit::Week),
        ("weeks", TimeUnit::Week),
        ("wk", TimeUnit::Week),
        ("wks", TimeUnit::Week),
        ("w", TimeUnit::Week),
        ("month", TimeUnit::Month),
        ("months", TimeUnit::Month),
        ("mo", TimeUnit::Month),
        ("mos", TimeUnit::Month),
        ("year", TimeUnit::Year),
        ("years", TimeUnit::Year),
        ("yr", TimeUnit::Year),
        ("yrs", TimeUnit::Year),
        ("y", TimeUnit::Year),
    ])
    .labelled("time unit")
}

fn weekday<'a>() -> impl Parser<'a, &'a str, Weekday, ParserError<'a>> + Clone {
    keywords_ci([
        ("monday", Weekday::Monday),
        ("mon", Weekday::Monday),
        ("tuesday", Weekday::Tuesday),
        ("tue", Weekday::Tuesday),
        ("wednesday", Weekday::Wednesday),
        ("wed", Weekday::Wednesday),
        ("thursday", Weekday::Thursday),
        ("thu", Weekday::Thursday),
        ("friday", Weekday::Friday),
        ("fri", Weekday::Friday),
        ("saturday", Weekday::Saturday),
        ("sat", Weekday::Saturday),
        ("sunday", Weekday::Sunday),
        ("sun", Weekday::Sunday),
    ])
    .labelled("weekday")
}

fn day_shortcuts<'a>() -> impl Parser<'a, &'a str, DayReference, ParserError<'a>> + Clone {
    keywords_ci([
        ("today", DayReference::Today),
        ("yesterday", DayReference::Yesterday),
        ("tomorrow", DayReference::Tomorrow),
        ("day after tomorrow", DayReference::DayAfterTomorrow),
        ("day before yesterday", DayReference::DayBeforeYesterday),
    ])
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
    longest(vec![
        the_day_after_tomorrow().boxed(),
        the_day_before_yesterday().boxed(),
        day_shortcuts().boxed(),
        modified_weekday().boxed(),
        this_weekday().boxed(),
        weekend_ref().boxed(),
        simple_weekday().boxed(),
    ])
}

fn meridiem<'a>() -> impl Parser<'a, &'a str, Meridiem, ParserError<'a>> + Clone {
    keywords_ci([
        ("am", Meridiem::AM),
        ("pm", Meridiem::PM),
        ("a.m.", Meridiem::AM),
        ("p.m.", Meridiem::PM),
    ])
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
            modifier: Some(WeekdayModifier::This),
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

/// "the day before yesterday" — synonym with "the" prefix.
fn the_day_before_yesterday<'a>() -> impl Parser<'a, &'a str, DayReference, ParserError<'a>> + Clone
{
    keyword_ci("the day before yesterday").to(DayReference::DayBeforeYesterday)
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
    choice((
        keyword_ci("later today").to(TimeExpression::LaterToday),
        keyword_ci("later").to(TimeExpression::Relative(RelativeTime {
            amount: 2,
            unit: TimeUnit::Hour,
            direction: Direction::Future,
        })),
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
    // Note: a `YYYY-MM-DD` alternative would be dead code here — `iso_datetime()`
    // is tried first at the top level and accepts exactly that shape.
    two_digit_number()
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
        })
}

fn parser<'a>() -> impl Parser<'a, &'a str, TimeExpression, ParserError<'a>> {
    // Longest-match: whichever alternative consumes the most wins, so no
    // alternative can shadow a longer one by appearing earlier.
    longest(vec![
        iso_datetime().labelled("ISO 8601 datetime").boxed(),
        date_format().labelled("calendar date").boxed(),
        day_at_time().labelled("day with time").boxed(),
        day_with_part_of_day()
            .labelled("day with part of day")
            .boxed(),
        now_expr().labelled("`now`").boxed(),
        standalone_daytime()
            .labelled("standalone (tonight, EOD)")
            .boxed(),
        this_part_of_day()
            .labelled("this morning/afternoon/evening")
            .boxed(),
        day_reference()
            .map(TimeExpression::Day)
            .labelled("day reference")
            .boxed(),
        named_time()
            .labelled("named time (noon, midnight, teatime)")
            .boxed(),
        time_expr().labelled("time of day").boxed(),
        relative_past().labelled("`<n> <unit> ago`").boxed(),
        relative_future().labelled("`in <n> <unit>`").boxed(),
        week_from_now().labelled("a week from now/today").boxed(),
        fortnight().labelled("fortnight").boxed(),
        later_expr().labelled("later/later today").boxed(),
    ])
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
