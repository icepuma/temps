//! Whole-surface behaviour corpus.
//!
//! `all_tests.rs` and `regressions.rs` assert things somebody thought to assert.
//! This test asserts everything else: every phrase in
//! `tests/data/parser_corpus.tsv` is parsed in both languages and compared
//! against a recorded expectation, so any change in what a phrase *means* —
//! including phrases no hand-written test mentions — shows up as a diff instead
//! of as silence. It is the net that proved a full parser rewrite
//! behaviour-preserving.
//!
//! Expectations are the `Debug` rendering of the returned `TimeExpression`, or
//! the literal `ERR` for a parse failure. `Debug` is deliberate: it pins the
//! whole value, and a failing line reads as a diff.
//!
//! # Regenerating the expectations
//!
//! Only when a behaviour change is *intended*, and never to make a red test
//! green:
//!
//! ```text
//! TEMPS_BLESS_CORPUS=1 just test
//! ```
//!
//! That rewrites the third column of the data file from the current parser,
//! leaving the phrase list and the header comment alone, and fails the test so
//! the run cannot be mistaken for a pass. Review the resulting diff line by
//! line — each changed line is a phrase whose meaning changed.

use std::fmt::Write as _;
use temps_core::{Language, parse};

const CORPUS: &str = include_str!("data/parser_corpus.tsv");
const CORPUS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/parser_corpus.tsv");
const BLESS_VAR: &str = "TEMPS_BLESS_CORPUS";

/// One recorded expectation: which language, which phrase, what it meant.
struct Case<'a> {
    /// 1-based line number in the data file, for pointing a human at the row.
    line: usize,
    lang: Language,
    lang_tag: &'a str,
    input: &'a str,
    expected: &'a str,
}

fn cases() -> Vec<Case<'static>> {
    CORPUS
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with('#') && !line.trim().is_empty())
        .map(|(index, line)| {
            let number = index + 1;
            let mut fields = line.split('\t');
            let (Some(tag), Some(input), Some(expected), None) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                panic!(
                    "{CORPUS_PATH}:{number}: expected exactly three tab-separated fields \
                     `<lang>\\t<phrase>\\t<expectation>`, got: {line:?}"
                );
            };
            let lang = match tag {
                "en" => Language::English,
                "de" => Language::German,
                other => panic!("{CORPUS_PATH}:{number}: unknown language tag {other:?}"),
            };
            Case {
                line: number,
                lang,
                lang_tag: tag,
                input,
                expected,
            }
        })
        .collect()
}

/// What the parser says today, in the data file's own notation.
fn actual(case: &Case<'_>) -> String {
    match parse(case.input, case.lang) {
        Ok(expression) => format!("{expression:?}"),
        Err(_) => "ERR".to_string(),
    }
}

#[test]
fn corpus_of_phrases_parses_to_its_recorded_meaning() {
    let cases = cases();
    assert!(
        cases.len() > 2000,
        "the corpus lost most of its rows ({} left) — did the data file get truncated?",
        cases.len()
    );

    if std::env::var_os(BLESS_VAR).is_some() {
        bless(&cases);
        return;
    }

    let mut failures = Vec::new();
    for case in &cases {
        let actual = actual(case);
        if actual != case.expected {
            failures.push(format!(
                "{}:{}: [{}] {:?}\n     expected: {}\n       actual: {}",
                CORPUS_PATH, case.line, case.lang_tag, case.input, case.expected, actual
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} corpus phrases changed meaning:\n\n{}\n\n\
         If these changes are intended, regenerate with `{BLESS_VAR}=1 just test` \
         and review the diff line by line.",
        failures.len(),
        cases.len(),
        failures.join("\n\n"),
    );
}

/// Rewrite the expectations in place, then fail so a blessing run is never
/// mistaken for a passing one.
fn bless(cases: &[Case<'_>]) {
    let mut fresh: Vec<Option<String>> = vec![None; CORPUS.lines().count()];
    let mut changed = 0usize;
    for case in cases {
        let actual = actual(case);
        if actual != case.expected {
            changed += 1;
        }
        fresh[case.line - 1] = Some(format!("{}\t{}\t{}", case.lang_tag, case.input, actual));
    }

    let mut out = String::with_capacity(CORPUS.len());
    for (index, original) in CORPUS.lines().enumerate() {
        let line = fresh[index].as_deref().unwrap_or(original);
        let _ = writeln!(out, "{line}");
    }
    std::fs::write(CORPUS_PATH, out).expect("failed to rewrite the corpus data file");

    panic!(
        "blessed {changed} changed expectation(s) into {CORPUS_PATH}. \
         Review the diff, then re-run without {BLESS_VAR} to confirm it is green."
    );
}
