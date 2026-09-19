use crate::{
    domain::{Job, JobKind, JobStatus, QuickOutputKind},
    error::{AppError, AppResult},
    export as document_export, jobs,
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

pub async fn list(State(state): State<AppState>) -> Json<Vec<Job>> {
    let mut jobs = state
        .jobs
        .iter()
        .map(|entry| entry.value().clone())
        .collect::<Vec<_>>();
    jobs.sort_by_key(|job| std::cmp::Reverse(job.created_at_ms));
    Json(jobs)
}

pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Job>> {
    jobs::get_job(&state, &id)
        .map(Json)
        .map_err(|_| AppError::NotFound("job not found".into()))
}

pub async fn retry(State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Job>> {
    jobs::retry(state, &id)
        .map(Json)
        .map_err(|error| AppError::Conflict(error.to_string()))
}

pub async fn cancel(State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Job>> {
    let current =
        jobs::get_job(&state, &id).map_err(|_| AppError::NotFound("job not found".into()))?;
    if !current.status.is_active() {
        return Err(AppError::Conflict("job is not active".into()));
    }
    jobs::cancel(&state, &id)
        .map(Json)
        .map_err(AppError::Internal)
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let current =
        jobs::get_job(&state, &id).map_err(|_| AppError::NotFound("job not found".into()))?;
    if current.status.is_active() {
        return Err(AppError::Conflict(
            "cancel the active job before deleting it".into(),
        ));
    }
    state.jobs.remove(&id);
    state.db.delete("job", &id).map_err(AppError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

fn rfc5987(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~') {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

pub async fn markdown(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let job = jobs::get_job(&state, &id).map_err(|_| AppError::NotFound("job not found".into()))?;
    let quick = job
        .quick
        .as_ref()
        .ok_or_else(|| AppError::NotFound("job has no client Markdown result".into()))?;
    if job.kind != JobKind::Quick
        || quick.output_kind != QuickOutputKind::Client
        || job.status != JobStatus::Done
        || !job.markdown_published
    {
        return Err(AppError::Conflict(
            "Markdown result is not ready for download".into(),
        ));
    }
    let path = job
        .markdown_path
        .as_ref()
        .ok_or_else(|| AppError::NotFound("Markdown result is missing".into()))?;
    let root = std::fs::canonicalize(state.config.quick_result_dir()).map_err(AppError::from)?;
    let path = std::fs::canonicalize(path)
        .map_err(|_| AppError::NotFound("Markdown result expired or is missing".into()))?;
    if !path.starts_with(&root) || !path.is_file() {
        return Err(AppError::Forbidden("invalid Markdown result path".into()));
    }
    let bytes = tokio::fs::read(&path).await.map_err(AppError::from)?;
    let filename = quick.result_name.as_deref().unwrap_or("transcription.md");
    let disposition = format!(
        "attachment; filename=\"transcription.md\"; filename*=UTF-8''{}",
        rfc5987(filename)
    );
    let mut response = bytes.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/markdown; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid result filename")))?,
    );
    Ok(response)
}

fn downloadable_markdown_path(state: &AppState, job: &Job) -> AppResult<std::path::PathBuf> {
    if job.status != JobStatus::Done || !job.markdown_published {
        return Err(AppError::Conflict(
            "document result is not ready for export".into(),
        ));
    }
    let path = job
        .markdown_path
        .as_ref()
        .ok_or_else(|| AppError::NotFound("Markdown result is missing".into()))?;

    if job.kind == JobKind::Quick
        && job
            .quick
            .as_ref()
            .is_some_and(|quick| quick.output_kind == QuickOutputKind::Client)
    {
        let root =
            std::fs::canonicalize(state.config.quick_result_dir()).map_err(AppError::from)?;
        let path = std::fs::canonicalize(path)
            .map_err(|_| AppError::NotFound("Markdown result expired or is missing".into()))?;
        if !path.starts_with(&root) || !path.is_file() {
            return Err(AppError::Forbidden("invalid Markdown result path".into()));
        }
        return Ok(path);
    }

    state
        .config
        .resolve_allowed_file(path)
        .map_err(AppError::Internal)
}

pub async fn export(
    State(state): State<AppState>,
    Path((id, format)): Path<(String, String)>,
) -> AppResult<Response> {
    let job = jobs::get_job(&state, &id).map_err(|_| AppError::NotFound("job not found".into()))?;
    let source = downloadable_markdown_path(&state, &job)?;
    let format = document_export::ExportFormat::parse(&format)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    let generated =
        document_export::render_export(&source, &state.config.export_dir(), &job.id, format)
            .await
            .map_err(AppError::Internal)?;
    let bytes = tokio::fs::read(&generated).await.map_err(AppError::from)?;

    let base = job
        .quick
        .as_ref()
        .and_then(|quick| quick.result_name.as_deref())
        .and_then(|name| std::path::Path::new(name).file_stem())
        .and_then(|stem| stem.to_str())
        .or_else(|| source.file_stem().and_then(|stem| stem.to_str()))
        .unwrap_or("transcription");
    let filename = format!("{}.{}", base, format.extension());
    let disposition = format!(
        "attachment; filename=\"transcription.{}\"; filename*=UTF-8''{}",
        format.extension(),
        rfc5987(&filename)
    );
    let mut response = bytes.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(format.content_type()),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid export filename")))?,
    );

    if generated != source {
        let _ = tokio::fs::remove_file(generated).await;
    }
    Ok(response)
}
