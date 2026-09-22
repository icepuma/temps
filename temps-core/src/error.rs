//! Error types for the temps library.
//!
//! This module defines the error types used throughout the temps ecosystem.
//! All parsing and date calculation operations return `Result<T, TempsError>`.
//!
//! # Error Categories
//!
//! - **Parse Errors**: When input cannot be parsed as a valid time expression
//! - **Date Calculation Errors**: When date arithmetic results in invalid dates
//! - **Invalid Component Errors**: When date/time components are out of range
//! - **Backend Errors**: When the underlying datetime library reports an error
//!
//! # Examples
//!
//! ```
//! use temps_core::{parse, Language, TempsError};
//!
//! // Parse error example
//! let result = parse("invalid input", Language::English);
//! match result {
//!     Err(TempsError::ParseError { message, input, position }) => {
//!         println!("Parse failed: {}", message);
//!     }
//!     _ => {}
//! }
//! ```

use thiserror::Error;

/// The main error type for the temps library.
///
/// This enum represents all possible errors that can occur during
/// parsing and time calculation operations.
#[derive(Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TempsError {
    /// Error that occurs during parsing of time expressions.
    ///
    /// This error is returned when the input string cannot be parsed
    /// as a valid time expression in the specified language.
    ///
    /// # Example
    ///
    /// ```
    /// use temps_core::TempsError;
    ///
    /// let err = TempsError::parse_error("Unrecognized time unit", "in 5 blargs");
    /// ```
    #[error("Failed to parse time expression: {message}")]
    ParseError {
        /// The specific parsing error message. For the built-in parsers this
        /// is a rendered report that stays small however long the input is:
        /// see [`rich_errors_to_temps_error`]
        message: String,
        /// The input that failed to parse
        input: String,
        /// Optional position in the input where parsing failed
        position: Option<usize>,
    },

    /// Error that occurs during date/time calculations.
    ///
    /// This error is returned when date arithmetic operations fail,
    /// such as when adding months to January 31st would result in
    /// February 31st (which doesn't exist).
    ///
    /// It displays as `Date calculation error: {message}`, followed by
    /// `: {context}` when a context is present.
    ///
    /// # Example
    ///
    /// ```
    /// use temps_core::TempsError;
    ///
    /// let err = TempsError::date_calculation("Month overflow");
    /// assert_eq!(err.to_string(), "Date calculation error: Month overflow");
    /// ```
    #[error("Date calculation error: {message}{}", format_context(context.as_deref()))]
    DateCalculationError {
        /// The specific calculation error message
        message: String,
        /// Optional context about what caused the error, such as the
        /// backend's own error message; rendered after `message`
        context: Option<String>,
    },

    /// Error for invalid date components
    #[error("{}", crate::errors::format_invalid_date(*year, *month, *day))]
    InvalidDate {
        /// The year component
        year: u16,
        /// The month component (1-12)
        month: u8,
        /// The day component (1-31)
        day: u8,
    },

    /// Error for invalid time components
    #[error("{}", crate::errors::format_invalid_time(*hour, *minute, *second))]
    InvalidTime {
        /// The hour component (0-23)
        hour: u8,
        /// The minute component (0-59)
        minute: u8,
        /// The second component (0-59)
        second: u8,
    },

    /// Error for invalid timezone offset
    #[error("{}", crate::errors::format_invalid_timezone_offset(*total_minutes))]
    InvalidTimezoneOffset {
        /// The offset from UTC in minutes (-720 to +840)
        total_minutes: i16,
    },

    /// Error for ambiguous local time (e.g., during DST transitions)
    #[error("Ambiguous local time: {message}")]
    AmbiguousTime {
        /// Description of the ambiguity
        message: String,
    },

    /// Error for arithmetic overflow in date calculations
    #[error("Arithmetic overflow: {operation}")]
    ArithmeticOverflow {
        /// The operation that caused the overflow
        operation: String,
    },

    /// Error for unsupported operations
    #[error("Unsupported operation: {operation}")]
    UnsupportedOperation {
        /// Description of the unsupported operation
        operation: String,
    },

    /// Error from the underlying datetime backend (chrono, jiff, etc.)
    #[error("Backend error: {message}")]
    BackendError {
        /// The error message from the backend
        message: String,
        /// The backend that produced the error
        backend: String,
    },
}

