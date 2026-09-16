use crate::{
    domain::{
        Job, JobStatus, Provider, TranscriptionAttempt, TranscriptionAttemptOutcome,
        TranscriptionRoute, Workflow,
    },
    jobs::{get_job, now_ms, update_job},
    provider::{self, TranscriptionError},
    quick::QuickOptions,
    state::AppState,
};
use anyhow::{Result, anyhow, bail};
use std::{path::Path, time::Duration};
use tokio_util::sync::CancellationToken;

fn normalize_explicit_routes(
    routes: &[TranscriptionRoute],
    providers: &[Provider],
) -> Result<Vec<TranscriptionRoute>> {
    let mut normalized = Vec::with_capacity(routes.len());
    for (index, route) in routes.iter().enumerate() {
        let provider_id = route.provider_id.trim();
        let model = route.model.trim();
        if provider_id.is_empty() {
            bail!("transcription route {} providerId is required", index + 1);
        }
        if model.is_empty() {
            bail!("transcription route {} model is required", index + 1);
        }
        if route.fallback_after_seconds == Some(0) {
            bail!(
                "transcription route {} fallback timeout must be greater than zero",
                index + 1
            );
        }
        if !providers.iter().any(|provider| provider.id == provider_id) {
            bail!(
                "unknown providerId in transcription route {}: {provider_id}",
                index + 1
            );
        }
        normalized.push(TranscriptionRoute {
            provider_id: provider_id.to_owned(),
            model: model.to_owned(),
            fallback_after_seconds: route.fallback_after_seconds,
        });
    }
    Ok(normalized)
}

fn normalize_legacy_route(
    provider_id: &str,
    model: &str,
    providers: &[Provider],
) -> Result<Vec<TranscriptionRoute>> {
    let provider_id = provider_id.trim();
    if provider_id.is_empty() {
        bail!("providerId is required");
    }
    let provider = providers
        .iter()
        .find(|provider| provider.id == provider_id)
        .ok_or_else(|| anyhow!("unknown providerId: {provider_id}"))?;
    let model = if model.trim().is_empty() {
        provider.model.trim()
    } else {
        model.trim()
    };
    if model.is_empty() {
        bail!("model is required");
    }
    Ok(vec![TranscriptionRoute {
        provider_id: provider_id.to_owned(),
        model: model.to_owned(),
        fallback_after_seconds: None,
    }])
}

pub fn normalize_workflow_chain(
    workflow: &Workflow,
    providers: &[Provider],
) -> Result<Vec<TranscriptionRoute>> {
    if workflow.transcription_chain.is_empty() {
        normalize_legacy_route(&workflow.provider_id, &workflow.model, providers)
    } else {
        normalize_explicit_routes(&workflow.transcription_chain, providers)
    }
}

pub fn normalize_quick_chain(
    options: &QuickOptions,
    providers: &[Provider],
) -> Result<Vec<TranscriptionRoute>> {
    if options.transcription_chain.is_empty() {
        normalize_legacy_route(&options.provider_id, &options.model, providers)
    } else {
        normalize_explicit_routes(&options.transcription_chain, providers)
    }
}

