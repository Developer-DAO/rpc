use crate::{
    database::types::{Api, RELATIONAL_DATABASE},
    routes::errors::ApiError,
};
use axum::{
    extract::{Query, Request},
    middleware::Next,
    response::IntoResponse,
};
use core::fmt;
use sqlx::types::time::OffsetDateTime;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
struct SubscriptionInfo {
    plan_expiration: OffsetDateTime,
    callcount: i64,
}

pub async fn validate_subscription_and_update_user_calls(
    Query(key): Query<String>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, ApiError<RpcAuthErrors>> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();

    let res = sqlx::query_as!(Api, "SELECT * FROM Api WHERE apiKey = $1", key)
        .fetch_optional(db_connection)
        .await?
        .ok_or_else(|| ApiError::new(RpcAuthErrors::InvalidApiKey))?;

    let sub_info: SubscriptionInfo = sqlx::query_as!(
        SubscriptionInfo,
        "SELECT PaymentInfo.planExpiration AS plan_expiration,
        callCount
        FROM PaymentInfo
        WHERE PaymentInfo.customerEmail = $1",
        &res.customeremail
    )
    .fetch_optional(db_connection)
    .await?
    .ok_or_else(|| ApiError::new(RpcAuthErrors::PaymentNotFound))?;

    if sub_info.plan_expiration < OffsetDateTime::now_utc() {
        Err(ApiError::new(RpcAuthErrors::PaymentExpired))?
    }

    let ret = next.run(request).await;

    sqlx::query!(
        "UPDATE PaymentInfo set callCount = $1 WHERE customerEmail = $2",
        sub_info.callcount + 1,
        res.customeremail,
    )
    .execute(db_connection)
    .await?;

    Ok(ret)
}

#[derive(Debug)]
pub enum RpcAuthErrors {
    InvalidApiKey,
    DatabaseError(sqlx::Error),
    PaymentExpired,
    PaymentNotFound,
}

impl Display for RpcAuthErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RpcAuthErrors::InvalidApiKey => write!(f, "The API key used is invalid or expired"),
            RpcAuthErrors::DatabaseError(_) => write!(
                f,
                "An issue occured while authenticating your API key against the Database"
            ),
            RpcAuthErrors::PaymentExpired => write!(f, "The user's plan has expired. Please resubscribe if you enjoy our service. Thank you for choosing the Developer DAO RPC"),
            RpcAuthErrors::PaymentNotFound => write!(f, "Payment not found. Please enter manually."),
        }
    }
}

impl Error for RpcAuthErrors {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RpcAuthErrors::InvalidApiKey => None,
            RpcAuthErrors::DatabaseError(e) => Some(e),
            RpcAuthErrors::PaymentExpired => None,
            RpcAuthErrors::PaymentNotFound => None,
        }
    }
}

impl From<sqlx::Error> for ApiError<RpcAuthErrors> {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(RpcAuthErrors::DatabaseError(value))
    }
}
