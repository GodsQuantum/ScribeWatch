use crate::{
    api::embedded::{EmbeddedManager, Mode, Modes},
    config::Config,
    db::Database,
    domain::{Job, JobEvent, JobStatus, LlmProvider, Provider, StructureProfile, Workflow},
};
use anyhow::Result;
use dashmap::DashMap;
use std::{sync::Arc, time::Duration};
use tokio::sync::{RwLock, Semaphore, broadcast};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub embedded: Arc<EmbeddedManager>,
    pub db: Database,
    pub providers: Arc<RwLock<Vec<Provider>>>,
    pub llm_providers: Arc<RwLock<Vec<LlmProvider>>>,
    pub structure_profiles: Arc<RwLock<Vec<StructureProfile>>>,
    pub workflows: Arc<RwLock<Vec<Workflow>>>,
    pub jobs: Arc<DashMap<String, Job>>,
    pub job_tokens: Arc<DashMap<String, CancellationToken>>,
    pub watcher_tokens: Arc<DashMap<String, (String, CancellationToken)>>,
    pub events: broadcast::Sender<JobEvent>,
    pub http: reqwest::Client,
    pub transcription_slots: Arc<Semaphore>,
}

const AUTO_ASR_ID: &str = "scribewatch-native-parakeet";
pub const AUTO_LLM_ID: &str = "scribewatch-native-qwen4b";

fn native_zero_config_eligible(
    config: &Config,
    providers: &[Provider],
    llm_providers: &[LlmProvider],
    profiles: &[StructureProfile],
    has_no_engine_selection: bool,
) -> bool {
    // Do not run on a custom desktop CLI, Docker, network server or smoke CI.
    std::env::var("SCRIBEWATCH_NATIVE_AUTOINIT").as_deref() == Ok("1")
        && std::env::var("SCRIBEWATCH_DESKTOP_MODE").as_deref() == Ok("1")
        && config.host == "127.0.0.1"
        && std::env::var_os("SCRIBEWATCH_EMBEDDED_DIR")
            .is_some_and(|dir| std::path::PathBuf::from(dir).is_dir())
        && is_pristine_desktop_setup(providers, llm_providers, profiles, has_no_engine_selection)
}

fn is_pristine_desktop_setup(
    providers: &[Provider],
    llm_providers: &[LlmProvider],
    profiles: &[StructureProfile],
    has_no_engine_selection: bool,
) -> bool {
    providers.is_empty()
        && llm_providers.is_empty()
        && profiles.is_empty()
        && has_no_engine_selection
}

fn default_native_transcriber() -> Provider {
    Provider {
        id: AUTO_ASR_ID.into(),
        name: "Parakeet Ultra (local)".into(),
        transcription_url: "http://127.0.0.1:38005/v1/audio/transcriptions".into(),
        model: "handy-computer/parakeet-ultra-gguf".into(),
        api_key: String::new(),
        timeout_seconds: 1800,
        enabled: true,
    }
}

fn default_native_writer() -> LlmProvider {
    LlmProvider {
        id: AUTO_LLM_ID.into(),
        name: "Qwen3.5-4B Q4_K_M (local)".into(),
        chat_completions_url: "http://127.0.0.1:38080/v1/chat/completions".into(),
        model: "unsloth/Qwen3.5-4B-GGUF:Q4_K_M".into(),
        api_key: String::new(),
        timeout_seconds: 1200,
        enabled: true,
    }
}

