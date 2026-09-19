use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

pub fn is_staging_name(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.contains(".partial-") || name.ends_with(".uploading")
}

async fn run_cancellable(
    mut command: Command,
    token: &CancellationToken,
    context: &str,
) -> Result<std::process::Output> {
    command.kill_on_drop(true);
    command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = command
        .spawn()
        .with_context(|| format!("spawn {context}"))?;
    let output = child.wait_with_output();
    tokio::pin!(output);
    tokio::select! {
        result = &mut output => result.with_context(|| context.to_owned()),
        _ = token.cancelled() => bail!("cancelled"),
    }
}

pub async fn probe_audio(path: &Path, token: &CancellationToken) -> Result<bool> {
    if is_staging_name(path) || !path.is_file() {
        return Ok(false);
    }
    let mut command = Command::new("ffprobe");
    command
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("a:0")
        .arg("-show_entries")
        .arg("stream=codec_type")
        .arg("-of")
        .arg("csv=p=0")
        .arg("--")
        .arg(path);
    let output = run_cancellable(command, token, "ffprobe audio").await?;
    if !output.status.success() {
        return Ok(false);
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.trim() == "audio"))
}

pub async fn normalize_for_transcription(
    source: &Path,
    normalized_dir: &Path,
    job_id: &str,
    token: &CancellationToken,
) -> Result<PathBuf> {
    std::fs::create_dir_all(normalized_dir)
        .with_context(|| format!("create {}", normalized_dir.display()))?;
    let destination = normalized_dir.join(format!("{job_id}.wav"));
    let _ = std::fs::remove_file(&destination);

    let mut command = Command::new("ffmpeg");
    command
        .arg("-nostdin")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-y")
        .arg("-i")
        .arg(source)
        .arg("-map")
        .arg("0:a:0")
        .arg("-vn")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg("--")
        .arg(&destination);
    let output = run_cancellable(command, token, "ffmpeg audio normalization").await?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&destination);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "audio normalization failed: {}",
            stderr.trim().chars().take(1200).collect::<String>()
        );
    }
    let metadata = std::fs::metadata(&destination)
        .with_context(|| format!("stat normalized audio {}", destination.display()))?;
    if metadata.len() <= 44 {
        let _ = std::fs::remove_file(&destination);
        bail!("audio normalization produced an empty stream");
    }
    Ok(destination)
}

pub fn cleanup_normalized(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_names_are_ignored_without_extension_assumptions() {
        assert!(is_staging_name(Path::new("memo.partial-123")));
        assert!(is_staging_name(Path::new("memo.wav.uploading")));
        assert!(!is_staging_name(Path::new("memo.amr")));
        assert!(!is_staging_name(Path::new("recording-without-extension")));
    }
}
