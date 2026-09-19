use crate::{
    domain::{Job, QuickOutputKind, TranscriptionRoute},
    error::{AppError, AppResult},
    jobs,
    quick::{self, QuickOptions},
    state::AppState,
    workflows::is_audio_candidate,
};
use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerQuickRequest {
    pub source_path: String,
    #[serde(flatten)]
    pub options: QuickOptions,
}

pub async fn server(
    State(state): State<AppState>,
    Json(request): Json<ServerQuickRequest>,
) -> AppResult<(StatusCode, Json<Job>)> {
    let job = quick::create_server_job(&state, Path::new(&request.source_path), request.options)
        .await
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    jobs::enqueue(state, job.id.clone()).map_err(AppError::Internal)?;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

fn text_bool(value: Option<String>, default: bool) -> Result<bool, AppError> {
    match value.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        None => Ok(default),
        Some("true" | "1" | "yes" | "on") => Ok(true),
        Some("false" | "0" | "no" | "off") => Ok(false),
        Some(other) => Err(AppError::BadRequest(format!("invalid boolean: {other}"))),
    }
}

fn parse_transcription_chain(value: Option<String>) -> Result<Vec<TranscriptionRoute>, AppError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let routes: Vec<TranscriptionRoute> = serde_json::from_str(value)
        .map_err(|error| AppError::BadRequest(format!("invalid transcriptionChain: {error}")))?;
    if routes
        .iter()
        .any(|route| route.fallback_after_seconds == Some(0))
    {
        return Err(AppError::BadRequest(
            "fallbackAfterSeconds must be greater than zero".into(),
        ));
    }
    Ok(routes)
}

fn parse_output_kind(value: Option<String>) -> Result<QuickOutputKind, AppError> {
    match value.as_deref().map(str::trim) {
        Some("server") => Ok(QuickOutputKind::Server),
        Some("client") => Ok(QuickOutputKind::Client),
        _ => Err(AppError::BadRequest(
            "outputKind must be server or client".into(),
        )),
    }
}

