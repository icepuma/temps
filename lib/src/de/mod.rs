use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "de/de.time.pest"]
pub struct DeTimeParser;
