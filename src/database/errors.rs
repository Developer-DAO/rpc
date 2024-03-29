use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug)]
pub struct ParsingError(pub String, pub &'static str);
#[derive(Debug)]
pub struct ChainidError(pub String , pub &'static str);
impl Display for ParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "An error was encountered while parsing {} into {}",
            self.0, self.1
        )
    }
}

impl Error for ParsingError {}

impl Display for ChainidError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "An error was encountered while retrieving Chainid {} into {}",
            self.0, self.1
        )
    }
}

impl Error for ChainidError {}