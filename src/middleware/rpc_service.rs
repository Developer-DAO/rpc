use crate::{
    database::types::{Plan, RELATIONAL_DATABASE},
    routes::errors::ApiError,
};
use axum::{
    extract::{Path, Request},
    middleware::Next,
    response::IntoResponse,
};
use core::fmt;
use sqlx::{prelude::FromRow, types::time::OffsetDateTime, Database, Decode};
use std::{
    error::Error,
    fmt::{Display, Formatter},
};
use tokio::join;

#[derive(Debug, FromRow)]
struct SubscriptionInfo {
    plan_expiration: OffsetDateTime,
    callcount: i64,
    subscription: Plan,
    customeremail: String,
}

impl<'r, DB: Database> Decode<'r, DB> for Plan
where
    &'r str: Decode<'r, DB>,
{
    fn decode(
        value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let value = <&str as Decode<DB>>::decode(value)?;
        Ok(match value {
            "gigachad" | "Gigachad" => Plan::Gigachad,
            "premier" | "Premier" => Plan::Premier,
            "based" | "Based" => Plan::Based,
            _ => Plan::None,
        })
    }
}

impl Plan {
    // todo
    // these are arbitrary numbers for now
    pub fn calls_per_month(&self) -> u64 {
        match self {
            Plan::None => 0,
            Plan::Based => 3_000_000,
            Plan::Premier => 50_000_000,
            Plan::Gigachad => 420_690_000,
        }
    }
}

pub async fn validate_subscription_and_update_user_calls(
    Path(key): Path<[String; 2]>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, ApiError<RpcAuthErrors>> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();

    let sub_info: SubscriptionInfo = sqlx::query_as!(
        SubscriptionInfo,
        r#"SELECT PaymentInfo.planExpiration AS plan_expiration,
        callCount,
        subscription as "subscription: Plan",
        PaymentInfo.customerEmail
        FROM PaymentInfo
        WHERE PaymentInfo.customerEmail = (SELECT customerEmail FROM Api WHERE apiKey = $1)"#,
        key.get(1)
            .ok_or_else(|| ApiError::new(RpcAuthErrors::InvalidApiKey))?
    )
    .fetch_optional(db_connection)
    .await?
    .ok_or_else(|| ApiError::new(RpcAuthErrors::PaymentNotFound))?;

    if sub_info.plan_expiration < OffsetDateTime::now_utc() {
        Err(ApiError::new(RpcAuthErrors::PaymentExpired))?
    }

    let absv = sub_info.callcount.unsigned_abs();
    // check callcount
    if absv >= sub_info.subscription.calls_per_month() {
        Err(ApiError::new(RpcAuthErrors::PlanLimitReached))?
    }

    let inc = tokio::spawn(async move {
        sqlx::query!(
            "UPDATE PaymentInfo set callCount = $1 WHERE customerEmail = $2",
            sub_info.callcount + 1,
            sub_info.customeremail,
        )
        .execute(db_connection)
        .await
    });
    let ret = tokio::spawn(async { next.run(request).await });

    let (res, inc) = join!(ret, inc);

    inc.unwrap()?;

    Ok(res.unwrap())
}

#[derive(Debug)]
pub enum RpcAuthErrors {
    InvalidApiKey,
    DatabaseError(sqlx::Error),
    PaymentExpired,
    PaymentNotFound,
    PlanLimitReached,
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
            RpcAuthErrors::PlanLimitReached => write!(f, "The current call could not be processed because this account reached it monthly limit. Please upgrade your plan with us!")
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
            RpcAuthErrors::PlanLimitReached => None,
        }
    }
}

impl From<sqlx::Error> for ApiError<RpcAuthErrors> {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(RpcAuthErrors::DatabaseError(value))
    }
}
