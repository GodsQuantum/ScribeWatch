use super::*;
use crate::{
    config::Config,
    domain::{JobStatus, Provider, QuickOutputKind},
    pipeline,
    state::AppState,
};
use axum::{Json, Router, body::Bytes, http::StatusCode, routing::post};
use serde_json::json;
use std::path::Path;
use tokio_util::sync::CancellationToken;

fn tiny_wav() -> Vec<u8> {
    const SAMPLE_RATE: u32 = 16_000;
    const SAMPLES: u32 = 1_600;
    const DATA_BYTES: u32 = SAMPLES * 2;
    let mut bytes = Vec::with_capacity((44 + DATA_BYTES) as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + DATA_BYTES).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&DATA_BYTES.to_le_bytes());
    bytes.resize((44 + DATA_BYTES) as usize, 0);
    bytes
}

fn config(root: &Path) -> Config {
    Config {
        host: "127.0.0.1".into(),
        port: 0,
        config_dir: root.join("config"),
        dist_dir: root.join("dist"),
        data_dir: root.join("data"),
        allowed_roots: vec![root.join("media")],
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

async fn provider_url(ok: bool) -> String {
    let app = if ok {
        Router::new().route(
            "/v1/audio/transcriptions",
            post(|_body: Bytes| async {
                (StatusCode::OK, Json(json!({"text":"hello from provider"})))
            }),
        )
    } else {
        Router::new().route(
            "/v1/audio/transcriptions",
            post(|_body: Bytes| async {
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"error":"provider down"})),
                )
            }),
        )
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{address}/v1/audio/transcriptions")
}

async fn state(root: &Path, ok: bool) -> AppState {
    std::fs::create_dir_all(root.join("media/notes")).unwrap();
    let state = AppState::load(config(root)).await.unwrap();
    let provider = Provider {
        id: "provider".into(),
        name: "Provider".into(),
        transcription_url: provider_url(ok).await,
        model: "model".into(),
        api_key: String::new(),
        timeout_seconds: 5,
        enabled: true,
    };
    state
        .db
        .upsert("provider", &provider.id, &provider)
        .unwrap();
    state.providers.write().await.push(provider);
    state
}

fn options(output_kind: QuickOutputKind, output_dir: Option<String>) -> QuickOptions {
    QuickOptions {
        provider_id: "provider".into(),
        model: String::new(),
        transcription_chain: Vec::new(),
        language: None,
        output_kind,
        output_dir,
        frontmatter: true,
        paragraphs: true,
        structure_profile_id: None,
    }
}

#[tokio::test]
async fn server_quick_job_publishes_note_without_moving_source() {
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path(), true).await;
    let source = temp.path().join("media/server.m4a");
    std::fs::write(&source, tiny_wav()).unwrap();
    let notes = temp.path().join("media/notes");
    let job = create_server_job(
        &state,
        &source,
        options(
            QuickOutputKind::Server,
            Some(notes.to_string_lossy().into_owned()),
        ),
    )
    .await
    .unwrap();
    pipeline::process_job(&state, &job.id, &CancellationToken::new())
        .await
        .unwrap();
    let finished = crate::jobs::get_job(&state, &job.id).unwrap();
    assert_eq!(finished.status, JobStatus::Done);
    assert!(source.is_file());
    assert!(notes.join("hello from provider.md").is_file());
    assert!(finished.archive_path.is_none());
}

#[tokio::test]
async fn uploaded_quick_job_retains_source_and_client_markdown() {
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path(), true).await;
    let staged = state.config.quick_upload_dir().join("upload.m4a");
    std::fs::write(&staged, tiny_wav()).unwrap();
    let job = create_uploaded_job(
        &state,
        staged.clone(),
        "voice note.m4a".into(),
        options(QuickOutputKind::Client, None),
    )
    .await
    .unwrap();
    pipeline::process_job(&state, &job.id, &CancellationToken::new())
        .await
        .unwrap();
    let finished = crate::jobs::get_job(&state, &job.id).unwrap();
    assert_eq!(finished.status, JobStatus::Done);
    assert!(staged.exists());
    assert!(finished.markdown_path.as_ref().unwrap().is_file());
    assert_eq!(
        finished.quick.as_ref().unwrap().result_name.as_deref(),
        Some("hello from provider.md")
    );
    assert!(finished.archive_path.is_none());
}

#[tokio::test]
async fn failed_uploaded_quick_job_retains_source_for_retry() {
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path(), false).await;
    let staged = state.config.quick_upload_dir().join("failure.m4a");
    std::fs::write(&staged, tiny_wav()).unwrap();
    let job = create_uploaded_job(
        &state,
        staged.clone(),
        "failure.m4a".into(),
        options(QuickOutputKind::Client, None),
    )
    .await
    .unwrap();
    assert!(
        pipeline::process_job(&state, &job.id, &CancellationToken::new())
            .await
            .is_err()
    );
    assert!(staged.exists());
}

