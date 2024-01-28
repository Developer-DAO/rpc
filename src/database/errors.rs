use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug)]
pub struct ParsingError(pub String, pub &'static str);

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
