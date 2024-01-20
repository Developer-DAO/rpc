use std::{fmt::{self, Display, Formatter}, error::Error};

#[derive(Debug)]
pub struct ParsingError; 

impl Display for ParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "An error was encountered while parsing the provided string into the Plan enum")
    }
}

impl Error for ParsingError {}
