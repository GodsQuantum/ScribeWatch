use crate::{
    audio,
    domain::{Job, JobStatus, QuickJobMeta, QuickOutputKind, QuickSourceKind},
    jobs::update_job,
    llm,
    markdown::{self, NoteContext},
    state::AppState,
    transcription_chain,
};
use anyhow::{Context, Result, anyhow, bail};
use std::path::Path;
use time::OffsetDateTime;
use tokio_util::sync::CancellationToken;

pub(crate) async fn process_quick_job(
    state: &AppState,
    job: &Job,
    token: &CancellationToken,
) -> Result<()> {
    let meta = job
        .quick
        .clone()
        .ok_or_else(|| anyhow!("quick job metadata is missing"))?;
    let source = match meta.source_kind {
        QuickSourceKind::Server => state.config.resolve_allowed_file(&job.source_path)?,
        QuickSourceKind::Upload => {
            let root = std::fs::canonicalize(state.config.quick_upload_dir())?;
            let path = std::fs::canonicalize(&job.source_path)?;
            if !path.starts_with(&root) || !path.is_file() {
                bail!("quick upload source is unavailable");
            }
            path
        }
    };
    let transient = meta.source_kind == QuickSourceKind::Upload;
    let result = process_quick_inner(state, job, &meta, &source, token).await;
    if transient {
        let _ = std::fs::remove_file(&source);
    }
    result
}

async fn process_quick_inner(
    state: &AppState,
    job: &Job,
    meta: &QuickJobMeta,
    source: &Path,
    token: &CancellationToken,
) -> Result<()> {
    if token.is_cancelled() {
        bail!("cancelled");
    }
    let normalized =
        audio::normalize_for_transcription(source, &state.config.normalized_dir(), &job.id, token)
            .await
            .context("audio normalization")?;
    let transcription = transcription_chain::transcribe_job_chain(
        state,
        &job.id,
        &normalized,
        job.language.as_deref(),
        token,
    )
    .await;
    audio::cleanup_normalized(&normalized);
    let success = transcription.context("transcription")?;
    let transcript = success.transcript;
    if token.is_cancelled() {
        bail!("cancelled");
    }
    let structured = if let Some(profile_id) = meta.structure_profile_id.as_deref() {
        update_job(state, &job.id, |job| {
            job.status = JobStatus::Structuring;
            job.structuring_error = None;
        })?;
        match llm::structure_transcript(state, profile_id, &transcript, token).await {
            Ok(result) => {
                let markdown = result.markdown;
                update_job(state, &job.id, |job| {
                    job.structured_profile_id = Some(result.profile_id);
                    job.structured_profile_name = Some(result.profile_name);
                    job.structured_model = Some(result.model);
                    job.structuring_error = None;
                })?;
                Some(markdown)
            }
            Err(error) if token.is_cancelled() => return Err(error).context("structuring"),
            Err(error) => {
                update_job(state, &job.id, |job| {
                    job.structuring_error = Some(error.to_string());
                })?;
                tracing::warn!(job_id = %job.id, %error, "LLM structuring failed; publishing canonical transcript");
                None
            }
        }
    } else {
        None
    };
    update_job(state, &job.id, |job| job.status = JobStatus::Publishing)?;
    let title = markdown::derive_title(&transcript, &job.original_name);
    let context = NoteContext {
        title: &title,
        source_name: &job.original_name,
        workflow_name: None,
        provider_name: &success.provider_name,
        model: &success.model,
        language: job.language.as_deref(),
        tags: &[],
        frontmatter: meta.frontmatter,
        paragraphs: meta.paragraphs,
        created_at: OffsetDateTime::now_utc(),
    };
    let document =
        markdown::render_note_with_structure(&context, &transcript, structured.as_deref())?;
    let (markdown_path, result_name) = match meta.output_kind {
        QuickOutputKind::Server => {
            let raw = meta
                .output_dir
                .as_deref()
                .ok_or_else(|| anyhow!("server quick output has no output directory"))?;
            let output = state.config.resolve_allowed_dir(Path::new(raw))?;
            let publish_title = title.clone();
            let path = tokio::task::spawn_blocking(move || {
                markdown::publish_unique(&output, &publish_title, &document)
            })
            .await
            .context("Markdown publication task failed")??;
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("transcription.md")
                .to_owned();
            (path, name)
        }
        QuickOutputKind::Client => {
            let result_name = format!("{}.md", markdown::sanitize_note_filename(&title));
            let path = state
                .config
                .quick_result_dir()
                .join(format!("{}.md", job.id));
            let write_path = path.clone();
            tokio::task::spawn_blocking(move || {
                markdown::publish_atomic_no_overwrite(&write_path, &document)
            })
            .await
            .context("client Markdown staging task failed")??;
            (path, result_name)
        }
    };
    update_job(state, &job.id, |job| {
        job.status = JobStatus::Done;
        job.markdown_published = true;
        job.markdown_path = Some(markdown_path.clone());
        job.archive_path = None;
        job.error = None;
        if let Some(quick) = job.quick.as_mut() {
            quick.result_name = Some(result_name.clone());
        }
    })?;
    Ok(())
}
