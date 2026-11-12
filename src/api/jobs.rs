use crate::{
    domain::{Job, JobKind, JobStatus, QuickOutputKind, QuickSourceKind},
    error::{AppError, AppResult},
    export as document_export, jobs, reports,
    state::AppState,
};
use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

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
    // Explicitly deleting a completed history entry also erases its private
    // editable document, preventing inaccessible patient-data orphans.
    state.db.delete("report", &id).map_err(AppError::Internal)?;
    state
        .db
        .delete("report_encounter", &id)
        .map_err(AppError::Internal)?;
    // Drop only the optional grouping link; never cascade to another recording.
    state
        .db
        .delete("job_space", &id)
        .map_err(AppError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Reads local saved report first; pre-0.6.0 completed jobs are imported
/// once without altering their generated original Markdown file.
fn persisted_report(state: &AppState, job: &Job) -> AppResult<reports::SavedReport> {
    if job.status != JobStatus::Done || !job.markdown_published {
        return Err(AppError::Conflict("Report is not yet complete".into()));
    }
    if let Some(saved) = reports::get(&state.db, &job.id).map_err(AppError::Internal)? {
        return Ok(saved);
    }
    let path = downloadable_markdown_path(state, job)?;
    let source = std::fs::read_to_string(path).map_err(AppError::from)?;
    reports::retain_generated(&state.db, &job.id, &source).map_err(AppError::Internal)?;
    reports::get(&state.db, &job.id)
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Saved report is missing")))
}

pub async fn report(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let job = jobs::get_job(&state, &id).map_err(|_| AppError::NotFound("job not found".into()))?;
    let saved = persisted_report(&state, &job)?;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(saved)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditReportRequest {
    markdown: String,
    expected_revision: u64,
}

pub async fn edit_report(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<EditReportRequest>,
) -> AppResult<impl IntoResponse> {
    let job = jobs::get_job(&state, &id).map_err(|_| AppError::NotFound("job not found".into()))?;
    let existing = persisted_report(&state, &job)?;
    let saved = reports::save_edit(
        &state.db,
        &existing,
        input.expected_revision,
        &input.markdown,
    )
    .map_err(|error| AppError::BadRequest(error.to_string()))?
    .ok_or_else(|| {
        AppError::Conflict("Report changed in another tab; reload before saving".into())
    })?;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(saved)))
}

pub async fn report_encounter(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let _job =
        jobs::get_job(&state, &id).map_err(|_| AppError::NotFound("job not found".into()))?;
    let data = reports::get_encounter(&state.db, &id).map_err(AppError::Internal)?;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(data)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditEncounterRequest {
    encounter: reports::ReportEncounter,
    #[serde(default)]
    previous: Option<reports::ReportEncounter>,
}

pub async fn save_report_encounter(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<EditEncounterRequest>,
) -> AppResult<impl IntoResponse> {
    let _job =
        jobs::get_job(&state, &id).map_err(|_| AppError::NotFound("job not found".into()))?;
    if !input.encounter.valid() {
        return Err(AppError::BadRequest("Invalid encounter metadata".into()));
    }
    let existing = reports::get_encounter(&state.db, &id).map_err(AppError::Internal)?;
    match existing {
        Some(current) if current == input.encounter => {}
        Some(current) => {
            if input.previous.as_ref() != Some(&current) {
                return Err(AppError::Conflict(
                    "Encounter details changed in another tab; reload before saving".into(),
                ));
            }
            if !state
                .db
                .compare_and_swap("report_encounter", &id, &current, &input.encounter)
                .map_err(AppError::Internal)?
            {
                return Err(AppError::Conflict(
                    "Concurrent encounter edit; reload".into(),
                ));
            }
        }
        None => {
            if input.previous.is_some() {
                return Err(AppError::Conflict(
                    "Encounter record changed; reload".into(),
                ));
            }
            if !reports::retain_encounter(&state.db, &id, &input.encounter)
                .map_err(AppError::Internal)?
            {
                return Err(AppError::Conflict(
                    "Concurrent encounter creation; reload".into(),
                ));
            }
        }
    }
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(input.encounter)))
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

