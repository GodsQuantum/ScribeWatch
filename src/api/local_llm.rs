//! Local-only LLM onboarding. No arbitrary commands, model uploads or remote installers.
use crate::error::{AppError, AppResult};
use axum::{
    Json,
    http::{HeaderMap, header},
};
use reqwest::{Client, Url, redirect::Policy};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

const LEMONADE: &str = "http://127.0.0.1:13305";
const MAX_RESPONSE: usize = 1024 * 1024;
const LIMIT: usize = 128;
const RUNTIMES: [(&str, u16); 4] = [
    ("Atomic Chat", 1337),
    ("LM Studio", 1234),
    ("Ollama", 11434),
    ("llama.cpp / Llama-Windows", 8080),
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Engine {
    name: &'static str,
    chat_completions_url: String,
    models: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineList {
    desktop: bool,
    engines: Vec<Engine>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryInput {
    pub chat_completions_url: String,
    #[serde(default)]
    pub api_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelList {
    models: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PullInput {
    pub model: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResponse {
    status: String,
    model: String,
}

/// Explicitly set only by the native launcher; container deployments cannot install models.
fn desktop() -> bool {
    std::env::var("SCRIBEWATCH_DESKTOP_MODE").as_deref() == Ok("1")
}

fn local_client(timeout: Duration) -> AppResult<Client> {
    Client::builder()
        .timeout(timeout)
        .redirect(Policy::none())
        .no_proxy()
        .build()
        .map_err(|error| AppError::Internal(error.into()))
}

pub(super) fn model_ids(value: &Value) -> Vec<String> {
    let entries = value
        .get("data")
        .or_else(|| value.get("models"))
        .and_then(Value::as_array);
    let mut models: Vec<String> = entries
        .into_iter()
        .flatten()
        .filter(|item| {
            item.get("type")
                .and_then(Value::as_str)
                .is_none_or(|kind| kind == "llm" || kind == "chat" || kind == "model")
        })
        .filter(|item| item.get("downloaded").and_then(Value::as_bool) != Some(false))
        .filter_map(|item| {
            item.get("id")
                .or_else(|| item.get("key"))
                .or_else(|| item.get("model"))
                .or_else(|| item.get("name"))
                .and_then(Value::as_str)
        })
        .filter(|id| !id.is_empty() && id.len() < 256)
        .map(str::to_owned)
        .collect();
    models.sort();
    models.dedup();
    models.truncate(LIMIT);
    models
}

async fn bounded_json(mut response: reqwest::Response) -> AppResult<Value> {
    if !response.status().is_success() {
        return Err(AppError::Upstream(format!(
            "Model discovery returned HTTP {}",
            response.status()
        )));
    }
    if response
        .content_length()
        .is_some_and(|len| len > MAX_RESPONSE as u64)
    {
        return Err(AppError::Upstream("Model list exceeds size limit".into()));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| AppError::Upstream(e.to_string()))?
    {
        if bytes.len() + chunk.len() > MAX_RESPONSE {
            return Err(AppError::Upstream("Model list exceeds size limit".into()));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| AppError::Upstream("Provider did not return valid model JSON".into()))
}

async fn probe(client: &Client, name: &'static str, port: u16) -> Option<Engine> {
    let origin = format!("http://127.0.0.1:{port}");
    // Prefer the native LM Studio catalog because it also exposes unloaded models.
    let paths: &[&str] = match name {
        "LM Studio" => &["/api/v1/models", "/v1/models"],
        "Atomic Chat" => &["/v1/models"],
        "Ollama" => &["/v1/models", "/api/tags"],
        _ => &["/v1/models"],
    };
    let mut models = None;
    for path in paths {
        let Ok(resp) = client.get(format!("{origin}{path}")).send().await else {
            continue;
        };
        let Ok(value) = bounded_json(resp).await else {
            continue;
        };
        // Recognition of a valid list is required to avoid false positives.
        if value.get("data").and_then(Value::as_array).is_none()
            && value.get("models").and_then(Value::as_array).is_none()
        {
            continue;
        }
        models = Some(model_ids(&value));
        break;
    }
    Some(Engine {
        name,
        chat_completions_url: format!("{origin}/v1/chat/completions"),
        models: models?,
    })
}

pub async fn engines() -> Json<EngineList> {
    if !desktop() {
        return Json(EngineList {
            desktop: false,
            engines: vec![],
        });
    }
    let Ok(client) = local_client(Duration::from_millis(1200)) else {
        return Json(EngineList {
            desktop: true,
            engines: vec![],
        });
    };
    let (lemonade, lmstudio, ollama, llamacpp) = tokio::join!(
        probe(&client, RUNTIMES[0].0, RUNTIMES[0].1),
        probe(&client, RUNTIMES[1].0, RUNTIMES[1].1),
        probe(&client, RUNTIMES[2].0, RUNTIMES[2].1),
        probe(&client, RUNTIMES[3].0, RUNTIMES[3].1),
    );
    Json(EngineList {
        desktop: true,
        engines: [lemonade, lmstudio, ollama, llamacpp]
            .into_iter()
            .flatten()
            .collect(),
    })
}

fn safe_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let a = v4.octets();
            // Refuse link-local/metadata, unspecified, multicast and broadcast.
            !(a[0] == 0
                || a[0] == 127 && !v4.is_loopback()
                || a[0] == 169 && a[1] == 254
                || a[0] >= 224
                || a[0] == 100 && (64..=127).contains(&a[1]))
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unique_local()
                || (!v6.is_multicast() && !v6.is_unicast_link_local() && !v6.is_unspecified())
        }
    }
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let a = v4.octets();
            a[0] == 10 || (a[0] == 172 && (16..=31).contains(&a[1])) || (a[0] == 192 && a[1] == 168)
        }
        IpAddr::V6(v6) => v6.is_unique_local(),
    }
}

async fn validate_endpoint(raw: &str) -> AppResult<(Url, Client)> {
    if raw.len() > 1024 {
        return Err(AppError::BadRequest("Endpoint too long".into()));
    }
    let url = Url::parse(raw).map_err(|_| AppError::BadRequest("Invalid LLM endpoint".into()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.path().ends_with("/chat/completions")
    {
        return Err(AppError::BadRequest(
            "Use a plain OpenAI-compatible /chat/completions URL".into(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| AppError::BadRequest("Missing LLM host".into()))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| AppError::BadRequest("Missing LLM port".into()))?;
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(Policy::none())
        .no_proxy();
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !safe_ip(ip) {
            return Err(AppError::BadRequest("Unsafe network address".into()));
        }
        if url.scheme() == "http" && !ip.is_loopback() && !is_private_ip(ip) {
            return Err(AppError::BadRequest(
                "Remote endpoints require HTTPS".into(),
            ));
        }
    } else {
        if host != "localhost" && !host.ends_with(".local") && url.scheme() != "https" {
            return Err(AppError::BadRequest(
                "Public LLM endpoints require HTTPS".into(),
            ));
        }
        let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
            .await
            .map_err(|_| AppError::BadRequest("Cannot resolve provider host".into()))?
            .filter(|a| safe_ip(a.ip()))
            .filter(|a| {
                if host == "localhost" || host.ends_with(".local") {
                    a.ip().is_loopback() || a.ip().is_ipv4() && is_private_ip(a.ip())
                } else {
                    !a.ip().is_loopback() && !is_private_ip(a.ip())
                }
            })
            .collect();
        if addresses.is_empty() {
            return Err(AppError::BadRequest(
                "Provider host has no safe address".into(),
            ));
        }
        builder = builder.resolve_to_addrs(host, &addresses);
    }
    let client = builder
        .build()
        .map_err(|e| AppError::Upstream(e.to_string()))?;
    Ok((url, client))
}

pub async fn discover(Json(input): Json<DiscoveryInput>) -> AppResult<Json<ModelList>> {
    // The existing saved-provider endpoint remains available in server installs.
    // Draft URL probes are desktop-only to avoid adding an open SSRF proxy.
    if !desktop() {
        return Err(AppError::BadRequest(
            "Draft endpoint discovery requires the desktop app".into(),
        ));
    }
    let (url, client) = validate_endpoint(&input.chat_completions_url).await?;
    let prefix = url
        .path()
        .strip_suffix("/chat/completions")
        .ok_or_else(|| AppError::BadRequest("Expected chat/completions URL".into()))?;
    let mut target = url.clone();
    target.set_path(&format!("{prefix}/models"));
    let mut request = client.get(target);
    if !input.api_key.trim().is_empty() {
        request = request.bearer_auth(input.api_key.trim());
    }
    let value = bounded_json(
        request
            .send()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?,
    )
    .await?;
    Ok(Json(ModelList {
        models: model_ids(&value),
    }))
}

fn local_host(headers: &HeaderMap) -> bool {
    let Some(host) = headers.get(header::HOST).and_then(|x| x.to_str().ok()) else {
        return false;
    };
    (host.starts_with("127.0.0.1:") || host.starts_with("localhost:"))
        && host
            .split(':')
            .next_back()
            .is_some_and(|port| port.parse::<u16>().is_ok())
}

pub(super) fn same_origin(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|x| x.to_str().ok()) else {
        return false;
    };
    if !local_host(headers) {
        return false;
    }
    let Some(host) = headers.get(header::HOST).and_then(|x| x.to_str().ok()) else {
        return false;
    };
    origin == format!("http://{host}")
}

/// User-confirmed, desktop-only model download using Lemonade's own job manager.
pub async fn pull(
    headers: HeaderMap,
    Json(input): Json<PullInput>,
) -> AppResult<Json<InstallResponse>> {
    if !desktop() || !same_origin(&headers) {
        return Err(AppError::BadRequest(
            "Model installation requires the local desktop UI".into(),
        ));
    }
    let (name, repo) = match input.model.as_str() {
        "lfm3-1.7b" => (
            "user.ScribeWatch-LFM3-1.7B",
            "LiquidAI/LFM3-1.7B-GGUF:Q4_K_M",
        ),
        "qwen3.5-4b" => (
            "user.ScribeWatch-Qwen3.5-4B",
            "unsloth/Qwen3.5-4B-GGUF:Q4_K_M",
        ),
        _ => return Err(AppError::BadRequest("Unsupported model selection".into())),
    };
    let client = local_client(Duration::from_secs(10))?;
    // No implicit runtime installation, no shell and no arbitrary URL.
    let resp = client
        .post(format!("{LEMONADE}/v1/pull"))
        .json(&json!({
            "model_name": name,
            "checkpoint": repo,
            "recipe": "llamacpp",
            "stream": true,
            "subscribe": false
        }))
        .send()
        .await
        .map_err(|_| AppError::BadRequest("Lemonade is not running on this computer".into()))?;
    if !resp.status().is_success() {
        return Err(AppError::Upstream(format!(
            "Lemonade rejected the model download: HTTP {}",
            resp.status()
        )));
    }
    Ok(Json(InstallResponse {
        status: "download-started".into(),
        model: name.into(),
    }))
}

pub async fn downloads(headers: HeaderMap) -> AppResult<Json<Value>> {
    if !desktop() || !local_host(&headers) {
        return Err(AppError::BadRequest(
            "Model status requires local desktop mode".into(),
        ));
    }
    let client = local_client(Duration::from_secs(3))?;
    let response = client
        .get(format!("{LEMONADE}/v1/downloads"))
        .send()
        .await
        .map_err(|_| AppError::BadRequest("Lemonade is not running".into()))?;
    Ok(Json(bounded_json(response).await?))
}

/// Install Lemonade only after a deliberate click in the Windows desktop app.
/// WinGet remains the source of truth and performs normal OS installer prompts.
pub async fn install_engine(headers: HeaderMap) -> AppResult<Json<InstallResponse>> {
    if !desktop() || !same_origin(&headers) || !cfg!(target_os = "windows") {
        return Err(AppError::BadRequest(
            "Lemonade installation is available only in the Windows desktop app".into(),
        ));
    }
    // No user-supplied command, URL, script or package identifier.
    // WinGet package ID is the vendor-supported published installation channel.
    let output = tokio::time::timeout(
        Duration::from_secs(180),
        tokio::process::Command::new("winget")
            .args([
                "install",
                "--id",
                "AMD.LemonadeServer",
                "--exact",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| AppError::Upstream("Windows package installation timed out".into()))?
    .map_err(|_| {
        AppError::BadRequest(
            "WinGet is unavailable. Install Lemonade from its official Windows download page."
                .into(),
        )
    })?;
    if !output.status.success() {
        return Err(AppError::Upstream(format!(
            "WinGet returned exit code {:?}; check Windows installer prompts or install manually",
            output.status.code(),
        )));
    }
    Ok(Json(InstallResponse {
        status: "installed".into(),
        model: "AMD.LemonadeServer".into(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_openai_and_native_model_ids() {
        let a = json!({"data":[{"id":"z"},{"id":"a"},{"id":"a"},{"id":"embedding","type":"embedding"}]});
        assert_eq!(model_ids(&a), ["a", "z"]);
        let b = json!({"models":[{"key":"qwen","type":"llm"},{"name":"none","downloaded":false}]});
        assert_eq!(model_ids(&b), ["qwen"]);
    }
    #[test]
    fn invalid_local_addresses_are_refused() {
        for addr in ["169.254.169.254", "0.0.0.0", "224.0.0.1", "100.100.100.100"] {
            assert!(!safe_ip(addr.parse::<std::net::Ipv4Addr>().unwrap().into()));
        }
        assert!(safe_ip("127.0.0.1".parse().unwrap()));
        assert!(safe_ip("172.20.20.10".parse().unwrap()));
    }
    #[tokio::test]
    async fn probe_real_local_http_catalogue() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut connection, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let received = connection.read(&mut buf).await.unwrap();
            assert!(received > 0);
            let json = r#"{"data":[{"id":"lfm3-1.7b"},{"id":"qwen3.5-4b"}]}"#;
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                json.len(),
                json
            );
            connection.write_all(reply.as_bytes()).await.unwrap();
        });
        let client = local_client(Duration::from_secs(2)).unwrap();
        let engine = probe(&client, "Lemonade Server", port).await.unwrap();
        assert_eq!(engine.models, ["lfm3-1.7b", "qwen3.5-4b"]);
        assert_eq!(
            engine.chat_completions_url,
            format!("http://127.0.0.1:{port}/v1/chat/completions")
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn lm_studio_native_model_catalogue_includes_unloaded_llms() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = [0u8; 2048];
            let len = stream.read(&mut bytes).await.unwrap();
            assert!(len > 0);
            assert!(String::from_utf8_lossy(&bytes[..len]).contains("/api/v1/models"));
            let payload = r#"{"models":[{"key":"lfm3-1.7b","type":"llm"},{"key":"embeddings","type":"embedding"}]}"#;
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            stream.write_all(reply.as_bytes()).await.unwrap();
        });
        let client = local_client(Duration::from_secs(2)).unwrap();
        let engine = probe(&client, "LM Studio", port).await.unwrap();
        assert_eq!(engine.models, ["lfm3-1.7b"]);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn ollama_native_tags_fallback_is_discovered() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            for expected in ["/v1/models", "/api/tags"] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut bytes = [0u8; 2048];
                let len = stream.read(&mut bytes).await.unwrap();
                assert!(len > 0);
                assert!(String::from_utf8_lossy(&bytes[..len]).contains(expected));
                let payload = if expected == "/api/tags" {
                    r#"{"models":[{"name":"lfm3:latest"}]}"#
                } else {
                    "{}"
                };
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
                stream.write_all(reply.as_bytes()).await.unwrap();
            }
        });
        let client = local_client(Duration::from_secs(2)).unwrap();
        let engine = probe(&client, "Ollama", port).await.unwrap();
        assert_eq!(engine.models, ["lfm3:latest"]);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn untrusted_endpoint_addresses_are_rejected_before_fetching() {
        for endpoint in [
            "http://169.254.169.254/v1/chat/completions",
            "file:///etc/passwd",
            "http://127.0.0.1:1/v1/chat/completions?token=abc",
            "http://example.com/v1/chat/completions",
        ] {
            assert!(validate_endpoint(endpoint).await.is_err(), "{endpoint}");
        }
        assert!(
            validate_endpoint("http://127.0.0.1:13305/v1/chat/completions")
                .await
                .is_ok()
        );
        assert!(
            validate_endpoint("http://172.20.20.10:13305/v1/chat/completions")
                .await
                .is_ok()
        );
    }

    #[test]
    fn pull_requires_local_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "127.0.0.1:3052".parse().unwrap());
        headers.insert(header::ORIGIN, "http://127.0.0.1:3052".parse().unwrap());
        assert!(same_origin(&headers));
        headers.insert(header::ORIGIN, "https://evil.example".parse().unwrap());
        assert!(!same_origin(&headers));
    }
}
