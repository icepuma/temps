//! German day adverbs beyond heute/gestern/morgen.
//!
//! `DayReference::DayAfterTomorrow` and `DayReference::DayBeforeYesterday` were
//! reachable from English ("the day after tomorrow", "day before yesterday")
//! long before German had words for them. German says each in a single word,
//! `übermorgen` and `vorgestern`, and those two words sit uncomfortably close
//! to three keywords the grammar already uses: `morgen`, `gestern`, and the
//! `vor <n> <Einheit>` preposition. Matching is per whole token, so no
//! collision is possible — these tests exist to keep that true.

use temps_core::{
    DayReference, DayTime, Direction, Language, RelativeTime, Time, TimeExpression, TimeUnit, parse,
};

fn parse_de(input: &str) -> TimeExpression {
    parse(input, Language::German).unwrap_or_else(|e| panic!("failed to parse {input:?}: {e}"))
}

fn day(input: &str) -> DayReference {
    match parse_de(input) {
        TimeExpression::Day(day) => day,
        other => panic!("expected a bare day for {input:?}, got {other:?}"),
    }
}

// ===== The two new words =====

#[test]
fn uebermorgen_is_the_day_after_tomorrow() {
    assert_eq!(day("übermorgen"), DayReference::DayAfterTomorrow);
}

#[test]
fn vorgestern_is_the_day_before_yesterday() {
    assert_eq!(day("vorgestern"), DayReference::DayBeforeYesterday);
}

/// Both are adverbs rather than nouns, so unlike `Montag` or `Tagen` they are
/// matched case-insensitively: a sentence-initial capital must still parse.
#[test]
fn the_new_adverbs_are_case_insensitive() {
    for (input, expected) in [
        ("Übermorgen", DayReference::DayAfterTomorrow),
        ("ÜBERMORGEN", DayReference::DayAfterTomorrow),
        ("Vorgestern", DayReference::DayBeforeYesterday),
        ("VORGESTERN", DayReference::DayBeforeYesterday),
    ] {
        assert_eq!(day(input), expected, "mismatch for {input:?}");
    }
}

// ===== No shadowing in either direction =====

/// `morgen` is a suffix of `übermorgen` and `gestern` a suffix of `vorgestern`.
/// Were keywords matched against character prefixes rather than whole tokens,
/// one of each pair would eat the other; whichever way the alternation happened
/// to be ordered, one of these four assertions would fail.
#[test]
fn the_new_words_neither_shadow_nor_are_shadowed_by_the_old_ones() {
    assert_eq!(day("morgen"), DayReference::Tomorrow);
    assert_eq!(day("übermorgen"), DayReference::DayAfterTomorrow);
    assert_eq!(day("gestern"), DayReference::Yesterday);
    assert_eq!(day("vorgestern"), DayReference::DayBeforeYesterday);
}

/// The nearest miss: `vorgestern` starts with the letters of `vor`, the
/// preposition that introduces a past relative time. The lexer hands the
/// grammar one word, not two, so `vor` cannot be peeled off the front.
#[test]
fn vor_the_preposition_still_introduces_a_past_relative_time() {
    assert_eq!(
        parse_de("vor zwei Tagen"),
        TimeExpression::Relative(RelativeTime {
            amount: 2,
            unit: TimeUnit::Day,
            direction: Direction::Past,
        })
    );
}

/// The converse of the above: `vorgestern` is not `vor` with a stray tail, so
/// it must not be readable as a relative time missing its unit.
#[test]
fn vorgestern_is_not_a_truncated_relative_time() {
    assert!(
        parse("vorgestern zwei Tagen", Language::German).is_err(),
        "`vorgestern` must not act as the `vor` preposition"
    );
    assert!(
        parse("vor gestern", Language::German).is_err(),
        "split across two tokens, neither reading applies"
    );
}

// ===== Composition with a time =====

/// `day_expr` left-factors the bare day and the `um <Uhrzeit>` form, so the new
/// words get the qualified form for free — provided they really are day
/// references and not a separate top-level alternative.
#[test]
fn the_new_words_take_a_time_like_any_other_day() {
    let at = |hour, minute| Time {
        hour,
        minute,
        second: 0,
        meridiem: None,
    };

    for (input, expected) in [
        (
            "übermorgen um 15:30",
            DayTime {
                day: DayReference::DayAfterTomorrow,
                time: at(15, 30),
            },
        ),
        (
            "Übermorgen um 08:00 Uhr",
            DayTime {
                day: DayReference::DayAfterTomorrow,
                time: at(8, 0),
            },
        ),
        (
            "vorgestern um 23:59",
            DayTime {
                day: DayReference::DayBeforeYesterday,
                time: at(23, 59),
            },
        ),
    ] {
        assert_eq!(
            parse_de(input),
            TimeExpression::DayTime(expected),
            "mismatch for {input:?}"
        );
    }
}

/// Time validation is shared, and stays shared: an out-of-range hour is
/// rejected after `übermorgen` exactly as it is after `morgen`.
#[test]
fn an_invalid_clock_time_is_still_rejected_after_the_new_words() {
    assert!(parse("übermorgen um 24:00", Language::German).is_err());
}

// ===== These words stay German =====

#[test]
fn the_english_parser_does_not_learn_them() {
    assert!(parse("übermorgen", Language::English).is_err());
    assert!(parse("vorgestern", Language::English).is_err());
}
