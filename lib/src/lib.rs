use temps_macros::TimeParsers;
use thiserror::Error;

pub mod interpreter;

#[derive(TimeParsers)]
pub enum LocalizedParsers {
    #[time_parser]
    DE,
}

#[derive(Error, Debug)]
pub enum TempsError {
    #[error(transparent)]
    DeTimeParseError(#[from] crate::DE::TimeParseError),

    #[error("chrono error")]
    ChronoError,

    #[error("unknown language")]
    UnknownLanguage,
}

pub fn parse<Tz: chrono::TimeZone>(
    input: &str,
    parser: LocalizedParsers,
    now: chrono::DateTime<Tz>,
) -> Result<chrono::DateTime<Tz>, TempsError> {
    let time = match parser {
        LocalizedParsers::DE => crate::DE::parse(input)?,
    };

    interpreter::interpret(time, now)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    #[test]
    fn de_test_parse_now() {
        let now = Utc::now();
        let actual = parse("jetzt", LocalizedParsers::DE, now).unwrap();

        assert_eq!(actual, now);
    }

    #[test]
    fn de_test_parse_date() {
        let now = Utc::now();
        let expected = Utc.with_ymd_and_hms(1990, 10, 10, 0, 0, 0).unwrap();
        let actual = parse("10.10.1990", LocalizedParsers::DE, now).unwrap();

        assert_eq!(actual, expected);
    }
}
