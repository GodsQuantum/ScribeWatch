pub mod ai;
pub mod browse;
pub mod dashboard;
pub mod embedded;
pub mod events;
pub mod jobs;
pub mod live;
pub mod local_llm;
pub mod providers;
pub mod quick;
pub mod spaces;
pub mod speaches;
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
        .route("/api/v1/speaches/probe", post(speaches::probe))
        .route("/api/v1/ready", get(ready))
        .route("/api/v1/dashboard", get(dashboard::stats))
        .route("/api/v1/events", get(events::events))
        .route("/api/v1/browse", get(browse::browse))
        .route(
            "/api/v1/providers",
            get(providers::list).post(providers::upsert),
        )
        .route(
            "/api/v1/llm-providers",
            get(ai::list_providers).post(ai::upsert_provider),
        )
        .route("/api/v1/llm-providers/{id}", delete(ai::delete_provider))
        .route("/api/v1/llm-providers/{id}/models", get(ai::models))
        .route("/api/v1/embedded/status", get(embedded::status))
        .route("/api/v1/structure/existing", post(ai::structure_existing))
        .route(
            "/api/v1/embedded/llm/models-directory/open",
            post(embedded::open_llm_models_directory),
        )
        .route("/api/v1/embedded/mode", post(embedded::set_mode))
        .route("/api/v1/embedded/models/{kind}", get(embedded::models))
        .route("/api/v1/embedded/download", post(embedded::download))
        .route(
            "/api/v1/embedded/stt/variants",
            post(embedded::asr_variants),
        )
        .route("/api/v1/embedded/stt/install", post(embedded::asr_install))
        .route("/api/v1/llm-local", get(local_llm::engines))
        .route("/api/v1/llm-local/discover", post(local_llm::discover))
        .route("/api/v1/llm-local/pull", post(local_llm::pull))
        .route("/api/v1/llm-local/downloads", get(local_llm::downloads))
        .route(
            "/api/v1/llm-local/install-engine",
            post(local_llm::install_engine),
        )
        .route(
            "/api/v1/structure-profiles",
            get(ai::list_profiles).post(ai::upsert_profile),
        )
        .route(
            "/api/v1/structure-profiles/sync-locale",
            post(ai::sync_profile_locale),
        )
        .route(
            "/api/v1/structure-profiles/{id}",
            delete(ai::delete_profile),
        )
        .route("/api/v1/providers/{id}", delete(providers::delete))
        .route("/api/v1/providers/{id}/models", get(providers::models))
        .route(
            "/api/v1/workflows",
            get(workflows::list).post(workflows::upsert),
        )
        .route("/api/v1/workflows/{id}", delete(workflows::delete))
        .route("/api/v1/spaces", get(spaces::list).post(spaces::create))
        .route("/api/v1/spaces/assignments", get(spaces::assignments))
        .route("/api/v1/spaces/{id}", delete(spaces::delete))
        .route("/api/v1/jobs/{id}/space", put(spaces::assign))
        .route("/api/v1/jobs", get(jobs::list))
        .route("/api/v1/jobs/{id}", get(jobs::get).delete(jobs::delete))
        .route("/api/v1/jobs/{id}/retry", post(jobs::retry))
        .route("/api/v1/jobs/{id}/cancel", post(jobs::cancel))
        .route(
            "/api/v1/jobs/{id}/report",
            get(jobs::report).put(jobs::edit_report),
        )
        .route(
            "/api/v1/jobs/{id}/report/encounter",
            get(jobs::report_encounter).put(jobs::save_report_encounter),
        )
        .route("/api/v1/jobs/{id}/markdown", get(jobs::markdown))
        .route("/api/v1/jobs/{id}/export/{format}", get(jobs::export))
        .route("/api/v1/jobs/{id}/audio", get(jobs::audio))
        .route(
            "/api/v1/live/upload",
            post(live::upload).layer(axum::extract::DefaultBodyLimit::disable()),
        )
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
