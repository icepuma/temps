use chumsky::{error::Rich, prelude::*};

use crate::{
    DayReference, DayTime, Direction, LanguageParser, RelativeTime, Result, StandardDate, Time,
    TimeExpression, TimeUnit, Weekday, WeekdayModifier,
    common::{
        ParserError, TokenInput, digit_number, exactly_two_digit_number, four_digit_number,
        iso_datetime, opt_space, phrases_ci, phrases_cs, punct, space, token_stream,
        two_digit_number, word_ci,
    },
    error::rich_errors_to_temps_error_with_empty_hint,
    lexer::lex,
    time_utils,
};

/// Parser for German natural language time expressions.
///
/// German nouns (e.g., "Sekunden", "Minuten") are matched case-sensitively
/// to follow German orthographic rules, while abbreviations (e.g., "sek", "min")
/// are matched case-insensitively for convenience. A noun in the wrong case
/// (`montag`, `MINUTEN`) is still rejected, but with a hint naming the
/// correctly capitalised spelling instead of the error an unknown word gets.
pub struct GermanParser;

fn number<'t, 's: 't, I>() -> impl Parser<'t, I, i64, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    choice((
        digit_number(),
        phrases_cs([
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

/// The unit nouns, capitalised as German writes them.
const UNIT_NOUNS: [(&str, TimeUnit); 17] = [
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
];

/// The weekday nouns, capitalised as German writes them.
const WEEKDAY_NOUNS: [(&str, Weekday); 7] = [
    ("Montag", Weekday::Monday),
    ("Dienstag", Weekday::Tuesday),
    ("Mittwoch", Weekday::Wednesday),
    ("Donnerstag", Weekday::Thursday),
    ("Freitag", Weekday::Friday),
    ("Samstag", Weekday::Saturday),
    ("Sonntag", Weekday::Sunday),
];

/// Recognise one of `nouns` written in the wrong case — `montag`, `MINUTEN` —
/// only to reject it with a hint naming the correct spelling.
///
/// The noun tables match case-sensitively, so without this a miscased noun
/// got the same "expected Wochentag, found `montag`" a nonsense word gets.
/// It must come after the case-sensitive table in a `choice`, which then
/// claims every correctly cased noun first.
///
/// The hint is emitted through `validate`, which records a secondary error
/// but lets the parse carry on: any emitted error still fails the parse as a
/// whole, so this accepts nothing new, but the message survives. A `try_map`
/// failure would be a primary error instead, and the enclosing category
/// `.labelled(...)` would replace it with just the category name.
fn miscased_noun<'t, 's: 't, I, T, const N: usize>(
    nouns: [(&'static str, T); N],
) -> impl Parser<'t, I, T, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
    T: Clone + 't,
{
    phrases_ci(nouns.map(|(noun, value)| (noun, (value, noun)))).validate(
        |(value, noun), extra, emitter| {
            emitter.emit(Rich::custom(
                extra.span(),
                format!("German nouns are capitalised; write `{noun}`"),
            ));
            value
        },
    )
}

fn time_unit<'t, 's: 't, I>() -> impl Parser<'t, I, TimeUnit, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    choice((
        // Nouns keep their capitalisation; abbreviations stay case-insensitive.
        phrases_cs(UNIT_NOUNS),
        phrases_ci([
            ("sek", TimeUnit::Second),
            ("min", TimeUnit::Minute),
            ("std", TimeUnit::Hour),
        ]),
        miscased_noun(UNIT_NOUNS),
    ))
    .labelled("Zeiteinheit")
}

fn weekday<'t, 's: 't, I>() -> impl Parser<'t, I, Weekday, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    choice((
        phrases_cs(WEEKDAY_NOUNS),
        phrases_ci([
            ("mo", Weekday::Monday),
            ("di", Weekday::Tuesday),
            ("mi", Weekday::Wednesday),
            ("do", Weekday::Thursday),
            ("fr", Weekday::Friday),
            ("sa", Weekday::Saturday),
            ("so", Weekday::Sunday),
        ]),
        miscased_noun(WEEKDAY_NOUNS),
    ))
    .labelled("Wochentag")
}

fn day_shortcuts<'t, 's: 't, I>() -> impl Parser<'t, I, DayReference, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    // All five are adverbs, not nouns, so they stay case-insensitive: a
    // sentence-initial `Übermorgen` reads the same as `übermorgen`.
    //
    // `übermorgen` and `vorgestern` are single words rather than the phrases
    // English needs ("the day after tomorrow"), and matching is per whole
    // token, so neither can shadow `morgen`/`gestern` nor be shadowed by them,
    // and `vorgestern` cannot be mistaken for the `vor <n> <Einheit>`
    // preposition. `german_lexicon.rs` pins that.
    phrases_ci([
        ("heute", DayReference::Today),
        ("gestern", DayReference::Yesterday),
        ("morgen", DayReference::Tomorrow),
        ("übermorgen", DayReference::DayAfterTomorrow),
        ("vorgestern", DayReference::DayBeforeYesterday),
    ])
}

fn weekday_modifier<'t, 's: 't, I>()
-> impl Parser<'t, I, WeekdayModifier, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    choice((
        word_ci("letzte").to(WeekdayModifier::Last),
        word_ci("letzten").to(WeekdayModifier::Last),
        word_ci("nächste").to(WeekdayModifier::Next),
        word_ci("nächsten").to(WeekdayModifier::Next),
    ))
}

