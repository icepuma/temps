use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Ident};

fn capitalize(input: &str) -> String {
    let mut chars = input.chars();

    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(chars).collect(),
    }
}

#[proc_macro]
pub fn time_parser(token_stream: TokenStream) -> TokenStream {
    let lang = parse_macro_input!(token_stream as syn::Path);
    let lang = lang.get_ident().unwrap();

    let span = Span::call_site().into();
    let fn_name = Ident::new(&format!("parse_time_{}", lang), span);
    let lang_capitalized = capitalize(&lang.to_string());

    let error_name = Ident::new(&format!("{}TimeParseError", lang_capitalized), span);
    let time_parser_name = Ident::new(&format!("{}TimeParser", lang_capitalized), span);

    let expanded = quote! {
        use pest::iterators::Pair;
        use pest::Parser;

        #[derive(Error, Debug)]
        pub enum #error_name {
            #[error(transparent)]
            PestError(#[from] pest::error::Error<crate::#lang::Rule>),

            #[error("unexpected pattern")]
            UnexpectedPattern,
        }

        pub fn #fn_name(
            input: &str,
        ) -> Result<Time, #error_name> {
            let pairs =
                crate::#lang::#time_parser_name::parse(crate::#lang::Rule::time, input)?;
            let pairs = pairs.flatten().collect::<Vec<Pair<crate::#lang::Rule>>>();

            let rules_and_str = pairs
                .iter()
                .map(|pair| (pair.as_rule(), pair.as_str()))
                .collect::<Vec<(crate::#lang::Rule, &str)>>();

            match rules_and_str.as_slice() {
                [(crate::#lang::Rule::now, _), (crate::#lang::Rule::EOI, _)] => {
                    Ok(Time::Now)
                }
                _ => Err(#error_name::UnexpectedPattern),
            }
        }
    };

    TokenStream::from(expanded)
}
