use pest_derive::Parser;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "de/de.time.pest"]
pub struct DeTimeParser;

#[derive(Error, Debug)]
pub enum DeTimeParseError {
    #[error(transparent)]
    PestError(#[from] pest::error::Error<crate::de::Rule>),

    #[error("unexpected pattern")]
    UnexpectedPattern,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Time {
    Now,
}
