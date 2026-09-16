use crate::{
    config::Config,
    db::Database,
    domain::{Job, JobEvent, JobStatus, Provider, Workflow},
};
use anyhow::Result;
use dashmap::DashMap;
use std::{sync::Arc, time::Duration};
use tokio::sync::{RwLock, Semaphore, broadcast};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Database,
    pub providers: Arc<RwLock<Vec<Provider>>>,
    pub workflows: Arc<RwLock<Vec<Workflow>>>,
    pub jobs: Arc<DashMap<String, Job>>,
    pub job_tokens: Arc<DashMap<String, CancellationToken>>,
    pub watcher_tokens: Arc<DashMap<String, (String, CancellationToken)>>,
    pub events: broadcast::Sender<JobEvent>,
    pub http: reqwest::Client,
    pub transcription_slots: Arc<Semaphore>,
}

impl AppState {
    pub async fn load(mut config: Config) -> Result<Self> {
        config.init()?;
        let db = Database::open(&config.db_file())?;
        let providers: Vec<Provider> = db.list("provider")?;
        let workflows: Vec<Workflow> = db.list("workflow")?;
        let jobs = Arc::new(DashMap::new());
        for mut job in db.list::<Job>("job")? {
            if job.status.is_active() {
                job.status = JobStatus::Interrupted;
                job.error = Some("ScribeWatch restarted before this job completed".into());
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
        Ok(Self {
            config: Arc::new(config),
            db,
            providers: Arc::new(RwLock::new(providers)),
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
