//! Optional ScribeWatch desktop-only acoustic and LLM subprocesses.
//! Model downloads and memory management use upstream engines, never custom daemons.
use crate::{
    config::Config,
    db::Database,
    error::{AppError, AppResult},
    state::AppState,
};
use axum::{
    Json,
    extract::{Path as ApiPath, State},
    http::HeaderMap,
};
use reqwest::{Client, redirect::Policy};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs::OpenOptions,
    net::TcpListener,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{
    process::{Child, Command},
    sync::{Mutex, RwLock},
    time::sleep,
};

const KIND: &str = "embedded_engines";
const ID: &str = "selection";
const STT_PORT: u16 = 38005;
const LLM_PORT: u16 = 38080;
const MAX_BYTES: usize = 1024 * 1024;
const PACK_MODELS: &[&str] = &[
    "handy-computer/whisper-large-v3-turbo-gguf",
    "handy-computer/parakeet-ultra-gguf",
    "handy-computer/Qwen3-ASR-0.6B-gguf",
    "handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf",
];
const LLM_SUGGESTIONS: &[&str] = &[
    "LiquidAI/LFM3-1.7B-GGUF:Q4_K_M",
    "unsloth/Qwen3.5-4B-GGUF:Q4_K_M",
    "unsloth/Qwen3.5-9B-GGUF:Q4_K_M",
];

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineKind {
    Stt,
    Llm,
}

