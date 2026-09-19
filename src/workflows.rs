use crate::{
    audio,
    domain::{JobStatus, Workflow},
    jobs,
    state::AppState,
};
use anyhow::{Result, anyhow};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub async fn run_supervisor(state: AppState, shutdown: CancellationToken) {
    let mut tick = tokio::time::interval(Duration::from_secs(3));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = tick.tick() => if let Err(error) = reconcile(&state).await {
                tracing::error!(%error, "workflow reconcile failed");
            }
        }
    }
    for entry in state.watcher_tokens.iter() {
        entry.value().1.cancel();
    }
}

pub async fn reconcile(state: &AppState) -> Result<()> {
    let workflows = state.workflows.read().await.clone();
    let enabled: HashSet<String> = workflows
        .iter()
        .filter(|w| w.enabled)
        .map(|w| w.id.clone())
        .collect();
    let existing: Vec<String> = state
        .watcher_tokens
        .iter()
        .map(|entry| entry.key().clone())
        .collect();
    for id in existing {
        if !enabled.contains(&id)
            && let Some((_, (_, token))) = state.watcher_tokens.remove(&id)
        {
            token.cancel();
        }
    }
    for workflow in workflows.into_iter().filter(|w| w.enabled) {
        let signature = serde_json::to_string(&workflow)?;
        if state
            .watcher_tokens
            .get(&workflow.id)
            .is_some_and(|entry| entry.value().0 == signature)
        {
            continue;
        }
        if let Some((_, (_, token))) = state.watcher_tokens.remove(&workflow.id) {
            token.cancel();
        }
        let token = CancellationToken::new();
        state
            .watcher_tokens
            .insert(workflow.id.clone(), (signature.clone(), token.clone()));
        let task_state = state.clone();
        tokio::spawn(async move {
            if let Err(error) =
                watch_workflow(task_state.clone(), workflow.clone(), token.clone()).await
                && !token.is_cancelled()
            {
                tracing::error!(workflow = %workflow.name, %error, "workflow stopped");
            }
            let should_remove = task_state
                .watcher_tokens
                .get(&workflow.id)
                .is_some_and(|entry| entry.value().0 == signature);
            if should_remove {
                task_state.watcher_tokens.remove(&workflow.id);
            }
        });
    }
    Ok(())
}

async fn watch_workflow(
    state: AppState,
    mut workflow: Workflow,
    token: CancellationToken,
) -> Result<()> {
    workflow.watch_dir = state
        .config
        .resolve_allowed_dir(Path::new(&workflow.watch_dir))?
        .to_string_lossy()
        .into_owned();
    workflow.archive_dir = state
        .config
        .resolve_allowed_dir(Path::new(&workflow.archive_dir))?
        .to_string_lossy()
        .into_owned();
    let watch_dir = PathBuf::from(&workflow.watch_dir);
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<PathBuf>();
    let callback_tx = event_tx.clone();
    let mut watcher: Option<RecommendedWatcher> =
        notify::recommended_watcher(move |result: notify::Result<Event>| {
            if let Ok(event) = result {
                for path in event.paths {
                    let _ = callback_tx.send(path);
                }
            }
        })
        .ok();
    if let Some(error) = watcher
        .as_mut()
        .and_then(|w| w.watch(&watch_dir, RecursiveMode::NonRecursive).err())
    {
        tracing::warn!(%error, "native watcher unavailable; periodic reconciliation remains active");
        watcher = None;
    }
    let mut interval = tokio::time::interval(Duration::from_secs(state.config.scan_seconds.max(1)));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut attempted: HashMap<PathBuf, (u64, i128)> = HashMap::new();
    loop {
        tokio::select! {
            _ = token.cancelled() => break,
            Some(path) = event_rx.recv() => {
                try_process_path(&state, &workflow, &path, &token, &mut attempted).await;
            }
            _ = interval.tick() => {
                let mut entries = match tokio::fs::read_dir(&watch_dir).await {
                    Ok(entries) => entries,
                    Err(error) => { tracing::warn!(%error, "workflow scan failed"); continue; }
                };
                while let Ok(Some(entry)) = entries.next_entry().await {
                    try_process_path(&state, &workflow, &entry.path(), &token, &mut attempted).await;
                }
            }
        }
    }
    drop(watcher);
    Ok(())
}
async fn try_process_path(
    state: &AppState,
    workflow: &Workflow,
    path: &Path,
    token: &CancellationToken,
    attempted: &mut HashMap<PathBuf, (u64, i128)>,
) {
    if token.is_cancelled() || !path.is_file() || audio::is_staging_name(path) {
        return;
    }
    let Ok((size, mtime)) = stable_fingerprint(
        path,
        Duration::from_millis(state.config.file_stability_ms.max(250)),
        token,
    )
    .await
    else {
        return;
    };
    if attempted.get(path).is_some_and(|fp| *fp == (size, mtime)) {
        return;
    }
    match audio::probe_audio(path, token).await {
        Ok(true) => {}
        Ok(false) => {
            attempted.insert(path.to_path_buf(), (size, mtime));
            return;
        }
        Err(error) => {
            if !token.is_cancelled() {
                tracing::warn!(path = %path.display(), %error, "audio probing failed");
            }
            return;
        }
    }
    let path_text = path.to_string_lossy().into_owned();
    if state
        .db
        .seen(&workflow.id, &path_text, size, mtime)
        .unwrap_or(false)
    {
        attempted.insert(path.to_path_buf(), (size, mtime));
        return;
    }
    attempted.insert(path.to_path_buf(), (size, mtime));

    if let Some(existing) = state
        .jobs
        .iter()
        .find(|entry| {
            let job = entry.value();
            job.workflow_id.as_deref() == Some(workflow.id.as_str())
                && job.source_path == path
                && job.source_size == size
                && job.source_mtime_ns == mtime
        })
        .map(|entry| entry.value().clone())
    {
        if matches!(existing.status, JobStatus::Pending | JobStatus::Interrupted) {
            let _ = jobs::enqueue(state.clone(), existing.id);
        }
        return;
    }
    match jobs::create_job(state, workflow, path.to_path_buf(), size, mtime).await {
        Ok(job) => {
            if let Err(error) = jobs::enqueue(state.clone(), job.id.clone()) {
                tracing::error!(job_id = %job.id, %error, "could not enqueue job");
            }
        }
        Err(error) => {
            tracing::error!(workflow = %workflow.name, path = %path.display(), %error, "could not create job")
        }
    }
}

