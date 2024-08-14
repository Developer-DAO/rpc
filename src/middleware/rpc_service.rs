use crate::database::types::RELATIONAL_DATABASE;
use axum::{
    extract::{Path, Request},
    middleware::Next,
    response::IntoResponse,
};
use thiserror::Error;
use tokio::join;

pub struct Credits {
    credits: i64,
    email: String,
}

pub async fn validate_subscription_and_update_user_calls(
    Path(key): Path<[String; 2]>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, RpcAuthErrors> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let sub_info: Credits = sqlx::query_as!(
        Credits,
        "SELECT credits, email
        FROM Customers
        WHERE Customers.email = (SELECT customerEmail FROM Api WHERE apiKey = $1)",
        key.get(1).ok_or_else(|| RpcAuthErrors::InvalidApiKey)?
    )
    .fetch_optional(db_connection)
    .await?
    .ok_or_else(|| RpcAuthErrors::InvalidApiKey)?;

    // check callcount
    if sub_info.credits == 0 {
        Err(RpcAuthErrors::OutOfCredits)?
    }

    let inc = tokio::spawn(async move {
        sqlx::query!(
            // atomically decriment the credits field
            "UPDATE Customers SET credits = credits - 1 WHERE email = $1",
            sub_info.email,
        )
        .execute(db_connection)
        .await
    });
    let ret = tokio::spawn(async { next.run(request).await });

    let (res, inc) = join!(ret, inc);

    inc.unwrap()?;

    Ok(res.unwrap())
}

#[derive(Debug, Error)]
pub enum RpcAuthErrors {
    #[error("The supplied api key is invalid.")]
    InvalidApiKey,
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("You have ran out of credits. Please buy more if you love our service!")]
    OutOfCredits,
}

impl IntoResponse for RpcAuthErrors {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            self.to_string(),
        )
            .into_response()
    }
}
