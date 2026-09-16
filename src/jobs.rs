use crate::{
    domain::{Job, JobStatus, Workflow},
    state::AppState,
};
use anyhow::{Result, anyhow, bail};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub fn persist_job(state: &AppState, job: &Job) -> Result<()> {
    state.db.upsert("job", &job.id, job)
}

pub fn get_job(state: &AppState, id: &str) -> Result<Job> {
    state
        .jobs
        .get(id)
        .map(|job| job.clone())
        .ok_or_else(|| anyhow!("job not found: {id}"))
}

pub fn update_job<F>(state: &AppState, id: &str, update: F) -> Result<Job>
where
    F: FnOnce(&mut Job),
{
    let job = {
        let mut entry = state
            .jobs
            .get_mut(id)
            .ok_or_else(|| anyhow!("job not found: {id}"))?;
        update(&mut entry);
        entry.updated_at_ms = now_ms();
        entry.clone()
    };
    persist_job(state, &job)?;
    state.emit_job(&job);
    Ok(job)
}
pub async fn create_job(
    state: &AppState,
    workflow: &Workflow,
    source_path: PathBuf,
    source_size: u64,
    source_mtime_ns: i128,
) -> Result<Job> {
    let providers = state.providers.read().await.clone();
    let transcription_chain =
        crate::transcription_chain::normalize_workflow_chain(workflow, &providers)?;
    let primary = transcription_chain
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("transcription chain is empty"))?;
    let original_name = source_path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| anyhow!("invalid source filename"))?
        .to_owned();
    let now = now_ms();
    let job = Job {
        id: Uuid::new_v4().to_string(),
        kind: crate::domain::JobKind::Workflow,
        workflow_id: Some(workflow.id.clone()),
        quick: None,
        provider_id: primary.provider_id.clone(),
        transcription_chain,
        transcription_attempts: Vec::new(),
        used_provider_id: None,
        used_provider_name: None,
        used_model: None,
        original_name,
        source_path,
        source_size,
        source_mtime_ns,
        model: primary.model,
        language: workflow.language.clone(),
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
fn fresh_token(state: &AppState, id: &str) -> CancellationToken {
    if let Some((_, old)) = state.job_tokens.remove(id) {
        old.cancel();
    }
    let token = CancellationToken::new();
    state.job_tokens.insert(id.to_owned(), token.clone());
    token
}

pub fn enqueue(state: AppState, id: String) -> Result<()> {
    if state.job_tokens.contains_key(&id) {
        return Ok(());
    }
    let token = fresh_token(&state, &id);
    tokio::spawn(async move {
        let permit = state.transcription_slots.clone().acquire_owned().await;
        if permit.is_err() {
            finish_error(&state, &id, "worker pool closed".into(), false);
            return;
        }
        let _permit = permit.unwrap();
        let result = crate::pipeline::process_job(&state, &id, &token).await;
        if let Err(error) = result {
            let cancelled = token.is_cancelled();
            finish_error(&state, &id, error.to_string(), cancelled);
        }
        state.job_tokens.remove(&id);
    });
    Ok(())
}
fn finish_error(state: &AppState, id: &str, message: String, cancelled: bool) {
    let _ = update_job(state, id, |job| {
        job.status = if cancelled {
            JobStatus::Cancelled
        } else {
            JobStatus::Error
        };
        job.error = if cancelled {
            Some("cancelled".into())
        } else {
            Some(message)
        };
    });
}

pub fn cancel(state: &AppState, id: &str) -> Result<Job> {
    if let Some(token) = state.job_tokens.get(id) {
        token.cancel();
    }
    update_job(state, id, |job| {
        job.status = JobStatus::Cancelled;
        job.error = Some("cancelled".into());
    })
}

pub fn retry(state: AppState, id: &str) -> Result<Job> {
    let current = get_job(&state, id)?;
    if current.status.is_active() {
        bail!("job is already active");
    }
    if !current.source_path.is_file() {
        bail!("source audio no longer exists");
    }
    let job = update_job(&state, id, |job| {
        job.status = JobStatus::Pending;
        job.error = None;
        job.attempts = job.attempts.saturating_add(1);
    })?;
    enqueue(state, id.to_owned())?;
    Ok(job)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::process_job;
    use crate::{
        config::Config,
        domain::{MarkdownOptions, Provider},
    };
    use axum::{Json, Router, body::Bytes, http::StatusCode, routing::post};
    use serde_json::json;
    use std::{path::Path, time::Duration};

    async fn success_provider() -> String {
        let app = Router::new().route(
            "/v1/audio/transcriptions",
            post(|_body: Bytes| async { Json(json!({"text": "hello from provider"})) }),
        );
        spawn_server(app).await
    }

    async fn failing_provider() -> String {
        let app = Router::new().route(
            "/v1/audio/transcriptions",
            post(|_body: Bytes| async {
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"error": "provider down"})),
                )
            }),
        );
        spawn_server(app).await
    }
    async fn spawn_server(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{address}/v1/audio/transcriptions")
    }

    fn test_config(root: &Path) -> Config {
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

    async fn setup(root: &Path, url: String) -> (AppState, Workflow) {
        let watch = root.join("watch");
        let archive = root.join("archive");
        std::fs::create_dir_all(&watch).unwrap();
        std::fs::create_dir_all(&archive).unwrap();
        let state = AppState::load(test_config(root)).await.unwrap();
        let provider = Provider {
            id: "provider".into(),
            name: "Test provider".into(),
            transcription_url: url,
            model: "test-model".into(),
            api_key: "secret".into(),
            timeout_seconds: 5,
            enabled: true,
        };
        state
            .db
            .upsert("provider", &provider.id, &provider)
            .unwrap();
        state.providers.write().await.push(provider);
        let workflow = Workflow {
            id: "workflow".into(),
            name: "Test workflow".into(),
            watch_dir: watch.to_string_lossy().into_owned(),
            output_dir: None,
            archive_dir: archive.to_string_lossy().into_owned(),
            tags: Vec::new(),
            provider_id: "provider".into(),
            model: String::new(),
            transcription_chain: Vec::new(),
            language: None,
            markdown: MarkdownOptions::default(),
            enabled: true,
        };
        state
            .db
            .upsert("workflow", &workflow.id, &workflow)
            .unwrap();
        state.workflows.write().await.push(workflow.clone());
        (state, workflow)
    }
    async fn make_job(state: &AppState, workflow: &Workflow, name: &str) -> Job {
        let source = Path::new(&workflow.watch_dir).join(name);
        tokio::fs::write(&source, b"fake audio bytes")
            .await
            .unwrap();
        let metadata = std::fs::metadata(&source).unwrap();
        let mtime = metadata
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i128;
        create_job(state, workflow, source, metadata.len(), mtime)
            .await
            .unwrap()
    }

    async fn wait_for_terminal(state: &AppState, id: &str) -> Job {
        for _ in 0..100 {
            let job = get_job(state, id).unwrap();
            if !job.status.is_active() {
                return job;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("job did not reach a terminal state");
    }

    #[tokio::test]
    async fn success_publishes_markdown_before_archiving_audio() {
        let temp = tempfile::tempdir().unwrap();
        let (state, workflow) = setup(temp.path(), success_provider().await).await;
        let job = make_job(&state, &workflow, "interview.m4a").await;
        process_job(&state, &job.id, &CancellationToken::new())
            .await
            .unwrap();
        let finished = get_job(&state, &job.id).unwrap();
        assert_eq!(finished.status, JobStatus::Done);
        let markdown = Path::new(&workflow.watch_dir).join("hello from provider.md");
        assert!(markdown.is_file());
        assert!(
            std::fs::read_to_string(markdown)
                .unwrap()
                .contains("hello from provider")
        );
        assert!(
            !Path::new(&workflow.watch_dir)
                .join("interview.m4a")
                .exists()
        );
        assert!(
            Path::new(&workflow.archive_dir)
                .join("interview.m4a")
                .is_file()
        );
    }

    #[tokio::test]
    async fn configured_output_directory_receives_note() {
        let temp = tempfile::tempdir().unwrap();
        let (state, mut workflow) = setup(temp.path(), success_provider().await).await;
        let notes = temp.path().join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        workflow.output_dir = Some(notes.to_string_lossy().into_owned());
        state.workflows.write().await[0] = workflow.clone();
        state
            .db
            .upsert("workflow", &workflow.id, &workflow)
            .unwrap();
        let job = make_job(&state, &workflow, "memo.m4a").await;
        process_job(&state, &job.id, &CancellationToken::new())
            .await
            .unwrap();
        assert!(notes.join("hello from provider.md").is_file());
        assert!(
            !Path::new(&workflow.watch_dir)
                .join("hello from provider.md")
                .exists()
        );
        assert!(Path::new(&workflow.archive_dir).join("memo.m4a").is_file());
    }

    #[tokio::test]
    async fn provider_failure_never_archives_source() {
        let temp = tempfile::tempdir().unwrap();
        let (state, workflow) = setup(temp.path(), failing_provider().await).await;
        let job = make_job(&state, &workflow, "failure.m4a").await;
        assert!(
            process_job(&state, &job.id, &CancellationToken::new())
                .await
                .is_err()
        );
        assert!(Path::new(&workflow.watch_dir).join("failure.m4a").is_file());
        assert!(!Path::new(&workflow.watch_dir).join("failure.md").exists());
        assert!(
            !Path::new(&workflow.archive_dir)
                .join("failure.m4a")
                .exists()
        );
    }

    #[tokio::test]
    async fn markdown_collision_numbers_note_and_still_archives_source() {
        let temp = tempfile::tempdir().unwrap();
        let (state, workflow) = setup(temp.path(), success_provider().await).await;
        let existing = Path::new(&workflow.watch_dir).join("hello from provider.md");
        std::fs::write(&existing, "existing note").unwrap();
        let job = make_job(&state, &workflow, "collision.m4a").await;
        process_job(&state, &job.id, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(existing).unwrap(), "existing note");
        assert!(
            Path::new(&workflow.watch_dir)
                .join("hello from provider (2).md")
                .is_file()
        );
        assert!(
            !Path::new(&workflow.watch_dir)
                .join("collision.m4a")
                .exists()
        );
        assert!(
            Path::new(&workflow.archive_dir)
                .join("collision.m4a")
                .is_file()
        );
    }

    #[tokio::test]
    async fn archive_collision_uses_numbered_name_without_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let (state, workflow) = setup(temp.path(), success_provider().await).await;
        let existing = Path::new(&workflow.archive_dir).join("same.m4a");
        std::fs::write(&existing, b"existing").unwrap();
        let job = make_job(&state, &workflow, "same.m4a").await;
        process_job(&state, &job.id, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(std::fs::read(&existing).unwrap(), b"existing");
        assert!(
            Path::new(&workflow.archive_dir)
                .join("same (1).m4a")
                .is_file()
        );
    }
    #[tokio::test]
    async fn retry_reuses_failed_job_and_completes() {
        let temp = tempfile::tempdir().unwrap();
        let (state, workflow) = setup(temp.path(), failing_provider().await).await;
        let job = make_job(&state, &workflow, "retry.m4a").await;
        let error = process_job(&state, &job.id, &CancellationToken::new())
            .await
            .unwrap_err();
        finish_error(&state, &job.id, error.to_string(), false);
        let success_url = success_provider().await;
        state.providers.write().await[0].transcription_url = success_url;
        retry(state.clone(), &job.id).unwrap();
        let finished = wait_for_terminal(&state, &job.id).await;
        assert_eq!(finished.status, JobStatus::Done);
        assert_eq!(finished.attempts, 2);
        assert!(
            Path::new(&workflow.watch_dir)
                .join("hello from provider.md")
                .is_file()
        );
    }

    #[tokio::test]
    async fn restart_marks_active_job_interrupted_without_duplication() {
        let temp = tempfile::tempdir().unwrap();
        let config = test_config(temp.path());
        let (state, workflow) = setup(temp.path(), success_provider().await).await;
        let job = make_job(&state, &workflow, "restart.m4a").await;
        assert_eq!(job.status, JobStatus::Pending);
        drop(state);
        let reloaded = AppState::load(config).await.unwrap();
        let restored = get_job(&reloaded, &job.id).unwrap();
        assert_eq!(restored.status, JobStatus::Interrupted);
        assert_eq!(reloaded.jobs.len(), 1);
    }
}
