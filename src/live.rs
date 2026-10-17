use crate::{audio, config::Config, quick::QuickOptions};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum LiveAudioRetention {
    None,
    Server { directory: String },
    Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveUploadSegment {
    pub id: u32,
    pub mime_type: String,
    pub started_at_ms: u128,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at_ms: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_ms_before: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveUploadManifest {
    pub schema_version: u32,
    pub session_id: String,
    pub started_at_ms: u128,
    pub segments: Vec<LiveUploadSegment>,
    pub retention: LiveAudioRetention,
    pub options: QuickOptions,
}

#[derive(Debug, Clone)]
pub struct FinalizedLiveAudio {
    pub source_path: PathBuf,
    pub audio_name: String,
    pub server_copy: Option<PathBuf>,
}

pub fn validate_manifest(manifest: &LiveUploadManifest) -> Result<Uuid> {
    if manifest.schema_version != 1 {
        bail!("unsupported LIVE manifest schema version");
    }
    let session_id = Uuid::parse_str(manifest.session_id.trim())?;
    if manifest.started_at_ms == 0 {
        bail!("LIVE session start timestamp is required");
    }
    if manifest.segments.is_empty() {
        bail!("LIVE upload requires at least one segment");
    }
    for (index, segment) in manifest.segments.iter().enumerate() {
        let expected = u32::try_from(index + 1).map_err(anyhow::Error::from)?;
        if segment.id != expected {
            bail!("LIVE segment ids must be contiguous starting at 1");
        }
        if segment.mime_type.trim().is_empty() {
            bail!("LIVE segment MIME type is required");
        }
        if segment.started_at_ms < manifest.started_at_ms {
            bail!("LIVE segment starts before the session");
        }
        if segment
            .ended_at_ms
            .is_some_and(|ended| ended < segment.started_at_ms)
        {
            bail!("LIVE segment end timestamp precedes its start");
        }
        if index == 0 && segment.gap_ms_before.is_some() {
            bail!("first LIVE segment cannot have an interruption gap");
        }
    }
    Ok(session_id)
}

pub fn resolve_server_destination(
    config: &Config,
    retention: &LiveAudioRetention,
) -> Result<Option<PathBuf>> {
    match retention {
        LiveAudioRetention::Server { directory } => {
            let path = config.resolve_allowed_dir(Path::new(directory.trim()))?;
            Ok(Some(path))
        }
        LiveAudioRetention::None | LiveAudioRetention::Client => Ok(None),
    }
}

pub fn segment_paths_in_manifest_order(
    manifest: &LiveUploadManifest,
    paths: &HashMap<u32, PathBuf>,
) -> Result<Vec<PathBuf>> {
    validate_manifest(manifest)?;
    manifest
        .segments
        .iter()
        .map(|segment| {
            paths
                .get(&segment.id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing LIVE segment {}", segment.id))
        })
        .collect()
}

fn audio_name(manifest: &LiveUploadManifest, session_id: Uuid) -> String {
    let short = &session_id.simple().to_string()[..8];
    format!("live-{}-{short}.m4a", manifest.started_at_ms)
}

fn sync_file(path: &Path) -> Result<()> {
    let file = OpenOptions::new().read(true).open(path)?;
    file.sync_all()?;
    Ok(())
}

fn sync_dir(path: &Path) {
    if let Ok(file) = File::open(path) {
        let _ = file.sync_all();
    }
}

fn files_equal(left: &Path, right: &Path) -> Result<bool> {
    let lm = std::fs::metadata(left)?;
    let rm = std::fs::metadata(right)?;
    if lm.len() != rm.len() {
        return Ok(false);
    }
    let mut l = File::open(left)?;
    let mut r = File::open(right)?;
    let mut lb = [0u8; 64 * 1024];
    let mut rb = [0u8; 64 * 1024];
    loop {
        let ln = l.read(&mut lb)?;
        let rn = r.read(&mut rb)?;
        if ln != rn || lb[..ln] != rb[..rn] {
            return Ok(false);
        }
        if ln == 0 {
            return Ok(true);
        }
    }
}

fn publish_server_copy(source: &Path, directory: &Path, name: &str) -> Result<PathBuf> {
    let destination = directory.join(name);
    if destination.exists() {
        if files_equal(source, &destination)? {
            return Ok(destination);
        }
        bail!("LIVE audio destination already exists with different content");
    }
    let temp = directory.join(format!(".scribewatch-live-{}.partial", Uuid::new_v4()));
    let mut src = File::open(source)?;
    let mut dst = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    std::io::copy(&mut src, &mut dst)?;
    dst.flush()?;
    dst.sync_all()?;
    drop(dst);
    match std::fs::hard_link(&temp, &destination) {
        Ok(()) => {
            std::fs::remove_file(&temp)?;
            sync_dir(directory);
            Ok(destination)
        }
        Err(error) => {
            let _ = std::fs::remove_file(&temp);
            Err(error).with_context(|| format!("publish LIVE audio {}", destination.display()))
        }
    }
}

pub async fn finalize_segments(
    config: &Config,
    manifest: &LiveUploadManifest,
    paths: &HashMap<u32, PathBuf>,
    token: &CancellationToken,
) -> Result<FinalizedLiveAudio> {
    let session_id = validate_manifest(manifest)?;
    let ordered = segment_paths_in_manifest_order(manifest, paths)?;
    let destination_dir = resolve_server_destination(config, &manifest.retention)?;
    std::fs::create_dir_all(config.live_source_dir())?;
    let source_path = config.live_source_dir().join(format!("{session_id}.m4a"));
    let temp_path = config
        .live_source_dir()
        .join(format!(".{session_id}.{}.partial.m4a", Uuid::new_v4()));

    let mut command = Command::new("ffmpeg");
    command
        .arg("-nostdin")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-y");
    for path in &ordered {
        command.arg("-i").arg(path);
    }
    let filter = (0..ordered.len())
        .map(|index| format!("[{index}:a:0]"))
        .collect::<String>()
        + &format!("concat=n={}:v=0:a=1[a]", ordered.len());
    command
        .arg("-filter_complex")
        .arg(filter)
        .arg("-map")
        .arg("[a]")
        .arg("-vn")
        .arg("-ac")
        .arg("1")
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("64k")
        .arg("-threads")
        .arg(config.ffmpeg_threads.to_string())
        .arg("--")
        .arg(&temp_path)
        .kill_on_drop(true);

    let child = command.spawn().context("spawn ffmpeg LIVE finalization")?;
    let output = child.wait_with_output();
    tokio::pin!(output);
    let output = tokio::select! {
        result = &mut output => result.context("wait for ffmpeg LIVE finalization")?,
        _ = token.cancelled() => {
            let _ = std::fs::remove_file(&temp_path);
            bail!("cancelled");
        }
    };
    if !output.status.success() {
        let _ = std::fs::remove_file(&temp_path);
        bail!(
            "LIVE audio finalization failed: {}",
            String::from_utf8_lossy(&output.stderr)
                .trim()
                .chars()
                .take(1200)
                .collect::<String>()
        );
    }
    if !audio::probe_audio(&temp_path, token).await? {
        let _ = std::fs::remove_file(&temp_path);
        bail!("LIVE audio finalization produced no decodable audio");
    }
    sync_file(&temp_path)?;
    std::fs::rename(&temp_path, &source_path)?;
    sync_dir(&config.live_source_dir());

    let name = audio_name(manifest, session_id);
    let server_copy = destination_dir
        .as_deref()
        .map(|directory| publish_server_copy(&source_path, directory, &name))
        .transpose()?;

    Ok(FinalizedLiveAudio {
        source_path,
        audio_name: name,
        server_copy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, domain::QuickOutputKind, quick::QuickOptions};
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
    };

    fn config(root: &Path) -> Config {
        let allowed = root.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        let mut config = Config {
            host: "127.0.0.1".into(),
            port: 0,
            config_dir: root.join("config"),
            dist_dir: root.join("dist"),
            data_dir: root.join("data"),
            allowed_roots: vec![allowed],
            scan_seconds: 1,
            file_stability_ms: 20,
            max_transcription_jobs: 1,
            max_upload_bytes: 1024 * 1024,
            quick_result_retention_hours: 24,
            quick_source_retention_hours: 24,
            normalized_audio_format: "wav".into(),
            ffmpeg_threads: 1,
        };
        config.init().unwrap();
        config
    }

    fn options() -> QuickOptions {
        QuickOptions {
            provider_id: "provider".into(),
            model: "model".into(),
            transcription_chain: Vec::new(),
            language: None,
            output_kind: QuickOutputKind::Client,
            output_dir: None,
            frontmatter: true,
            paragraphs: true,
            structure_profile_id: None,
        }
    }

    fn segment(id: u32) -> LiveUploadSegment {
        LiveUploadSegment {
            id,
            mime_type: "audio/wav".into(),
            started_at_ms: 1_700_000_000_000 + (id as u128 * 10_000),
            ended_at_ms: Some(1_700_000_005_000 + (id as u128 * 10_000)),
            gap_ms_before: (id > 1).then_some(250),
        }
    }

    fn manifest() -> LiveUploadManifest {
        LiveUploadManifest {
            schema_version: 1,
            session_id: "550e8400-e29b-41d4-a716-446655440000".into(),
            started_at_ms: 1_700_000_000_000,
            segments: vec![segment(1), segment(2)],
            retention: LiveAudioRetention::None,
            options: options(),
        }
    }

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

    #[test]
    fn manifest_requires_schema_v1_and_uuid_session() {
        let valid = manifest();
        assert_eq!(
            validate_manifest(&valid).unwrap().to_string(),
            valid.session_id
        );
        let mut bad_schema = valid.clone();
        bad_schema.schema_version = 2;
        assert!(validate_manifest(&bad_schema).is_err());
        let mut bad_session = valid;
        bad_session.session_id = "not-a-uuid".into();
        assert!(validate_manifest(&bad_session).is_err());
    }

    #[test]
    fn manifest_rejects_duplicate_or_non_contiguous_segment_ids() {
        let mut duplicate = manifest();
        duplicate.segments[1].id = 1;
        assert!(validate_manifest(&duplicate).is_err());
        let mut skipped = manifest();
        skipped.segments[1].id = 3;
        assert!(validate_manifest(&skipped).is_err());
        let mut empty = manifest();
        empty.segments.clear();
        assert!(validate_manifest(&empty).is_err());
    }

    #[test]
    fn manifest_order_controls_segment_order_not_hashmap_order() {
        let manifest = manifest();
        let one = PathBuf::from("/tmp/segment-one.wav");
        let two = PathBuf::from("/tmp/segment-two.wav");
        let mut paths = HashMap::new();
        paths.insert(2, two.clone());
        paths.insert(1, one.clone());
        assert_eq!(
            segment_paths_in_manifest_order(&manifest, &paths).unwrap(),
            vec![one, two]
        );
        paths.remove(&2);
        assert!(segment_paths_in_manifest_order(&manifest, &paths).is_err());
    }

    #[test]
    fn server_retention_path_must_resolve_inside_allowed_roots() {
        let temp = tempfile::tempdir().unwrap();
        let config = config(temp.path());
        let inside = config.allowed_roots[0].join("audio");
        let outside = temp.path().join("outside");
        std::fs::create_dir_all(&inside).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let allowed = LiveAudioRetention::Server {
            directory: inside.to_string_lossy().into_owned(),
        };
        assert_eq!(
            resolve_server_destination(&config, &allowed).unwrap(),
            Some(std::fs::canonicalize(&inside).unwrap())
        );
        let escaped = LiveAudioRetention::Server {
            directory: outside.to_string_lossy().into_owned(),
        };
        assert!(resolve_server_destination(&config, &escaped).is_err());
    }

    #[test]
    fn live_manifest_json_contract_is_camel_case_and_tagged() {
        let mut manifest = manifest();
        manifest.retention = LiveAudioRetention::Server {
            directory: "/allowed/audio".into(),
        };
        let value = serde_json::to_value(manifest).unwrap();
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["sessionId"], "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(value["segments"][0]["mimeType"], "audio/wav");
        assert_eq!(value["retention"]["mode"], "server");
        assert_eq!(value["retention"]["directory"], "/allowed/audio");
        assert!(value.get("schema_version").is_none());
    }

    #[tokio::test]
    async fn finalization_uses_manifest_order_and_produces_playable_m4a() {
        let temp = tempfile::tempdir().unwrap();
        let config = config(temp.path());
        let one = temp.path().join("one.wav");
        let two = temp.path().join("two.wav");
        std::fs::write(&one, tiny_wav()).unwrap();
        std::fs::write(&two, tiny_wav()).unwrap();
        let mut paths = HashMap::new();
        paths.insert(2, two);
        paths.insert(1, one);
        let result = finalize_segments(&config, &manifest(), &paths, &CancellationToken::new())
            .await
            .unwrap();
        assert!(result.source_path.is_file());
        assert_eq!(
            result.source_path.file_name().unwrap().to_string_lossy(),
            "550e8400-e29b-41d4-a716-446655440000.m4a"
        );
        assert!(
            audio::probe_audio(&result.source_path, &CancellationToken::new())
                .await
                .unwrap()
        );
        assert!(result.server_copy.is_none());
    }

    #[tokio::test]
    async fn server_copy_is_deterministic_and_not_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let config = config(temp.path());
        let destination = config.allowed_roots[0].join("audio");
        std::fs::create_dir_all(&destination).unwrap();
        let one = temp.path().join("one.wav");
        let two = temp.path().join("two.wav");
        std::fs::write(&one, tiny_wav()).unwrap();
        std::fs::write(&two, tiny_wav()).unwrap();
        let paths = HashMap::from([(1, one), (2, two)]);
        let mut manifest = manifest();
        manifest.retention = LiveAudioRetention::Server {
            directory: destination.to_string_lossy().into_owned(),
        };
        let first = finalize_segments(&config, &manifest, &paths, &CancellationToken::new())
            .await
            .unwrap();
        let first_copy = first.server_copy.unwrap();
        assert!(first_copy.is_file());
        let bytes = std::fs::read(&first_copy).unwrap();

        let second = finalize_segments(&config, &manifest, &paths, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(second.server_copy.unwrap(), first_copy);
        assert_eq!(std::fs::read(first_copy).unwrap(), bytes);
    }
}
