use temps_macros::time_parser;

pub mod de;
pub mod parser;

#[derive(Debug, PartialEq, Eq)]
pub enum Time {
    Now,
}

time_parser!(de);