pub async fn stable_fingerprint(
    path: &Path,
    delay: Duration,
    token: &CancellationToken,
) -> Result<(u64, i128)> {
    let first = tokio::fs::metadata(path).await?;
    if !first.is_file() {
        return Err(anyhow!("not a file"));
    }
    tokio::select! {
        _ = token.cancelled() => return Err(anyhow!("cancelled")),
        _ = tokio::time::sleep(delay) => {}
    }
    let second = tokio::fs::metadata(path).await?;
    let a = fingerprint(&first);
    let b = fingerprint(&second);
    if a != b {
        return Err(anyhow!("file is still changing"));
    }
    Ok(b)
}

fn fingerprint(meta: &std::fs::Metadata) -> (u64, i128) {
    let modified = meta
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos() as i128)
        .unwrap_or(0);
    (meta.len(), modified)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stable_file_detection_rejects_changes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("audio.m4a");
        tokio::fs::write(&path, b"a").await.unwrap();
        let token = CancellationToken::new();
        let writer = path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            tokio::fs::write(writer, b"changed").await.unwrap();
        });
        assert!(
            stable_fingerprint(&path, Duration::from_millis(60), &token)
                .await
                .is_err()
        );
    }
}

pub async fn normalize_definition(state: &AppState, mut workflow: Workflow) -> Result<Workflow> {
    workflow.name = workflow.name.trim().to_owned();
    if workflow.name.is_empty() {
        anyhow::bail!("workflow name is required");
    }
    let watch = state
        .config
        .resolve_allowed_dir(Path::new(&workflow.watch_dir))?;
    let archive = state
        .config
        .resolve_allowed_dir(Path::new(&workflow.archive_dir))?;
    let output = workflow
        .output_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| state.config.resolve_allowed_dir(Path::new(value)))
        .transpose()?;
    if archive == watch || archive.starts_with(&watch) {
        anyhow::bail!("archive directory must be outside the watch directory");
    }
    let providers = state.providers.read().await.clone();
    let chain = crate::transcription_chain::normalize_workflow_chain(&workflow, &providers)?;
    let primary = chain
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("transcription chain is empty"))?;
    workflow.transcription_chain = chain;
    workflow.provider_id = primary.provider_id;
    workflow.model = primary.model;
    workflow.watch_dir = watch.to_string_lossy().into_owned();
    workflow.output_dir = output.map(|path| path.to_string_lossy().into_owned());
    workflow.archive_dir = archive.to_string_lossy().into_owned();
    workflow.tags = crate::markdown::normalize_custom_tags(&workflow.tags);
    workflow.language = workflow
        .language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("auto"))
        .map(str::to_owned);
    workflow.structure_profile_id =
        crate::llm::validate_profile_selection(state, workflow.structure_profile_id.as_deref())
            .await?;
    if workflow.markdown.transcript_heading.trim().is_empty() {
        workflow.markdown.transcript_heading = "Transcript".into();
    }
    Ok(workflow)
}

