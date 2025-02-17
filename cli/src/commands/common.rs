use clap::{Parser, ValueEnum};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Parser, Debug, Clone, ValueEnum)]
pub enum Field {
    Goldilocks,
    // Add other variants here as needed
}

impl FromStr for Field {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "goldilocks" => Ok(Field::Goldilocks),
            // Add parsing for other variants here
            _ => Err(format!("'{}' is not a valid value for Field", s)),
        }
    }
}

impl Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Field::Goldilocks => write!(f, "goldilocks"),
        }
    }
}
