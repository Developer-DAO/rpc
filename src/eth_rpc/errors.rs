use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub struct EthCallError {
    err: &'static dyn std::error::Error,
}


impl Display for EthCallError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\nAn error occured while calling this API route: {}",
            self.err,
        )?;

        let mut err: Option<&dyn std::error::Error> = self.err.source();

        while let Some(src) = err {
            write!(f, "\nCaused by: {}", src)?;
            err = src.source();
        }
        Ok(())
    }
}

impl std::error::Error for EthCallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.err.source() {
            Some(s) => Some(s),
            None => None,
        }
    }
}

impl From<reqwest::Error> for EthCallError {
    fn from(value: reqwest::Error) -> Self {
        EthCallError { err: Box::leak(Box::new(value))}
    }
}