pub async fn scan_once(state: &AppState, workflow_id: &str) -> Result<()> {
    let workflow = state
        .workflows
        .read()
        .await
        .iter()
        .find(|workflow| workflow.id == workflow_id)
        .cloned()
        .ok_or_else(|| anyhow!("workflow not found"))?;
    if !workflow.enabled {
        anyhow::bail!("workflow is disabled");
    }
    let watch_dir = state
        .config
        .resolve_allowed_dir(Path::new(&workflow.watch_dir))?;
    let token = CancellationToken::new();
    let mut attempted = HashMap::new();
    let mut entries = tokio::fs::read_dir(watch_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        try_process_path(state, &workflow, &entry.path(), &token, &mut attempted).await;
    }
    Ok(())
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use crate::{
        config::Config,
        domain::{MarkdownOptions, Provider},
    };

    fn config(root: &Path) -> Config {
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
            max_upload_bytes: 2_147_483_648,
            quick_result_retention_hours: 24,
        }
    }

    #[tokio::test]
    async fn workflow_configuration_is_canonicalized_and_validated() {
        let temp = tempfile::tempdir().unwrap();
        let watch = temp.path().join("watch");
        let archive = temp.path().join("archive");
        std::fs::create_dir_all(&watch).unwrap();
        std::fs::create_dir_all(&archive).unwrap();
        let state = AppState::load(config(temp.path())).await.unwrap();
        state.providers.write().await.push(Provider {
            id: "provider".into(),
            name: "Provider".into(),
            transcription_url: "http://127.0.0.1:1/v1/audio/transcriptions".into(),
            model: "model".into(),
            api_key: String::new(),
            timeout_seconds: 5,
            enabled: true,
        });
        let workflow = Workflow {
            id: "workflow".into(),
            name: "Workflow".into(),
            watch_dir: watch.to_string_lossy().into_owned(),
            output_dir: None,
            archive_dir: archive.to_string_lossy().into_owned(),
            tags: Vec::new(),
            provider_id: "provider".into(),
            model: String::new(),
            transcription_chain: Vec::new(),
            language: Some("auto".into()),
            structure_profile_id: None,
            markdown: MarkdownOptions::default(),
            enabled: true,
        };
        let validated = normalize_definition(&state, workflow).await.unwrap();
        assert_eq!(validated.language, None);
        assert_eq!(
            PathBuf::from(validated.watch_dir),
            std::fs::canonicalize(&watch).unwrap()
        );
        assert_eq!(
            PathBuf::from(validated.archive_dir),
            std::fs::canonicalize(&archive).unwrap()
        );
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn workflow_rejects_output_symlink_escape() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let watch = temp.path().join("watch");
        let archive = temp.path().join("archive");
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(&watch).unwrap();
        std::fs::create_dir_all(&archive).unwrap();
        symlink(outside.path(), watch.join("escaped-notes")).unwrap();
        let state = AppState::load(config(temp.path())).await.unwrap();
        state.providers.write().await.push(Provider {
            id: "provider".into(),
            name: "Provider".into(),
            transcription_url: "http://127.0.0.1:1/v1/audio/transcriptions".into(),
            model: "model".into(),
            api_key: String::new(),
            timeout_seconds: 5,
            enabled: true,
        });
        let workflow = Workflow {
            id: "workflow".into(),
            name: "Workflow".into(),
            watch_dir: watch.to_string_lossy().into_owned(),
            output_dir: Some(watch.join("escaped-notes").to_string_lossy().into_owned()),
            archive_dir: archive.to_string_lossy().into_owned(),
            tags: Vec::new(),
            provider_id: "provider".into(),
            model: String::new(),
            transcription_chain: Vec::new(),
            language: None,
            structure_profile_id: None,
            markdown: MarkdownOptions::default(),
            enabled: true,
        };
        assert!(normalize_definition(&state, workflow).await.is_err());
    }

    #[tokio::test]
    async fn workflow_rejects_archive_inside_watch_tree() {
        let temp = tempfile::tempdir().unwrap();
        let watch = temp.path().join("watch");
        let archive = watch.join("archive");
        std::fs::create_dir_all(&archive).unwrap();
        let state = AppState::load(config(temp.path())).await.unwrap();
        state.providers.write().await.push(Provider {
            id: "provider".into(),
            name: "Provider".into(),
            transcription_url: "http://127.0.0.1:1/v1/audio/transcriptions".into(),
            model: "model".into(),
            api_key: String::new(),
            timeout_seconds: 5,
            enabled: true,
        });
        let workflow = Workflow {
            id: "workflow".into(),
            name: "Workflow".into(),
            watch_dir: watch.to_string_lossy().into_owned(),
            output_dir: None,
            archive_dir: archive.to_string_lossy().into_owned(),
            tags: Vec::new(),
            provider_id: "provider".into(),
            model: String::new(),
            transcription_chain: Vec::new(),
            language: None,
            structure_profile_id: None,
            markdown: MarkdownOptions::default(),
            enabled: true,
        };
        assert!(normalize_definition(&state, workflow).await.is_err());
    }
}
