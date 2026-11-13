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
    process::{Command as StdCommand, Stdio},
    sync::{Arc, OnceLock},
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
    "handy-computer/parakeet-ultra-gguf",
    "handy-computer/whisper-large-v3-turbo-gguf",
    "handy-computer/Qwen3-ASR-0.6B-gguf",
    "handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf",
];
// Preference order reflects full-transcript dental benchmarks, not file size.
// Other Hugging Face GGUFs remain manually installable: do not promote models
// whose clinical notes were too sparse or stopped at their output limit.
const LLM_SUGGESTIONS: &[&str] = &[
    "unsloth/Qwen3.5-4B-GGUF:Q4_K_M",
    "unsloth/Qwen3.5-9B-GGUF:Q4_K_M", // optional ~5.68GB weights, >= 6.5GB combined memory
    "unsloth/Qwen3.5-2B-GGUF:Q4_K_M", // small-memory fallback; requires review
];

/// The local llama.cpp router may advertise incomplete/downloading files
/// in /v1/models. A catalog entry alone is NOT model readiness.
fn default_qwen4b_downloaded(entry: &Value) -> bool {
    let id = entry.get("id").and_then(Value::as_str);
    let correct_model = matches!(
        id,
        Some("unsloth/Qwen3.5-4B-GGUF:Q4_K_M" | "Qwen3.5-4B-Q4_K_M" | "Qwen3.5-4B-Q4_K_M.gguf")
    );
    correct_model
        && matches!(
            entry.pointer("/status/value").and_then(Value::as_str),
            Some("loaded" | "unloaded" | "sleeping")
        )
}

// Use the upstream llama.cpp server on every supported desktop OS: no
// extra model manager and no GPU-specific custom daemon. Memory heuristics
// select a safe default; expert overrides are opt-in and range checked.
// Selecting the best *model* still requires clinical evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LlmRuntimePlan {
    context_tokens: u32,
    kv_cache: &'static str,
}

fn memory_mib_from_nvidia() -> Option<u64> {
    let output = StdCommand::new("nvidia-smi")
        .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .max()
}

fn parse_cgroup_memory_limit_mib(text: &str) -> Option<u64> {
    text.trim()
        .parse::<u64>()
        .ok()
        .map(|bytes| bytes / 1024 / 1024)
        .filter(|mib| *mib >= 1_024 && *mib < 1_048_576)
}

fn system_memory_mib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
        let host_memory_mib = meminfo
            .lines()
            .find_map(|line| {
                line.strip_prefix("MemTotal:")
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(|kb| kb.parse::<u64>().ok())
            })
            .map(|kb| kb / 1024)?;
        // An LXC/Docker guest may have much less RAM than /proc/meminfo
        // exposes. Use the cgroup limit when present to avoid OOM.
        let cgroup_mib = [
            "/sys/fs/cgroup/memory.max",
            "/sys/fs/cgroup/memory/memory.limit_in_bytes",
        ]
        .iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .find_map(|raw| parse_cgroup_memory_limit_mib(&raw));
        Some(cgroup_mib.map_or(host_memory_mib, |limit| host_memory_mib.min(limit)))
    }
    #[cfg(target_os = "macos")]
    {
        let output = StdCommand::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()?;
        return String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u64>()
            .ok()
            .map(|bytes| bytes / 1024 / 1024);
    }
    #[cfg(target_os = "windows")]
    {
        // Caching and timing diagnostics stay in the existing native process.
        // CIM is available on supported Windows installations; failure simply
        // selects conservative defaults rather than preventing startup.
        let output = StdCommand::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1MB)",
            ])
            .output()
            .ok()?;
        return String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u64>()
            .ok();
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

