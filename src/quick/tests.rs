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
    }
}

#[tokio::test]
async fn server_quick_job_publishes_note_without_moving_source() {
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path(), true).await;
    let source = temp.path().join("media/server.m4a");
    std::fs::write(&source, b"audio").unwrap();
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
async fn uploaded_quick_job_cleans_staging_and_retains_client_markdown() {
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path(), true).await;
    let staged = state.config.quick_upload_dir().join("upload.m4a");
    std::fs::write(&staged, b"audio").unwrap();
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
    assert!(!staged.exists());
    assert!(finished.markdown_path.as_ref().unwrap().is_file());
    assert_eq!(
        finished.quick.as_ref().unwrap().result_name.as_deref(),
        Some("hello from provider.md")
    );
    assert!(finished.archive_path.is_none());
}

#[tokio::test]
async fn failed_uploaded_quick_job_removes_staging() {
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path(), false).await;
    let staged = state.config.quick_upload_dir().join("failure.m4a");
    std::fs::write(&staged, b"audio").unwrap();
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
    assert!(!staged.exists());
}
