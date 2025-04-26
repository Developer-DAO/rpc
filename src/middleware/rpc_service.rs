use crate::database::types::{Plan, RELATIONAL_DATABASE};
use axum::{
    extract::{Path, Request},
    middleware::Next,
    response::IntoResponse,
};
use thiserror::Error;
use time::OffsetDateTime;

pub struct Credits {
    calls: i64,
    email: String,
    plan: Plan,
    expires: OffsetDateTime,
}

#[tracing::instrument]
pub async fn validate_subscription_and_update_user_calls(
    Path(key): Path<[String; 2]>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, RpcAuthErrors> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let sub_info: Credits = sqlx::query_as!(
        Credits,
        r#"
        SELECT email, calls, plan as "plan!: Plan", expires
        FROM RpcPlans
        WHERE 
        email = (SELECT customerEmail FROM Api WHERE apiKey = $1) 
        "#,
        key.get(1).ok_or_else(|| RpcAuthErrors::InvalidApiKey)?
    )
    .fetch_optional(db_connection)
    .await?
    .ok_or_else(|| RpcAuthErrors::InvalidApiKey)?;

    // check callcount
    if sub_info.calls > sub_info.plan.get_plan_limit() as i64 {
        Err(RpcAuthErrors::OutOfCredits)?
    }

    if matches!(sub_info.plan, Plan::Tier1 | Plan::Tier2 | Plan::Tier3)
        && sub_info.expires > OffsetDateTime::now_utc()
    {
        Err(RpcAuthErrors::PlanExpired)?
    }

    tokio::spawn(async move {
        sqlx::query!(
            // atomically decriment the credits field
            "UPDATE RpcPlans SET calls = calls + 1 WHERE email = $1",
            sub_info.email,
        )
        .execute(db_connection)
        .await
    });

    Ok(next.run(request).await)
}

#[derive(Debug, Error)]
pub enum RpcAuthErrors {
    #[error("The supplied api key is invalid.")]
    InvalidApiKey,
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("You have ran out of credits. Please resubscribe if you love our service!")]
    OutOfCredits,
    #[error("Plan expired. Please resubscribe if you love our service!")]
    PlanExpired,
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
