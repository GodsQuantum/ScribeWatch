use crate::{
    audio,
    domain::{Job, LiveJobMeta},
    error::{AppError, AppResult},
    jobs,
    live::{self as live_core, LiveUploadManifest},
    quick,
    state::AppState,
};
use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
};
use std::{collections::HashMap, path::PathBuf};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MAX_MANIFEST_BYTES: usize = 64 * 1024;

fn parse_segment_field(name: &str) -> Option<u32> {
    let raw = name.strip_prefix("segment-")?;
    if raw.len() != 4 || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let id = raw.parse::<u32>().ok()?;
    (id > 0 && format!("segment-{id:04}") == name).then_some(id)
}

async fn remove_staging(path: &PathBuf) {
    let _ = tokio::fs::remove_dir_all(path).await;
}

pub async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<Job>)> {
    let first = multipart
        .next_field()
        .await
        .map_err(|error| AppError::BadRequest(format!("invalid LIVE multipart upload: {error}")))?
        .ok_or_else(|| AppError::BadRequest("LIVE manifest is required".into()))?;
    if first.name() != Some("manifest") {
        return Err(AppError::BadRequest(
            "manifest must be the first LIVE multipart field".into(),
        ));
    }
    let mut first = first;
    let mut manifest_bytes = Vec::new();
    while let Some(chunk) = first
        .chunk()
        .await
        .map_err(|error| AppError::BadRequest(format!("invalid LIVE manifest: {error}")))?
    {
        if manifest_bytes.len().saturating_add(chunk.len()) > MAX_MANIFEST_BYTES {
            return Err(AppError::BadRequest(
                "LIVE manifest exceeds 64 KiB limit".into(),
            ));
        }
        manifest_bytes.extend_from_slice(&chunk);
    }
    drop(first);
    let manifest: LiveUploadManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| AppError::BadRequest(format!("invalid LIVE manifest JSON: {error}")))?;
    let session_id = live_core::validate_manifest(&manifest)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    live_core::resolve_server_destination(&state.config, &manifest.retention)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;

    if let Some(existing) = quick::find_live_job_by_session(&state, &session_id.to_string()) {
        if existing.source_path.is_file() {
            return Ok((StatusCode::ACCEPTED, Json(existing)));
        }
        return Err(AppError::Conflict(
            "LIVE session was already accepted but its retained source is missing".into(),
        ));
    }

    let staging = state
        .config
        .live_staging_dir()
        .join(Uuid::new_v4().to_string());
    tokio::fs::create_dir(&staging)
        .await
        .map_err(AppError::from)?;
    let mut paths = HashMap::<u32, PathBuf>::new();
    let mut total_bytes = manifest_bytes.len() as u64;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(error) => {
                remove_staging(&staging).await;
                return Err(AppError::BadRequest(format!(
                    "LIVE multipart upload interrupted: {error}"
                )));
            }
        };
        let name = field.name().unwrap_or_default().to_owned();
        let Some(id) = parse_segment_field(&name) else {
            remove_staging(&staging).await;
            return Err(AppError::BadRequest(format!(
                "unexpected LIVE multipart field: {name}"
            )));
        };
        if !manifest.segments.iter().any(|segment| segment.id == id) {
            remove_staging(&staging).await;
            return Err(AppError::BadRequest(format!(
                "LIVE segment {id} is not declared in the manifest"
            )));
        }
        if paths.contains_key(&id) {
            remove_staging(&staging).await;
            return Err(AppError::BadRequest(format!(
                "LIVE segment {id} was uploaded more than once"
            )));
        }

        let partial = staging.join(format!("segment-{id:04}.uploading"));
        let final_path = staging.join(format!("segment-{id:04}.media"));
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .await
            .map_err(AppError::from)?;
        let mut field = field;
        let mut segment_bytes = 0u64;
        loop {
            match field.chunk().await {
                Ok(Some(chunk)) => {
                    total_bytes = total_bytes.saturating_add(chunk.len() as u64);
                    segment_bytes = segment_bytes.saturating_add(chunk.len() as u64);
                    if total_bytes > state.config.max_upload_bytes {
                        drop(file);
                        remove_staging(&staging).await;
                        return Err(AppError::BadRequest(
                            "LIVE upload exceeds configured size limit".into(),
                        ));
                    }
                    if let Err(error) = file.write_all(&chunk).await {
                        drop(file);
                        remove_staging(&staging).await;
                        return Err(AppError::Internal(error.into()));
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    drop(file);
                    remove_staging(&staging).await;
                    return Err(AppError::BadRequest(format!(
                        "LIVE segment upload interrupted: {error}"
                    )));
                }
            }
        }
        if segment_bytes == 0 {
            drop(file);
            remove_staging(&staging).await;
            return Err(AppError::BadRequest(format!("LIVE segment {id} is empty")));
        }
        file.flush().await.map_err(AppError::from)?;
        file.sync_all().await.map_err(AppError::from)?;
        drop(file);
        tokio::fs::rename(&partial, &final_path)
            .await
            .map_err(AppError::from)?;
        paths.insert(id, final_path);
    }

    if paths.len() != manifest.segments.len()
        || manifest
            .segments
            .iter()
            .any(|segment| !paths.contains_key(&segment.id))
    {
        remove_staging(&staging).await;
        return Err(AppError::BadRequest(
            "LIVE upload is missing one or more declared segments".into(),
        ));
    }

    let probe_token = CancellationToken::new();
    for segment in &manifest.segments {
        let path = paths.get(&segment.id).expect("validated LIVE segment path");
        match audio::probe_audio(path, &probe_token).await {
            Ok(true) => {}
            Ok(false) => {
                remove_staging(&staging).await;
                return Err(AppError::BadRequest(format!(
                    "LIVE segment {} does not contain decodable audio",
                    segment.id
                )));
            }
            Err(error) => {
                remove_staging(&staging).await;
                return Err(AppError::Internal(error));
            }
        }
    }

    let finalized =
        live_core::finalize_segments(&state.config, &manifest, &paths, &CancellationToken::new())
            .await
            .map_err(AppError::Internal)?;

    let live = LiveJobMeta {
        session_id: session_id.to_string(),
        started_at_ms: manifest.started_at_ms,
        audio_result_name: finalized.audio_name.clone(),
        interruption_gaps_ms: manifest
            .segments
            .iter()
            .filter_map(|segment| segment.gap_ms_before)
            .collect(),
    };
    let job = quick::create_live_job(
        &state,
        finalized.source_path,
        finalized.audio_name,
        live,
        manifest.options,
    )
    .await
    .map_err(|error| AppError::BadRequest(error.to_string()))?;

    remove_staging(&staging).await;
    jobs::enqueue(state, job.id.clone()).map_err(AppError::Internal)?;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

