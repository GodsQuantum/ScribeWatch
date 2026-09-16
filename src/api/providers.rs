use crate::{
    domain::{Provider, ProviderView},
    error::{AppError, AppResult},
    provider,
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub transcription_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub clear_api_key: bool,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}
fn default_timeout() -> u64 {
    600
}
fn default_true() -> bool {
    true
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsResponse {
    models: Vec<String>,
}

pub async fn list(State(state): State<AppState>) -> Json<Vec<ProviderView>> {
    Json(
        state
            .providers
            .read()
            .await
            .iter()
            .map(ProviderView::from)
            .collect(),
    )
}

pub async fn upsert(
    State(state): State<AppState>,
    Json(input): Json<ProviderInput>,
) -> AppResult<Json<ProviderView>> {
    let id = if input.id.trim().is_empty() {
        Uuid::new_v4().to_string()
    } else {
        input.id.clone()
    };
    let existing_key = state
        .providers
        .read()
        .await
        .iter()
        .find(|provider| provider.id == id)
        .map(|provider| provider.api_key.clone())
        .unwrap_or_default();
    let api_key = if input.clear_api_key {
        String::new()
    } else {
        input
            .api_key
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(existing_key)
    };
    let provider = Provider {
        id,
        name: input.name.trim().to_owned(),
        transcription_url: input.transcription_url.trim().to_owned(),
        model: input.model.trim().to_owned(),
        api_key,
        timeout_seconds: input.timeout_seconds,
        enabled: input.enabled,
    };
    provider::validate_provider(&provider)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    state
        .db
        .upsert("provider", &provider.id, &provider)
        .map_err(AppError::Internal)?;
    {
        let mut providers = state.providers.write().await;
        if let Some(current) = providers
            .iter_mut()
            .find(|current| current.id == provider.id)
        {
            *current = provider.clone();
        } else {
            providers.push(provider.clone());
        }
    }
    Ok(Json(ProviderView::from(&provider)))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    if state.workflows.read().await.iter().any(|workflow| {
        workflow.provider_id == id
            || workflow
                .transcription_chain
                .iter()
                .any(|route| route.provider_id == id)
    }) {
        return Err(AppError::Conflict(
            "provider is still used by a workflow".into(),
        ));
    }
    if !state
        .db
        .delete("provider", &id)
        .map_err(AppError::Internal)?
    {
        return Err(AppError::NotFound("provider not found".into()));
    }
    state
        .providers
        .write()
        .await
        .retain(|provider| provider.id != id);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn models(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<ModelsResponse>> {
    let provider = state
        .providers
        .read()
        .await
        .iter()
        .find(|provider| provider.id == id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("provider not found".into()))?;
    let base = provider.transcription_url.trim_end_matches('/');
    let prefix = base.strip_suffix("/audio/transcriptions").ok_or_else(|| {
        AppError::BadRequest(
            "model discovery requires a transcription URL ending in /audio/transcriptions".into(),
        )
    })?;
    let url = format!("{prefix}/models");
    let mut request = state.http.get(url).timeout(std::time::Duration::from_secs(
        provider.timeout_seconds.max(1),
    ));
    if !provider.api_key.trim().is_empty() {
        request = request.bearer_auth(&provider.api_key);
    }
    let response = request
        .send()
        .await
        .map_err(|error| AppError::Upstream(error.to_string()))?;
    let status = response.status();
    let value: Value = response
        .json()
        .await
        .map_err(|error| AppError::Upstream(format!("invalid model response: {error}")))?;
    if !status.is_success() {
        return Err(AppError::Upstream(format!("HTTP {status}: {value}")));
    }
    let mut models = value
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();
    Ok(Json(ModelsResponse { models }))
}
