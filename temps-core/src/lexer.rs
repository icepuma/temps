//! Tokenizer for natural-language time expressions.
//!
//! The parsers used to run directly over `&str`, fusing lexing and parsing into
//! a single character-level pass — a "larser". That design makes keyword
//! matching prefix-based: `"day"` matches inside `"days"`, `"m"` inside
//! `"min"`, so every alternation has to be hand-ordered longest-first, a
//! convention that fails silently when broken.
//!
//! Splitting the lexer out removes that class of bug structurally. A word is
//! consumed maximally and then compared as a whole, so a keyword can never
//! match part of a longer word regardless of the order alternatives appear in.
//!
//! Tokenising does not by itself fix the *phrase*-level version of the same
//! hazard — `choice` still commits to the first alternative that succeeds, so a
//! bare `tomorrow` could shadow `tomorrow morning`. That one is handled in the
//! grammar rather than here, by left-factoring the shared prefix and making the
//! remainder optional (`day_reference().then(part_of_day().or_not())`), and
//! inside keyword tables by [`phrases_ci`](crate::common::phrases_ci), which
//! sorts its entries so the table's source order stays irrelevant.

use chumsky::span::SimpleSpan;

/// A single lexical unit of a time expression.
///
/// [`Token::Word`] and [`Token::Number`] carry their source slice rather than a
/// parsed value: callers need the original text to compare keywords
/// case-insensitively and to tell `01` from `1` when validating field widths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Token<'a> {
    /// A maximal run of alphabetic characters, e.g. `tomorrow`, `übermorgen`.
    ///
    /// A combining diacritical mark (U+0300..=U+036F and the other Combining
    /// Diacritical Marks blocks) continues the run it follows, so a decomposed
    /// (NFD) `u\u{308}bermorgen` — `u` then U+0308 COMBINING DIAERESIS — is one
    /// word, just like its precomposed (NFC) spelling. A mark never starts a
    /// word, not even one that Unicode calls alphabetic, such as U+036F
    /// COMBINING LATIN SMALL LETTER X: a word starts on a letter. The slice is
    /// the input exactly as written; keyword matching composes the German
    /// umlauts itself (see [`word_ci`](crate::common::word_ci)).
    Word(&'a str),
    /// A maximal run of ASCII digits, kept as text to preserve width.
    Number(&'a str),
    /// A single character that is not a letter, an ASCII digit or whitespace,
    /// and does not continue a [`Token::Word`]. A combining mark is no letter,
    /// even where Unicode calls it alphabetic, so it is a `Punct` unless it
    /// follows a word.
    Punct(char),
    /// A run of whitespace.
    ///
    /// Whitespace is significant here: `5 minutes` is a time expression while
    /// `5minutes` is not, so the tokens have to record where the gaps were.
    Space,
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Word(w) => write!(f, "{w}"),
            Token::Number(n) => write!(f, "{n}"),
            Token::Punct(c) => write!(f, "{c}"),
            Token::Space => write!(f, "whitespace"),
        }
    }
}

/// Split `input` into tokens, each paired with its byte span in the source.
///
/// Spans are byte offsets into `input` so that diagnostics can point back at
/// the original text.
#[must_use]
pub fn lex(input: &str) -> Vec<(Token<'_>, SimpleSpan)> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some(&(start, c)) = chars.peek() {
        let kind = char_kind(c);

        if kind == CharKind::Punct {
            chars.next();
            let end = start + c.len_utf8();
            tokens.push((Token::Punct(c), SimpleSpan::from(start..end)));
            continue;
        }

        // Consume the whole run so a word is never matched piecemeal.
        let mut end = start;
        while let Some(&(offset, next)) = chars.peek() {
            let continues =
                char_kind(next) == kind || (kind == CharKind::Word && is_combining_mark(next));
            if !continues {
                break;
            }
            end = offset + next.len_utf8();
            chars.next();
        }

        let slice = &input[start..end];
        let token = match kind {
            CharKind::Word => Token::Word(slice),
            CharKind::Number => Token::Number(slice),
            CharKind::Space => Token::Space,
            CharKind::Punct => unreachable!("punctuation is handled above"),
        };
        tokens.push((token, SimpleSpan::from(start..end)));
    }

    tokens
}