#[tokio::test]
async fn restart_interrupted_quick_job_keeps_source_during_cleanup() {
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path(), true).await;
    let staged = state.config.quick_upload_dir().join("restart.m4a");
    std::fs::write(&staged, tiny_wav()).unwrap();
    let job = create_uploaded_job(
        &state,
        staged.clone(),
        "restart.m4a".into(),
        options(QuickOutputKind::Client, None),
    )
    .await
    .unwrap();
    drop(state);

    let restarted = AppState::load(config(temp.path())).await.unwrap();
    let restored = crate::jobs::get_job(&restarted, &job.id).unwrap();
    assert_eq!(restored.status, JobStatus::Interrupted);
    cleanup_stale(&restarted).unwrap();
    assert!(staged.exists());
}

#[tokio::test]
async fn expired_terminal_upload_becomes_cleanup_eligible() {
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path(), true).await;
    let staged = state.config.quick_upload_dir().join("expired.m4a");
    std::fs::write(&staged, tiny_wav()).unwrap();
    let job = create_uploaded_job(
        &state,
        staged.clone(),
        "expired.m4a".into(),
        options(QuickOutputKind::Client, None),
    )
    .await
    .unwrap();
    let mut expired = crate::jobs::get_job(&state, &job.id).unwrap();
    expired.status = JobStatus::Done;
    expired.updated_at_ms = 0;
    state.jobs.insert(expired.id.clone(), expired.clone());
    crate::jobs::persist_job(&state, &expired).unwrap();

    cleanup_stale(&state).unwrap();
    assert!(!staged.exists());
}

#[tokio::test]
async fn deleted_upload_job_becomes_cleanup_eligible() {
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path(), true).await;
    let staged = state.config.quick_upload_dir().join("deleted.m4a");
    std::fs::write(&staged, tiny_wav()).unwrap();
    let job = create_uploaded_job(
        &state,
        staged.clone(),
        "deleted.m4a".into(),
        options(QuickOutputKind::Client, None),
    )
    .await
    .unwrap();
    state.jobs.remove(&job.id);
    state.db.delete("job", &job.id).unwrap();

    cleanup_stale(&state).unwrap();
    assert!(!staged.exists());
}

#[tokio::test]
async fn expired_terminal_live_source_becomes_cleanup_eligible() {
    use crate::domain::LiveJobMeta;
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path(), true).await;
    let session = uuid::Uuid::new_v4().to_string();
    let source = state
        .config
        .live_source_dir()
        .join(format!("{session}.m4a"));
    std::fs::write(&source, tiny_wav()).unwrap();
    let job = create_live_job(
        &state,
        source.clone(),
        "consultation.m4a".into(),
        LiveJobMeta {
            session_id: session,
            started_at_ms: 1_700_000_000_000,
            audio_result_name: "consultation.m4a".into(),
            interruption_gaps_ms: Vec::new(),
        },
        options(QuickOutputKind::Client, None),
    )
    .await
    .unwrap();
    let mut expired = crate::jobs::get_job(&state, &job.id).unwrap();
    expired.status = JobStatus::Done;
    expired.updated_at_ms = 0;
    state.jobs.insert(expired.id.clone(), expired.clone());
    crate::jobs::persist_job(&state, &expired).unwrap();

    let stats = cleanup_stale(&state).unwrap();
    assert_eq!(stats.live_sources_removed, 1);
    assert!(!source.exists());
}

#[tokio::test]
async fn referenced_live_source_is_preserved_within_retention_window() {
    use crate::domain::LiveJobMeta;
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path(), true).await;
    let session = uuid::Uuid::new_v4().to_string();
    let source = state
        .config
        .live_source_dir()
        .join(format!("{session}.m4a"));
    std::fs::write(&source, tiny_wav()).unwrap();
    let job = create_live_job(
        &state,
        source.clone(),
        "consultation.m4a".into(),
        LiveJobMeta {
            session_id: session,
            started_at_ms: 1_700_000_000_000,
            audio_result_name: "consultation.m4a".into(),
            interruption_gaps_ms: Vec::new(),
        },
        options(QuickOutputKind::Client, None),
    )
    .await
    .unwrap();
    let mut done = crate::jobs::get_job(&state, &job.id).unwrap();
    done.status = JobStatus::Done;
    state.jobs.insert(done.id.clone(), done.clone());
    crate::jobs::persist_job(&state, &done).unwrap();

    cleanup_stale(&state).unwrap();
    assert!(source.exists());
}

#[tokio::test]
async fn stale_live_staging_directory_is_removed() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("media/notes")).unwrap();
    let mut cfg = config(temp.path());
    cfg.quick_source_retention_hours = 0;
    let state = AppState::load(cfg).await.unwrap();
    let staging = state.config.live_staging_dir().join("orphan");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::write(staging.join("segment.media"), tiny_wav()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let stats = cleanup_stale(&state).unwrap();
    assert_eq!(stats.live_staging_dirs_removed, 1);
    assert!(!staging.exists());
}
