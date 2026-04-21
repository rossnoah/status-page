use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::db::DbPool;
use crate::error::AppError;
use crate::AppState;

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub services: Vec<ServiceStatus>,
}

#[derive(Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub category: String,
    pub status: String,
    pub uptime_pct: f64,
    pub latency_ms: f64,
}

async fn status(State(db): State<DbPool>) -> Result<Json<StatusResponse>, AppError> {
    let services = db
        .read(|conn| crate::db::services::list_public(conn))
        .await?;

    let overall = if services.iter().any(|s| s.status == "down") {
        "down"
    } else if services.iter().any(|s| s.status == "degraded") {
        "degraded"
    } else {
        "ok"
    };

    Ok(Json(StatusResponse {
        status: overall.into(),
        services: services
            .into_iter()
            .map(|s| ServiceStatus {
                name: s.name,
                category: s.category,
                status: s.status,
                uptime_pct: s.uptime_pct,
                latency_ms: s.avg_latency_ms,
            })
            .collect(),
    }))
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/status", get(status))
}
