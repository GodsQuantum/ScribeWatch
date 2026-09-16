use crate::{
    domain::{Job, Provider, TranscriptionRoute, Workflow},
    quick::QuickOptions,
};
use anyhow::{Result, anyhow, bail};

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
}