fn llm_runtime_plan(
    gpu_mib: Option<u64>,
    ram_mib: Option<u64>,
    context_override: Option<&str>,
    kv_override: Option<&str>,
) -> LlmRuntimePlan {
    // On Apple's unified memory or low-dedicated-VRAM GPUs (including AMD
    // integrated graphics), usable context depends primarily on system RAM.
    let dedicated_gpu = gpu_mib.filter(|mib| *mib >= 2_048);
    let memory_budget = dedicated_gpu.or_else(|| ram_mib.map(|mib| mib / 2));
    let context_tokens = match memory_budget {
        // With no dedicated accelerator and only 8 GiB RAM, preserve headroom
        // for the OS, model weights, transcription engine and PDF renderer.
        _ if dedicated_gpu.is_none() && ram_mib.is_some_and(|mib| mib <= 8_192) => 16_384,
        Some(mib) if mib >= 24_576 => 131_072,
        Some(mib) if mib >= 16_384 => 98_304,
        Some(mib) if mib >= 12_288 => 65_536,
        Some(mib) if mib >= 8_192 => 49_152,
        Some(mib) if mib >= 6_144 => 32_768,
        Some(mib) if mib >= 3_840 => 24_576,
        Some(_) => 16_384,
        None => 16_384,
    };
    let context_tokens = context_override
        .and_then(|raw| raw.parse::<u32>().ok())
        .filter(|tokens| (8_192..=131_072).contains(tokens) && tokens % 1024 == 0)
        .unwrap_or(context_tokens);
    let kv_cache = match kv_override {
        Some("q4_0") => "q4_0",
        Some("q8_0") => "q8_0",
        Some("f16") => "f16",
        _ if memory_budget.is_some_and(|mib| mib >= 12_288) => "q8_0",
        _ => "q4_0",
    };
    LlmRuntimePlan {
        context_tokens,
        kv_cache,
    }
}

static HOST_LLM_RUNTIME: OnceLock<LlmRuntimePlan> = OnceLock::new();

fn host_llm_runtime_plan() -> LlmRuntimePlan {
    *HOST_LLM_RUNTIME.get_or_init(|| {
        llm_runtime_plan(
            memory_mib_from_nvidia(),
            system_memory_mib(),
            std::env::var("SCRIBEWATCH_LLM_CONTEXT_TOKENS")
                .ok()
                .as_deref(),
            std::env::var("SCRIBEWATCH_LLM_KV_CACHE").ok().as_deref(),
        )
    })
}

pub(crate) fn embedded_llm_context_tokens() -> usize {
    host_llm_runtime_plan().context_tokens as usize
}

fn llm_runtime_arguments(plan: LlmRuntimePlan) -> Vec<String> {
    vec![
        "--models-max".into(),
        "1".into(),
        "--models-memory-margin".into(),
        "512".into(),
        "--sleep-idle-seconds".into(),
        "45".into(),
        "--ctx-size".into(),
        plan.context_tokens.to_string(),
        "--cache-type-k".into(),
        plan.kv_cache.into(),
        "--cache-type-v".into(),
        plan.kv_cache.into(),
        "--flash-attn".into(),
        "auto".into(),
        "--gpu-layers".into(),
        "99".into(),
        "--parallel".into(),
        "1".into(),
    ]
}

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

