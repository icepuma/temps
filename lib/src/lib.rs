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

    Ok(interpreter::interpret(time, now))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_parse_time_de() {
        let now = Utc::now();
        let actual = parse("jetzt", LocalizedParsers::DE, now).unwrap();

        assert_eq!(actual, now);
    }
}
