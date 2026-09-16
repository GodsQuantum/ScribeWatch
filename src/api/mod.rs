pub mod browse;
pub mod dashboard;
pub mod events;
pub mod jobs;
pub mod providers;
pub mod quick;
pub mod workflows;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{delete, get, post, put},
};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Health {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn ready(State(state): State<AppState>) -> AppResult<Json<Health>> {
    if state.config.allowed_roots.iter().any(|root| !root.is_dir()) {
        return Err(AppError::Internal(anyhow::anyhow!(
            "an allowed root is unavailable"
        )));
    }
    state
        .db
        .list::<crate::domain::Job>("job")
        .map_err(AppError::Internal)?;
    Ok(Json(Health {
        status: "ready",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/ready", get(ready))
        .route("/api/v1/dashboard", get(dashboard::stats))
        .route("/api/v1/events", get(events::events))
        .route("/api/v1/browse", get(browse::browse))
        .route(
            "/api/v1/providers",
            get(providers::list).post(providers::upsert),
        )
        .route("/api/v1/providers/{id}", delete(providers::delete))
        .route("/api/v1/providers/{id}/models", get(providers::models))
        .route(
            "/api/v1/workflows",
            get(workflows::list).post(workflows::upsert),
        )
        .route("/api/v1/workflows/{id}", delete(workflows::delete))
        .route("/api/v1/jobs", get(jobs::list))
        .route("/api/v1/jobs/{id}", get(jobs::get).delete(jobs::delete))
        .route("/api/v1/jobs/{id}/retry", post(jobs::retry))
        .route("/api/v1/jobs/{id}/cancel", post(jobs::cancel))
        .route("/api/v1/jobs/{id}/markdown", get(jobs::markdown))
        .route("/api/v1/quick/server", post(quick::server))
        .route(
            "/api/v1/quick/upload",
            post(quick::upload).layer(axum::extract::DefaultBodyLimit::disable()),
        )
        .route("/api/v1/workflows/{id}/scan", put(workflows::scan))
        .fallback(api_not_found)
}

async fn api_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