impl TempsError {
    /// Creates a new parse error without position information.
    ///
    /// Use this when you know parsing failed but don't have a specific
    /// position in the input where the error occurred.
    ///
    /// # Arguments
    ///
    /// * `message` - Description of what went wrong
    /// * `input` - The input string that failed to parse
    ///
    /// # Example
    ///
    /// ```
    /// use temps_core::TempsError;
    ///
    /// let err = TempsError::parse_error(
    ///     "Expected time unit",
    ///     "in 5"
    /// );
    /// ```
    #[must_use]
    pub fn parse_error(message: impl Into<String>, input: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
            input: input.into(),
            position: None,
        }
    }

    /// Creates a new parse error with position information.
    ///
    /// Use this when you know exactly where in the input the parse error occurred.
    ///
    /// # Arguments
    ///
    /// * `message` - Description of what went wrong
    /// * `input` - The input string that failed to parse
    /// * `position` - Character position where parsing failed
    ///
    /// # Example
    ///
    /// ```
    /// use temps_core::TempsError;
    ///
    /// let err = TempsError::parse_error_with_position(
    ///     "Unexpected character",
    ///     "in 5 minuts",
    ///     9  // Points to the 't' in "minuts"
    /// );
    /// ```
    #[must_use]
    pub fn parse_error_with_position(
        message: impl Into<String>,
        input: impl Into<String>,
        position: usize,
    ) -> Self {
        Self::ParseError {
            message: message.into(),
            input: input.into(),
            position: Some(position),
        }
    }

    /// Creates a new date calculation error.
    ///
    /// Use this for errors that occur during date arithmetic operations.
    ///
    /// # Example
    ///
    /// ```
    /// use temps_core::TempsError;
    ///
    /// let err = TempsError::date_calculation(
    ///     "Cannot subtract 13 months from January"
    /// );
    /// ```
    #[must_use]
    pub fn date_calculation(message: impl Into<String>) -> Self {
        Self::DateCalculationError {
            message: message.into(),
            context: None,
        }
    }

    /// Creates a new date calculation error with additional context.
    ///
    /// Use this when you want to include information about what caused
    /// the calculation to fail (e.g., an error from the backend library).
    /// The context is kept in the `context` field and is also part of the
    /// error's `Display`, after the message. It is plain text, not a chained
    /// [`std::error::Error::source`].
    ///
    /// # Example
    ///
    /// ```
    /// use temps_core::TempsError;
    ///
    /// let err = TempsError::date_calculation_with_source(
    ///     "Failed to add months",
    ///     "chrono error: date out of range"
    /// );
    /// assert_eq!(
    ///     err.to_string(),
    ///     "Date calculation error: Failed to add months: chrono error: date out of range"
    /// );
    /// ```
    #[must_use]
    pub fn date_calculation_with_source(
        message: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self::DateCalculationError {
            message: message.into(),
            context: Some(context.into()),
        }
    }

    /// Creates an invalid date error
    #[must_use]
    pub fn invalid_date(year: u16, month: u8, day: u8) -> Self {
        Self::InvalidDate { year, month, day }
    }

    /// Creates an invalid time error
    #[must_use]
    pub fn invalid_time(hour: u8, minute: u8, second: u8) -> Self {
        Self::InvalidTime {
            hour,
            minute,
            second,
        }
    }

    /// Creates an invalid timezone offset error
    #[must_use]
    pub fn invalid_timezone_offset(total_minutes: i16) -> Self {
        Self::InvalidTimezoneOffset { total_minutes }
    }

    /// Creates an ambiguous time error
    #[must_use]
    pub fn ambiguous_time(message: impl Into<String>) -> Self {
        Self::AmbiguousTime {
            message: message.into(),
        }
    }

    /// Creates an arithmetic overflow error
    #[must_use]
    pub fn arithmetic_overflow(operation: impl Into<String>) -> Self {
        Self::ArithmeticOverflow {
            operation: operation.into(),
        }
    }

    /// Creates an unsupported operation error
    #[must_use]
    pub fn unsupported_operation(operation: impl Into<String>) -> Self {
        Self::UnsupportedOperation {
            operation: operation.into(),
        }
    }

    /// Creates a backend error
    #[must_use]
    pub fn backend_error(message: impl Into<String>, backend: impl Into<String>) -> Self {
        Self::BackendError {
            message: message.into(),
            backend: backend.into(),
        }
    }
}

