use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(TimeParsers, attributes(time_parser))]
pub fn derived_time_parser(input: TokenStream) -> TokenStream {
    let name = parse_macro_input!(input as DeriveInput);

    let mut output = TokenStream::new();

    output.extend(TokenStream::from(quote! {
        #[derive(Debug, PartialEq, Eq)]
        pub enum Time {
            Now,
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
                        #[error(transparent)]
                        PestError(#[from] pest::error::Error<Rule>),

                        #[error("unexpected pattern")]
                        UnexpectedPattern,
                    }

                    pub fn parse(input: &str) -> Result<crate::Time, TimeParseError> {
                        let pairs = TimeParser::parse(Rule::time, input)?;
                        let pairs = pairs.flatten().collect::<Vec<Pair<Rule>>>();

                        let rules_and_str = pairs
                            .iter()
                            .map(|pair| (pair.as_rule(), pair.as_str()))
                            .collect::<Vec<(Rule, &str)>>();

                        match rules_and_str.as_slice() {
                            [(Rule::now, _), (Rule::EOI, _)] => {
                                Ok(crate::Time::Now)
                            }
                            _ => Err(TimeParseError::UnexpectedPattern),
                        }
                    }
                }
            }))
        }),
        _ => panic!("Only works on enums"),
    }

    output
}
