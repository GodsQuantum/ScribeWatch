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
use serde::Serialize;
use std::{path::PathBuf, time::Instant};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Short dictation is intentionally stateless: no database job, retained recording,
/// second STT engine, or background process. The active provider and model are reused.
const MAX_AUDIO_BYTES: usize = 12 * 1024 * 1024;
const MAX_ROUTES: usize = 4;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationResult {
    text: String,
    provider_id: String,
    model: String,
    elapsed_ms: u64,
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
    let mut bytes: Option<Vec<u8>> = None;
    let mut extension = "webm";
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
            if bytes.is_some() {
                return Err(AppError::BadRequest(
                    "only one audio file is accepted".into(),
                ));
            }
            extension = audio_extension(
                field.content_type().unwrap_or(""),
                field.file_name().unwrap_or(""),
            );
            let mut audio = Vec::new();
            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|_| AppError::BadRequest("invalid audio data".into()))?
            {
                if audio.len().saturating_add(chunk.len()) > MAX_AUDIO_BYTES {
                    return Err(AppError::BadRequest("dictation exceeds 12 MiB".into()));
                }
                audio.extend_from_slice(&chunk);
            }
            bytes = Some(audio);
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
    let audio = bytes.ok_or_else(|| AppError::BadRequest("audio file is required".into()))?;
    if audio.len() < 128 {
        return Err(AppError::BadRequest(
            "audio file is empty or too short".into(),
        ));
    }
    let routes = parse_routes(chain.as_deref(), &provider_id, &model)?;
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
    let path =
        state
            .config
            .quick_upload_dir()
            .join(format!("dictation-{}.{}", Uuid::new_v4(), extension));
    tokio::fs::write(&path, &audio)
        .await
        .map_err(AppError::from)?;
    drop(audio);
    let file = StagedAudio(path);
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
        let stub = Router::new().route(
            "/v1/audio/transcriptions",
            post(|_body: Bytes| async { Json(json!({"text":"bonjour dictée"})) }),
        );
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
            .multipart(form("local-test", vec![1u8; 12 * 1024 * 1024 + 1]))
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