fn modified_weekday<'t, 's: 't, I>() -> impl Parser<'t, I, DayReference, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    weekday_modifier()
        .then_ignore(space())
        .then(weekday())
        .map(|(modifier, day)| DayReference::Weekday {
            day,
            modifier: Some(modifier),
        })
}

fn simple_weekday<'t, 's: 't, I>() -> impl Parser<'t, I, DayReference, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    weekday().map(|day| DayReference::Weekday {
        day,
        modifier: None,
    })
}

fn day_reference<'t, 's: 't, I>() -> impl Parser<'t, I, DayReference, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    // A plain `choice`: the three alternatives start on disjoint words
    // (`heute`/`gestern`/`morgen`, a modifier, a weekday), so none can succeed
    // on a proper prefix of another's match.
    choice((day_shortcuts(), modified_weekday(), simple_weekday()))
}

fn time_digits<'t, 's: 't, I>() -> impl Parser<'t, I, (u8, u8, u8), ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    // The hour may drop its leading zero (`9:45 Uhr`); the minute and second
    // are fixed-width, so `14:5` is rejected rather than guessed at.
    two_digit_number()
        .then_ignore(punct(':'))
        .then(exactly_two_digit_number())
        .then(punct(':').ignore_then(exactly_two_digit_number()).or_not())
        .try_map(|((hour, minute), second), span| {
            let second = second.unwrap_or(0);
            if time_utils::is_valid_24_hour_time(hour, minute, second) {
                Ok((hour, minute, second))
            } else {
                Err(Rich::custom(span, "invalid time"))
            }
        })
}

/// The optional `Uhr` that may trail a clock time, as in `14:30 Uhr`.
fn uhr_suffix<'t, 's: 't, I>() -> impl Parser<'t, I, (), ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    space().ignore_then(word_ci("uhr")).or_not().ignored()
}

fn time_expr<'t, 's: 't, I>() -> impl Parser<'t, I, TimeExpression, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    time_digits()
        .then_ignore(uhr_suffix())
        .map(|(hour, minute, second)| {
            TimeExpression::Time(Time {
                hour,
                minute,
                second,
                meridiem: None,
            })
        })
}

/// A day reference, optionally qualified by `um <Uhrzeit>`.
///
/// The left-factored form of `morgen` and `morgen um 15:30`, which used to be
/// two top-level alternatives sharing the same [`day_reference`] prefix. Under
/// an ordered `choice` the bare form would commit on `morgen` and leave
/// `um 15:30` for `end()` to reject.
fn day_expr<'t, 's: 't, I>() -> impl Parser<'t, I, TimeExpression, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    let um_time = word_ci("um")
        .ignore_then(space())
        .ignore_then(time_digits())
        .then_ignore(uhr_suffix());

    day_reference()
        .then(space().ignore_then(um_time).or_not())
        .map(|(day, time)| match time {
            Some((hour, minute, second)) => TimeExpression::DayTime(DayTime {
                day,
                time: Time {
                    hour,
                    minute,
                    second,
                    meridiem: None,
                },
            }),
            None => TimeExpression::Day(day),
        })
}

fn relative_past<'t, 's: 't, I>() -> impl Parser<'t, I, TimeExpression, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    word_ci("vor")
        .ignore_then(space())
        .ignore_then(number())
        .then_ignore(space())
        .then(time_unit())
        .map(|(amount, unit)| {
            TimeExpression::Relative(RelativeTime {
                amount,
                unit,
                direction: Direction::Past,
            })
        })
}

