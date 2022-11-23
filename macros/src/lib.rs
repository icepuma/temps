use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(TimeParsers, attributes(time_parser))]
pub fn derived_time_parser(input: TokenStream) -> TokenStream {
    let name = parse_macro_input!(input as DeriveInput);

    let mut output = TokenStream::new();

    output.extend(TokenStream::from(quote! {
        #[derive(Debug, PartialEq, Eq)]
        pub enum Preposition {
            Next,
            Last,
        }

        #[derive(Debug, PartialEq, Eq)]
        pub enum Time {
            Now,
            DayAt {
                preposition: Option<Preposition>,
                day_of_week: chrono::Weekday,
                hour: u32,
                minute: u32,
                second: u32,
            }
            Date { day: u32, month: u32, year: i32 },
            Iso { year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32 },
        }
    }));

    match name.data {
        Data::Enum(data_enum) => data_enum.variants.iter().for_each(|variant| {
            let variant_ident = &variant.ident;
            let grammar_path = format!(
                "grammars/{}.time.pest",
                variant_ident.to_string().to_lowercase()
            );

            output.extend(TokenStream::from(quote! {
                pub mod #variant_ident {
                    use pest::iterators::Pair;
                    use pest_derive::Parser;
                    use pest::Parser;

                    #[derive(Parser)]
                    #[grammar = #grammar_path]
                    struct TimeParser;

                    #[derive(thiserror::Error, Debug)]
                    pub enum TimeParseError {
                        #[error("invalid integer")]
                        ParseInt(#[from] std::num::ParseIntError),

                        #[error(transparent)]
                        PestError(#[from] pest::error::Error<Rule>),

                        #[error("unexpected pattern")]
                        UnexpectedPattern,
                    }

                    pub fn parse(input: &str) -> Result<crate::Time, TimeParseError> {
                        let pairs = TimeParser::parse(Rule::times, input)?;
                        let pairs = pairs.flatten().collect::<Vec<Pair<Rule>>>();

                        let rules_and_str = pairs
                            .iter()
                            .map(|pair| (pair.as_rule(), pair.as_str()))
                            .collect::<Vec<(Rule, &str)>>();

                        match rules_and_str.as_slice() {
                            [(Rule::now, _), (Rule::EOI, _)] => {
                                Ok(crate::Time::Now)
                            }
                            [(Rule::day_at, _), (Rule::day_with_preposition, day_with_preposition), rest @ .., (Rule::EOI, _)] => {
                                match rest {
                                    [(Rule::specific_day, specific_day)] =>
                                }
                            }
                            [(Rule::iso, _), (Rule::year, year), (Rule::month, month), (Rule::day, day), rest @ .., (Rule::EOI, _)] => {
                                let (hour, minute, second) = match rest {
                                    [(Rule::hour, hour)] => (hour, &"0", &"0"),
                                    [(Rule::hour, hour), (Rule::minute, minute)] => (hour, minute, &"0"),
                                    [(Rule::hour, hour), (Rule::minute, minute), (Rule::second, second)] => (hour, minute, second),
                                    _ => return Err(TimeParseError::UnexpectedPattern),
                                };

                                Ok(crate::Time::Iso {
                                    year: year.parse()?,
                                    month: month.parse()?,
                                    day: day.parse()?,
                                    hour: hour.parse()?,
                                    minute: minute.parse()?,
                                    second: second.parse()?,
                                })
                            }
                            [(Rule::date, _), (Rule::day, day), (Rule::month, month), (Rule::year, year), (Rule::EOI, _)] => {
                                Ok(crate::Time::Date {
                                    day: day.parse()?,
                                    month: month.parse()?,
                                    year: year.parse()?,
                                })
                            }
                            _ => {
                                println!("{:?}", rules_and_str);
                                Err(TimeParseError::UnexpectedPattern)
                            },
                        }
                    }
                }
            }))
        }),
        _ => panic!("Only works on enums"),
    }

    output
}