/// Result type alias for temps operations.
///
/// All parsing and time calculation operations in the temps library
/// return this result type.
///
/// # Example
///
/// ```
/// use temps_core::Result;
///
/// fn parse_time(input: &str) -> Result<String> {
///     // Implementation
///     Ok("parsed".to_string())
/// }
/// ```
pub type Result<T> = std::result::Result<T, TempsError>;

/// Convert a collection of chumsky parser errors into a [`TempsError`]
/// and an ariadne-rendered diagnostic string.
///
/// The first error's span is used for the position field. The full
/// rendered report (with source context) is folded into the error's
/// message so callers that simply display the error still get a useful,
/// human-readable diagnostic.
/// The parsers run over tokens, so their spans are the lexer's BYTE offsets
/// into the original source — see [`crate::lexer::lex`]. That is exactly what
/// the byte-to-character translation below needs, and it is why umlaut input
/// still gets a caret in the right place.
///
/// The message does not grow with the input. A token longer than 32
/// characters is quoted only in part, ending in `…`. A source line longer than
/// 100 characters (not counting its line terminator) is cut down to the stretch
/// around the error, with at most 32 characters underlined. A `…` marks each
/// end of the stretch, front or back, that leaves text out: blanks at the edge
/// of the line are left out unmarked, and a single character is kept rather
/// than replaced by a `…`, which would save nothing. The report's header still
/// names the error's line and column in the whole input. The whole input stays
/// in the error's `input` field.
///
/// Empty input, and input that is nothing but whitespace (which every grammar
/// pads away, so it is empty to them too), gets a short language-neutral
/// message at position 0 instead of a report with nothing to underline. The
/// built-in language parsers give their own, localized message that names
/// example inputs of their language.
#[must_use]
pub fn rich_errors_to_temps_error(
    input: &str,
    errors: Vec<chumsky::error::Rich<'_, crate::lexer::Token<'_>>>,
) -> TempsError {
    rich_errors_to_temps_error_with_empty_hint(
        input,
        errors,
        "input is empty; expected a time expression",
    )
}

/// [`rich_errors_to_temps_error`], with `empty_hint` as the message for empty
/// or whitespace-only input, so that a language parser can suggest inputs its
/// own grammar accepts.
pub(crate) fn rich_errors_to_temps_error_with_empty_hint(
    input: &str,
    errors: Vec<chumsky::error::Rich<'_, crate::lexer::Token<'_>>>,
    empty_hint: &str,
) -> TempsError {
    if crate::lexer::is_blank(input) {
        return TempsError::parse_error_with_position(empty_hint, input, 0);
    }

    // Token spans are BYTE offsets, but ariadne's `Source` indexes by
    // CHARACTER. Feeding one to the other mislocates the caret on any
    // non-ASCII input and silently drops the label once the byte offset runs
    // past the character count. Translate up front.
    let byte_to_char = |byte: usize| -> usize {
        input
            .char_indices()
            .position(|(b, _)| b >= byte)
            .unwrap_or_else(|| input.chars().count())
    };
    let char_len = input.chars().count();

    let position = errors
        .first()
        .map(|e| byte_to_char(e.span().start))
        .unwrap_or(0);

    let source = ariadne::Source::from(input);
    let mut rendered = String::new();
    for err in &errors {
        let span = err.span();
        let start = byte_to_char(span.start);
        let end = byte_to_char(span.end).max(start + 1).min(char_len.max(1));
        let range = start..end;
        let (headline, detail) = format_rich(err);
        let report = match Excerpt::cut(input, &source, range.clone()) {
            None => render_report(&source, range, &headline, &detail),
            Some(excerpt) => excerpt.render(&headline, &detail),
        };

        match report {
            Some(report) => rendered.push_str(&report),
            // The same parts as a report, so no longer than one.
            None => {
                rendered.push_str(&headline);
                rendered.push_str(": ");
                rendered.push_str(&detail);
                rendered.push('\n');
            }
        }
    }

    let message = if rendered.is_empty() {
        "Failed to parse time expression".to_string()
    } else {
        rendered.trim_end().to_string()
    };

    TempsError::parse_error_with_position(message, input, position)
}

