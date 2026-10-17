use crate::{
    audio,
    domain::{
        Job, JobKind, JobStatus, LiveJobMeta, QuickJobMeta, QuickOutputKind, QuickSourceKind,
        TranscriptionRoute,
    },
    jobs::{now_ms, persist_job},
    llm,
    state::AppState,
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
    #[serde(default = "default_true")]
    pub paragraphs: bool,
    #[serde(default)]
    pub structure_profile_id: Option<String>,
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
    live: Option<LiveJobMeta>,
    mut options: QuickOptions,
) -> Result<Job> {
    if !audio::probe_audio(&source_path, &tokio_util::sync::CancellationToken::new()).await? {
        bail!("file does not contain a decodable audio stream: {original_name}");
    }
    options.structure_profile_id =
        llm::validate_profile_selection(state, options.structure_profile_id.as_deref()).await?;
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
            paragraphs: options.paragraphs,
            structure_profile_id: options.structure_profile_id.clone(),
            live,
        }),
        provider_id: primary.provider_id.clone(),
        transcription_chain,
        transcription_attempts: Vec::new(),
        used_provider_id: None,
        used_provider_name: None,
        used_model: None,
        structured_profile_id: None,
        structured_profile_name: None,
        structured_model: None,
        structuring_error: None,
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
        None,
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
        None,
        options,
    )
    .await
}

pub fn find_live_job_by_session(state: &AppState, session_id: &str) -> Option<Job> {
    state.jobs.iter().find_map(|entry| {
        let job = entry.value();
        job.quick
            .as_ref()
            .and_then(|quick| quick.live.as_ref())
            .filter(|live| live.session_id == session_id)
            .map(|_| job.clone())
    })
}

pub async fn create_live_job(
    state: &AppState,
    source_path: PathBuf,
    original_name: String,
    live: LiveJobMeta,
    options: QuickOptions,
) -> Result<Job> {
    let root = std::fs::canonicalize(state.config.live_source_dir())?;
    let source = std::fs::canonicalize(&source_path)?;
    if !source.starts_with(&root) || !source.is_file() {
        bail!("LIVE source is outside the LIVE source directory");
    }
    normalized_job(
        state,
        source,
        original_name,
        QuickSourceKind::Live,
        Some(live),
        options,
    )
    .await
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CleanupStats {
    pub uploads_removed: usize,
    pub live_sources_removed: usize,
    pub live_staging_dirs_removed: usize,
    pub results_removed: usize,
}
pub fn cleanup_stale(state: &AppState) -> Result<CleanupStats> {
    let now_ms = now_ms();
    let source_retention_ms =
        (state.config.quick_source_retention_hours as u128).saturating_mul(3_600_000);
    let protected = |kind: QuickSourceKind| {
        state
            .jobs
            .iter()
            .filter_map(|entry| {
                let quick = entry.quick.as_ref()?;
                if quick.source_kind != kind {
                    return None;
                }
                let age_ms = now_ms.saturating_sub(entry.updated_at_ms);
                (entry.status.is_active() || age_ms <= source_retention_ms)
                    .then(|| entry.source_path.clone())
            })
            .collect::<HashSet<_>>()
    };
    let keep_uploads = protected(QuickSourceKind::Upload);
    let keep_live = protected(QuickSourceKind::Live);
    let mut stats = CleanupStats::default();

    for entry in std::fs::read_dir(state.config.quick_upload_dir())? {
        let path = entry?.path();
        if path.is_file() && !keep_uploads.contains(&path) {
            std::fs::remove_file(path)?;
            stats.uploads_removed += 1;
        }
    }
    for entry in std::fs::read_dir(state.config.live_source_dir())? {
        let path = entry?.path();
        if path.is_file() && !keep_live.contains(&path) {
            std::fs::remove_file(path)?;
            stats.live_sources_removed += 1;
        }
    }

    let source_retention = Duration::from_secs(
        state
            .config
            .quick_source_retention_hours
            .saturating_mul(3600),
    );
    let now = SystemTime::now();
    for entry in std::fs::read_dir(state.config.live_staging_dir())? {
        let path = entry?.path();
        if !path.is_dir() {
            continue;
        }
        let modified = std::fs::metadata(&path)?.modified().unwrap_or(now);
        if now.duration_since(modified).unwrap_or_default() > source_retention {
            std::fs::remove_dir_all(path)?;
            stats.live_staging_dirs_removed += 1;
        }
    }

    let result_retention = Duration::from_secs(
        state
            .config
            .quick_result_retention_hours
            .saturating_mul(3600),
    );
    for entry in std::fs::read_dir(state.config.quick_result_dir())? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let modified = std::fs::metadata(&path)?.modified().unwrap_or(now);
        if now.duration_since(modified).unwrap_or_default() > result_retention {
            std::fs::remove_file(path)?;
            stats.results_removed += 1;
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests;
