use crate::{
    domain::TranscriptionRoute,
    error::{AppError, AppResult},
    provider,
    state::AppState,
};
use axum::{
    Json,
    extract::{Multipart, State},
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Instant};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Dictation is stateless: no retained recording, no separate ASR process.
/// Audio streams straight to a temporary file; the existing configurable
/// ScribeWatch upload quota guards disk exhaustion without a duration cap.
const MAX_ROUTES: usize = 4;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationResult {
    text: String,
    provider_id: String,
    model: String,
    elapsed_ms: u64,
}

const SETTINGS_KIND: &str = "dictation_settings";
const SETTINGS_ID: &str = "desktop";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationSettings {
    pub transcription_chain: Vec<TranscriptionRoute>,
    pub language: String,
    #[serde(default)]
    pub structure_profile_id: Option<String>,
}

async fn effective_settings(state: &AppState) -> AppResult<DictationSettings> {
    if let Some(saved) = state
        .db
        .get::<DictationSettings>(SETTINGS_KIND, SETTINGS_ID)?
    {
        return Ok(saved);
    }
    let providers = state.providers.read().await;
    let first = providers
        .iter()
        .filter(|p| p.enabled)
        .find(|p| p.model.to_ascii_lowercase().contains("parakeet"))
        .or_else(|| providers.iter().find(|p| p.enabled));
    Ok(DictationSettings {
        transcription_chain: first
            .map(|p| {
                vec![TranscriptionRoute {
                    provider_id: p.id.clone(),
                    model: p.model.clone(),
                    fallback_after_seconds: None,
                }]
            })
            .unwrap_or_default(),
        language: "fr".into(),
        structure_profile_id: None,
    })
}

pub async fn settings(State(state): State<AppState>) -> AppResult<Json<DictationSettings>> {
    Ok(Json(effective_settings(&state).await?))
}

pub async fn save_settings(
    State(state): State<AppState>,
    Json(settings): Json<DictationSettings>,
) -> AppResult<Json<DictationSettings>> {
    if settings.transcription_chain.is_empty() || settings.transcription_chain.len() > MAX_ROUTES {
        return Err(AppError::BadRequest(
            "select a valid transcription chain".into(),
        ));
    }
    let providers = state.providers.read().await;
    for route in &settings.transcription_chain {
        if route.provider_id.trim().is_empty()
            || route.fallback_after_seconds == Some(0)
            || !providers
                .iter()
                .any(|p| p.enabled && p.id == route.provider_id)
        {
            return Err(AppError::BadRequest(
                "transcription provider unavailable".into(),
            ));
        }
    }
    drop(providers);
    if let Some(id) = settings
        .structure_profile_id
        .as_ref()
        .filter(|v| !v.is_empty())
        && !state
            .structure_profiles
            .read()
            .await
            .iter()
            .any(|p| p.id == *id && !p.provider_id.is_empty())
    {
        return Err(AppError::BadRequest("AI profile unavailable".into()));
    }
    if settings.language.len() > 20
        || settings
            .language
            .chars()
            .any(|c| !c.is_ascii_alphabetic() && c != '-')
    {
        return Err(AppError::BadRequest(
            "invalid transcription language".into(),
        ));
    }
    state.db.upsert(SETTINGS_KIND, SETTINGS_ID, &settings)?;
    Ok(Json(settings))
}

struct StagedAudio(PathBuf);

impl Drop for StagedAudio {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn audio_extension(mime: &str, filename: &str) -> &'static str {
    let mime = mime.to_ascii_lowercase();
    if mime.contains("webm") {
        return "webm";
    }
    if mime.contains("mp4") || mime.contains("m4a") {
        return "m4a";
    }
    if mime.contains("ogg") {
        return "ogg";
    }
    if mime.contains("wav") {
        return "wav";
    }
    if mime.contains("flac") {
        return "flac";
    }
    match filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "webm" => "webm",
        "m4a" | "mp4" => "m4a",
        "ogg" | "opus" => "ogg",
        "wav" => "wav",
        "flac" => "flac",
        _ => "webm",
    }
}

