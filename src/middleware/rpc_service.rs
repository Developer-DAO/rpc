use crate::{
    database::types::{Plan, RELATIONAL_DATABASE},
    routes::types::EmailAddress,
};
use axum::{
    extract::{Path, Request},
    middleware::Next,
    response::IntoResponse,
};
use std::time::Duration;
use thiserror::Error;
use time::OffsetDateTime;
use tracing::info;

pub struct Credits<'a> {
    calls: i64,
    email: EmailAddress<'a>,
    plan: Plan,
    expires: OffsetDateTime,
}

pub struct SubscriptionInfo<'a> {
    email: EmailAddress<'a>,
    plan: Plan,
    expires: OffsetDateTime,
    balance: i64,
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
            sub_info.email.as_str(),
        )
        .execute(db_connection)
        .await
    });

    Ok(next.run(request).await)
}

pub async fn subsciption_task() -> Result<(), RpcAuthErrors> {
    loop {
        match refill_calls_and_renew_plans().await {
            Ok(_) => tokio::time::sleep(Duration::from_secs(86400)).await,
            Err(e) => {
                //retry an hour later if it fails
                info!("Error with refilling calls and renewing plans {}", e);
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        }
    }
}

pub async fn refill_calls_and_renew_plans() -> Result<(), RpcAuthErrors> {
    // Reset calls. If someone's plan is expired, attempt to renew the current plan
    // from the account balance. If the account does not have enough balance,
    // then switch to the free plan and reset calls.

    let user_info = sqlx::query_as!(
        SubscriptionInfo,
        r#"
        SELECT balance, Customers.email, plan as "plan!: Plan", expires FROM RpcPlans, Customers where now() > expires
        "#
    )
    .fetch_all(RELATIONAL_DATABASE.get().unwrap())
    .await?;

    sqlx::query!("UPDATE RpcPlans SET calls = 0 WHERE now() >= expires AND calls > 0",)
        .execute(RELATIONAL_DATABASE.get().unwrap())
        .await?;

    // sqlx::query!(r#"
    //     SELECT balance, Customers.email, plan as "plan!:Plan"
    //     FROM Customers, RpcPlans
    //
    //     "#);

    for user_info in user_info {
        tokio::spawn(async move {
            if matches!(user_info.plan, Plan::Tier1 | Plan::Tier2 | Plan::Tier3)
                && user_info.expires > OffsetDateTime::now_utc()
            {
                let cost = user_info.plan.get_cost() as i64;

                if user_info.balance >= cost {
                    sqlx::query!(
                        "UPDATE Customers SET balance = balance - $1 where email = $2",
                        cost,
                        user_info.email.as_str(),
                    )
                    .execute(RELATIONAL_DATABASE.get().unwrap())
                    .await?;
                } else {
                    sqlx::query!(
                        "UPDATE RpcPlans SET plan = $1 where email = $2",
                        Plan::Free as Plan,
                        user_info.email.as_str(),
                    )
                    .execute(RELATIONAL_DATABASE.get().unwrap())
                    .await?;
                }
            }
            Ok::<(), RpcAuthErrors>(())
        })
        .await
        .unwrap()?;
    }

    Ok(())
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