/// The name a rendered report gives its source, as in `input:1:6`.
const SOURCE_ID: &str = "input";

/// A source line of at most this many characters, not counting its line
/// terminator, is echoed whole in a diagnostic; a longer one is cut down to an
/// [`Excerpt`].
const MAX_ECHOED_LINE: usize = 100;

/// Characters kept on either side of the error when a long line is cut down.
const EXCERPT_CONTEXT: usize = 32;

/// Most characters of a single token that a diagnostic underlines or quotes.
const MAX_ECHOED_TOKEN: usize = 32;

/// The characters ariadne's `Source` ends a line on. It counts a line's
/// terminator, `\r\n` as a whole, as part of the line.
const LINE_TERMINATORS: [char; 7] = ['\r', '\n', '\x0B', '\x0C', '\u{85}', '\u{2028}', '\u{2029}'];

/// The characters of `line` (of `source`) without its terminator, which is
/// what [`MAX_ECHOED_LINE`] limits.
fn content_len<S: AsRef<str>>(source: &ariadne::Source<S>, line: ariadne::Line) -> usize {
    let text = source.get_line_text(line).unwrap_or_default();
    let terminator = if text.ends_with("\r\n") {
        2
    } else {
        usize::from(text.ends_with(LINE_TERMINATORS))
    };
    line.len() - terminator
}