fn remove_staged(path: &Option<PathBuf>) {
    if let Some(path) = path {
        let _ = std::fs::remove_file(path);
    }
}
pub async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<Job>)> {
    let mut staged: Option<PathBuf> = None;
    let mut original_name: Option<String> = None;
    let mut provider_id = None;
    let mut model = None;
    let mut transcription_chain = None;
    let mut language = None;
    let mut output_kind = None;
    let mut output_dir = None;
    let mut frontmatter = None;
    let mut paragraphs = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(error) => {
                remove_staged(&staged);
                return Err(AppError::BadRequest(format!(
                    "invalid multipart upload: {error}"
                )));
            }
        };
        let name = field.name().unwrap_or_default().to_owned();
        if name == "file" {
            if staged.is_some() {
                remove_staged(&staged);
                return Err(AppError::BadRequest(
                    "only one audio file is accepted".into(),
                ));
            }
            let filename = field.file_name().unwrap_or("audio").to_owned();
            if !is_audio_candidate(Path::new(&filename)) {
                return Err(AppError::BadRequest("unsupported audio file".into()));
            }
            let ext = Path::new(&filename)
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("audio")
                .to_ascii_lowercase();
            let path = state
                .config
                .quick_upload_dir()
                .join(format!("{}.{}", Uuid::new_v4(), ext));
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .await
                .map_err(AppError::from)?;
            let mut bytes = 0u64;
            let mut field = field;
            loop {
                match field.chunk().await {
                    Ok(Some(chunk)) => {
                        bytes = bytes.saturating_add(chunk.len() as u64);
                        if bytes > state.config.max_upload_bytes {
                            drop(file);
                            let _ = tokio::fs::remove_file(&path).await;
                            return Err(AppError::BadRequest(
                                "audio upload exceeds configured size limit".into(),
                            ));
                        }
                        if let Err(error) = file.write_all(&chunk).await {
                            drop(file);
                            let _ = tokio::fs::remove_file(&path).await;
                            return Err(AppError::Internal(error.into()));
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        drop(file);
                        let _ = tokio::fs::remove_file(&path).await;
                        return Err(AppError::BadRequest(format!(
                            "audio upload interrupted: {error}"
                        )));
                    }
                }
            }
            if bytes == 0 {
                drop(file);
                let _ = tokio::fs::remove_file(&path).await;
                return Err(AppError::BadRequest("audio file is empty".into()));
            }
            if let Err(error) = file.flush().await {
                drop(file);
                let _ = tokio::fs::remove_file(&path).await;
                return Err(AppError::Internal(error.into()));
            }
            staged = Some(path);
            original_name = Some(filename);
            continue;
        }
        let value = field.text().await.map_err(|error| {
            AppError::BadRequest(format!("invalid multipart field {name}: {error}"))
        })?;
        match name.as_str() {
            "providerId" => provider_id = Some(value),
            "model" => model = Some(value),
            "transcriptionChain" => transcription_chain = Some(value),
            "language" => language = Some(value),
            "outputKind" => output_kind = Some(value),
            "outputDir" => output_dir = Some(value),
            "frontmatter" => frontmatter = Some(value),
            "paragraphs" => paragraphs = Some(value),
            _ => {}
        }
    }

    let staged_path =
        staged.ok_or_else(|| AppError::BadRequest("audio file is required".into()))?;
    let original_name = original_name.unwrap_or_else(|| "audio".into());
    let options_result: AppResult<QuickOptions> = (|| {
        let transcription_chain = parse_transcription_chain(transcription_chain)?;
        let provider_id = provider_id.unwrap_or_default();
        if transcription_chain.is_empty() && provider_id.trim().is_empty() {
            return Err(AppError::BadRequest("providerId is required".into()));
        }
        Ok(QuickOptions {
            provider_id,
            model: model.unwrap_or_default(),
            transcription_chain,
            language,
            output_kind: parse_output_kind(output_kind)?,
            output_dir,
            frontmatter: text_bool(frontmatter, true)?,
            paragraphs: text_bool(paragraphs, true)?,
        })
    })();
    let options = match options_result {
        Ok(options) => options,
        Err(error) => {
            let _ = std::fs::remove_file(&staged_path);
            return Err(error);
        }
    };
    let job = match quick::create_uploaded_job(&state, staged_path.clone(), original_name, options)
        .await
    {
        Ok(job) => job,
        Err(error) => {
            let _ = std::fs::remove_file(&staged_path);
            return Err(AppError::BadRequest(error.to_string()));
        }
    };
    if let Err(error) = jobs::enqueue(state, job.id.clone()) {
        let _ = std::fs::remove_file(&staged_path);
        return Err(AppError::Internal(error));
    }
    Ok((StatusCode::ACCEPTED, Json(job)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_kind_parser_is_strict() {
        assert_eq!(
            parse_output_kind(Some("client".into())).unwrap(),
            QuickOutputKind::Client
        );
        assert_eq!(
            parse_output_kind(Some("server".into())).unwrap(),
            QuickOutputKind::Server
        );
        assert!(parse_output_kind(Some("browser".into())).is_err());
        assert!(parse_output_kind(None).is_err());
    }

    #[test]
    fn boolean_parser_accepts_form_values() {
        assert!(text_bool(None, true).unwrap());
        assert!(!text_bool(Some("false".into()), true).unwrap());
        assert!(text_bool(Some("1".into()), false).unwrap());
        assert!(text_bool(Some("sometimes".into()), true).is_err());
    }

    #[test]
    fn transcription_chain_parser_accepts_ordered_routes() {
        let chain = parse_transcription_chain(Some(
            r#"[
              {"providerId":"p","model":"large-v3"},
              {"providerId":"p","model":"distil-large-v3","fallbackAfterSeconds":1800}
            ]"#
            .into(),
        ))
        .unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].provider_id, "p");
        assert_eq!(chain[0].model, "large-v3");
        assert_eq!(chain[0].fallback_after_seconds, None);
        assert_eq!(chain[1].model, "distil-large-v3");
        assert_eq!(chain[1].fallback_after_seconds, Some(1800));
    }

    #[test]
    fn transcription_chain_parser_rejects_bad_json_and_zero_timeout() {
        assert!(parse_transcription_chain(Some("not-json".into())).is_err());
        assert!(
            parse_transcription_chain(Some(
                r#"[{"providerId":"p","model":"m","fallbackAfterSeconds":0}]"#.into()
            ))
            .is_err()
        );
        assert!(parse_transcription_chain(None).unwrap().is_empty());
    }
}