impl AppState {
    pub async fn load(mut config: Config) -> Result<Self> {
        config.init()?;
        let db = Database::open(&config.db_file())?;
        let mut providers: Vec<Provider> = db.list("provider")?;
        let mut llm_providers: Vec<LlmProvider> = db.list("llm_provider")?;
        let mut structure_profiles: Vec<StructureProfile> = db.list("structure_profile")?;
        // First launch of the fully bundled, localhost-only desktop application:
        // create working LOCAL endpoints and bindings without changing existing
        // installations or a user-chosen external provider. No cloud inference.
        // Explicitly enabled by the native launcher, never by server/Docker.
        if native_zero_config_eligible(
            &config,
            &providers,
            &llm_providers,
            &structure_profiles,
            db.get::<Modes>("embedded_engines", "selection")?.is_none(),
        ) {
            let stt = default_native_transcriber();
            let llm = default_native_writer();
            db.upsert("provider", &stt.id, &stt)?;
            db.upsert("llm_provider", &llm.id, &llm)?;
            db.upsert(
                "embedded_engines",
                "selection",
                &Modes {
                    stt: Mode::Internal,
                    llm: Mode::Internal,
                },
            )?;
            providers.push(stt);
            llm_providers.push(llm);
        }
        // Retain legacy rows for historical jobs; expose only the five
        // current built-ins. Repair only obviously mojibaked built-in text,
        // preserving clinician-customized clean prompts and model bindings.
        let default_llm = {
            let enabled = llm_providers
                .iter()
                .filter(|p| p.enabled)
                .collect::<Vec<_>>();
            (enabled.len() == 1).then(|| enabled[0])
        };
        for builtin in crate::llm::builtin_profiles() {
            if let Some(existing) = structure_profiles.iter_mut().find(|p| p.id == builtin.id) {
                if existing.built_in {
                    let garbled = [&existing.name, &existing.description, &existing.prompt]
                        .iter()
                        .any(|value| {
                            value.contains('Ã')
                                || value.contains('Â')
                                || value.contains('�')
                                || value.contains("â€")
                        });
                    let mut changed = false;
                    // Upgrade only the untouched historical dental default.
                    // A customized prompt or provider/model binding stays intact.
                    if existing.id == "dental-consultation-complete-fr"
                        && crate::llm::dental_profile_has_unmodified_v1_prompt(&existing.prompt)
                    {
                        existing.prompt = builtin.prompt.clone();
                        changed = true;
                    }
                    if garbled {
                        existing.name = builtin.name.clone();
                        existing.description = builtin.description.clone();
                        existing.prompt = builtin.prompt.clone();
                        existing.output_template = builtin.output_template.clone();
                        existing.output_example = builtin.output_example.clone();
                        changed = true;
                    }
                    if existing.provider_id.is_empty()
                        && let Some(provider) = default_llm
                    {
                        existing.provider_id = provider.id.clone();
                        existing.model = provider.model.clone();
                        changed = true;
                    }
                    if changed {
                        db.upsert("structure_profile", &existing.id, existing)?;
                    }
                }
            } else {
                let mut profile = builtin;
                if let Some(provider) = default_llm {
                    profile.provider_id = provider.id.clone();
                    profile.model = provider.model.clone();
                }
                db.upsert("structure_profile", &profile.id, &profile)?;
                structure_profiles.push(profile);
            }
        }
        let workflows: Vec<Workflow> = db.list("workflow")?;
        let jobs = Arc::new(DashMap::new());
        for mut job in db.list::<Job>("job")? {
            if job.status.is_active() {
                job.status = JobStatus::Interrupted;
                job.error = Some("ScribeWatch restarted before this job completed".into());
                job.updated_at_ms = crate::jobs::now_ms();
                db.upsert("job", &job.id, &job)?;
            }
            jobs.insert(job.id.clone(), job);
        }
        let (events, _) = broadcast::channel(1024);
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(90))
            .user_agent(concat!("ScribeWatch/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let slots = config.max_transcription_jobs.max(1);
        let embedded = Arc::new(EmbeddedManager::new(&db, &config)?);
        Ok(Self {
            config: Arc::new(config),
            embedded,
            db,
            providers: Arc::new(RwLock::new(providers)),
            llm_providers: Arc::new(RwLock::new(llm_providers)),
            structure_profiles: Arc::new(RwLock::new(structure_profiles)),
            workflows: Arc::new(RwLock::new(workflows)),
            jobs,
            job_tokens: Arc::new(DashMap::new()),
            watcher_tokens: Arc::new(DashMap::new()),
            events,
            http,
            transcription_slots: Arc::new(Semaphore::new(slots)),
        })
    }

    pub fn emit_job(&self, job: &Job) {
        let _ = self.events.send(JobEvent {
            id: job.id.clone(),
            status: job.status.clone(),
            error: job.error.clone(),
        });
    }
}

#[cfg(test)]
mod zero_config_tests {
    use super::*;

    #[test]
    fn native_first_launch_provisions_4b_and_parakeet_without_cloud_endpoints() {
        let stt = default_native_transcriber();
        let llm = default_native_writer();
        assert_eq!(stt.model, "handy-computer/parakeet-ultra-gguf");
        assert_eq!(llm.model, "unsloth/Qwen3.5-4B-GGUF:Q4_K_M");
        assert!(stt.transcription_url.starts_with("http://127.0.0.1:"));
        assert!(llm.chat_completions_url.starts_with("http://127.0.0.1:"));
        assert!(stt.api_key.is_empty() && llm.api_key.is_empty());
        assert!(is_pristine_desktop_setup(&[], &[], &[], true));
        assert!(!is_pristine_desktop_setup(&[stt], &[], &[], true));
        assert!(!is_pristine_desktop_setup(&[], &[llm], &[], true));
        assert!(!is_pristine_desktop_setup(&[], &[], &[], false));
        assert!(!is_pristine_desktop_setup(
            &[],
            &[],
            &crate::llm::builtin_profiles(),
            true
        ));
    }
}