/// Render one uncoloured report headed `headline` that underlines `range`
/// (characters of `source`) with `detail`, or `None` if ariadne cannot.
fn render_report<S: AsRef<str>>(
    source: &ariadne::Source<S>,
    range: std::ops::Range<usize>,
    headline: &str,
    detail: &str,
) -> Option<String> {
    use ariadne::{Color, Config, Label, Report, ReportKind};

    let label = drawn_label(source, &range);
    let config = Config::default()
        .with_color(false)
        .with_label_attach(label_attach(source.text(), &label));
    let mut buf = Vec::new();
    // The header names where `range` starts, whatever the label takes in.
    Report::build(ReportKind::Error, (SOURCE_ID, range))
        .with_config(config)
        .with_message(headline)
        .with_label(
            Label::new((SOURCE_ID, label))
                .with_message(detail)
                .with_color(Color::Red),
        )
        .finish()
        .write((SOURCE_ID, source), &mut buf)
        .ok()?;
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// The characters of `source` to label for an error at `range`: `range`
/// itself, unless every character in it is a combining mark, which is drawn
/// zero columns wide. Then the label takes in the nearest character before it
/// on its line, which a terminal draws the marks on, or, for marks that begin
/// their line, the nearest one after them, so that [`label_attach`] has a
/// character to attach the pointer to.
///
/// Such a range is a stray mark, one with no letter before it, which the
/// lexer makes a [`Token::Punct`](crate::lexer::Token::Punct) of its own. A
/// line of nothing but marks has nothing to take in, and keeps a label with no
/// pointer.
fn drawn_label<S: AsRef<str>>(
    source: &ariadne::Source<S>,
    range: &std::ops::Range<usize>,
) -> std::ops::Range<usize> {
    let drawn = |c: &char| !crate::lexer::is_combining_mark(*c);
    let Some((line, _, _)) = source.get_offset_line(range.start) else {
        return range.clone();
    };
    let offset = line.offset();
    let chars: Vec<char> = source
        .text()
        .chars()
        .skip(offset)
        .take(content_len(source, line))
        .collect();
    let start = range.start - offset;
    let end = (range.end - offset).min(chars.len());
    if start >= end || chars[start..end].iter().any(drawn) {
        return range.clone();
    }
    if let Some(before) = chars[..start].iter().rposition(drawn) {
        offset + before..range.end
    } else if let Some(after) = chars[end..].iter().position(drawn) {
        range.start..offset + end + after + 1
    } else {
        range.clone()
    }
}

/// Where the pointer that joins the underline of `range` (characters of
/// `text`) to its message attaches: the label's middle character, as ariadne
/// does by default, unless that one is drawn zero columns wide.
///
/// ariadne draws the pointer, `┬` and the `╰` below it, as wide as the
/// character it attaches to, so on a zero-width character it is not drawn at
/// all and the message points nowhere. That is the combining mark of a
/// decomposed letter: the lexer keeps a mark in the word it follows (see
/// [`Token::Word`](crate::lexer::Token::Word)), so the middle of a word such as
/// NFD `Fu\u{308}nf` can be its U+0308. A word never starts on a mark, and a
/// label that [`drawn_label`] widened starts or ends on the character it took
/// in, so the pointer moves to the label's start, or failing that to its end.
///
/// Only the combining marks the lexer knows (the Combining Diacritical Marks
/// blocks) are recognised: this crate carries no Unicode width tables, and
/// ariadne, which does, does not expose them. A label whose middle is some
/// other zero-width character, such as a Mn vowel sign of an Indic script or a
/// zero-width joiner, can still lose its pointer, and so does a label made of
/// nothing but zero-width characters, such as a line of nothing but marks.
fn label_attach(text: &str, range: &std::ops::Range<usize>) -> ariadne::LabelAttach {
    use ariadne::LabelAttach;

    let drawn = |offset: usize| {
        text.chars()
            .nth(offset)
            .is_some_and(|c| !crate::lexer::is_combining_mark(c))
    };
    // The middle character exactly as ariadne picks it.
    let middle = (range.start + range.end) / 2;
    [
        (LabelAttach::Middle, middle),
        (LabelAttach::Start, range.start),
        (LabelAttach::End, range.end.saturating_sub(1)),
    ]
    .into_iter()
    .find(|&(_, offset)| drawn(offset))
    .map_or(LabelAttach::Middle, |(attach, _)| attach)
}

/// The input with the line an error starts on cut down to the stretch around
/// the error, so that rendering it costs the same however long the line is.
///
/// ariadne echoes every line a label touches in full, draws one underline
/// character per labelled character, and has no option to limit either.
struct Excerpt {
    /// The input up to the error's line, verbatim so that ariadne numbers the
    /// line as it would in the whole input (it renders only labelled lines),
    /// then the kept stretch of the line with `…` at each cut. Nothing after
    /// the stretch is kept.
    text: String,
    /// The part of the error to underline, in characters of `text`: at most
    /// [`MAX_ECHOED_TOKEN`] of them, all on the error's first line.
    range: std::ops::Range<usize>,
    /// The `input:line:column` ariadne puts in the header for `text`, whose
    /// column counts from the start of the kept stretch...
    shown_location: String,
    /// ...and the one the error has in the whole input.
    true_location: String,
}

impl Excerpt {
    /// Cut down the line that `range` (characters of `input`, whose rendering
    /// is `source`) starts on, or return `None` when every line the error
    /// touches is short enough to echo whole.
    fn cut(
        input: &str,
        source: &ariadne::Source<&str>,
        range: std::ops::Range<usize>,
    ) -> Option<Self> {
        let too_long = source.get_line_range(&range).any(|idx| {
            source
                .line(idx)
                .is_some_and(|line| content_len(source, line) > MAX_ECHOED_LINE)
        });
        if !too_long {
            return None;
        }

        // Character offsets to byte offsets: always a char boundary.
        let byte = |char_offset: usize| {
            input
                .char_indices()
                .nth(char_offset)
                .map_or(input.len(), |(byte, _)| byte)
        };

        // `line` covers its terminator, so `line_end` is where the next begins.
        let (line, line_idx, column) = source.get_offset_line(range.start)?;
        let line_end = line.offset() + line.len();
        let label_end = range.end.min(line_end).min(range.start + MAX_ECHOED_TOKEN);
        let mut keep_start = range
            .start
            .saturating_sub(EXCERPT_CONTEXT)
            .max(line.offset());
        let mut keep_end = (label_end + EXCERPT_CONTEXT).min(line_end);
        // Leaving out only blanks at the edge of the line is no cut, and a `…`
        // in place of a single character saves nothing, so either end is
        // marked only when it leaves out more. The line before the kept
        // stretch without its leading whitespace, which the header's column
        // accounts for...
        let head = input[byte(line.offset())..byte(keep_start)].trim_start();
        let cut_front = head.chars().nth(1).is_some();
        if !cut_front {
            keep_start -= head.chars().count();
        }
        // ...and the rest of the line as ariadne would echo it, which is
        // without its trailing whitespace, terminator included.
        let rest = input[byte(keep_end)..byte(line_end)].trim_end();
        let cut_back = rest.chars().nth(1).is_some();
        if !cut_back {
            keep_end += rest.chars().count();
        }

        let mut text = input[..byte(line.offset())].to_string();
        if cut_front {
            text.push('…');
        }
        text.push_str(&input[byte(keep_start)..byte(keep_end)]);
        if cut_back {
            text.push('…');
        }

        let shown_column = usize::from(cut_front) + (range.start - keep_start);
        let shown_start = line.offset() + shown_column;
        let line_no = line_idx + 1;
        Some(Self {
            text,
            range: shown_start..shown_start + (label_end - range.start),
            shown_location: format!("{SOURCE_ID}:{line_no}:{}", shown_column + 1),
            true_location: format!("{SOURCE_ID}:{line_no}:{}", column + 1),
        })
    }

    /// Render the report, with the header's column put back to the error's
    /// column in the whole input.
    fn render(&self, headline: &str, detail: &str) -> Option<String> {
        let source = ariadne::Source::from(self.text.as_str());
        let report = render_report(&source, self.range.clone(), headline, detail)?;
        // The header is the first place the location appears: the headline
        // above it is one of `format_rich`'s fixed phrases.
        Some(report.replacen(&self.shown_location, &self.true_location, 1))
    }
}

/// `text` cut to its first [`MAX_ECHOED_TOKEN`] characters, with `…` marking
/// a cut.
fn shorten(text: &str) -> std::borrow::Cow<'_, str> {
    match text.char_indices().nth(MAX_ECHOED_TOKEN) {
        Some((cut, _)) => format!("{}…", &text[..cut]).into(),
        None => text.into(),
    }
}

