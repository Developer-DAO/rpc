use std::{error::Error, fmt::{self, Display, Formatter}};
use axum::{http::StatusCode, response::IntoResponse};

#[derive(Debug)]
pub struct ApiError {
    pub err: Box<dyn Error>,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\nAn error occured while processing your API request: \n\t{}",
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

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.err.source() {
            Some(s) => Some(s),
            None => None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.err.to_string()).into_response()
    }
}
