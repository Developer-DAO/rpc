use std::fmt::{self, Display, Formatter};
use axum::{http::StatusCode, response::IntoResponse};

#[derive(Debug)]
pub struct ApiError<T> 
where T: std::error::Error
{
    pub err: T,
}

impl<T> ApiError<T> where T: std::error::Error  {
    pub fn new(err: T) -> Self {
        Self {
            err,
        }
    }
}

impl<T> Display for ApiError<T> where T: std::error::Error{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\nAn error occured while processing your request: \n\t{}",
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

impl<T> std::error::Error for ApiError<T> where T: std::error::Error{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.err.source() {
            Some(s) => Some(s),
            None => None,
        }
    }
}

impl<T> IntoResponse for ApiError<T> where T: std::error::Error {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.err.to_string()).into_response()
    }
}