#[cfg(target_os = "linux")]
fn native_linux_llm_candidates(
    folder: &Path,
    has_render_node: bool,
    prefer_cpu: bool,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let vulkan = folder.join("vulkan/llama-server");
    if has_render_node && !prefer_cpu && vulkan.is_file() {
        candidates.push(vulkan);
    }
    let cpu = folder.join("llama-server");
    if cpu.is_file() {
        candidates.push(cpu);
    }
    candidates
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
                // Speaches Reloaded retained the historical portable binary
                // name in RC7. Accept its future branded filename as well.
                ["Cloud9-Speaches", "Speaches-Reloaded"]
                    .into_iter()
                    .map(|name| {
                        root.join("speaches").join(if cfg!(windows) {
                            format!("{name}.exe")
                        } else {
                            name.to_owned()
                        })
                    })
                    .filter(|file| file.is_file())
                    .collect()
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
                #[cfg(target_os = "linux")]
                {
                    let has_render_node =
                        std::fs::read_dir("/dev/dri").ok().is_some_and(|entries| {
                            entries.filter_map(Result::ok).any(|entry| {
                                entry.file_name().to_string_lossy().starts_with("renderD")
                            })
                        });
                    let prefer_cpu =
                        std::env::var("SCRIBEWATCH_LLM_FORCE_CPU").as_deref() == Ok("1");
                    result.extend(native_linux_llm_candidates(
                        &folder,
                        has_render_node,
                        prefer_cpu,
                    ));
                }
                let fallback = folder.join(name);
                if fallback.is_file() && !result.contains(&fallback) {
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
                    .arg(&models);
                    let plan = host_llm_runtime_plan();
                    cmd.args(llm_runtime_arguments(plan));
                    tracing::info!(
                        context_tokens = plan.context_tokens,
                        kv_cache = plan.kv_cache,
                        "Embedded llama.cpp runtime configured"
                    );
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

    /// One-shot first-launch preparation. Uses the existing upstream model
    /// downloaders and the bundled runtime, without a helper service/timer.
    /// Only selected by the native launcher and its ScribeWatch-owned provider.
    /// A restart may safely resume incomplete upstream downloads.
    pub async fn prepare_default_models(&self) {
        if !self.available() {
            return;
        }
        let modes = self.modes.read().await.clone();
        if modes.llm != Mode::Internal || modes.stt != Mode::Internal {
            return; // A user-selected external/disabled engine is authoritative.
        }
        // Do not contact a port that could be owned by another application:
        // both engines must be child processes owned by this ScribeWatch.
        let owned = self.statuses().await;
        if owned.len() != 2 || owned.iter().any(|engine| !engine.running) {
            tracing::warn!("Native first-use models pending until both ScribeWatch engines start");
            return;
        }
        let client = match Client::builder()
            .timeout(Duration::from_secs(60))
            .redirect(Policy::none())
            .no_proxy()
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                tracing::warn!(%error, "Native model bootstrap HTTP client unavailable");
                return;
            }
        };
        // Request Qwen first; llama.cpp performs/resumes the full download
        // asynchronously while the acoustic pack is provisioned locally.
        let llm_model = "unsloth/Qwen3.5-4B-GGUF:Q4_K_M";
        let llm_url = format!("http://127.0.0.1:{LLM_PORT}");
        let cached = match client.get(format!("{llm_url}/v1/models")).send().await {
            Ok(response) if response.status().is_success() => response
                .json::<Value>()
                .await
                .ok()
                .and_then(|doc| doc.get("data").and_then(Value::as_array).cloned())
                .is_some_and(|items| items.iter().any(default_qwen4b_downloaded)),
            _ => false,
        };
        if !cached {
            *self.download.write().await = DownloadStatus {
                status: "downloading".into(),
                model: llm_model.into(),
            };
            let outcome = client
                .post(format!("{llm_url}/models"))
                .json(&json!({"model": llm_model, "sidecars": []}))
                .send()
                .await;
            let valid = match outcome {
                Ok(reply) if reply.status().is_success() => {
                    reply.json::<Value>().await.ok().is_some_and(|doc| {
                        doc.get("success").and_then(Value::as_bool) == Some(true)
                    })
                }
                _ => false,
            };
            self.download.write().await.status = if valid {
                "download-started".into()
            } else {
                tracing::warn!(
                    "Default Qwen4B download was not accepted; user can retry in Settings"
                );
                "error: Default Qwen4B download could not start".into()
            };
        }

        // The acoustic pack CLI defaults to WHISPER, not Parakeet. Request
        // exactly the benchmark-validated Parakeet Ultra through the upstream
        // native transcribe.cpp GGUF catalog API instead.
        let stt_url = format!("http://127.0.0.1:{STT_PORT}/v1/models");
        let stt_ready = match client.get(&stt_url).send().await {
            Ok(response) if response.status().is_success() => response
                .json::<Value>()
                .await
                .ok()
                .and_then(|doc| doc.get("data").and_then(Value::as_array).cloned())
                .is_some_and(|models| {
                    models.iter().any(|item| {
                        item.get("id").and_then(Value::as_str)
                            == Some("handy-computer/parakeet-ultra-gguf")
                    })
                }),
            _ => false,
        };
        if !stt_ready {
            let model = "handy-computer/parakeet-ultra-gguf";
            *self.download.write().await = DownloadStatus {
                status: "downloading".into(),
                model: model.into(),
            };
            let result = install_curated_asr(&client, model).await;
            let mut progress = self.download.write().await;
            progress.status = match result {
                Ok(()) => "download-started".into(),
                Err(error) => {
                    tracing::warn!("Native Parakeet download failed; Settings permits retry");
                    format!("error: {error}")
                }
            };
        }
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

pub(crate) fn require_desktop(
    manager: &EmbeddedManager,
    headers: Option<&HeaderMap>,
) -> AppResult<()> {
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDirectoryResponse {
    pub path: String,
    pub opened: bool,
}

/// Open only the native desktop model directory, never an arbitrary path.
/// The active llama.cpp router uses exactly this directory (--models-dir).
pub async fn open_llm_models_directory(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ModelDirectoryResponse>> {
    require_desktop(&state.embedded, Some(&headers))?;
    let directory = state.config.data_dir.join("embedded").join("llm-models");
    std::fs::create_dir_all(&directory).map_err(|error| AppError::Internal(error.into()))?;
    #[cfg(target_os = "windows")]
    {
        StdCommand::new("explorer.exe")
            .arg(&directory)
            .spawn()
            .map_err(|error| AppError::Internal(error.into()))?;
        Ok(Json(ModelDirectoryResponse {
            path: directory.to_string_lossy().into_owned(),
            opened: true,
        }))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(Json(ModelDirectoryResponse {
            path: directory.to_string_lossy().into_owned(),
            opened: false,
        }))
    }
}

pub async fn status(State(state): State<AppState>) -> Json<RuntimeStatusResponse> {
    let mut download = state.embedded.download.read().await.clone();
    if (download.status == "download-started" || download.status.starts_with("downloading"))
        && download.model.contains('/')
        && download.model.contains(':')
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
    // User-triggered refresh must rescan manually added local GGUF files.
    // llama.cpp router supports ?reload=1 for on-demand catalog invalidation.
    let endpoint = match kind {
        EngineKind::Stt => "/v1/models",
        EngineKind::Llm => "/models?reload=1",
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

/// Choose the highest-fidelity practical model from the upstream catalogue.
/// Compare actual available quantizations; never infer their existence from the
/// repository name. A reduced-memory machine gets smaller weights but keeps
/// the same transcription protocol and can still use CPU fallback.
fn select_asr_variant_for_memory(variants: &[Value], ram_mib: Option<u64>) -> Option<&str> {
    let preferred: &[&str] = if ram_mib.is_some_and(|mib| mib >= 6_144) {
        &["-Q8_0.GGUF", "-Q6_K.GGUF", "-Q5_K_M.GGUF", "-Q4_K_M.GGUF"]
    } else {
        &["-Q6_K.GGUF", "-Q5_K_M.GGUF", "-Q4_K_M.GGUF", "-Q8_0.GGUF"]
    };
    let catalog = variants
        .iter()
        .filter_map(|entry| entry.get("filename").and_then(Value::as_str))
        .collect::<Vec<_>>();
    preferred
        .iter()
        .find_map(|suffix| {
            catalog
                .iter()
                .copied()
                .find(|filename| filename.to_ascii_uppercase().ends_with(suffix))
        })
        .or_else(|| catalog.first().copied())
}

// Shared upstream-native curated ASR download used by initial desktop
// provisioning and explicit model selection. No secondary downloader service.
async fn install_curated_asr(client: &Client, model: &str) -> Result<(), String> {
    let origin = format!("http://127.0.0.1:{STT_PORT}");
    let variants = client
        .post(format!("{origin}/v1/desktop-asr/variants"))
        .json(&json!({"source": model}))
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
    let file = select_asr_variant_for_memory(entries, system_memory_mib())
        .ok_or_else(|| "ASR repo has no suitable GGUF variant".to_owned())?;
    let response = client
        .post(format!("{origin}/v1/desktop-asr/install"))
        .json(&json!({"source":model,"filename":file}))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("ASR download returned HTTP {}", response.status()));
    }

    Ok(())
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
                install_curated_asr(&client, &input.model).await?;
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

    #[cfg(target_os = "linux")]
    #[test]
    fn native_linux_llm_prefers_packaged_vulkan_only_with_gpu_and_falls_back_to_cpu() {
        let base = std::env::temp_dir().join(format!(
            "scribewatch-native-candidates-{}",
            std::process::id()
        ));
        let folder = base.join("llm");
        std::fs::create_dir_all(folder.join("vulkan")).unwrap();
        std::fs::write(folder.join("vulkan/llama-server"), b"gpu").unwrap();
        std::fs::write(folder.join("llama-server"), b"cpu").unwrap();
        assert_eq!(native_linux_llm_candidates(&folder, true, false).len(), 2);
        assert_eq!(native_linux_llm_candidates(&folder, false, false).len(), 1);
        assert_eq!(native_linux_llm_candidates(&folder, true, true).len(), 1);
        assert!(
            native_linux_llm_candidates(&folder, true, false)[0]
                .to_string_lossy()
                .contains("vulkan")
        );
        std::fs::remove_dir_all(base).unwrap();
    }

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
    fn model_catalog_does_not_equal_download_completed() {
        for id in [
            "unsloth/Qwen3.5-4B-GGUF:Q4_K_M",
            "Qwen3.5-4B-Q4_K_M",
            "Qwen3.5-4B-Q4_K_M.gguf",
        ] {
            assert!(default_qwen4b_downloaded(&json!({
                "id":id,"status":{"value":"unloaded"}
            })));
        }
        for status in ["downloading", "incomplete", "error", "pending"] {
            assert!(!default_qwen4b_downloaded(&json!({
                "id":"unsloth/Qwen3.5-4B-GGUF:Q4_K_M",
                "status":{"value":status}
            })));
        }
        assert!(!default_qwen4b_downloaded(&json!({
            "id":"unsloth/Qwen3.5-4B-GGUF:Q4_K_M"
        })));
        assert!(!default_qwen4b_downloaded(&json!({
            "id":"Qwen3.5-9B-Q4_K_M","status":{"value":"unloaded"}
        })));
    }

    #[test]
    fn clean_install_prefers_q8_asr_when_ram_allows_and_q6_when_constrained() {
        let variants = vec![
            json!({"filename":"parakeet-ultra-Q4_K_M.gguf"}),
            json!({"filename":"parakeet-ultra-Q6_K.gguf"}),
            json!({"filename":"parakeet-ultra-Q8_0.gguf"}),
        ];
        assert_eq!(
            select_asr_variant_for_memory(&variants, Some(16_384)),
            Some("parakeet-ultra-Q8_0.gguf")
        );
        assert_eq!(
            select_asr_variant_for_memory(&variants, Some(4_096)),
            Some("parakeet-ultra-Q6_K.gguf")
        );
        assert_eq!(
            select_asr_variant_for_memory(&variants, None),
            Some("parakeet-ultra-Q6_K.gguf")
        );
        assert_eq!(
            select_asr_variant_for_memory(&variants[..2], Some(32_768)),
            Some("parakeet-ultra-Q6_K.gguf")
        );
        assert_eq!(select_asr_variant_for_memory(&[], Some(32_768)), None);
    }

    #[test]
    fn clinically_tested_defaults_are_preferred_across_desktop_platforms() {
        assert_eq!(PACK_MODELS[0], "handy-computer/parakeet-ultra-gguf");
        assert_eq!(LLM_SUGGESTIONS[0], "unsloth/Qwen3.5-4B-GGUF:Q4_K_M");
        assert_eq!(LLM_SUGGESTIONS[1], "unsloth/Qwen3.5-9B-GGUF:Q4_K_M");
        assert!(allowed_model(EngineKind::Llm, LLM_SUGGESTIONS[1]));
        assert!(allowed_model(EngineKind::Stt, PACK_MODELS[0]));
        assert!(allowed_model(EngineKind::Llm, LLM_SUGGESTIONS[0]));
    }

    #[test]
    fn windows_four_gig_gpu_uses_tested_quantized_24k_context() {
        let plan = llm_runtime_plan(Some(4096), Some(16_384), None, None);
        assert_eq!(plan.context_tokens, 24_576);
        assert_eq!(plan.kv_cache, "q4_0");
        let args = llm_runtime_arguments(plan);
        assert!(args.windows(2).any(|pair| pair == ["--ctx-size", "24576"]));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--cache-type-v", "q4_0"])
        );
        assert!(args.windows(2).any(|pair| pair == ["--flash-attn", "auto"]));
        assert!(args.windows(2).any(|pair| pair == ["--parallel", "1"]));
    }

    #[test]
    fn cgroup_limit_parser_skips_unlimited_and_tiny_values() {
        assert_eq!(parse_cgroup_memory_limit_mib("max"), None);
        assert_eq!(parse_cgroup_memory_limit_mib("8589934592\n"), Some(8192));
        assert_eq!(parse_cgroup_memory_limit_mib("18446744073709551615"), None);
        assert_eq!(parse_cgroup_memory_limit_mib("1048576"), None);
    }

    #[test]
    fn different_hardware_profiles_are_portable() {
        let cpu_only = llm_runtime_plan(None, Some(8_192), None, None);
        assert_eq!(cpu_only.context_tokens, 16_384);
        assert_eq!(cpu_only.kv_cache, "q4_0");
        let high_memory_cpu = llm_runtime_plan(None, Some(65_536), None, None);
        assert_eq!(high_memory_cpu.context_tokens, 131_072);
        assert_eq!(high_memory_cpu.kv_cache, "q8_0");
        let discrete_gpu = llm_runtime_plan(Some(8192), Some(32_768), None, None);
        assert_eq!(discrete_gpu.context_tokens, 49_152);
        assert_eq!(discrete_gpu.kv_cache, "q4_0");
    }

    #[test]
    fn environment_overrides_are_bounded_and_validated() {
        let plan = llm_runtime_plan(Some(4096), Some(16_384), Some("32768"), Some("q8_0"));
        assert_eq!(plan.context_tokens, 32_768);
        assert_eq!(plan.kv_cache, "q8_0");
        let unsafe_plan = llm_runtime_plan(
            Some(4096),
            Some(16_384),
            Some("9999999999999"),
            Some("nonsense"),
        );
        assert_eq!(unsafe_plan.context_tokens, 24_576);
        assert_eq!(unsafe_plan.kv_cache, "q4_0");
        let non_multiple = llm_runtime_plan(Some(4096), Some(16_384), Some("20000"), None);
        assert_eq!(non_multiple.context_tokens, 24_576);
    }

    #[test]
    fn runtime_urls_are_loopback_and_separate() {
        assert!(EngineKind::Stt.url().starts_with("http://127.0.0.1:"));
        assert!(EngineKind::Llm.url().starts_with("http://127.0.0.1:"));
        assert_ne!(EngineKind::Stt.port(), EngineKind::Llm.port());
    }
}
