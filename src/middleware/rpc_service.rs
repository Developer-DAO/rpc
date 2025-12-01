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

#[tracing::instrument(skip(request))]
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

    if matches!(sub_info.plan, Plan::Tier1 | Plan::Tier2 | Plan::Tier3)
        && sub_info.expires > OffsetDateTime::now_utc()
    {
        // tokio::spawn(async move {
        //     if let Err(e) = refill_calls_and_renew_plans().await {
        //         info!("Failed to refill calls or reset plan for users:\n {}", e);
        //     }
        // });
        Err(RpcAuthErrors::PlanExpired)?
    }

    // check callcount
    if sub_info.calls > sub_info.plan.get_plan_limit() as i64 {
        Err(RpcAuthErrors::OutOfCredits)?
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

pub struct RenewInfo<'a> {
    email: EmailAddress<'a>,
    plan: Plan,
    expires: OffsetDateTime,
    balance: i64,
    downgradeto: Option<Plan>,
}

pub async fn refill_calls_and_renew_plans() -> Result<(), RpcAuthErrors> {
    // Reset calls. If someone's plan is expired, attempt to renew the current plan
    // from the account balance. If the account does not have enough balance,
    // then switch to the free plan and reset calls.

    let mut tx = RELATIONAL_DATABASE
        .get()
        .expect("We're cooked chat (no DB connection)")
        .begin()
        .await?;

    let user_info = sqlx::query_as!(
        RenewInfo,
        r#"
        SELECT balance, Customers.email, plan as "plan!: Plan", expires, downgradeto as "downgradeto!: Plan" FROM RpcPlans, Customers where now() >= expires
        "#
    )
    .fetch_all(&mut *tx)
    .await?;

    // reset user calls
    sqlx::query!("UPDATE RpcPlans SET calls = 0 WHERE now() >= expires AND calls > 0",)
        .execute(&mut *tx)
        .await?;

    for user_info in user_info {
        // handles downgrade or cancellation
        if let Some(dplan) = user_info.downgradeto
            && user_info.plan > dplan
        {
            sqlx::query!(
                "UPDATE RpcPlans SET plan = $1 where email = $2",
                dplan as Plan,
                user_info.email.as_str(),
            )
            .execute(&mut *tx)
            .await?;
        };

        if matches!(user_info.plan, Plan::Tier1 | Plan::Tier2 | Plan::Tier3)
            && user_info.expires >= OffsetDateTime::now_utc()
        {
            let cost = (user_info.plan.get_cost() * 100.0) as i64;

            if user_info.balance >= cost {
                sqlx::query!(
                    "UPDATE Customers SET balance = balance - $1 where email = $2",
                    cost,
                    user_info.email.as_str(),
                )
                .execute(&mut *tx)
                .await?;
            } else {
                // insufficient balance to cover plan
                // downgrades to Free
                sqlx::query!(
                    "UPDATE RpcPlans SET plan = $1 where email = $2",
                    Plan::Free as Plan,
                    user_info.email.as_str(),
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    tx.commit().await?;

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