/// Whether `input` holds nothing but whitespace, i.e. lexes to no tokens at
/// all or to a single [`Token::Space`].
///
/// That is empty input as far as any grammar is concerned, since the top-level
/// parsers pad their expression with optional whitespace.
pub(crate) fn is_blank(input: &str) -> bool {
    input.chars().all(|c| char_kind(c) == CharKind::Space)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CharKind {
    Word,
    Number,
    Punct,
    Space,
}

/// The kind of token a run starting with `c` becomes.
///
/// A combining mark is [`CharKind::Punct`] here even where it is alphabetic:
/// it can only continue a word, which [`lex`] sees to, never start one.
fn char_kind(c: char) -> CharKind {
    if c.is_whitespace() {
        CharKind::Space
    } else if is_combining_mark(c) {
        CharKind::Punct
    } else if c.is_alphabetic() {
        CharKind::Word
    } else if c.is_ascii_digit() {
        CharKind::Number
    } else {
        CharKind::Punct
    }
}

/// Whether `c` is a combining diacritical mark, which belongs to the letter
/// before it: it continues a word, but never starts one.
///
/// `char::is_alphabetic` is false for most such marks — U+0308 COMBINING
/// DIAERESIS, the one NFD German needs, is `Mn` but not `Alphabetic` — so
/// without this a decomposed umlaut would split its word in two. A few are
/// `Alphabetic`, U+0345 and U+0363..=U+036F among them, and without this
/// would start a word with no letter in it. std has no general-category query
/// and this crate carries no Unicode tables, so these are the five Combining
/// Diacritical Marks blocks, spelled out.
pub(crate) fn is_combining_mark(c: char) -> bool {
    matches!(
        c,
        '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE20}'..='\u{FE2F}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(input: &str) -> Vec<Token<'_>> {
        lex(input).into_iter().map(|(t, _)| t).collect()
    }

    #[test]
    fn words_are_consumed_maximally() {
        // The whole point: "day" cannot be seen inside "days".
        assert_eq!(kinds("days"), vec![Token::Word("days")]);
        assert_eq!(kinds("min"), vec![Token::Word("min")]);
    }

    #[test]
    fn numbers_keep_their_width() {
        assert_eq!(kinds("01"), vec![Token::Number("01")]);
        assert_eq!(kinds("2024"), vec![Token::Number("2024")]);
    }

    #[test]
    fn whitespace_is_preserved_as_a_token() {
        assert_eq!(
            kinds("5 min"),
            vec![Token::Number("5"), Token::Space, Token::Word("min")]
        );
        assert_eq!(kinds("5min"), vec![Token::Number("5"), Token::Word("min")]);
    }

    #[test]
    fn non_ascii_words_stay_whole() {
        assert_eq!(kinds("übermorgen"), vec![Token::Word("übermorgen")]);
        assert_eq!(kinds("nächsten"), vec![Token::Word("nächsten")]);
    }

    #[test]
    fn combining_marks_continue_a_word() {
        // NFD `übermorgen`: `u` + U+0308 COMBINING DIAERESIS + `bermorgen`.
        // U+0308 is not alphabetic, which used to end the word at the mark.
        let input = "u\u{308}bermorgen";
        assert_eq!(
            lex(input),
            vec![(Token::Word(input), SimpleSpan::from(0..input.len()))]
        );
        assert_eq!(
            kinds("na\u{308}chsten Mo"),
            vec![
                Token::Word("na\u{308}chsten"),
                Token::Space,
                Token::Word("Mo")
            ]
        );
        // With no letter to attach to, a mark is still punctuation.
        assert_eq!(kinds("\u{308}"), vec![Token::Punct('\u{308}')]);
        assert_eq!(
            kinds("5\u{308}"),
            vec![Token::Number("5"), Token::Punct('\u{308}')]
        );
    }

    /// Some combining marks are `Alphabetic`, such as U+036F COMBINING LATIN
    /// SMALL LETTER X, and used to start a word of their own. A mark belongs to
    /// the letter before it, so without one it is punctuation, whatever its
    /// Unicode properties.
    #[test]
    fn a_combining_mark_never_starts_a_word() {
        let marks: Vec<char> = ('\0'..=char::MAX)
            .filter(|&c| is_combining_mark(c))
            .collect();
        assert!(
            marks.iter().any(|c| c.is_alphabetic()),
            "the blocks hold Alphabetic marks, which is what this test is about"
        );

        for mark in marks {
            let alone = mark.to_string();
            assert_eq!(kinds(&alone), vec![Token::Punct(mark)], "{mark:?}");
            let after_digit = format!("5{mark}");
            assert_eq!(
                kinds(&after_digit),
                vec![Token::Number("5"), Token::Punct(mark)],
                "{mark:?}"
            );
            let before_word = format!(" {mark}xy");
            assert_eq!(
                kinds(&before_word),
                vec![Token::Space, Token::Punct(mark), Token::Word("xy")],
                "{mark:?}"
            );

            // After a letter, it continues that letter's word.
            let inside = format!("x{mark}y{mark}");
            assert_eq!(kinds(&inside), vec![Token::Word(&inside)], "{mark:?}");
        }
    }

    #[test]
    fn spans_are_byte_offsets_into_the_source() {
        let input = "in fünf Tagen";
        let tokens = lex(input);
        let (last, span) = tokens.last().copied().expect("non-empty");
        assert_eq!(last, Token::Word("Tagen"));
        assert_eq!(&input[span.start..span.end], "Tagen");
    }

    #[test]
    fn iso_datetimes_split_into_fields() {
        assert_eq!(
            kinds("2024-01-15T14:30"),
            vec![
                Token::Number("2024"),
                Token::Punct('-'),
                Token::Number("01"),
                Token::Punct('-'),
                Token::Number("15"),
                Token::Word("T"),
                Token::Number("14"),
                Token::Punct(':'),
                Token::Number("30"),
            ]
        );
    }
}
