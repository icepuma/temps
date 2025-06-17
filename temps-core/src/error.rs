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
#[derive(Error, Debug, Clone, PartialEq)]
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
        /// The specific parsing error message
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
    /// # Example
    ///
    /// ```
    /// use temps_core::TempsError;
    ///
    /// let err = TempsError::date_calculation("Month overflow");
    /// ```
    #[error("Date calculation error: {message}")]
    DateCalculationError {
        /// The specific calculation error message
        message: String,
        /// Optional context about what caused the error
        context: Option<String>,
    },

    /// Error for invalid date components
    #[error("Invalid date: year={year}, month={month}, day={day}")]
    InvalidDate {
        /// The year component
        year: u16,
        /// The month component (1-12)
        month: u8,
        /// The day component (1-31)
        day: u8,
    },

    /// Error for invalid time components
    #[error("Invalid time: {hour:02}:{minute:02}:{second:02}")]
    InvalidTime {
        /// The hour component (0-23)
        hour: u8,
        /// The minute component (0-59)
        minute: u8,
        /// The second component (0-59)
        second: u8,
    },

    /// Error for invalid timezone offset
    #[error("Invalid timezone offset: {hours:+03}:{minutes:02}")]
    InvalidTimezoneOffset {
        /// The hour offset (-12 to +14)
        hours: i8,
        /// The minute offset (0-59)
        minutes: u8,
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
    ///
    /// # Example
    ///
    /// ```
    /// use temps_core::TempsError;
    ///
    /// let err = TempsError::date_calculation_with_source(
    ///     "Failed to add months",
    ///     "chronos error: date out of range"
    /// );
    /// ```
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
    pub fn invalid_date(year: u16, month: u8, day: u8) -> Self {
        Self::InvalidDate { year, month, day }
    }

    /// Creates an invalid time error
    pub fn invalid_time(hour: u8, minute: u8, second: u8) -> Self {
        Self::InvalidTime {
            hour,
            minute,
            second,
        }
    }

    /// Creates an invalid timezone offset error
    pub fn invalid_timezone_offset(hours: i8, minutes: u8) -> Self {
        Self::InvalidTimezoneOffset { hours, minutes }
    }

    /// Creates an ambiguous time error
    pub fn ambiguous_time(message: impl Into<String>) -> Self {
        Self::AmbiguousTime {
            message: message.into(),
        }
    }

    /// Creates an arithmetic overflow error
    pub fn arithmetic_overflow(operation: impl Into<String>) -> Self {
        Self::ArithmeticOverflow {
            operation: operation.into(),
        }
    }

    /// Creates an unsupported operation error
    pub fn unsupported_operation(operation: impl Into<String>) -> Self {
        Self::UnsupportedOperation {
            operation: operation.into(),
        }
    }

    /// Creates a backend error
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

/// Extension trait for converting parser errors to TempsError.
///
/// This trait is implemented for winnow parser errors to provide
/// convenient conversion to our error type.
pub trait ParseErrorExt {
    /// Convert a parser error to a TempsError.
    ///
    /// This method extracts position information from the parser error
    /// and creates a properly formatted TempsError.
    fn to_temps_error(self, input: &str) -> TempsError;
}

impl ParseErrorExt for winnow::error::ParseError<&str, winnow::error::ContextError> {
    fn to_temps_error(self, input: &str) -> TempsError {
        let position = self.offset();
        let message = format!("Parser error: {}", self);
        TempsError::parse_error_with_position(message, input, position)
    }
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
}
