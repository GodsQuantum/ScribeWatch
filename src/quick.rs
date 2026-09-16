use crate::{
    domain::{
        Job, JobKind, JobStatus, QuickJobMeta, QuickOutputKind, QuickSourceKind, TranscriptionRoute,
    },
    jobs::{now_ms, persist_job},
    state::AppState,
    workflows::is_audio_candidate,
};
use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickOptions {
    #[serde(default)]
    pub provider_id: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub transcription_chain: Vec<TranscriptionRoute>,
    #[serde(default)]
    pub language: Option<String>,
    pub output_kind: QuickOutputKind,
    #[serde(default)]
    pub output_dir: Option<String>,
    #[serde(default = "default_true")]
    pub frontmatter: bool,
}
fn default_true() -> bool {
    true
}

fn mtime_ns(metadata: &std::fs::Metadata) -> i128 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos() as i128)
        .unwrap_or(0)
}

async fn normalized_job(
    state: &AppState,
    source_path: PathBuf,
    original_name: String,
    source_kind: QuickSourceKind,
    mut options: QuickOptions,
) -> Result<Job> {
    if !is_audio_candidate(Path::new(&original_name)) {
        bail!("unsupported audio file: {original_name}");
    }
    let providers = state.providers.read().await.clone();
    let transcription_chain =
        crate::transcription_chain::normalize_quick_chain(&options, &providers)?;
    let primary = transcription_chain
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("transcription chain is empty"))?;
    options.language = options
        .language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("auto"))
        .map(str::to_owned);
    let output_dir = match options.output_kind {
        QuickOutputKind::Server => {
            let raw = options
                .output_dir
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow!("server output requires outputDir"))?;
            Some(
                state
                    .config
                    .resolve_allowed_dir(Path::new(raw))?
                    .to_string_lossy()
                    .into_owned(),
            )
        }
        QuickOutputKind::Client => None,
    };
    let metadata = std::fs::metadata(&source_path)?;
    if metadata.len() == 0 {
        bail!("audio file is empty");
    }
    let now = now_ms();
    let job = Job {
        id: Uuid::new_v4().to_string(),
        kind: JobKind::Quick,
        workflow_id: None,
        quick: Some(QuickJobMeta {
            source_kind,
            output_kind: options.output_kind,
            output_dir,
            result_name: None,
            frontmatter: options.frontmatter,
        }),
        provider_id: primary.provider_id.clone(),
        transcription_chain,
        transcription_attempts: Vec::new(),
        used_provider_id: None,
        used_provider_name: None,
        used_model: None,
        original_name,
        source_path,
        source_size: metadata.len(),
        source_mtime_ns: mtime_ns(&metadata),
        model: primary.model,
        language: options.language,
        status: JobStatus::Pending,
        attempts: 1,
        error: None,
        markdown_path: None,
        archive_path: None,
        markdown_published: false,
        created_at_ms: now,
        updated_at_ms: now,
    };
    state.jobs.insert(job.id.clone(), job.clone());
    persist_job(state, &job)?;
    state.emit_job(&job);
    Ok(job)
}
pub async fn create_server_job(
    state: &AppState,
    source_path: &Path,
    options: QuickOptions,
) -> Result<Job> {
    let source = state.config.resolve_allowed_file(source_path)?;
    let original_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("invalid source filename"))?
        .to_owned();
    normalized_job(
        state,
        source,
        original_name,
        QuickSourceKind::Server,
        options,
    )
    .await
}

pub async fn create_uploaded_job(
    state: &AppState,
    staged_path: PathBuf,
    original_name: String,
    options: QuickOptions,
) -> Result<Job> {
    let upload_root = std::fs::canonicalize(state.config.quick_upload_dir())?;
    let staged = std::fs::canonicalize(&staged_path)?;
    if !staged.starts_with(&upload_root) || !staged.is_file() {
        bail!("uploaded source is outside the staging directory");
    }
    normalized_job(
        state,
        staged,
        original_name,
        QuickSourceKind::Upload,
        options,
    )
    .await
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CleanupStats {
    pub uploads_removed: usize,
    pub results_removed: usize,
}
pub fn cleanup_stale(state: &AppState) -> Result<CleanupStats> {
    let keep_uploads = state
        .jobs
        .iter()
        .filter(|entry| entry.status.is_active())
        .filter_map(|entry| {
            let quick = entry.quick.as_ref()?;
            (quick.source_kind == QuickSourceKind::Upload).then(|| entry.source_path.clone())
        })
        .collect::<HashSet<_>>();
    let mut stats = CleanupStats::default();
    for entry in std::fs::read_dir(state.config.quick_upload_dir())? {
        let path = entry?.path();
        if path.is_file() && !keep_uploads.contains(&path) {
            std::fs::remove_file(path)?;
            stats.uploads_removed += 1;
        }
    }
    let retention = Duration::from_secs(
        state
            .config
            .quick_result_retention_hours
            .saturating_mul(3600),
    );
    let now = SystemTime::now();
    for entry in std::fs::read_dir(state.config.quick_result_dir())? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let modified = std::fs::metadata(&path)?.modified().unwrap_or(now);
        if now.duration_since(modified).unwrap_or_default() > retention {
            std::fs::remove_file(path)?;
            stats.results_removed += 1;
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests;