fn parse_routes(
    json: Option<&str>,
    provider_id: &str,
    model: &str,
) -> Result<Vec<TranscriptionRoute>, AppError> {
    let routes: Vec<TranscriptionRoute> = match json.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) if raw.len() <= 4096 => serde_json::from_str(raw)
            .map_err(|_| AppError::BadRequest("invalid transcriptionChain".into()))?,
        Some(_) => return Err(AppError::BadRequest("transcriptionChain too large".into())),
        None if !provider_id.trim().is_empty() => vec![TranscriptionRoute {
            provider_id: provider_id.trim().to_owned(),
            model: model.trim().to_owned(),
            fallback_after_seconds: None,
        }],
        None => Vec::new(),
    };
    if routes.is_empty()
        || routes.len() > MAX_ROUTES
        || routes
            .iter()
            .any(|r| r.provider_id.trim().is_empty() || r.fallback_after_seconds == Some(0))
    {
        return Err(AppError::BadRequest(
            "select 1 to 4 valid transcription routes".into(),
        ));
    }
    Ok(routes)
}

/// Multipart: file, transcriptionChain (optional), providerId, model, language.
/// Audio and transcript exist only for this request; nothing is persisted to SQLite.
pub async fn transcribe(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<DictationResult>> {
    let mut staged: Option<StagedAudio> = None;
    let mut bytes_received = 0u64;
    let mut provider_id = String::new();
    let mut model = String::new();
    let mut language = String::new();
    let mut chain = None;
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("invalid dictation upload".into()))?
    {
        let name = field.name().unwrap_or_default().to_owned();
        if name == "file" {
            if staged.is_some() {
                return Err(AppError::BadRequest(
                    "only one audio file is accepted".into(),
                ));
            }
            let extension = audio_extension(
                field.content_type().unwrap_or(""),
                field.file_name().unwrap_or(""),
            );
            let path = state.config.quick_upload_dir().join(format!(
                "dictation-{}.{}",
                Uuid::new_v4(),
                extension
            ));
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .await
                .map_err(AppError::from)?;
            staged = Some(StagedAudio(path));
            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|_| AppError::BadRequest("invalid audio data".into()))?
            {
                bytes_received = bytes_received.saturating_add(chunk.len() as u64);
                if bytes_received > state.config.max_upload_bytes {
                    return Err(AppError::BadRequest(
                        "dictation exceeds the configured ScribeWatch upload quota".into(),
                    ));
                }
                file.write_all(&chunk).await.map_err(AppError::from)?;
            }
            file.flush().await.map_err(AppError::from)?;
            drop(file);
        } else if matches!(
            name.as_str(),
            "providerId" | "model" | "language" | "transcriptionChain"
        ) {
            let text = field
                .text()
                .await
                .map_err(|_| AppError::BadRequest("invalid dictation option".into()))?;
            if text.len() > 4096 {
                return Err(AppError::BadRequest("dictation option too large".into()));
            }
            match name.as_str() {
                "providerId" => provider_id = text,
                "model" => model = text,
                "language" => language = text,
                "transcriptionChain" => chain = Some(text),
                _ => {}
            }
        }
    }
    let file = staged.ok_or_else(|| AppError::BadRequest("audio file is required".into()))?;
    if bytes_received < 128 {
        return Err(AppError::BadRequest(
            "audio file is empty or too short".into(),
        ));
    }
    // Native Rust microphone submits only the audio: load all preferences
    // (model selection, fallback chain AND language) from the same saved
    // configuration as the browser UI. In particular, an empty language field
    // breaks strict forced alignment models such as Parakeet Ultra.
    let needs_routes =
        chain.as_deref().is_none_or(|v| v.trim().is_empty()) && provider_id.trim().is_empty();
    let saved = if needs_routes || language.trim().is_empty() {
        Some(effective_settings(&state).await?)
    } else {
        None
    };
    let routes = if needs_routes {
        saved
            .as_ref()
            .map(|s| s.transcription_chain.clone())
            .unwrap_or_default()
    } else {
        parse_routes(chain.as_deref(), &provider_id, &model)?
    };
    if language.trim().is_empty() {
        language = saved
            .as_ref()
            .map(|s| s.language.clone())
            .unwrap_or_else(|| "fr".into());
    }
    if routes.is_empty() {
        return Err(AppError::BadRequest(
            "no transcription provider is configured".into(),
        ));
    }
    if language.len() > 20
        || language
            .chars()
            .any(|c| !c.is_ascii_alphabetic() && c != '-')
    {
        return Err(AppError::BadRequest(
            "invalid transcription language".into(),
        ));
    }
    let available = state.providers.read().await.clone();
    // Reject an invalid/unavailable selection rather than silently routing elsewhere.
    for route in &routes {
        if !available
            .iter()
            .any(|p| p.id == route.provider_id && p.enabled)
        {
            return Err(AppError::BadRequest(
                "selected transcription provider is unavailable".into(),
            ));
        }
    }
    let _slot = state
        .transcription_slots
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| AppError::Internal(anyhow::anyhow!("transcription scheduler unavailable")))?;
    let started = Instant::now();
    for route in routes {
        let Some(mut current) = available
            .iter()
            .find(|p| p.id == route.provider_id)
            .cloned()
        else {
            continue;
        };
        if let Some(seconds) = route.fallback_after_seconds {
            current.timeout_seconds = current.timeout_seconds.min(seconds);
        }
        let selected_model = if route.model.trim().is_empty() {
            current.model.clone()
        } else {
            route.model.trim().to_owned()
        };
        let token = CancellationToken::new();
        match provider::transcribe(
            &current,
            &selected_model,
            Some(&language),
            &file.0,
            &state.http,
            &token,
        )
        .await
        {
            Ok(text) => {
                return Ok(Json(DictationResult {
                    text,
                    provider_id: current.id,
                    model: selected_model,
                    elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
                }));
            }
            Err(_error) => tracing::warn!(
                provider_id = %current.id,
                "dictation provider failed; trying next configured route"
            ),
        }
    }
    Err(AppError::Upstream(
        "all selected transcription routes failed".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_upload_format_without_trusting_filename_extensions() {
        assert_eq!(audio_extension("audio/webm;codecs=opus", "foo.wav"), "webm");
        assert_eq!(audio_extension("audio/mp4", "foo.webm"), "m4a");
        assert_eq!(audio_extension("", "test.FLAC"), "flac");
        assert_eq!(audio_extension("", "../../untrusted.exe"), "webm");
    }
    #[test]
    fn checks_provider_selection_and_bounded_chain() {
        assert!(parse_routes(None, "", "").is_err());
        assert!(parse_routes(Some("[]"), "", "").is_err());
        let one = parse_routes(None, "local", "").unwrap();
        assert_eq!(one[0].provider_id, "local");
        assert!(
            parse_routes(
                Some("[{\"providerId\":\"x\",\"model\":\"\",\"fallbackAfterSeconds\":0}]"),
                "",
                ""
            )
            .is_err()
        );
        assert!(parse_routes(Some("bad json"), "", "").is_err());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{config::Config, domain::Provider};
    use axum::{Router, body::Bytes, routing::post};
    use reqwest::multipart::{Form, Part};
    use serde_json::{Value, json};

    fn tiny_wav() -> Vec<u8> {
        let mut bytes = vec![0u8; 44 + 16000];
        bytes[..4].copy_from_slice(b"RIFF");
        bytes[4..8].copy_from_slice(&(16036u32).to_le_bytes());
        bytes[8..12].copy_from_slice(b"WAVE");
        bytes[12..16].copy_from_slice(b"fmt ");
        bytes[16..20].copy_from_slice(&16u32.to_le_bytes());
        bytes[20..22].copy_from_slice(&1u16.to_le_bytes());
        bytes[22..24].copy_from_slice(&1u16.to_le_bytes());
        bytes[24..28].copy_from_slice(&16000u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&32000u32.to_le_bytes());
        bytes[32..34].copy_from_slice(&2u16.to_le_bytes());
        bytes[34..36].copy_from_slice(&16u16.to_le_bytes());
        bytes[36..40].copy_from_slice(b"data");
        bytes[40..44].copy_from_slice(&16000u32.to_le_bytes());
        bytes
    }

    async fn setup() -> (tempfile::TempDir, AppState, String) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let allowed = root.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        let config = Config {
            host: "127.0.0.1".into(),
            port: 0,
            config_dir: root.join("config"),
            dist_dir: root.join("dist"),
            data_dir: root.join("data"),
            allowed_roots: vec![allowed],
            scan_seconds: 1,
            file_stability_ms: 20,
            max_transcription_jobs: 1,
            max_upload_bytes: 16 * 1024 * 1024,
            quick_result_retention_hours: 24,
            quick_source_retention_hours: 24,
            normalized_audio_format: "wav".into(),
            ffmpeg_threads: 1,
        };
        let state = AppState::load(config).await.unwrap();
        let stub = Router::new()
            .route(
                "/v1/audio/transcriptions",
                post(|_body: Bytes| async { Json(json!({"text":"bonjour dictée"})) }),
            )
            .layer(axum::extract::DefaultBodyLimit::disable());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, stub).await.unwrap() });
        let provider = Provider {
            id: "local-test".into(),
            name: "Local test".into(),
            transcription_url: format!("http://{upstream}/v1/audio/transcriptions"),
            model: "fake-asr".into(),
            api_key: String::new(),
            timeout_seconds: 5,
            enabled: true,
        };
        state
            .db
            .upsert("provider", &provider.id, &provider)
            .unwrap();
        state.providers.write().await.push(provider);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = crate::api::router().with_state(state.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (temp, state, format!("http://{addr}"))
    }

    fn form(provider: &str, file: Vec<u8>) -> Form {
        Form::new()
            .text("providerId", provider.to_string())
            .text("language", "fr")
            .part(
                "file",
                Part::bytes(file)
                    .file_name("test.wav")
                    .mime_str("audio/wav")
                    .unwrap(),
            )
    }

    #[tokio::test]
    async fn direct_dictation_returns_text_with_no_persistent_job_or_audio() {
        let (_tmp, state, base) = setup().await;
        let before = state.db.list::<crate::domain::Job>("job").unwrap().len();
        let response = reqwest::Client::new()
            .post(format!("{base}/api/v1/dictation/transcribe"))
            .multipart(form("local-test", tiny_wav()))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let result: Value = response.json().await.unwrap();
        assert_eq!(result["text"], "bonjour dictée");
        assert_eq!(result["providerId"], "local-test");
        assert_eq!(result["model"], "fake-asr");
        let after = state.db.list::<crate::domain::Job>("job").unwrap().len();
        assert_eq!(before, after, "dictation must not create a stored job");
        assert!(
            std::fs::read_dir(state.config.quick_upload_dir())
                .unwrap()
                .filter_map(Result::ok)
                .all(|entry| !entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("dictation-")),
            "dictation audio must be removed after completion"
        );
    }

    #[tokio::test]
    async fn provider_failure_falls_back_to_the_next_selected_endpoint() {
        let (_tmp, state, base) = setup().await;
        let fail = Router::new().route(
            "/v1/audio/transcriptions",
            post(|| async {
                (
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({"error":"test"})),
                )
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, fail).await.unwrap() });
        let provider = Provider {
            id: "failing-test".into(),
            name: "Failing test".into(),
            transcription_url: format!("http://{upstream}/v1/audio/transcriptions"),
            model: "missing".into(),
            api_key: String::new(),
            timeout_seconds: 3,
            enabled: true,
        };
        state.providers.write().await.push(provider);
        let chain = json!([
            {"providerId":"failing-test","model":"missing"},
            {"providerId":"local-test","model":"fake-asr"}
        ]);
        let response = reqwest::Client::new()
            .post(format!("{base}/api/v1/dictation/transcribe"))
            .multipart(
                Form::new()
                    .text("transcriptionChain", chain.to_string())
                    .part(
                        "file",
                        Part::bytes(tiny_wav())
                            .file_name("test.wav")
                            .mime_str("audio/wav")
                            .unwrap(),
                    ),
            )
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let result: Value = response.json().await.unwrap();
        assert_eq!(result["providerId"], "local-test");
        assert_eq!(result["text"], "bonjour dictée");
        assert!(
            state
                .db
                .list::<crate::domain::Job>("job")
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn native_file_only_request_uses_persisted_language_and_model() {
        let (_tmp, state, base) = setup().await;
        let upstream = Router::new().route(
            "/v1/audio/transcriptions",
            post(|body: Bytes| async move {
                let raw = String::from_utf8_lossy(&body);
                if raw.contains(r#"name="language""#)
                    && raw.contains("\r\n\r\nfr\r\n")
                    && raw.contains(r#"name="model""#)
                    && raw.contains("\r\n\r\nstrict-asr\r\n")
                {
                    (
                        axum::http::StatusCode::OK,
                        Json(json!({"text": "français vérifié"})),
                    )
                } else {
                    (
                        axum::http::StatusCode::BAD_REQUEST,
                        Json(json!({"error": "language or model missing"})),
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });
        let strict = Provider {
            id: "strict-asr".into(),
            name: "Forced-aligner".into(),
            transcription_url: format!("http://{addr}/v1/audio/transcriptions"),
            model: "strict-asr".into(),
            api_key: String::new(),
            timeout_seconds: 10,
            enabled: true,
        };
        state.providers.write().await.push(strict);
        let client = reqwest::Client::new();
        let settings = json!({
            "transcriptionChain": [{"providerId": "strict-asr", "model": "strict-asr"}],
            "language": "fr", "structureProfileId": null,
        });
        let saved = client
            .put(format!("{base}/api/v1/dictation/settings"))
            .json(&settings)
            .send()
            .await
            .unwrap();
        assert_eq!(saved.status(), reqwest::StatusCode::OK);
        let retrieved: Value = client
            .get(format!("{base}/api/v1/dictation/settings"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(retrieved, settings);
        let answer = client
            .post(format!("{base}/api/v1/dictation/transcribe"))
            .multipart(
                Form::new().part(
                    "file",
                    Part::bytes(tiny_wav())
                        .file_name("micro.wav")
                        .mime_str("audio/wav")
                        .unwrap(),
                ),
            )
            .send()
            .await
            .unwrap();
        assert_eq!(answer.status(), reqwest::StatusCode::OK);
        let result: Value = answer.json().await.unwrap();
        assert_eq!(result["text"], "français vérifié");
        assert_eq!(result["providerId"], "strict-asr");
    }

    #[tokio::test]
    async fn streamed_upload_over_the_old_twelve_mib_limit_succeeds() {
        let (_tmp, state, base) = setup().await;
        let large = vec![0u8; 13 * 1024 * 1024];
        let result = reqwest::Client::new()
            .post(format!("{base}/api/v1/dictation/transcribe"))
            .multipart(form("local-test", large))
            .send()
            .await
            .unwrap();
        assert_eq!(result.status(), reqwest::StatusCode::OK);
        assert!(
            state
                .db
                .list::<crate::domain::Job>("job")
                .unwrap()
                .is_empty()
        );
        assert!(
            std::fs::read_dir(state.config.quick_upload_dir())
                .unwrap()
                .filter_map(Result::ok)
                .all(|e| !e.file_name().to_string_lossy().starts_with("dictation-"))
        );
    }

    #[tokio::test]
    async fn invalid_provider_and_oversized_body_fail_without_jobs() {
        let (_tmp, state, base) = setup().await;
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{base}/api/v1/dictation/transcribe"))
            .multipart(form("missing", tiny_wav()))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
        let response = client
            .post(format!("{base}/api/v1/dictation/transcribe"))
            .multipart(form("local-test", vec![1u8; 16 * 1024 * 1024 + 1]))
            .send()
            .await
            .unwrap();
        assert!(matches!(response.status().as_u16(), 400 | 413));
        assert!(
            state
                .db
                .list::<crate::domain::Job>("job")
                .unwrap()
                .is_empty()
        );
    }
}