pub fn effective_job_chain(job: &Job) -> Result<Vec<TranscriptionRoute>> {
    if !job.transcription_chain.is_empty() {
        let mut normalized = Vec::with_capacity(job.transcription_chain.len());
        for (index, route) in job.transcription_chain.iter().enumerate() {
            let provider_id = route.provider_id.trim();
            let model = route.model.trim();
            if provider_id.is_empty() || model.is_empty() {
                bail!("persisted transcription route {} is invalid", index + 1);
            }
            if route.fallback_after_seconds == Some(0) {
                bail!(
                    "persisted transcription route {} has invalid timeout",
                    index + 1
                );
            }
            normalized.push(TranscriptionRoute {
                provider_id: provider_id.to_owned(),
                model: model.to_owned(),
                fallback_after_seconds: route.fallback_after_seconds,
            });
        }
        return Ok(normalized);
    }
    let provider_id = job.provider_id.trim();
    let model = job.model.trim();
    if provider_id.is_empty() || model.is_empty() {
        bail!("legacy persisted job has no usable provider/model");
    }
    Ok(vec![TranscriptionRoute {
        provider_id: provider_id.to_owned(),
        model: model.to_owned(),
        fallback_after_seconds: None,
    }])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptionSuccess {
    pub transcript: String,
    pub provider_id: String,
    pub provider_name: String,
    pub model: String,
}

fn safe_attempt_error(value: impl ToString) -> String {
    let flattened = value.to_string().replace(['\r', '\n'], " ");
    flattened.chars().take(300).collect()
}

fn append_attempt(state: &AppState, job_id: &str, attempt: TranscriptionAttempt) -> Result<()> {
    update_job(state, job_id, |job| {
        job.transcription_attempts.push(attempt)
    })?;
    Ok(())
}

fn finished_attempt(
    run: u32,
    route_index: usize,
    route: &TranscriptionRoute,
    provider_name: &str,
    started_at_ms: u128,
    outcome: TranscriptionAttemptOutcome,
    error: Option<String>,
) -> TranscriptionAttempt {
    TranscriptionAttempt {
        run,
        route_index,
        provider_id: route.provider_id.clone(),
        provider_name: provider_name.to_owned(),
        model: route.model.clone(),
        started_at_ms,
        finished_at_ms: now_ms(),
        outcome,
        error,
    }
}

pub async fn transcribe_job_chain(
    state: &AppState,
    job_id: &str,
    source: &Path,
    language: Option<&str>,
    token: &CancellationToken,
) -> Result<TranscriptionSuccess> {
    let job = get_job(state, job_id)?;
    let routes = effective_job_chain(&job)?;
    let run = job.attempts.max(1);
    let mut failures = Vec::new();

    for (route_index, route) in routes.iter().enumerate() {
        if token.is_cancelled() {
            bail!("cancelled");
        }

        let provider = state
            .providers
            .read()
            .await
            .iter()
            .find(|provider| provider.id == route.provider_id)
            .cloned();
        let started_at_ms = now_ms();
        let Some(provider) = provider else {
            let message = "provider is no longer configured".to_owned();
            append_attempt(
                state,
                job_id,
                finished_attempt(
                    run,
                    route_index,
                    route,
                    &route.provider_id,
                    started_at_ms,
                    TranscriptionAttemptOutcome::Failed,
                    Some(message.clone()),
                ),
            )?;
            failures.push(format!("{}/{}: {message}", route.provider_id, route.model));
            continue;
        };
        if !provider.enabled {
            let message = "provider is disabled".to_owned();
            append_attempt(
                state,
                job_id,
                finished_attempt(
                    run,
                    route_index,
                    route,
                    &provider.name,
                    started_at_ms,
                    TranscriptionAttemptOutcome::Failed,
                    Some(message.clone()),
                ),
            )?;
            failures.push(format!("{}/{}: {message}", provider.name, route.model));
            continue;
        }

        update_job(state, job_id, |job| {
            job.status = JobStatus::Transcribing;
            job.error = None;
        })?;

        let request = provider::transcribe(
            &provider,
            &route.model,
            language,
            source,
            &state.http,
            token,
        );
        let result = if let Some(seconds) = route.fallback_after_seconds {
            match tokio::time::timeout(Duration::from_secs(seconds), request).await {
                Ok(result) => result,
                Err(_) => {
                    let message = format!("fallback timeout after {seconds}s");
                    append_attempt(
                        state,
                        job_id,
                        finished_attempt(
                            run,
                            route_index,
                            route,
                            &provider.name,
                            started_at_ms,
                            TranscriptionAttemptOutcome::TimedOut,
                            Some(message.clone()),
                        ),
                    )?;
                    failures.push(format!("{}/{}: {message}", provider.name, route.model));
                    continue;
                }
            }
        } else {
            request.await
        };

        match result {
            Ok(transcript) => {
                let attempt = finished_attempt(
                    run,
                    route_index,
                    route,
                    &provider.name,
                    started_at_ms,
                    TranscriptionAttemptOutcome::Success,
                    None,
                );
                update_job(state, job_id, |job| {
                    job.transcription_attempts.push(attempt);
                    job.used_provider_id = Some(provider.id.clone());
                    job.used_provider_name = Some(provider.name.clone());
                    job.used_model = Some(route.model.clone());
                })?;
                return Ok(TranscriptionSuccess {
                    transcript,
                    provider_id: provider.id,
                    provider_name: provider.name,
                    model: route.model.clone(),
                });
            }
            Err(TranscriptionError::Cancelled) => {
                append_attempt(
                    state,
                    job_id,
                    finished_attempt(
                        run,
                        route_index,
                        route,
                        &provider.name,
                        started_at_ms,
                        TranscriptionAttemptOutcome::Cancelled,
                        Some("cancelled".into()),
                    ),
                )?;
                bail!("cancelled");
            }
            Err(TranscriptionError::Io(error)) => {
                let message = safe_attempt_error(&error);
                append_attempt(
                    state,
                    job_id,
                    finished_attempt(
                        run,
                        route_index,
                        route,
                        &provider.name,
                        started_at_ms,
                        TranscriptionAttemptOutcome::Failed,
                        Some(message.clone()),
                    ),
                )?;
                return Err(error.into());
            }
            Err(error) => {
                if token.is_cancelled() {
                    append_attempt(
                        state,
                        job_id,
                        finished_attempt(
                            run,
                            route_index,
                            route,
                            &provider.name,
                            started_at_ms,
                            TranscriptionAttemptOutcome::Cancelled,
                            Some("cancelled".into()),
                        ),
                    )?;
                    bail!("cancelled");
                }
                let message = safe_attempt_error(&error);
                append_attempt(
                    state,
                    job_id,
                    finished_attempt(
                        run,
                        route_index,
                        route,
                        &provider.name,
                        started_at_ms,
                        TranscriptionAttemptOutcome::Failed,
                        Some(message.clone()),
                    ),
                )?;
                failures.push(format!("{}/{}: {message}", provider.name, route.model));
            }
        }
    }

    bail!("all transcription routes failed: {}", failures.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MarkdownOptions, Provider, TranscriptionRoute, Workflow};

    fn provider(id: &str, model: &str) -> Provider {
        Provider {
            id: id.into(),
            name: id.into(),
            transcription_url: "http://localhost/v1/audio/transcriptions".into(),
            model: model.into(),
            api_key: String::new(),
            timeout_seconds: 600,
            enabled: true,
        }
    }

    fn route(provider_id: &str, model: &str) -> TranscriptionRoute {
        TranscriptionRoute {
            provider_id: provider_id.into(),
            model: model.into(),
            fallback_after_seconds: None,
        }
    }

    fn workflow_with_chain(transcription_chain: Vec<TranscriptionRoute>) -> Workflow {
        Workflow {
            id: "w".into(),
            name: "test".into(),
            watch_dir: "/w".into(),
            output_dir: None,
            archive_dir: "/a".into(),
            tags: vec![],
            provider_id: "p".into(),
            model: String::new(),
            transcription_chain,
            language: None,
            markdown: MarkdownOptions::default(),
            enabled: true,
        }
    }

    #[test]
    fn legacy_workflow_becomes_one_route_with_provider_default_model() {
        let workflow: Workflow = serde_json::from_str(
            r#"{
          "id":"w","name":"old","watchDir":"/w","archiveDir":"/a",
          "providerId":"p","model":"","enabled":true
        }"#,
        )
        .unwrap();
        let providers = vec![provider("p", "whisper-large-v3")];
        let chain = normalize_workflow_chain(&workflow, &providers).unwrap();
        assert_eq!(
            chain,
            vec![TranscriptionRoute {
                provider_id: "p".into(),
                model: "whisper-large-v3".into(),
                fallback_after_seconds: None,
            }]
        );
    }

    #[test]
    fn same_provider_can_appear_twice_with_different_models() {
        let routes = vec![route("p", "large-v3"), route("p", "distil-large-v3")];
        let workflow = workflow_with_chain(routes.clone());
        assert_eq!(
            normalize_workflow_chain(&workflow, &[provider("p", "default")]).unwrap(),
            routes
        );
    }

    #[test]
    fn explicit_routes_are_strict_but_allow_disabled_provider() {
        let mut p = provider("p", "default");
        p.enabled = false;
        let mut workflow = workflow_with_chain(vec![route(" p ", " large-v3 ")]);
        workflow.transcription_chain[0].fallback_after_seconds = Some(30);
        assert_eq!(
            normalize_workflow_chain(&workflow, &[p]).unwrap(),
            vec![TranscriptionRoute {
                provider_id: "p".into(),
                model: "large-v3".into(),
                fallback_after_seconds: Some(30),
            }]
        );
        workflow.transcription_chain[0].fallback_after_seconds = Some(0);
        assert!(normalize_workflow_chain(&workflow, &[provider("p", "default")]).is_err());
    }

    use crate::{
        config::Config,
        domain::{Job, JobKind, JobStatus},
        jobs::{get_job, persist_job},
        state::AppState,
    };
    use axum::{Json, Router, body::Bytes, http::StatusCode, routing::post};
    use serde_json::json;
    use std::{
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };
    use tokio_util::sync::CancellationToken;

    async fn mock_provider(
        status: StatusCode,
        text: &'static str,
        delay_ms: u64,
        calls: Arc<AtomicUsize>,
    ) -> String {
        let app = Router::new().route("/v1/audio/transcriptions", post(move |_body: Bytes| {
            let calls = calls.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                if delay_ms > 0 { tokio::time::sleep(Duration::from_millis(delay_ms)).await; }
                (status, Json(json!({"text": text, "error": if status.is_success() { "" } else { text }})))
            }
        }));
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

    async fn state_with_job(
        root: &Path,
        providers: Vec<Provider>,
        routes: Vec<TranscriptionRoute>,
    ) -> (AppState, Job, PathBuf) {
        let state = AppState::load(test_config(root)).await.unwrap();
        for provider in &providers {
            state.db.upsert("provider", &provider.id, provider).unwrap();
        }
        *state.providers.write().await = providers;
        let source = root.join("audio.m4a");
        std::fs::write(&source, b"audio").unwrap();
        let primary = routes.first().unwrap().clone();
        let now = crate::jobs::now_ms();
        let job = Job {
            id: "chain-job".into(),
            kind: JobKind::Quick,
            workflow_id: None,
            quick: None,
            provider_id: primary.provider_id.clone(),
            transcription_chain: routes,
            transcription_attempts: vec![],
            used_provider_id: None,
            used_provider_name: None,
            used_model: None,
            original_name: "audio.m4a".into(),
            source_path: source.clone(),
            source_size: 5,
            source_mtime_ns: 1,
            model: primary.model,
            language: None,
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
        persist_job(&state, &job).unwrap();
        (state, job, source)
    }

    fn test_provider(id: &str, url: String, enabled: bool) -> Provider {
        Provider {
            id: id.into(),
            name: id.into(),
            transcription_url: url,
            model: "default".into(),
            api_key: String::new(),
            timeout_seconds: 1,
            enabled,
        }
    }

    #[tokio::test]
    async fn primary_failure_then_fallback_success_records_both_attempts() {
        let temp = tempfile::tempdir().unwrap();
        let c1 = Arc::new(AtomicUsize::new(0));
        let c2 = Arc::new(AtomicUsize::new(0));
        let p1 = test_provider(
            "p1",
            mock_provider(StatusCode::BAD_GATEWAY, "down", 0, c1.clone()).await,
            true,
        );
        let p2 = test_provider(
            "p2",
            mock_provider(StatusCode::OK, "fallback works", 0, c2.clone()).await,
            true,
        );
        let routes = vec![route("p1", "m1"), route("p2", "m2")];
        let (state, job, source) = state_with_job(temp.path(), vec![p1, p2], routes).await;
        let result =
            transcribe_job_chain(&state, &job.id, &source, None, &CancellationToken::new())
                .await
                .unwrap();
        assert_eq!(result.transcript, "fallback works");
        assert_eq!(result.provider_id, "p2");
        assert_eq!(result.model, "m2");
        let saved = get_job(&state, &job.id).unwrap();
        assert_eq!(saved.transcription_attempts.len(), 2);
        assert_eq!(
            saved.transcription_attempts[0].outcome,
            crate::domain::TranscriptionAttemptOutcome::Failed
        );
        assert_eq!(
            saved.transcription_attempts[1].outcome,
            crate::domain::TranscriptionAttemptOutcome::Success
        );
        assert_eq!(c1.load(Ordering::SeqCst), 1);
        assert_eq!(c2.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn disabled_or_missing_provider_falls_through() {
        let temp = tempfile::tempdir().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let disabled = test_provider(
            "disabled",
            "http://127.0.0.1:1/v1/audio/transcriptions".into(),
            false,
        );
        let good = test_provider(
            "good",
            mock_provider(StatusCode::OK, "ok", 0, calls.clone()).await,
            true,
        );
        let routes = vec![
            route("missing", "m0"),
            route("disabled", "m1"),
            route("good", "m2"),
        ];
        let (state, job, source) = state_with_job(temp.path(), vec![disabled, good], routes).await;
        let result =
            transcribe_job_chain(&state, &job.id, &source, None, &CancellationToken::new())
                .await
                .unwrap();
        assert_eq!(result.provider_id, "good");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            get_job(&state, &job.id)
                .unwrap()
                .transcription_attempts
                .len(),
            3
        );
    }

    #[tokio::test]
    async fn route_timeout_falls_through() {
        let temp = tempfile::tempdir().unwrap();
        let slow = Arc::new(AtomicUsize::new(0));
        let fast = Arc::new(AtomicUsize::new(0));
        let p1 = test_provider(
            "slow",
            mock_provider(StatusCode::OK, "too late", 1500, slow.clone()).await,
            true,
        );
        let p2 = test_provider(
            "fast",
            mock_provider(StatusCode::OK, "fast", 0, fast.clone()).await,
            true,
        );
        let mut first = route("slow", "m1");
        first.fallback_after_seconds = Some(1);
        let (state, job, source) =
            state_with_job(temp.path(), vec![p1, p2], vec![first, route("fast", "m2")]).await;
        let result =
            transcribe_job_chain(&state, &job.id, &source, None, &CancellationToken::new())
                .await
                .unwrap();
        assert_eq!(result.transcript, "fast");
        let saved = get_job(&state, &job.id).unwrap();
        assert_eq!(
            saved.transcription_attempts[0].outcome,
            crate::domain::TranscriptionAttemptOutcome::TimedOut
        );
        assert_eq!(slow.load(Ordering::SeqCst), 1);
        assert_eq!(fast.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn no_route_timeout_waits_for_slow_success() {
        let temp = tempfile::tempdir().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let p = test_provider(
            "slow",
            mock_provider(StatusCode::OK, "eventually", 150, calls.clone()).await,
            true,
        );
        let (state, job, source) =
            state_with_job(temp.path(), vec![p], vec![route("slow", "m1")]).await;
        let result =
            transcribe_job_chain(&state, &job.id, &source, None, &CancellationToken::new())
                .await
                .unwrap();
        assert_eq!(result.transcript, "eventually");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn cancellation_stops_chain_before_second_provider() {
        let temp = tempfile::tempdir().unwrap();
        let slow = Arc::new(AtomicUsize::new(0));
        let second = Arc::new(AtomicUsize::new(0));
        let p1 = test_provider(
            "slow",
            mock_provider(StatusCode::OK, "late", 1000, slow.clone()).await,
            true,
        );
        let p2 = test_provider(
            "second",
            mock_provider(StatusCode::OK, "should not run", 0, second.clone()).await,
            true,
        );
        let (state, job, source) = state_with_job(
            temp.path(),
            vec![p1, p2],
            vec![route("slow", "m1"), route("second", "m2")],
        )
        .await;
        let token = CancellationToken::new();
        let cancel = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancel.cancel();
        });
        assert!(
            transcribe_job_chain(&state, &job.id, &source, None, &token)
                .await
                .is_err()
        );
        assert_eq!(second.load(Ordering::SeqCst), 0);
        assert_eq!(
            get_job(&state, &job.id)
                .unwrap()
                .transcription_attempts
                .last()
                .unwrap()
                .outcome,
            crate::domain::TranscriptionAttemptOutcome::Cancelled
        );
    }

    #[tokio::test]
    async fn all_routes_fail_returns_aggregate_error_and_history() {
        let temp = tempfile::tempdir().unwrap();
        let c1 = Arc::new(AtomicUsize::new(0));
        let c2 = Arc::new(AtomicUsize::new(0));
        let p1 = test_provider(
            "p1",
            mock_provider(StatusCode::BAD_GATEWAY, "first down", 0, c1).await,
            true,
        );
        let p2 = test_provider(
            "p2",
            mock_provider(StatusCode::TOO_MANY_REQUESTS, "rate limited", 0, c2).await,
            true,
        );
        let (state, job, source) = state_with_job(
            temp.path(),
            vec![p1, p2],
            vec![route("p1", "m1"), route("p2", "m2")],
        )
        .await;
        let error = transcribe_job_chain(&state, &job.id, &source, None, &CancellationToken::new())
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("p1/m1"));
        assert!(error.contains("p2/m2"));
        assert_eq!(
            get_job(&state, &job.id)
                .unwrap()
                .transcription_attempts
                .len(),
            2
        );
    }
}
