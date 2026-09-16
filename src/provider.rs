use crate::domain::Provider;
use futures_util::TryStreamExt;
use reqwest::{Client, multipart};
use serde_json::Value;
use std::path::Path;
use thiserror::Error;
use tokio::fs::File;
use tokio_util::{io::ReaderStream, sync::CancellationToken};

#[derive(Debug, Error)]
pub enum TranscriptionError {
    #[error("operation cancelled")]
    Cancelled,
    #[error("transcription request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("transcription response is invalid: {0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub async fn transcribe(
    provider: &Provider,
    model: &str,
    language: Option<&str>,
    audio: &Path,
    client: &Client,
    token: &CancellationToken,
) -> Result<String, TranscriptionError> {
    let file = File::open(audio).await?;
    let stream = ReaderStream::new(file).map_err(std::io::Error::other);
    let body = reqwest::Body::wrap_stream(stream);
    let filename = audio
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("audio");
    let mime = mime_guess::from_path(audio).first_or_octet_stream();
    let part = multipart::Part::stream(body)
        .file_name(filename.to_owned())
        .mime_str(mime.as_ref())
        .map_err(TranscriptionError::Http)?;

    let mut form = multipart::Form::new()
        .part("file", part)
        .text("model", model.to_owned())
        .text("response_format", "json");
    if let Some(language) = language
        .map(str::trim)
        .filter(|v| !v.is_empty() && *v != "auto")
    {
        form = form.text("language", language.to_owned());
    }

    let mut request = client
        .post(&provider.transcription_url)
        .timeout(std::time::Duration::from_secs(
            provider.timeout_seconds.max(1),
        ))
        .multipart(form);
    if !provider.api_key.trim().is_empty() {
        request = request.bearer_auth(&provider.api_key);
    }

    let response = tokio::select! {
        response = request.send() => response?,
        _ = token.cancelled() => return Err(TranscriptionError::Cancelled),
    };
    let status = response.status();
    let bytes = tokio::select! {
        bytes = response.bytes() => bytes?,
        _ = token.cancelled() => return Err(TranscriptionError::Cancelled),
    };
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| TranscriptionError::Invalid(format!("malformed JSON: {error}")))?;
    if !status.is_success() {
        return Err(TranscriptionError::Invalid(format!(
            "HTTP {status}: {value}"
        )));
    }
    transcript_text(&value)
}
pub fn transcript_text(value: &Value) -> Result<String, TranscriptionError> {
    if let Some(text) = value.get("text").and_then(Value::as_str) {
        let text = text.trim();
        if !text.is_empty() {
            return Ok(text.to_owned());
        }
    }
    let root = value.get("result").unwrap_or(value);
    if let Some(text) = root.get("text").and_then(Value::as_str) {
        let text = text.trim();
        if !text.is_empty() {
            return Ok(text.to_owned());
        }
    }
    if let Some(segments) = root.get("segments").and_then(Value::as_array) {
        let text = segments
            .iter()
            .filter_map(|segment| segment.get("text").and_then(Value::as_str))
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() {
            return Ok(text);
        }
    }
    Err(TranscriptionError::Invalid(
        "no transcript text in response".into(),
    ))
}

pub fn validate_provider(provider: &Provider) -> anyhow::Result<()> {
    anyhow::ensure!(
        !provider.name.trim().is_empty(),
        "provider name is required"
    );
    anyhow::ensure!(
        !provider.model.trim().is_empty(),
        "provider model is required"
    );
    let url = url::Url::parse(&provider.transcription_url)?;
    anyhow::ensure!(
        matches!(url.scheme(), "http" | "https"),
        "provider URL must use http or https"
    );
    anyhow::ensure!(
        provider.timeout_seconds > 0,
        "provider timeout must be greater than zero"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_text_and_segment_responses() {
        assert_eq!(
            transcript_text(&serde_json::json!({"text":" hello "})).unwrap(),
            "hello"
        );
        assert_eq!(
            transcript_text(&serde_json::json!({"segments":[{"text":"one"},{"text":"two"}]}))
                .unwrap(),
            "one two"
        );
    }
    #[test]
    fn malformed_semantic_response_is_rejected() {
        assert!(transcript_text(&serde_json::json!({"ok":true})).is_err());
    }
}