/// Render a chumsky [`Rich`](chumsky::error::Rich) error as a `(headline, detail)`
/// pair suitable for an ariadne report.
fn format_rich(err: &chumsky::error::Rich<'_, crate::lexer::Token<'_>>) -> (String, String) {
    use crate::lexer::Token;
    use chumsky::error::RichReason;

    match err.reason() {
        RichReason::Custom(msg) => ("invalid time expression".to_string(), msg.clone()),
        _ => {
            // `Token`'s `Display` already spells `Space` out as "whitespace",
            // which reads badly inside backticks.
            let found = match err.found() {
                Some(Token::Space) => "whitespace".to_string(),
                Some(Token::Word(text) | Token::Number(text)) => format!("`{}`", shorten(text)),
                Some(token) => format!("`{token}`"),
                None => "end of input".to_string(),
            };

            let mut seen = std::collections::BTreeSet::new();
            let mut expected: Vec<String> = Vec::new();
            for pat in err.expected() {
                let rendered = pat.to_string();
                if seen.insert(rendered.clone()) {
                    expected.push(rendered);
                }
            }

            let detail = match expected.as_slice() {
                [] => format!("unexpected {found}"),
                [one] => format!("expected {one}, found {found}"),
                many => {
                    let last = many.last().expect("non-empty");
                    let head = &many[..many.len() - 1];
                    format!(
                        "expected one of {} or {}, found {found}",
                        head.join(", "),
                        last
                    )
                }
            };

            ("could not parse time expression".to_string(), detail)
        }
    }
}

