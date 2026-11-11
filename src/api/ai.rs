use crate::{
    domain::{LlmProvider, LlmProviderView, StructureProfile},
    error::{AppError, AppResult},
    llm, profile_locale,
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Reprocess an existing ASR transcript with the exact production structuring
/// path. No model-generated text, patient identity or request body is persisted.
/// This is desktop-only and follows the same same-origin restriction as the
/// native engine controls. It is useful for QA and for imported ASR transcripts.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureExistingRequest {
    profile_id: String,
    transcript: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureExistingResponse {
    markdown: String,
    elapsed_ms: u64,
    profile_id: String,
    model: String,
}

fn validate_existing_transcript(input: &StructureExistingRequest) -> AppResult<()> {
    let chars = input.transcript.chars().count();
    if !(50..=250_000).contains(&chars) {
        return Err(AppError::BadRequest(
            "Transcription must contain between 50 and 250000 characters".into(),
        ));
    }
    if input.profile_id.trim().is_empty() || input.profile_id.len() > 150 {
        return Err(AppError::BadRequest("Valid AI profile is required".into()));
    }
    Ok(())
}

pub async fn structure_existing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<StructureExistingRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    super::embedded::require_desktop(&state.embedded, Some(&headers))?;
    validate_existing_transcript(&input)?;
    let began = std::time::Instant::now();
    let structured = llm::structure_transcript(
        &state,
        &input.profile_id,
        &input.transcript,
        &tokio_util::sync::CancellationToken::new(),
    )
    .await
    .map_err(|_| AppError::Upstream("Local structuring failed; original ASR unchanged".into()))?;
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(StructureExistingResponse {
            markdown: structured.markdown,
            elapsed_ms: began.elapsed().as_millis().min(u64::MAX as u128) as u64,
            profile_id: structured.profile_id,
            model: structured.model,
        }),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderInput {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub chat_completions_url: String,
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
    180
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureProfileInput {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub prompt: String,
    #[serde(default)]
    pub output_template: String,
    #[serde(default)]
    pub output_example: String,
    #[serde(default)]
    pub provider_id: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsResponse {
    models: Vec<String>,
}

pub async fn list_providers(State(state): State<AppState>) -> Json<Vec<LlmProviderView>> {
    Json(
        state
            .llm_providers
            .read()
            .await
            .iter()
            .map(LlmProviderView::from)
            .collect(),
    )
}

pub async fn upsert_provider(
    State(state): State<AppState>,
    Json(input): Json<LlmProviderInput>,
) -> AppResult<Json<LlmProviderView>> {
    let id = if input.id.trim().is_empty() {
        Uuid::new_v4().to_string()
    } else {
        input.id.trim().to_owned()
    };
    let existing_key = state
        .llm_providers
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
    let provider = LlmProvider {
        id,
        name: input.name.trim().to_owned(),
        chat_completions_url: input.chat_completions_url.trim().to_owned(),
        model: input.model.trim().to_owned(),
        api_key,
        timeout_seconds: input.timeout_seconds,
        enabled: input.enabled,
    };
    llm::validate_provider(&provider).map_err(|error| AppError::BadRequest(error.to_string()))?;
    state
        .db
        .upsert("llm_provider", &provider.id, &provider)
        .map_err(AppError::Internal)?;
    {
        let mut providers = state.llm_providers.write().await;
        if let Some(current) = providers
            .iter_mut()
            .find(|current| current.id == provider.id)
        {
            *current = provider.clone();
        } else {
            providers.push(provider.clone());
        }
    }
    Ok(Json(LlmProviderView::from(&provider)))
}

pub async fn delete_provider(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    if state
        .structure_profiles
        .read()
        .await
        .iter()
        .any(|profile| profile.provider_id == id)
    {
        return Err(AppError::Conflict(
            "LLM provider is still used by a structure profile".into(),
        ));
    }
    if !state
        .db
        .delete("llm_provider", &id)
        .map_err(AppError::Internal)?
    {
        return Err(AppError::NotFound("LLM provider not found".into()));
    }
    state
        .llm_providers
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
        .llm_providers
        .read()
        .await
        .iter()
        .find(|provider| provider.id == id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("LLM provider not found".into()))?;
    let base = provider.chat_completions_url.trim_end_matches('/');
    let prefix = base.strip_suffix("/chat/completions").ok_or_else(|| {
        AppError::BadRequest("model discovery requires a URL ending in /chat/completions".into())
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
    let models = super::local_llm::model_ids(&value);
    Ok(Json(ModelsResponse { models }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileLocaleInput {
    locale: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileLocaleResponse {
    updated_defaults: usize,
    locale: String,
}

/// Synchronize only untouched built-in prompts with the interface language.
/// Customized prompts, provider assignments and model selections survive.
pub async fn sync_profile_locale(
    State(state): State<AppState>,
    Json(input): Json<ProfileLocaleInput>,
) -> AppResult<Json<ProfileLocaleResponse>> {
    if !profile_locale::SUPPORTED.contains(&input.locale.as_str()) {
        return Err(AppError::BadRequest("unsupported interface locale".into()));
    }
    let mut profiles = state.structure_profiles.write().await;
    let mut updated = 0;
    for existing in profiles.iter_mut() {
        if let Some(translation) = profile_locale::replacement_for_locale(existing, &input.locale) {
            state
                .db
                .upsert("structure_profile", &translation.id, &translation)
                .map_err(AppError::Internal)?;
            *existing = translation;
            updated += 1;
        }
    }
    Ok(Json(ProfileLocaleResponse {
        updated_defaults: updated,
        locale: input.locale,
    }))
}

pub async fn list_profiles(State(state): State<AppState>) -> Json<Vec<StructureProfile>> {
    let canonical = llm::builtin_profiles()
        .into_iter()
        .map(|profile| profile.id)
        .collect::<std::collections::HashSet<_>>();
    let mut profiles = state
        .structure_profiles
        .read()
        .await
        .iter()
        .filter(|profile| !profile.built_in || canonical.contains(&profile.id))
        .cloned()
        .collect::<Vec<_>>();
    profiles.sort_by(|left, right| {
        right
            .built_in
            .cmp(&left.built_in)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Json(profiles)
}

pub async fn upsert_profile(
    State(state): State<AppState>,
    Json(input): Json<StructureProfileInput>,
) -> AppResult<Json<StructureProfile>> {
    let id = if input.id.trim().is_empty() {
        Uuid::new_v4().to_string()
    } else {
        input.id.trim().to_owned()
    };
    let existing = state
        .structure_profiles
        .read()
        .await
        .iter()
        .find(|profile| profile.id == id)
        .cloned();
    let profile = StructureProfile {
        id,
        name: input.name.trim().to_owned(),
        description: input.description.trim().to_owned(),
        prompt: input.prompt.trim().to_owned(),
        output_template: input.output_template.trim().to_owned(),
        output_example: input.output_example.trim().to_owned(),
        provider_id: input.provider_id.trim().to_owned(),
        model: input.model.trim().to_owned(),
        built_in: existing.as_ref().is_some_and(|profile| profile.built_in),
    };
    llm::validate_profile(&profile).map_err(|error| AppError::BadRequest(error.to_string()))?;
    if profile.provider_id.is_empty() != profile.model.is_empty() {
        return Err(AppError::BadRequest(
            "profile provider and model must either both be set or both be empty".into(),
        ));
    }
    if !profile.provider_id.is_empty()
        && !state
            .llm_providers
            .read()
            .await
            .iter()
            .any(|provider| provider.id == profile.provider_id)
    {
        return Err(AppError::BadRequest(
            "profile LLM provider does not exist".into(),
        ));
    }

    state
        .db
        .upsert("structure_profile", &profile.id, &profile)
        .map_err(AppError::Internal)?;
    {
        let mut profiles = state.structure_profiles.write().await;
        if let Some(current) = profiles.iter_mut().find(|current| current.id == profile.id) {
            *current = profile.clone();
        } else {
            profiles.push(profile.clone());
        }
    }
    Ok(Json(profile))
}

pub async fn delete_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let profile = state
        .structure_profiles
        .read()
        .await
        .iter()
        .find(|profile| profile.id == id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("structure profile not found".into()))?;
    if profile.built_in {
        return Err(AppError::Conflict(
            "built-in profiles can be edited but not deleted".into(),
        ));
    }
    if state
        .workflows
        .read()
        .await
        .iter()
        .any(|workflow| workflow.structure_profile_id.as_deref() == Some(id.as_str()))
    {
        return Err(AppError::Conflict(
            "structure profile is still used by a workflow".into(),
        ));
    }
    state
        .db
        .delete("structure_profile", &id)
        .map_err(AppError::Internal)?;
    state
        .structure_profiles
        .write()
        .await
        .retain(|profile| profile.id != id);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod imported_transcript_tests {
    use super::*;

    #[test]
    fn text_structuring_rejects_empty_or_oversized_untrusted_payloads() {
        for (profile_id, transcript) in [
            ("", "synthetic text ".repeat(10)),
            ("clinical", "tiny".to_owned()),
            ("clinical", "x".repeat(250_001)),
        ] {
            assert!(
                validate_existing_transcript(&StructureExistingRequest {
                    profile_id: profile_id.into(),
                    transcript,
                })
                .is_err()
            );
        }
        assert!(
            validate_existing_transcript(&StructureExistingRequest {
                profile_id: "synthetic".into(),
                transcript: "this is synthetic example text without personal data ".repeat(4),
            })
            .is_ok()
        );
    }
}