impl EngineKind {
    fn port(self) -> u16 {
        match self {
            Self::Stt => STT_PORT,
            Self::Llm => LLM_PORT,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Stt => "Transcription",
            Self::Llm => "LLM",
        }
    }
    fn url(self) -> String {
        match self {
            Self::Stt => format!("http://127.0.0.1:{STT_PORT}/v1/audio/transcriptions"),
            Self::Llm => format!("http://127.0.0.1:{LLM_PORT}/v1/chat/completions"),
        }
    }
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Internal,
    #[default]
    External,
    Disabled,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Modes {
    pub stt: Mode,
    pub llm: Mode,
}

impl Modes {
    fn get(&self, kind: EngineKind) -> Mode {
        match kind {
            EngineKind::Stt => self.stt,
            EngineKind::Llm => self.llm,
        }
    }
    fn set(&mut self, kind: EngineKind, mode: Mode) {
        match kind {
            EngineKind::Stt => self.stt = mode,
            EngineKind::Llm => self.llm = mode,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeStatus {
    kind: EngineKind,
    name: &'static str,
    mode: Mode,
    bundled: bool,
    running: bool,
    endpoint: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatusResponse {
    desktop: bool,
    engines: Vec<RuntimeStatus>,
    download: DownloadStatus,
}
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStatus {
    pub status: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetMode {
    kind: EngineKind,
    mode: Mode,
}
#[derive(Debug, Deserialize)]
pub struct DownloadModel {
    kind: EngineKind,
    model: String,
}

#[derive(Clone)]
pub struct EmbeddedManager {
    root: Option<PathBuf>,
    data: PathBuf,
    modes: Arc<RwLock<Modes>>,
    children: Arc<Mutex<HashMap<EngineKind, Child>>>,
    download: Arc<RwLock<DownloadStatus>>,
}

impl EmbeddedManager {
    pub fn new(db: &Database, config: &Config) -> anyhow::Result<Self> {
        let root = if std::env::var("SCRIBEWATCH_DESKTOP_MODE").as_deref() == Ok("1")
            && config.host == "127.0.0.1"
        {
            std::env::var_os("SCRIBEWATCH_EMBEDDED_DIR").map(PathBuf::from)
        } else {
            None
        };
        let modes = db.get::<Modes>(KIND, ID)?.unwrap_or_default();
        Ok(Self {
            root,
            data: config.data_dir.join("embedded"),
            modes: Arc::new(RwLock::new(modes)),
            children: Arc::new(Mutex::new(HashMap::new())),
            download: Arc::new(RwLock::new(DownloadStatus::default())),
        })
    }

    pub fn available(&self) -> bool {
        self.root.is_some()
    }

    fn executables(&self, kind: EngineKind) -> Vec<PathBuf> {
        let Some(root) = &self.root else {
            return vec![];
        };
        match kind {
            EngineKind::Stt => {
                let file = root.join("speaches").join(if cfg!(windows) {
                    "Cloud9-Speaches.exe"
                } else {
                    "Cloud9-Speaches"
                });
                file.is_file().then_some(file).into_iter().collect()
            }
            EngineKind::Llm => {
                let folder = root.join("llm");
                let name = if cfg!(windows) {
                    "llama-server.exe"
                } else {
                    "llama-server"
                };
                let mut result = Vec::new();
                if cfg!(windows) {
                    let system = PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_default())
                        .join("System32");
                    for (enabled, variant) in [
                        (system.join("nvcuda.dll").is_file(), "cuda"),
                        (system.join("vulkan-1.dll").is_file(), "vulkan"),
                        (true, "cpu"),
                    ] {
                        let file = folder.join(variant).join(name);
                        if enabled && file.is_file() {
                            result.push(file);
                        }
                    }
                }
                let fallback = folder.join(name);
                if fallback.is_file() {
                    result.push(fallback);
                }
                result
            }
        }
    }

    fn executable(&self, kind: EngineKind) -> Option<PathBuf> {
        self.executables(kind).into_iter().next()
    }

    async fn launch(&self, kind: EngineKind) -> AppResult<()> {
        let candidates = self.executables(kind);
        if candidates.is_empty() {
            return Err(AppError::BadRequest(format!(
                "Bundled {} binary is absent; choose an external endpoint or full installer",
                kind.name()
            )));
        }
        let mut children = self.children.lock().await;
        if let Some(child) = children.get_mut(&kind) {
            if child
                .try_wait()
                .map_err(|e| AppError::Internal(e.into()))?
                .is_none()
            {
                return Ok(());
            }
            children.remove(&kind);
        }
        // Never connect or modify a third-party process merely because it shares our port.
        let reservation = TcpListener::bind(("127.0.0.1", kind.port())).map_err(|_| {
            AppError::Conflict(format!(
                "Port {} already belongs to another process",
                kind.port()
            ))
        })?;
        drop(reservation);
        let log_dir = self.data.join("logs");
        std::fs::create_dir_all(&log_dir).map_err(|e| AppError::Internal(e.into()))?;
        let label = match kind {
            EngineKind::Stt => "transcription",
            EngineKind::Llm => "llm",
        };
        let log_path = log_dir.join(format!("{label}.log"));
        if log_path
            .metadata()
            .is_ok_and(|stat| stat.len() > 16 * 1024 * 1024)
        {
            // Keep the previous log for diagnosis without unbounded growth.
            let _ = std::fs::rename(&log_path, log_dir.join(format!("{label}.log.previous")));
        }
        let log = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&log_path)
            .map_err(|e| AppError::Internal(e.into()))?;
        let mut previous_error = String::new();
        // Try CUDA, then Vulkan, then CPU when native GPU DLLs are unavailable or
        // incompatible. A failed accelerator must not prevent CPU transcription.
        for binary in candidates {
            let mut cmd = Command::new(&binary);
            cmd.current_dir(binary.parent().unwrap_or(Path::new(".")))
                .stdin(Stdio::null())
                .stdout(Stdio::from(
                    log.try_clone().map_err(|e| AppError::Internal(e.into()))?,
                ))
                .stderr(Stdio::from(
                    log.try_clone().map_err(|e| AppError::Internal(e.into()))?,
                ))
                .kill_on_drop(true);
            match kind {
                EngineKind::Stt => {
                    cmd.args([
                        "--no-browser",
                        "--no-tray",
                        "--port",
                        &kind.port().to_string(),
                    ]);
                    cmd.env("CLOUD9_PORTABLE_DATA_DIR", self.data.join("speaches"));
                }
                EngineKind::Llm => {
                    let cache = self.data.join("llm-cache");
                    let models = self.data.join("llm-models");
                    std::fs::create_dir_all(&cache).map_err(|e| AppError::Internal(e.into()))?;
                    std::fs::create_dir_all(&models).map_err(|e| AppError::Internal(e.into()))?;
                    cmd.args([
                        "--host",
                        "127.0.0.1",
                        "--port",
                        &kind.port().to_string(),
                        "--models-dir",
                    ])
                    .arg(&models)
                    .args([
                        "--models-max",
                        "1",
                        "--models-memory-margin",
                        "512",
                        "--sleep-idle-seconds",
                        "45",
                        "-c",
                        "8192",
                        "-ngl",
                        "99",
                    ]);
                    cmd.env("LLAMA_CACHE", cache);
                }
            }
            let mut child = match cmd.spawn() {
                Ok(child) => child,
                Err(error) => {
                    previous_error = format!("{}: {error}", binary.display());
                    continue;
                }
            };
            let path = match kind {
                EngineKind::Stt => "/health",
                EngineKind::Llm => "/health",
            };
            let url = format!("http://127.0.0.1:{}{path}", kind.port());
            let client = reqwest::Client::builder()
                .timeout(Duration::from_millis(900))
                .redirect(Policy::none())
                .no_proxy()
                .build()
                .map_err(|e| AppError::Internal(e.into()))?;
            let attempts = if kind == EngineKind::Stt { 360 } else { 150 };
            let mut ready = false;
            for _ in 0..attempts {
                if let Ok(Some(exit)) = child.try_wait() {
                    previous_error = format!("{} exited during startup: {exit}", binary.display());
                    break;
                }
                if let Ok(reply) = client.get(&url).send().await
                    && reply.status().is_success()
                {
                    ready = true;
                    break;
                }
                sleep(Duration::from_millis(350)).await;
            }
            if !ready {
                if child.try_wait().ok().flatten().is_none() {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
                if previous_error.is_empty() {
                    previous_error = format!("{} did not become ready", binary.display());
                }
                continue;
            }
            children.insert(kind, child);
            return Ok(());
        }
        Err(AppError::Upstream(previous_error))
    }

    async fn stop(&self, kind: EngineKind) -> AppResult<()> {
        if let Some(mut child) = self.children.lock().await.remove(&kind) {
            child
                .kill()
                .await
                .map_err(|e| AppError::Internal(e.into()))?;
            let _ = child.wait().await;
        }
        Ok(())
    }

    pub async fn recover(&self) {
        if !self.available() {
            return;
        }
        let modes = self.modes.read().await.clone();
        for kind in [EngineKind::Stt, EngineKind::Llm] {
            if modes.get(kind) == Mode::Internal
                && let Err(error) = self.launch(kind).await
            {
                tracing::warn!(engine = kind.name(), %error, "Embedded engine could not restart");
            }
        }
    }

    async fn statuses(&self) -> Vec<RuntimeStatus> {
        let modes = self.modes.read().await.clone();
        let mut children = self.children.lock().await;
        [EngineKind::Stt, EngineKind::Llm]
            .into_iter()
            .map(|kind| {
                let running = children
                    .get_mut(&kind)
                    .is_some_and(|child| child.try_wait().ok().flatten().is_none());
                if !running {
                    children.remove(&kind);
                }
                RuntimeStatus {
                    kind,
                    name: kind.name(),
                    mode: modes.get(kind),
                    bundled: self.executable(kind).is_some(),
                    running,
                    endpoint: kind.url(),
                }
            })
            .collect()
    }
}

fn require_desktop(manager: &EmbeddedManager, headers: Option<&HeaderMap>) -> AppResult<()> {
    if !manager.available() {
        return Err(AppError::BadRequest(
            "Bundled runtimes are available only on native desktop".into(),
        ));
    }
    if headers.is_some_and(|h| !super::local_llm::same_origin(h)) {
        return Err(AppError::BadRequest(
            "Only this desktop UI can control bundled engines".into(),
        ));
    }
    Ok(())
}

fn llama_model_progress(document: &Value, id: &str) -> Option<String> {
    let model = document
        .get("data")?
        .as_array()?
        .iter()
        .find(|model| model.get("id").and_then(Value::as_str) == Some(id))?;
    let state = model.get("status")?.get("value")?.as_str()?;
    match state {
        "unloaded" | "loaded" | "sleeping" => Some("installed".into()),
        "failed" => Some("error: model runtime reported failure".into()),
        "incomplete" => Some("incomplete - restart download to resume".into()),
        "downloading" => {
            let mut done = 0u64;
            let mut total = 0u64;
            if let Some(files) = model
                .get("status")
                .and_then(|status| status.get("progress"))
                .and_then(Value::as_object)
            {
                for value in files.values() {
                    done =
                        done.saturating_add(value.get("done").and_then(Value::as_u64).unwrap_or(0));
                    total = total
                        .saturating_add(value.get("total").and_then(Value::as_u64).unwrap_or(0));
                }
            }
            if total > 0 {
                Some(format!(
                    "downloading {}%",
                    done.saturating_mul(100).saturating_div(total)
                ))
            } else {
                Some("downloading".into())
            }
        }
        _ => None,
    }
}

pub async fn status(State(state): State<AppState>) -> Json<RuntimeStatusResponse> {
    let mut download = state.embedded.download.read().await.clone();
    if (download.status == "download-started" || download.status.starts_with("downloading"))
        && download.model.contains('/')
        && state.embedded.available()
    {
        // Refresh only when the user explicitly requests the status. The upstream
        // router maintains downloads and memory status; no timer or polling loop.
        if let Ok(client) = http()
            && let Ok(response) = client
                .get(format!("http://127.0.0.1:{LLM_PORT}/models"))
                .send()
                .await
            && let Ok(doc) = bounded_json(response).await
            && let Some(new_status) = llama_model_progress(&doc, &download.model)
        {
            download.status = new_status.clone();
            state.embedded.download.write().await.status = new_status;
        }
    }
    Json(RuntimeStatusResponse {
        desktop: state.embedded.available(),
        engines: state.embedded.statuses().await,
        download,
    })
}

pub async fn set_mode(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<SetMode>,
) -> AppResult<Json<RuntimeStatusResponse>> {
    require_desktop(&state.embedded, Some(&headers))?;
    match input.mode {
        Mode::Internal => state.embedded.launch(input.kind).await?,
        Mode::External | Mode::Disabled => state.embedded.stop(input.kind).await?,
    }
    let mut modes = state.embedded.modes.write().await;
    let mut updated = modes.clone();
    updated.set(input.kind, input.mode);
    state
        .db
        .upsert(KIND, ID, &updated)
        .map_err(AppError::Internal)?;
    *modes = updated;
    drop(modes);
    Ok(status(State(state)).await)
}

async fn bounded_json(response: reqwest::Response) -> AppResult<Value> {
    if !response.status().is_success() {
        return Err(AppError::Upstream(format!(
            "Internal engine returned HTTP {}",
            response.status()
        )));
    }
    if response
        .content_length()
        .is_some_and(|x| x > MAX_BYTES as u64)
    {
        return Err(AppError::Upstream("Model list too large".into()));
    }
    let mut response = response;
    let mut buffer = Vec::new();
    while let Some(data) = response
        .chunk()
        .await
        .map_err(|e| AppError::Upstream(e.to_string()))?
    {
        if buffer.len() + data.len() > MAX_BYTES {
            return Err(AppError::Upstream("Model list too large".into()));
        }
        buffer.extend_from_slice(&data);
    }
    serde_json::from_slice(&buffer)
        .map_err(|_| AppError::Upstream("Invalid internal model list".into()))
}

fn http() -> AppResult<Client> {
    Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(Policy::none())
        .no_proxy()
        .build()
        .map_err(|e| AppError::Internal(e.into()))
}

pub async fn models(
    State(state): State<AppState>,
    ApiPath(kind): ApiPath<EngineKind>,
) -> AppResult<Json<Value>> {
    require_desktop(&state.embedded, None)?;
    let endpoint = match kind {
        EngineKind::Stt => "/v1/models",
        EngineKind::Llm => "/models",
    };
    let response = http()?
        .get(format!("http://127.0.0.1:{}{endpoint}", kind.port()))
        .send()
        .await
        .map_err(|_| AppError::BadRequest("Internal engine is not running".into()))?;
    Ok(Json(bounded_json(response).await?))
}

#[derive(Debug, Deserialize)]
pub struct AsrSource {
    source: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AsrVariant {
    source: String,
    filename: String,
}

fn valid_asr_source(source: &str) -> bool {
    let raw = source.trim();
    raw.len() >= 3
        && raw.len() <= 2048
        && !raw.contains("..")
        && !raw.contains(['\n', '\r', '\0'])
        && (!raw.contains("://")
            || raw.starts_with("https://huggingface.co/")
            || raw.starts_with("https://hf.co/"))
}

pub async fn asr_variants(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<AsrSource>,
) -> AppResult<Json<Value>> {
    require_desktop(&state.embedded, Some(&headers))?;
    if !valid_asr_source(&input.source) {
        return Err(AppError::BadRequest(
            "Use a public Hugging Face repository or URL".into(),
        ));
    }
    let client = Client::builder()
        .timeout(Duration::from_secs(25))
        .redirect(Policy::none())
        .no_proxy()
        .build()
        .map_err(|e| AppError::Internal(e.into()))?;
    let response = client
        .post(format!(
            "http://127.0.0.1:{STT_PORT}/v1/desktop-asr/variants"
        ))
        .json(&json!({"source":input.source}))
        .send()
        .await
        .map_err(|_| AppError::BadRequest("Bundled Speaches is not running".into()))?;
    Ok(Json(bounded_json(response).await?))
}

pub async fn asr_install(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<AsrVariant>,
) -> AppResult<Json<DownloadStatus>> {
    require_desktop(&state.embedded, Some(&headers))?;
    if !valid_asr_source(&input.source)
        || input.filename.len() > 255
        || input.filename.contains("..")
        || !input.filename.to_ascii_lowercase().ends_with(".gguf")
    {
        return Err(AppError::BadRequest(
            "Invalid ASR GGUF source or variant".into(),
        ));
    }
    let mut progress = state.embedded.download.write().await;
    if progress.status == "downloading" {
        return Err(AppError::Conflict(
            "Another model download is already active".into(),
        ));
    }
    *progress = DownloadStatus {
        status: "downloading".into(),
        model: input.filename.clone(),
    };
    drop(progress);
    let tracker = state.embedded.download.clone();
    let current = DownloadStatus {
        status: "downloading".into(),
        model: input.filename.clone(),
    };
    tokio::spawn(async move {
        let result = async {
            let client = Client::builder()
                .timeout(Duration::from_secs(1800))
                .redirect(Policy::none())
                .no_proxy()
                .build()
                .map_err(|e| e.to_string())?;
            let response = client
                .post(format!(
                    "http://127.0.0.1:{STT_PORT}/v1/desktop-asr/install"
                ))
                .json(&input)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !response.status().is_success() {
                return Err(format!("Speaches returned HTTP {}", response.status()));
            }
            Ok::<_, String>(())
        }
        .await;
        tracker.write().await.status = match result {
            Ok(()) => "installed".into(),
            Err(error) => format!("error: {error}"),
        };
    });
    Ok(Json(current))
}

fn allowed_model(kind: EngineKind, model: &str) -> bool {
    match kind {
        EngineKind::Stt => PACK_MODELS.contains(&model) || model == "scribewatch-pack",
        EngineKind::Llm => LLM_SUGGESTIONS.contains(&model) || valid_hf_repo(model),
    }
}

fn valid_hf_repo(model: &str) -> bool {
    if model.len() > 160 || model.contains("..") || model.contains("://") {
        return false;
    }
    let Some((repo, quant)) = model.rsplit_once(':') else {
        return false;
    };
    let mut parts = repo.split('/');
    let valid = |x: &str| {
        !x.is_empty()
            && x.len() < 95
            && x.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    };
    let a = parts.next().unwrap_or("");
    let b = parts.next().unwrap_or("");
    valid(a) && valid(b) && parts.next().is_none() && quant.len() < 30 && valid(quant)
}

pub async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<DownloadModel>,
) -> AppResult<Json<DownloadStatus>> {
    require_desktop(&state.embedded, Some(&headers))?;
    if !allowed_model(input.kind, &input.model) {
        return Err(AppError::BadRequest(
            "Select a supported ASR model or valid Hugging Face GGUF repo".into(),
        ));
    }
    let mut current = state.embedded.download.write().await;
    if current.status == "downloading" {
        return Err(AppError::Conflict(
            "Another model download is already running".into(),
        ));
    }
    *current = DownloadStatus {
        status: "downloading".into(),
        model: input.model.clone(),
    };
    drop(current);
    let tracker = state.embedded.download.clone();
    let manager = state.embedded.clone();
    let result_model = input.model.clone();
    tokio::spawn(async move {
        let result = async {
            if input.kind == EngineKind::Stt && input.model == "scribewatch-pack" {
                // Reuse the verified portable-native bootstrap (GGUF + alignment).
                // It uses the same CLOUD9_PORTABLE_DATA_DIR and does not start a server.
                let exe = manager
                    .executable(EngineKind::Stt)
                    .ok_or_else(|| "Speaches native binary missing".to_owned())?;
                let mut child = Command::new(&exe);
                child
                    .args(["--install-default-models", "--model-group", "both"])
                    .current_dir(exe.parent().unwrap_or(Path::new(".")))
                    .env("CLOUD9_PORTABLE_DATA_DIR", manager.data.join("speaches"))
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .kill_on_drop(true);
                let result = tokio::time::timeout(Duration::from_secs(1800), async {
                    let mut running = child.spawn().map_err(|e| e.to_string())?;
                    running.wait().await.map_err(|e| e.to_string())
                })
                .await
                .map_err(|_| "Native acoustic pack install timeout".to_owned())??;
                if !result.success() {
                    return Err(format!("Acoustic pack installer returned {result}"));
                }
                return Ok::<_, String>(());
            }
            let client = Client::builder()
                .timeout(Duration::from_secs(1200))
                .redirect(Policy::none())
                .no_proxy()
                .build()
                .map_err(|e| e.to_string())?;
            if input.kind == EngineKind::Stt {
                // Curated transcribe.cpp HF GGUFs must use the portable-native
                // cache downloader, not the older Whisper registry endpoint.
                let origin = format!("http://127.0.0.1:{STT_PORT}");
                let variants = client
                    .post(format!("{origin}/v1/desktop-asr/variants"))
                    .json(&json!({"source": input.model}))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                if !variants.status().is_success() {
                    return Err(format!(
                        "ASR model catalog returned HTTP {}",
                        variants.status()
                    ));
                }
                let body = variants.json::<Value>().await.map_err(|e| e.to_string())?;
                let entries = body
                    .get("variants")
                    .and_then(Value::as_array)
                    .ok_or_else(|| "Invalid ASR quantization catalog".to_owned())?;
                let file = entries
                    .iter()
                    .filter_map(|entry| entry.get("filename").and_then(Value::as_str))
                    .find(|name| name.to_ascii_uppercase().ends_with("-Q6_K.GGUF"))
                    .or_else(|| {
                        entries
                            .iter()
                            .filter_map(|entry| entry.get("filename").and_then(Value::as_str))
                            .find(|name| name.to_ascii_uppercase().ends_with("-Q4_K_M.GGUF"))
                    })
                    .or_else(|| {
                        entries
                            .iter()
                            .filter_map(|entry| entry.get("filename").and_then(Value::as_str))
                            .next()
                    })
                    .ok_or_else(|| "ASR repo has no suitable GGUF variant".to_owned())?;
                let response = client
                    .post(format!("{origin}/v1/desktop-asr/install"))
                    .json(&json!({"source":input.model,"filename":file}))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                if !response.status().is_success() {
                    return Err(format!("ASR download returned HTTP {}", response.status()));
                }
                return Ok::<_, String>(());
            }
            let response = client
                .post(format!("http://127.0.0.1:{LLM_PORT}/models"))
                .json(&json!({"model": input.model, "sidecars": []}))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !response.status().is_success() {
                return Err(format!(
                    "LLM downloader returned HTTP {}",
                    response.status()
                ));
            }
            let outcome = response.json::<Value>().await.map_err(|e| e.to_string())?;
            if outcome.get("success").and_then(Value::as_bool) != Some(true) {
                return Err("LLM engine did not accept the download".into());
            }
            Ok::<_, String>(())
        }
        .await;
        let mut status = tracker.write().await;
        status.status = match result {
            Ok(()) if input.kind == EngineKind::Stt => "installed".into(),
            Ok(()) => "download-started".into(),
            Err(error) => format!("error: {error}"),
        };
    });
    Ok(Json(DownloadStatus {
        status: "downloading".into(),
        model: result_model,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_are_optional_and_default_external() {
        let settings = Modes::default();
        assert_eq!(settings.stt, Mode::External);
        assert_eq!(settings.llm, Mode::External);
    }
    #[test]
    fn no_arbitrary_model_or_path_is_executable() {
        assert!(allowed_model(
            EngineKind::Llm,
            "LiquidAI/LFM3-1.7B-GGUF:Q4_K_M"
        ));
        assert!(!valid_hf_repo("../tmp/model:Q4"));
        assert!(!valid_hf_repo("http://localhost/secret:model"));
        assert!(!valid_hf_repo("a/b/c:Q4"));
        assert!(!allowed_model(EngineKind::Stt, "unknown/repo"));
    }
    #[test]
    fn runtime_urls_are_loopback_and_separate() {
        assert!(EngineKind::Stt.url().starts_with("http://127.0.0.1:"));
        assert!(EngineKind::Llm.url().starts_with("http://127.0.0.1:"));
        assert_ne!(EngineKind::Stt.port(), EngineKind::Llm.port());
    }
}