#[cfg(test)]
mod tests {
    use crate::{
        config::Config,
        domain::{Job, Provider, QuickOutputKind},
        live::{LiveAudioRetention, LiveUploadManifest, LiveUploadSegment},
        quick::QuickOptions,
        state::AppState,
    };
    use axum::{Json, Router, body::Bytes, routing::post};
    use reqwest::multipart::{Form, Part};
    use serde_json::json;
    use std::path::Path;
    use uuid::Uuid;

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

    async fn fake_provider() -> String {
        let app = Router::new().route(
            "/v1/audio/transcriptions",
            post(|_body: Bytes| async { Json(json!({"text":"live transcript"})) }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}/v1/audio/transcriptions")
    }

    async fn state(root: &Path) -> AppState {
        let allowed = root.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        let config = Config {
            host: "127.0.0.1".into(),
            port: 0,
            config_dir: root.join("config"),
            dist_dir: root.join("dist"),
            data_dir: root.join("data"),
            allowed_roots: vec![allowed],
            scan_seconds: 1,
            file_stability_ms: 20,
            max_transcription_jobs: 1,
            max_upload_bytes: 4 * 1024 * 1024,
            quick_result_retention_hours: 24,
            quick_source_retention_hours: 24,
            normalized_audio_format: "wav".into(),
            ffmpeg_threads: 1,
        };
        let state = AppState::load(config).await.unwrap();
        let provider = Provider {
            id: "provider".into(),
            name: "Provider".into(),
            transcription_url: fake_provider().await,
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

    async fn api_server(state: AppState) -> String {
        let app = crate::api::router().with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn manifest(session_id: &str, segment_count: u32) -> LiveUploadManifest {
        LiveUploadManifest {
            schema_version: 1,
            session_id: session_id.into(),
            started_at_ms: 1_700_000_000_000,
            segments: (1..=segment_count)
                .map(|id| LiveUploadSegment {
                    id,
                    mime_type: "audio/wav".into(),
                    started_at_ms: 1_700_000_000_000 + (id as u128 - 1) * 10_000,
                    ended_at_ms: Some(1_700_000_005_000 + (id as u128 - 1) * 10_000),
                    gap_ms_before: (id > 1).then_some(500),
                })
                .collect(),
            retention: LiveAudioRetention::None,
            options: QuickOptions {
                provider_id: "provider".into(),
                model: "model".into(),
                transcription_chain: Vec::new(),
                language: None,
                output_kind: QuickOutputKind::Client,
                output_dir: None,
                frontmatter: true,
                paragraphs: true,
                structure_profile_id: None,
            },
        }
    }

    fn form(manifest: &LiveUploadManifest, segments: Vec<(u32, Vec<u8>)>) -> Form {
        let mut form = Form::new().text("manifest", serde_json::to_string(manifest).unwrap());
        for (id, bytes) in segments {
            form = form.part(
                format!("segment-{id:04}"),
                Part::bytes(bytes)
                    .file_name(format!("segment-{id:04}.wav"))
                    .mime_str("audio/wav")
                    .unwrap(),
            );
        }
        form
    }

    #[tokio::test]
    async fn live_upload_streams_segments_and_returns_accepted_job() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let base = api_server(state.clone()).await;
        let session = Uuid::new_v4().to_string();
        let response = reqwest::Client::new()
            .post(format!("{base}/api/v1/live/upload"))
            .multipart(form(
                &manifest(&session, 2),
                vec![(1, tiny_wav()), (2, tiny_wav())],
            ))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.bytes().await.unwrap();
        assert_eq!(
            status,
            reqwest::StatusCode::ACCEPTED,
            "unexpected LIVE upload response: {}",
            String::from_utf8_lossy(&body)
        );
        let job: Job = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            job.quick
                .as_ref()
                .unwrap()
                .live
                .as_ref()
                .unwrap()
                .session_id,
            session
        );
        assert!(job.source_path.is_file());
        assert_eq!(job.source_path.extension().unwrap(), "m4a");
    }

    #[tokio::test]
    async fn live_upload_rejects_segment_before_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let base = api_server(state).await;
        let form = Form::new()
            .part("segment-0001", Part::bytes(tiny_wav()).file_name("one.wav"))
            .text(
                "manifest",
                serde_json::to_string(&manifest(&Uuid::new_v4().to_string(), 1)).unwrap(),
            );
        let response = reqwest::Client::new()
            .post(format!("{base}/api/v1/live/upload"))
            .multipart(form)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn live_upload_rejects_duplicate_unknown_or_missing_segment_part() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let base = api_server(state).await;
        let m = manifest(&Uuid::new_v4().to_string(), 2);
        let response = reqwest::Client::new()
            .post(format!("{base}/api/v1/live/upload"))
            .multipart(form(&m, vec![(1, tiny_wav())]))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);

        let response = reqwest::Client::new()
            .post(format!("{base}/api/v1/live/upload"))
            .multipart(form(
                &m,
                vec![(1, tiny_wav()), (1, tiny_wav()), (2, tiny_wav())],
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn live_upload_rejects_invalid_audio_segment() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let base = api_server(state).await;
        let m = manifest(&Uuid::new_v4().to_string(), 1);
        let response = reqwest::Client::new()
            .post(format!("{base}/api/v1/live/upload"))
            .multipart(form(&m, vec![(1, b"not audio".to_vec())]))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn replayed_session_id_returns_existing_job_without_duplicate_copy() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let base = api_server(state.clone()).await;
        let session = Uuid::new_v4().to_string();
        let m = manifest(&session, 1);
        let client = reqwest::Client::new();
        let first = client
            .post(format!("{base}/api/v1/live/upload"))
            .multipart(form(&m, vec![(1, tiny_wav())]))
            .send()
            .await
            .unwrap();
        assert_eq!(first.status(), reqwest::StatusCode::ACCEPTED);
        let first_job: Job = first.json().await.unwrap();

        let second = client
            .post(format!("{base}/api/v1/live/upload"))
            .multipart(form(&m, vec![(1, tiny_wav())]))
            .send()
            .await
            .unwrap();
        assert_eq!(second.status(), reqwest::StatusCode::ACCEPTED);
        let second_job: Job = second.json().await.unwrap();
        assert_eq!(first_job.id, second_job.id);
        assert_eq!(
            state
                .jobs
                .iter()
                .filter(|j| j
                    .quick
                    .as_ref()
                    .and_then(|q| q.live.as_ref())
                    .is_some_and(|l| l.session_id == session))
                .count(),
            1
        );
    }

    async fn direct_live_job(state: &AppState) -> Job {
        use crate::domain::LiveJobMeta;
        let session = Uuid::new_v4().to_string();
        let source = state
            .config
            .live_source_dir()
            .join(format!("{session}.m4a"));
        std::fs::write(&source, tiny_wav()).unwrap();
        crate::quick::create_live_job(
            state,
            source,
            format!("live-{session}.m4a"),
            LiveJobMeta {
                session_id: session,
                started_at_ms: 1_700_000_000_000,
                audio_result_name: "consultation.m4a".into(),
                interruption_gaps_ms: Vec::new(),
            },
            manifest(&Uuid::new_v4().to_string(), 1).options,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn live_job_audio_download_returns_retained_m4a() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let job = direct_live_job(&state).await;
        let base = api_server(state).await;
        let response = reqwest::get(format!("{base}/api/v1/jobs/{}/audio", job.id))
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .unwrap(),
            "audio/mp4"
        );
        assert!(!response.bytes().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn audio_download_rejects_non_live_job() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let source = state.config.quick_upload_dir().join("ordinary.wav");
        std::fs::write(&source, tiny_wav()).unwrap();
        let options = manifest(&Uuid::new_v4().to_string(), 1).options;
        let job = crate::quick::create_uploaded_job(&state, source, "ordinary.wav".into(), options)
            .await
            .unwrap();
        let base = api_server(state).await;
        let response = reqwest::get(format!("{base}/api/v1/jobs/{}/audio", job.id))
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn audio_download_rejects_source_outside_live_source_dir() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let job = direct_live_job(&state).await;
        let outside = state.config.allowed_roots[0].join("outside.m4a");
        std::fs::write(&outside, tiny_wav()).unwrap();
        let mut escaped = job.clone();
        escaped.source_path = outside;
        state.jobs.insert(escaped.id.clone(), escaped.clone());
        crate::jobs::persist_job(&state, &escaped).unwrap();

        let base = api_server(state).await;
        let response = reqwest::get(format!("{base}/api/v1/jobs/{}/audio", job.id))
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn replayed_accepted_session_with_missing_source_returns_conflict() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let base = api_server(state.clone()).await;
        let session = Uuid::new_v4().to_string();
        let m = manifest(&session, 1);
        let client = reqwest::Client::new();
        let first = client
            .post(format!("{base}/api/v1/live/upload"))
            .multipart(form(&m, vec![(1, tiny_wav())]))
            .send()
            .await
            .unwrap();
        let job: Job = first.json().await.unwrap();
        std::fs::remove_file(&job.source_path).unwrap();

        let second = client
            .post(format!("{base}/api/v1/live/upload"))
            .multipart(form(&m, vec![(1, tiny_wav())]))
            .send()
            .await
            .unwrap();
        assert_eq!(second.status(), reqwest::StatusCode::CONFLICT);
    }
}
