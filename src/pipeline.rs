use crate::{
    domain::{JobKind, JobStatus, Workflow},
    jobs::{get_job, update_job},
    markdown::{self, NoteContext},
    state::AppState,
    transcription_chain,
};
use anyhow::{Context, Result, anyhow, bail};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};
use time::OffsetDateTime;
use tokio_util::sync::CancellationToken;

pub async fn process_job(state: &AppState, id: &str, token: &CancellationToken) -> Result<()> {
    let job = get_job(state, id)?;
    match job.kind {
        JobKind::Workflow => process_workflow_job(state, &job, token).await,
        JobKind::Quick => crate::pipeline_quick::process_quick_job(state, &job, token).await,
    }
}

async fn process_workflow_job(
    state: &AppState,
    job: &crate::domain::Job,
    token: &CancellationToken,
) -> Result<()> {
    let workflow_id = job
        .workflow_id
        .as_deref()
        .ok_or_else(|| anyhow!("workflow job has no workflow id"))?;
    let workflow = state
        .workflows
        .read()
        .await
        .iter()
        .find(|workflow| workflow.id == workflow_id)
        .cloned()
        .ok_or_else(|| anyhow!("workflow not found: {workflow_id}"))?;
    let source = state.config.resolve_allowed_file(&job.source_path)?;
    let watch_dir = state
        .config
        .resolve_allowed_dir(Path::new(&workflow.watch_dir))?;
    if !source.starts_with(&watch_dir) {
        bail!("source is outside workflow watch directory");
    }
    if token.is_cancelled() {
        bail!("cancelled");
    }
    if job.markdown_published {
        let markdown_path = job
            .markdown_path
            .clone()
            .ok_or_else(|| anyhow!("published job has no Markdown path"))?;
        if !markdown_path.is_file() {
            bail!("published Markdown is missing: {}", markdown_path.display());
        }
        return finish_archive(state, &job.id, &workflow, &source, token).await;
    }

    let success = transcription_chain::transcribe_job_chain(
        state,
        &job.id,
        &source,
        job.language.as_deref(),
        token,
    )
    .await
    .context("transcription")?;
    let transcript = success.transcript;
    if token.is_cancelled() {
        bail!("cancelled");
    }

    update_job(state, &job.id, |job| job.status = JobStatus::Publishing)?;
    let output_dir = match workflow.output_dir.as_deref() {
        Some(path) => state.config.resolve_allowed_dir(Path::new(path))?,
        None => watch_dir,
    };
    let title = markdown::derive_title(&transcript, &job.original_name);
    let context = NoteContext {
        title: &title,
        source_name: &job.original_name,
        workflow_name: Some(&workflow.name),
        provider_name: &success.provider_name,
        model: &success.model,
        language: job.language.as_deref(),
        tags: &workflow.tags,
        frontmatter: workflow.markdown.frontmatter,
        created_at: OffsetDateTime::now_utc(),
    };
    let document = markdown::render_note(&context, &transcript)?;
    let publish_dir = output_dir.clone();
    let publish_title = title.clone();
    let markdown_path = tokio::task::spawn_blocking(move || {
        markdown::publish_unique(&publish_dir, &publish_title, &document)
    })
    .await
    .context("Markdown publication task failed")??;
    update_job(state, &job.id, |job| {
        job.markdown_published = true;
        job.markdown_path = Some(markdown_path.clone());
    })?;
    if token.is_cancelled() {
        bail!("cancelled");
    }
    finish_archive(state, &job.id, &workflow, &source, token).await
}

async fn finish_archive(
    state: &AppState,
    id: &str,
    workflow: &Workflow,
    source: &Path,
    token: &CancellationToken,
) -> Result<()> {
    update_job(state, id, |job| job.status = JobStatus::Archiving)?;
    if token.is_cancelled() {
        bail!("cancelled");
    }
    let archive_dir = state
        .config
        .resolve_allowed_dir(Path::new(&workflow.archive_dir))?;
    let source_owned = source.to_path_buf();
    let archived = tokio::task::spawn_blocking(move || archive_source(&source_owned, &archive_dir))
        .await
        .context("archive task failed")??;
    let job = update_job(state, id, |job| {
        job.status = JobStatus::Done;
        job.archive_path = Some(archived.clone());
        job.error = None;
    })?;
    let workflow_id = job
        .workflow_id
        .as_deref()
        .ok_or_else(|| anyhow!("workflow job has no workflow id"))?;
    if let Err(error) = state.db.mark_seen(
        workflow_id,
        &job.source_path.to_string_lossy(),
        job.source_size,
        job.source_mtime_ns,
    ) {
        tracing::error!(%error, job_id = %job.id, "could not persist completed fingerprint");
    }
    Ok(())
}

fn archive_source(source: &Path, archive_dir: &Path) -> Result<PathBuf> {
    let filename = source
        .file_name()
        .ok_or_else(|| anyhow!("source has no filename"))?;
    let original = Path::new(filename);
    let stem = original
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("audio");
    let extension = original.extension().and_then(|v| v.to_str());
    for n in 0usize..10_000 {
        let suffix = if n == 0 {
            String::new()
        } else {
            format!(" ({n})")
        };
        let name = match extension {
            Some(ext) => format!("{stem}{suffix}.{ext}"),
            None => format!("{stem}{suffix}"),
        };
        let destination = archive_dir.join(name);
        match move_candidate(source, &destination) {
            Ok(()) => return Ok(destination),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| format!("archive {}", source.display()));
            }
        }
    }
    bail!("could not reserve a unique archive filename")
}

fn move_candidate(source: &Path, destination: &Path) -> std::io::Result<()> {
    match std::fs::hard_link(source, destination) {
        Ok(()) => {
            if let Err(error) = std::fs::remove_file(source) {
                let _ = std::fs::remove_file(destination);
                return Err(error);
            }
            return Ok(());
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => return Err(error),
        Err(_) => {}
    }
    let mut input = std::fs::File::open(source)?;
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    if let Err(error) = std::io::copy(&mut input, &mut output) {
        drop(output);
        let _ = std::fs::remove_file(destination);
        return Err(error);
    }
    output.sync_all()?;
    if std::fs::metadata(source)?.len() != std::fs::metadata(destination)?.len() {
        let _ = std::fs::remove_file(destination);
        return Err(std::io::Error::other("archive copy size mismatch"));
    }
    std::fs::remove_file(source)?;
    Ok(())
}
