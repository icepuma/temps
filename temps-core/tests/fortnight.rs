//! "fortnight" as a unit of time.
//!
//! A fortnight is two weeks, and `TimeUnit` deliberately has no `Fortnight`
//! variant: every expectation below is expressed in [`TimeUnit::Week`] with the
//! amount already doubled. These tests pin both halves of that — the doubling
//! itself, and the fact that it is *checked*, so an amount that cannot be
//! doubled is a parse failure rather than a wrapped-around answer.

use temps_core::{Direction, Language, RelativeTime, TimeExpression, TimeUnit, parse};

fn parse_en(input: &str) -> TimeExpression {
    parse(input, Language::English).unwrap_or_else(|e| panic!("failed to parse {input:?}: {e}"))
}

fn weeks(amount: i64, direction: Direction) -> TimeExpression {
    TimeExpression::Relative(RelativeTime {
        amount,
        unit: TimeUnit::Week,
        direction,
    })
}

/// The standalone reading predates fortnight-as-a-unit and is unchanged by it.
#[test]
fn bare_fortnight_still_means_two_weeks_ahead() {
    assert_eq!(parse_en("fortnight"), weeks(2, Direction::Future));
}

#[test]
fn in_a_fortnight_is_two_weeks_ahead() {
    for input in ["in a fortnight", "in one fortnight", "in 1 fortnight"] {
        assert_eq!(
            parse_en(input),
            weeks(2, Direction::Future),
            "for {input:?}"
        );
    }
}

#[test]
fn a_fortnight_ago_is_two_weeks_back() {
    for input in ["a fortnight ago", "one fortnight ago", "1 fortnight ago"] {
        assert_eq!(parse_en(input), weeks(2, Direction::Past), "for {input:?}");
    }
}

#[test]
fn several_fortnights_are_counted_in_pairs_of_weeks() {
    assert_eq!(parse_en("in 2 fortnights"), weeks(4, Direction::Future));
    assert_eq!(parse_en("in 3 fortnights"), weeks(6, Direction::Future));
    assert_eq!(parse_en("3 fortnights ago"), weeks(6, Direction::Past));
    assert_eq!(parse_en("two fortnights ago"), weeks(4, Direction::Past));
}

/// The singular and the plural spelling are the same unit — a speaker writing
/// "in 2 fortnight" or "a fortnights ago" is understood, not corrected.
#[test]
fn singular_and_plural_spellings_agree() {
    assert_eq!(parse_en("in 2 fortnight"), parse_en("in 2 fortnights"));
    assert_eq!(parse_en("a fortnights ago"), parse_en("a fortnight ago"));
}

/// Doubling the amount is the one place this feature can overflow, and the
/// amount comes straight from user input. `i64::MAX` fortnights has no `i64`
/// answer in weeks, so the parse must fail — quietly wrapping to a negative
/// count would turn a future date into a past one.
#[test]
fn an_amount_that_cannot_be_doubled_is_rejected_not_wrapped() {
    let max = i64::MAX;
    let half_plus_one = max / 2 + 1;

    for input in [
        format!("in {max} fortnights"),
        format!("{max} fortnights ago"),
        format!("in {half_plus_one} fortnights"),
        format!("{half_plus_one} fortnights ago"),
    ] {
        assert!(
            parse(&input, Language::English).is_err(),
            "{input:?} should not parse: it has no representable answer"
        );
    }
}

/// The largest amount that *does* survive doubling still parses, so the
/// rejection above is a boundary and not a blanket refusal of large numbers.
#[test]
fn the_largest_representable_count_of_fortnights_still_parses() {
    let half = i64::MAX / 2;
    assert_eq!(
        parse_en(&format!("in {half} fortnights")),
        weeks(half * 2, Direction::Future)
    );
}

/// A number too large for an `i64` at all is rejected the same way, before any
/// doubling is attempted.
#[test]
fn an_amount_beyond_i64_is_rejected() {
    assert!(parse("in 99999999999999999999 fortnights", Language::English).is_err());
}

/// The new unit sits inside the same rule as every other unit, so the
/// neighbours it shares that rule with are pinned here too.
#[test]
fn ordinary_units_are_unaffected() {
    assert_eq!(parse_en("in 2 weeks"), weeks(2, Direction::Future));
    assert_eq!(parse_en("2 weeks ago"), weeks(2, Direction::Past));
    assert_eq!(
        parse_en("in 4 days"),
        TimeExpression::Relative(RelativeTime {
            amount: 4,
            unit: TimeUnit::Day,
            direction: Direction::Future,
        })
    );
    assert_eq!(
        parse_en("a week from now"),
        weeks(1, Direction::Future),
        "the `a week from ...` reading must still win over `a week` + leftovers"
    );
}

/// "fortnight" is an English colloquialism only; German is untouched.
#[test]
fn german_does_not_learn_fortnight() {
    for input in ["fortnight", "in einem fortnight", "in 2 fortnights"] {
        assert!(parse(input, Language::German).is_err(), "for {input:?}");
    }
}
