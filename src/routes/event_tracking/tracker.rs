use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, extract::Query};
use uuid::Uuid;
use crate::database::types::RELATIONAL_DATABASE;
use crate::routes::event_tracking::tracker::EventTrackingError::InvalidRequest;
use crate::routes::event_tracking::types::{SubscribeRequest, UnsubscribeRequest, GetEventsQuery, EventsResponse, TrackedEvent};
use sqlx::Row;

#[derive(Debug, thiserror::Error)]
pub enum EventTrackingError {
    #[error("Database not initialized")]
    DatabaseNotInitialized,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

impl IntoResponse for EventTrackingError {
    fn into_response(self) -> axum::response::Response {
        match self {
            EventTrackingError::DatabaseNotInitialized => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database not initialized").into_response()
            }
            EventTrackingError::InvalidRequest(msg) => {
                (StatusCode::BAD_REQUEST, msg).into_response()
            }
            EventTrackingError::DatabaseError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
        }
    }
}


pub async fn get_events(
    Query(query): Query<GetEventsQuery>,
) -> Result<impl IntoResponse, EventTrackingError> {
    let db = RELATIONAL_DATABASE
        .get()
        .ok_or(EventTrackingError::DatabaseNotInitialized)?;

    if query.customer_email.trim().is_empty() {
        return Err(InvalidRequest("Email is required".to_string()));
    }

    // Pagination defaults and clamps
    let page = query.page.unwrap_or(1).max(1);
    let mut per_page = query.per_page.unwrap_or(50);
    if per_page == 0 { per_page = 50; }
    if per_page > 100 { per_page = 100; }
    let offset: i64 = ((page - 1) * per_page) as i64;
    let limit: i64 = per_page as i64;

    let confirmed_only = query.confirmed_only.unwrap_or(true);

    use sqlx::{Postgres, QueryBuilder};

    // Build WHERE predicate shared by count and page queries
    // Base: events that match user's subscriptions
    // Subscription matches if exact event signature or subscription has NULL (all events)
    
    // COUNT(*)
    let mut qb_count: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT COUNT(*)::BIGINT as count FROM tracked_events te WHERE EXISTS (\n            SELECT 1 FROM event_subscriptions es\n            WHERE es.user_email = "
    );
    qb_count.push_bind(&query.customer_email);
    qb_count.push(
        " AND es.chain_id = te.chain_id\n              AND es.contract_address = te.contract_address\n              AND (es.event_signature IS NULL OR es.event_signature = te.event_signature)\n        )",
    );

    if let Some(ref c) = query.chain_id {
        qb_count.push(" AND te.chain_id = ").push_bind(c);
    }
    if let Some(ref addr) = query.contract_address {
        qb_count.push(" AND te.contract_address = ").push_bind(addr.to_lowercase());
    }
    if let Some(ref sig) = query.event_signature {
        qb_count.push(" AND te.event_signature = ").push_bind(sig);
    }
    if confirmed_only {
        qb_count.push(" AND te.confirmed = true");
    }

    let total: i64 = qb_count
        .build()
        .fetch_one(db)
        .await
        .map_err(EventTrackingError::DatabaseError)?
        .get::<i64, _>(0);

    // Page query
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT te.id, te.chain_id, te.contract_address, te.block_number, te.tx_hash, te.log_index, te.event_signature, te.event_data, te.block_timestamp, te.confirmed, te.created_at\n         FROM tracked_events te WHERE EXISTS (\n            SELECT 1 FROM event_subscriptions es\n            WHERE es.user_email = "
    );
    qb.push_bind(&query.customer_email);
    qb.push(
        " AND es.chain_id = te.chain_id\n              AND es.contract_address = te.contract_address\n              AND (es.event_signature IS NULL OR es.event_signature = te.event_signature)\n        )",
    );

    if let Some(ref c) = query.chain_id {
        qb.push(" AND te.chain_id = ").push_bind(c);
    }
    if let Some(ref addr) = query.contract_address {
        qb.push(" AND te.contract_address = ").push_bind(addr.to_lowercase());
    }
    if let Some(ref sig) = query.event_signature {
        qb.push(" AND te.event_signature = ").push_bind(sig);
    }
    if confirmed_only {
        qb.push(" AND te.confirmed = true");
    }

    qb.push(" ORDER BY te.block_timestamp DESC, te.log_index DESC, te.created_at DESC ");
    qb.push(" LIMIT ").push_bind(limit);
    qb.push(" OFFSET ").push_bind(offset);

    let events: Vec<TrackedEvent> = qb
        .build_query_as::<TrackedEvent>()
        .fetch_all(db)
        .await
        .map_err(EventTrackingError::DatabaseError)?;

    let response = EventsResponse {
        events,
        total: total as usize,
        page,
        per_page,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

pub async fn subscribe_to_event(
    Json(payload): Json<SubscribeRequest>,
) -> Result<impl IntoResponse, EventTrackingError> {
    let db = RELATIONAL_DATABASE
        .get()
        .ok_or(EventTrackingError::DatabaseNotInitialized)?;

    if !payload.contract_address.starts_with("0x") || payload.contract_address.len() != 42 {
        return Err(EventTrackingError::InvalidRequest(
            "Invalid contract address".to_string(),
        ));
    }

    let contract_address = payload.contract_address.to_lowercase();
    let chain_id = payload.chain_id;
    let event_signatures = payload.event_signatures.unwrap_or_default();
    let customer_email = payload.customer_email;


    let mut tx = db
        .begin()
        .await
        .map_err(|e| InvalidRequest(format!("db begin error: {}", e)))?;

    if event_signatures.is_empty() {
        // Subscribe to all events (NULL event_signature)
        sqlx::query!(
            r#"INSERT INTO event_subscriptions (id, user_email, chain_id, contract_address, event_signature)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (user_email, chain_id, contract_address, event_signature) DO NOTHING"#,
            Uuid::new_v4(),
            customer_email,
            chain_id,
            contract_address,
            Option::<String>::None,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| EventTrackingError::DatabaseError(e))?;

        tx.commit()
            .await
            .map_err(|e| InvalidRequest(format!("commit error: {}", e)))?;

        return Ok((StatusCode::CREATED, "Subscribed to all events").into_response());
    }

    for sig in event_signatures {
        sqlx::query!(
            r#"INSERT INTO event_subscriptions (id, user_email, chain_id, contract_address, event_signature)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (user_email, chain_id, contract_address, event_signature) DO NOTHING"#,
            Uuid::new_v4(),
            customer_email,
            chain_id,
            contract_address,
            Some(sig),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| EventTrackingError::DatabaseError(e))?;
    }

    tx.commit()
        .await
        .map_err(|e| InvalidRequest(format!("commit error: {}", e)))?;

    Ok((StatusCode::CREATED, "Subscriptions created").into_response())
}

pub async fn unsubscribe_from_event(
    Json(payload): Json<UnsubscribeRequest>,
) -> Result<impl IntoResponse, EventTrackingError> {
    let db = RELATIONAL_DATABASE
        .get()
        .ok_or(EventTrackingError::DatabaseNotInitialized)?;

    // email is mandatory
    if payload.customer_email.trim().is_empty() {
        return Err(InvalidRequest("Email is required".to_string()));
    }

    // Optional validations/normalization
    let chain_id = payload.chain_id.map(|c| c);
    let contract_address = payload.contract_address.map(|addr| {
        let a = addr.to_lowercase();
        a
    });
    // If contract address provided, do a light validation similar to subscribe
    if let Some(ref addr) = contract_address {
        if !addr.starts_with("0x") || addr.len() != 42 {
            return Err(InvalidRequest("Invalid contract address".to_string()));
        }
    }

    // Build dynamic delete based on provided filters
    use sqlx::{QueryBuilder, Postgres};

    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("DELETE FROM event_subscriptions WHERE user_email = ");
    qb.push_bind(&payload.customer_email);

    if let Some(c) = chain_id {
        qb.push(" AND chain_id = ").push_bind(c);
    }

    if let Some(addr) = contract_address {
        qb.push(" AND contract_address = ").push_bind(addr);
    }

    if let Some(sigs) = payload.event_signature {
        if !sigs.is_empty() {
            qb.push(" AND event_signature IN (");
            let mut separated = qb.separated(", ");
            for s in sigs {
                separated.push_bind(s);
            }
            qb.push(")");
        }
    }

    let result = qb
        .build()
        .execute(db)
        .await
        .map_err(EventTrackingError::DatabaseError)?;

    let deleted = result.rows_affected();

    Ok((StatusCode::OK, format!("Unsubscribed from {} event(s)", deleted)).into_response())
}