/// Render the optional context of [`TempsError::DateCalculationError`] for its
/// `Display`: `": {context}"`, or nothing.
fn format_context(context: Option<&str>) -> String {
    context
        .map(|context| format!(": {context}"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TempsError::invalid_date(2024, 13, 32);
        assert_eq!(err.to_string(), "Invalid date: year=2024, month=13, day=32");

        let err = TempsError::invalid_time(25, 61, 61);
        assert_eq!(err.to_string(), "Invalid time: 25:61:61");

        let err = TempsError::parse_error("unexpected token", "in 5 minuts");
        assert_eq!(
            err.to_string(),
            "Failed to parse time expression: unexpected token"
        );
    }

    #[test]
    fn test_error_creation_helpers() {
        let err = TempsError::date_calculation("month out of range");
        match err {
            TempsError::DateCalculationError { message, context } => {
                assert_eq!(message, "month out of range");
                assert!(context.is_none());
            }
            _ => panic!("Wrong error type"),
        }

        let err = TempsError::backend_error("conversion failed", "chrono");
        match err {
            TempsError::BackendError { message, backend } => {
                assert_eq!(message, "conversion failed");
                assert_eq!(backend, "chrono");
            }
            _ => panic!("Wrong error type"),
        }
    }

    /// A context is the cause a backend reported, so it belongs in the text a
    /// caller logs, not only in `Debug`.
    #[test]
    fn date_calculation_display_carries_the_context() {
        let err = TempsError::date_calculation_with_source(
            "Failed to add months",
            "chrono error: date out of range",
        );
        assert_eq!(
            err.to_string(),
            "Date calculation error: Failed to add months: chrono error: date out of range"
        );

        let err = TempsError::date_calculation("Month overflow");
        assert_eq!(err.to_string(), "Date calculation error: Month overflow");
    }

    /// The public `errors::format_*` helpers are the text of the matching
    /// `TempsError` variant, not a second spelling of it.
    #[test]
    fn error_helpers_render_exactly_the_display_text() {
        use crate::errors::{
            format_invalid_date, format_invalid_time, format_invalid_timezone_offset,
        };

        for (year, month, day) in [(2024, 13, 32), (1, 2, 3), (0, 0, 0), (u16::MAX, 255, 255)] {
            assert_eq!(
                format_invalid_date(year, month, day),
                TempsError::invalid_date(year, month, day).to_string()
            );
        }

        for (hour, minute, second) in [(9, 5, 3), (25, 61, 61), (0, 0, 0), (255, 255, 255)] {
            assert_eq!(
                format_invalid_time(hour, minute, second),
                TempsError::invalid_time(hour, minute, second).to_string()
            );
        }
        assert_eq!(format_invalid_time(9, 5, 3), "Invalid time: 09:05:03");

        for total_minutes in [-30, 0, 90, -720, 840, i16::MIN, i16::MAX] {
            assert_eq!(
                format_invalid_timezone_offset(total_minutes),
                TempsError::invalid_timezone_offset(total_minutes).to_string()
            );
        }
    }

    /// The lexer's spans are byte offsets, ariadne indexes characters. A
    /// multi-byte character *before* the error site is what tells the two
    /// apart: here the error is at byte 5 but at character 4.
    #[test]
    fn parse_error_position_is_a_character_offset() {
        use crate::common::{ParserError, TokenInput, space, token_stream, word_ci};
        use crate::lexer::lex;
        use chumsky::prelude::*;

        fn expects_zwei<'t, 's: 't, I>() -> impl Parser<'t, I, (), ParserError<'t, 's>> + Clone
        where
            I: TokenInput<'t, 's>,
        {
            word_ci("für")
                .then_ignore(space())
                .then_ignore(word_ci("zwei"))
                .ignored()
        }

        let input = "für fünf";
        assert_eq!(input.find("fünf"), Some(5), "the byte offset");
        let tokens = lex(input);
        let errors = expects_zwei()
            .then_ignore(end())
            .parse(token_stream(input, &tokens))
            .into_result()
            .expect_err("`fünf` is not `zwei`");

        match rich_errors_to_temps_error(input, errors) {
            TempsError::ParseError {
                message, position, ..
            } => {
                assert_eq!(position, Some(4), "{message}");
                assert!(message.contains("input:1:5"), "{message}");

                // The underline starts in the same column as `fünf` does in
                // the echoed line above it (both rows share the gutter).
                let lines: Vec<&str> = message.lines().collect();
                let row = lines
                    .iter()
                    .position(|line| line.ends_with(input))
                    .unwrap_or_else(|| panic!("no source row:\n{message}"));
                let token = lines[row].chars().count() - "fünf".chars().count();
                let underline = lines[row + 1]
                    .chars()
                    .position(|c| c == '─' || c == '┬')
                    .unwrap_or_else(|| panic!("no underline:\n{message}"));
                assert_eq!(underline, token, "{message}");
            }
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn empty_input_gets_a_dedicated_message() {
        // Whitespace-only input is empty to the grammar, so it counts too.
        for input in ["", " ", "\t\n", "\u{a0}"] {
            match rich_errors_to_temps_error(input, Vec::new()) {
                TempsError::ParseError {
                    message, position, ..
                } => {
                    assert_eq!(position, Some(0), "{input:?}");
                    // The fallback knows no language, so it names no examples.
                    assert_eq!(
                        message, "input is empty; expected a time expression",
                        "{input:?}"
                    );
                }
                other => panic!("expected a parse error for {input:?}, got {other:?}"),
            }
        }
    }
}