fn relative_future<'t, 's: 't, I>()
-> impl Parser<'t, I, TimeExpression, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    word_ci("in")
        .ignore_then(space())
        .ignore_then(number())
        .then_ignore(space())
        .then(time_unit())
        .map(|(amount, unit)| {
            TimeExpression::Relative(RelativeTime {
                amount,
                unit,
                direction: Direction::Future,
            })
        })
}

fn now_expr<'t, 's: 't, I>() -> impl Parser<'t, I, TimeExpression, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    word_ci("jetzt").to(TimeExpression::Now)
}

fn date_format<'t, 's: 't, I>() -> impl Parser<'t, I, TimeExpression, ParserError<'t, 's>> + Clone
where
    I: TokenInput<'t, 's>,
{
    two_digit_number()
        .then_ignore(punct('.'))
        .then(two_digit_number())
        .then_ignore(punct('.'))
        .then(four_digit_number())
        .try_map(|((day, month), year), span| {
            if time_utils::is_valid_calendar_date(year, month, day) {
                Ok(TimeExpression::Date(StandardDate { day, month, year }))
            } else {
                Err(Rich::custom(span, "invalid calendar date"))
            }
        })
}

fn parser<'t, 's: 't, I>() -> impl Parser<'t, I, TimeExpression, ParserError<'t, 's>>
where
    I: TokenInput<'t, 's>,
{
    // An ordered `choice`, safe for the same reason as in the English parser:
    // the one family that shared a leading token — a day with and without a
    // time — is left-factored into [`day_expr`], and what is left starts on
    // disjoint tokens or fails without committing, so the order below is
    // documentation rather than semantics.
    choice((
        iso_datetime().labelled("ISO 8601 datetime"),
        date_format().labelled("Datum (TT.MM.JJJJ)"),
        day_expr().labelled("Tagesangabe, optional mit Uhrzeit"),
        now_expr().labelled("`jetzt`"),
        time_expr().labelled("Uhrzeit"),
        relative_past().labelled("`vor <n> <Einheit>`"),
        relative_future().labelled("`in <n> <Einheit>`"),
    ))
    .padded_by(opt_space())
    .then_ignore(end())
}

/// What empty (or whitespace-only) input is told to try instead; every
/// backticked example must parse in German, which a regression test checks.
const EMPTY_INPUT_HINT: &str = "Eingabe ist leer; erwartet wird ein Zeitausdruck wie `jetzt`, \
                                `in 5 Minuten` oder ein ISO-Datum";

impl LanguageParser for GermanParser {
    fn parse(&self, input: &str) -> Result<TimeExpression> {
        let tokens = lex(input);
        parser()
            .parse(token_stream(input, &tokens))
            .into_result()
            .map_err(|errs| {
                rich_errors_to_temps_error_with_empty_hint(input, errs, EMPTY_INPUT_HINT)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hour: u8, minute: u8) -> Time {
        Time {
            hour,
            minute,
            second: 0,
            meridiem: None,
        }
    }

    /// Each left-factored rule, run on its own up to `end()`, must parse the
    /// bare form *and* every extension of it to the right value. Un-factoring
    /// a rule into sibling alternatives lets the bare form commit first and
    /// strand the tail, which turns the extended rows here into `None`.
    #[test]
    fn left_factored_rules_parse_the_bare_and_the_extended_form() {
        use DayReference::{DayAfterTomorrow, Tomorrow};
        use TimeExpression::{Day, DayTime as At};
        let with = |day, time| At(DayTime { day, time });
        let next_monday = DayReference::Weekday {
            day: Weekday::Monday,
            modifier: Some(WeekdayModifier::Next),
        };

        for (input, expected) in [
            ("morgen", Day(Tomorrow)),
            ("morgen um 15:30", with(Tomorrow, at(15, 30))),
            ("morgen um 15:30 Uhr", with(Tomorrow, at(15, 30))),
            ("nächsten Montag", Day(next_monday)),
            ("nächsten Montag um 9:45 Uhr", with(next_monday, at(9, 45))),
            ("übermorgen", Day(DayAfterTomorrow)),
            ("übermorgen um 08:00", with(DayAfterTomorrow, at(8, 0))),
        ] {
            assert_eq!(
                run!(input, day_expr()),
                Some(expected),
                "day_expr on {input:?}"
            );
        }

        // The optional `Uhr` is the same pattern one level down.
        for (input, expected) in [
            ("14:30", TimeExpression::Time(at(14, 30))),
            ("14:30 Uhr", TimeExpression::Time(at(14, 30))),
        ] {
            assert_eq!(
                run!(input, time_expr()),
                Some(expected),
                "time_expr on {input:?}"
            );
        }
    }
}
