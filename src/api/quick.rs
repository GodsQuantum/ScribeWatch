use crate::{
    domain::{Job, QuickOutputKind},
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
    let mut language = None;
    let mut output_kind = None;
    let mut output_dir = None;
    let mut frontmatter = None;

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
            "language" => language = Some(value),
            "outputKind" => output_kind = Some(value),
            "outputDir" => output_dir = Some(value),
            "frontmatter" => frontmatter = Some(value),
            _ => {}
        }
    }

    let staged_path =
        staged.ok_or_else(|| AppError::BadRequest("audio file is required".into()))?;
    let original_name = original_name.unwrap_or_else(|| "audio".into());
    let options_result: AppResult<QuickOptions> = (|| {
        Ok(QuickOptions {
            provider_id: provider_id
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| AppError::BadRequest("providerId is required".into()))?,
            model: model.unwrap_or_default(),
            language,
            output_kind: parse_output_kind(output_kind)?,
            output_dir,
            frontmatter: text_bool(frontmatter, true)?,
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
}
