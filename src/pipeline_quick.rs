use crate::{
    domain::{Job, JobStatus, QuickJobMeta, QuickOutputKind, QuickSourceKind},
    jobs::update_job,
    markdown::{self, NoteContext},
    provider,
    state::AppState,
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
    let provider = state
        .providers
        .read()
        .await
        .iter()
        .find(|provider| provider.id == job.provider_id)
        .cloned()
        .ok_or_else(|| anyhow!("provider not found: {}", job.provider_id))?;
    if !provider.enabled {
        bail!("provider '{}' is disabled", provider.name);
    }
    if token.is_cancelled() {
        bail!("cancelled");
    }
    update_job(state, &job.id, |job| {
        job.status = JobStatus::Transcribing;
        job.error = None;
    })?;
    let transcript = provider::transcribe(
        &provider,
        &job.model,
        job.language.as_deref(),
        source,
        &state.http,
        token,
    )
    .await
    .context("transcription")?;
    if token.is_cancelled() {
        bail!("cancelled");
    }
    update_job(state, &job.id, |job| job.status = JobStatus::Publishing)?;
    let title = markdown::derive_title(&transcript, &job.original_name);
    let context = NoteContext {
        title: &title,
        source_name: &job.original_name,
        workflow_name: None,
        provider_name: &provider.name,
        model: &job.model,
        language: job.language.as_deref(),
        tags: &[],
        frontmatter: meta.frontmatter,
        created_at: OffsetDateTime::now_utc(),
    };
    let document = markdown::render_note(&context, &transcript)?;
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