pub async fn audio(State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Response> {
    let job = jobs::get_job(&state, &id).map_err(|_| AppError::NotFound("job not found".into()))?;
    let quick = job
        .quick
        .as_ref()
        .ok_or_else(|| AppError::Conflict("job has no LIVE audio".into()))?;
    let live = quick
        .live
        .as_ref()
        .ok_or_else(|| AppError::Conflict("job has no LIVE audio".into()))?;
    if job.kind != JobKind::Quick || quick.source_kind != QuickSourceKind::Live {
        return Err(AppError::Conflict("job has no LIVE audio".into()));
    }

    let root = std::fs::canonicalize(state.config.live_source_dir()).map_err(AppError::from)?;
    let path = std::fs::canonicalize(&job.source_path)
        .map_err(|_| AppError::NotFound("LIVE audio expired or is missing".into()))?;
    if !path.starts_with(&root) || !path.is_file() {
        return Err(AppError::Forbidden("invalid LIVE audio path".into()));
    }
    let file = tokio::fs::File::open(&path).await.map_err(AppError::from)?;
    let len = file.metadata().await.map_err(AppError::from)?.len();
    let stream = ReaderStream::new(file);
    let disposition = format!(
        "attachment; filename=\"live.m4a\"; filename*=UTF-8''{}",
        rfc5987(&live.audio_result_name)
    );
    let mut response = Response::new(Body::from_stream(stream));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("audio/mp4"));
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid audio filename")))?,
    );
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&len.to_string())
            .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid audio length")))?,
    );
    Ok(response)
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
    let bytes = persisted_report(&state, &job)?.markdown.into_bytes();
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
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
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
    let saved = persisted_report(&state, &job)?;
    let format = document_export::ExportFormat::parse(&format)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;

    let bytes = if format == document_export::ExportFormat::Markdown {
        saved.markdown.into_bytes()
    } else {
        // One isolated input/output pair per export avoids concurrent requests
        // overwriting each other; the generated files are always removed.
        let staging = state.config.export_dir();
        tokio::fs::create_dir_all(&staging)
            .await
            .map_err(AppError::from)?;
        let unique = Uuid::new_v4().to_string();
        let source = staging.join(format!("{unique}.md"));
        let mut input = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&source)
            .await
            .map_err(AppError::from)?;
        input
            .write_all(saved.markdown.as_bytes())
            .await
            .map_err(AppError::from)?;
        input.flush().await.map_err(AppError::from)?;
        drop(input);
        let generated = document_export::render_export(&source, &staging, &unique, format).await;
        let result = match generated {
            Ok(path) => {
                let content = tokio::fs::read(&path).await.map_err(AppError::from);
                let _ = tokio::fs::remove_file(&path).await;
                content
            }
            Err(error) => Err(AppError::Internal(error)),
        };
        let _ = tokio::fs::remove_file(&source).await;
        result?
    };
    let base = job
        .quick
        .as_ref()
        .and_then(|quick| quick.result_name.as_deref())
        .and_then(|name| std::path::Path::new(name).file_stem())
        .and_then(|stem| stem.to_str())
        .unwrap_or("compte-rendu");
    let revision_suffix = if saved.revision > 0 {
        format!("_v{}", saved.revision)
    } else {
        String::new()
    };
    let filename = format!("{base}{revision_suffix}.{}", format.extension());
    let disposition = format!(
        "attachment; filename=\"report.{}\"; filename*=UTF-8''{}",
        format.extension(),
        rfc5987(&filename)
    );
    let mut response = bytes.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(format.content_type()),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid export filename")))?,
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, state::AppState};
    use axum::{body::to_bytes, http::header};
    use serde_json::json;
    use std::path::Path as FilePath;

    fn test_config(root: &FilePath) -> Config {
        Config {
            host: "127.0.0.1".into(),
            port: 0,
            config_dir: root.join("config"),
            dist_dir: root.join("dist"),
            data_dir: root.join("data"),
            allowed_roots: vec![root.to_path_buf()],
            scan_seconds: 1,
            file_stability_ms: 20,
            max_transcription_jobs: 1,
            max_upload_bytes: 1024 * 1024,
            quick_result_retention_hours: 24,
            quick_source_retention_hours: 24,
            normalized_audio_format: "wav".into(),
            ffmpeg_threads: 1,
        }
    }

    #[tokio::test]
    async fn report_edit_revision_export_restart_and_source_integrity() {
        let tmp = tempfile::tempdir().unwrap();
        let state = AppState::load(test_config(tmp.path())).await.unwrap();
        let id = "technical-test-report";
        let initial = "# Report\n\n## Structured notes\n\nInitial note.\n\n## Transcript\n\nOriginal synthetic audio.";
        let revised = "# Report\n\n## Structured notes\n\nCorrected note.\n\n## Transcript\n\nOriginal synthetic audio.";
        let path = state.config.quick_result_dir().join("original.md");
        tokio::fs::write(&path, initial).await.unwrap();
        let job: Job = serde_json::from_value(json!({
            "id":id, "kind":"quick",
            "quick":{"sourceKind":"upload","outputKind":"client","frontmatter":true,"paragraphs":true},
            "providerId":"local", "originalName":"synthetic.wav",
            "sourcePath":tmp.path().join("synthetic.wav"),
            "sourceSize":4400, "sourceMtimeNs":0, "model":"parakeet-ultra",
            "status":"done", "attempts":1,
            "markdownPath":path, "markdownPublished":true,
            "createdAtMs":100, "updatedAtMs":100
        })).unwrap();
        state.db.upsert("job", id, &job).unwrap();
        state.jobs.insert(id.into(), job.clone());

        let response = report(State(state.clone()), Path(id.into()))
            .await
            .unwrap()
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let initial_saved: reports::SavedReport =
            serde_json::from_slice(&to_bytes(response.into_body(), 1_048_576).await.unwrap())
                .unwrap();
        assert_eq!(initial_saved.markdown, initial);
        assert_eq!(initial_saved.revision, 0);

        let corrected = edit_report(
            State(state.clone()),
            Path(id.into()),
            Json(EditReportRequest {
                markdown: revised.into(),
                expected_revision: 0,
            }),
        )
        .await
        .unwrap()
        .into_response();
        let saved: reports::SavedReport =
            serde_json::from_slice(&to_bytes(corrected.into_body(), 1_048_576).await.unwrap())
                .unwrap();
        assert_eq!(saved.revision, 1);
        assert_eq!(saved.original_markdown, initial);

        let stale = edit_report(
            State(state.clone()),
            Path(id.into()),
            Json(EditReportRequest {
                markdown: revised.into(),
                expected_revision: 0,
            }),
        )
        .await
        .err()
        .expect("conflicting edit must fail");
        assert_eq!(stale.into_response().status(), StatusCode::CONFLICT);
        let corrupted = edit_report(
            State(state.clone()),
            Path(id.into()),
            Json(EditReportRequest {
                markdown: revised.replace("Original synthetic audio.", "Corrupt ASR."),
                expected_revision: 1,
            }),
        )
        .await
        .err()
        .expect("conflicting edit must fail");
        assert_eq!(corrupted.into_response().status(), StatusCode::BAD_REQUEST);

        // Editing must never rewrite the original file.
        assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), initial);
        tokio::fs::remove_file(&path).await.unwrap();
        // Saved report remains available after original quick-result expiry.
        let downloaded = markdown(State(state.clone()), Path(id.into()))
            .await
            .unwrap();
        let bytes = to_bytes(downloaded.into_body(), 1_048_576).await.unwrap();
        assert_eq!(bytes.as_ref(), revised.as_bytes());
        let export_response = export(State(state.clone()), Path((id.into(), "md".into())))
            .await
            .unwrap();
        assert!(
            export_response.headers()[header::CONTENT_DISPOSITION]
                .to_str()
                .unwrap()
                .contains("_v1.md")
        );
        let exported = to_bytes(export_response.into_body(), 1_048_576)
            .await
            .unwrap();
        assert_eq!(exported.as_ref(), revised.as_bytes());

        let restarted = AppState::load(test_config(tmp.path())).await.unwrap();
        let report_after_restart = report(State(restarted.clone()), Path(id.into()))
            .await
            .unwrap()
            .into_response();
        let persisted: reports::SavedReport = serde_json::from_slice(
            &to_bytes(report_after_restart.into_body(), 1_048_576)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(persisted.revision, 1);
        assert_eq!(persisted.original_markdown, initial);
        assert_eq!(persisted.markdown, revised);
        assert_eq!(persisted.history.len(), 1);

        let existing_metadata = reports::ReportEncounter {
            surname: "TECHNIQUE".into(),
            forename: "TEST".into(),
            date_of_birth: String::new(),
            consultation_date: String::new(),
            practitioner: String::new(),
            practice_address: String::new(),
            practice_contact: String::new(),
            session_fields: vec![],
            medical_report_enabled: true,
        };
        save_report_encounter(
            State(restarted.clone()),
            Path(id.into()),
            Json(EditEncounterRequest {
                encounter: existing_metadata.clone(),
                previous: None,
            }),
        )
        .await
        .unwrap();
        let mut changed_metadata = existing_metadata.clone();
        changed_metadata.practitioner = "Dr Demo".into();
        save_report_encounter(
            State(restarted.clone()),
            Path(id.into()),
            Json(EditEncounterRequest {
                encounter: changed_metadata.clone(),
                previous: Some(existing_metadata.clone()),
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            reports::get_encounter(&restarted.db, id).unwrap(),
            Some(changed_metadata)
        );
        let stale_metadata = save_report_encounter(
            State(restarted.clone()),
            Path(id.into()),
            Json(EditEncounterRequest {
                encounter: existing_metadata.clone(),
                previous: Some(existing_metadata),
            }),
        )
        .await
        .err()
        .expect("conflicting edit must fail");
        assert_eq!(
            stale_metadata.into_response().status(),
            StatusCode::CONFLICT
        );
        delete(State(restarted.clone()), Path(id.into()))
            .await
            .unwrap();
        assert!(reports::get(&restarted.db, id).unwrap().is_none());
        assert!(reports::get_encounter(&restarted.db, id).unwrap().is_none());
    }
}
