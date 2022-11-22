use temps_macros::time_parser;
use thiserror::Error;

pub mod de;
pub mod interpreter;

#[derive(Debug, PartialEq, Eq)]
pub enum Time {
    Now,
}

pub enum Language {
    De,
}

#[derive(Error, Debug)]
pub enum TempsError {
    #[error(transparent)]
    DeTimeParseError(#[from] DeTimeParseError),
}

time_parser!(de);

pub fn parse<Tz: chrono::TimeZone>(
    input: &str,
    now: chrono::DateTime<Tz>,
) -> Result<chrono::DateTime<Tz>, TempsError> {
    let time = parse_time_de(input)?;
    Ok(interpreter::interpret(time, now))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_now() {
        let input = "jetzt";
        let actual = crate::parse_time_de(input).unwrap();

        let expected = crate::Time::Now;

        assert_eq!(actual, expected);
    }
}
