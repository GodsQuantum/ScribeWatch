use crate::{
    domain::Workflow,
    error::{AppError, AppResult},
    state::AppState,
    workflows,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

pub async fn list(State(state): State<AppState>) -> Json<Vec<Workflow>> {
    Json(state.workflows.read().await.clone())
}

pub async fn upsert(
    State(state): State<AppState>,
    Json(mut workflow): Json<Workflow>,
) -> AppResult<Json<Workflow>> {
    if workflow.id.trim().is_empty() {
        workflow.id = Uuid::new_v4().to_string();
    }
    let workflow = workflows::normalize_definition(&state, workflow)
        .await
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    state
        .db
        .upsert("workflow", &workflow.id, &workflow)
        .map_err(AppError::Internal)?;
    {
        let mut workflows = state.workflows.write().await;
        if let Some(current) = workflows
            .iter_mut()
            .find(|current| current.id == workflow.id)
        {
            *current = workflow.clone();
        } else {
            workflows.push(workflow.clone());
        }
    }
    workflows::reconcile(&state)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(workflow))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    if state
        .jobs
        .iter()
        .any(|job| job.workflow_id.as_deref() == Some(id.as_str()) && job.status.is_active())
    {
        return Err(AppError::Conflict("workflow has an active job".into()));
    }
    if !state
        .db
        .delete("workflow", &id)
        .map_err(AppError::Internal)?
    {
        return Err(AppError::NotFound("workflow not found".into()));
    }
    state
        .workflows
        .write()
        .await
        .retain(|workflow| workflow.id != id);
    if let Some((_, (_, token))) = state.watcher_tokens.remove(&id) {
        token.cancel();
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn scan(State(state): State<AppState>, Path(id): Path<String>) -> AppResult<StatusCode> {
    workflows::scan_once(&state, &id)
        .await
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    Ok(StatusCode::ACCEPTED)
}
