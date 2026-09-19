use anyhow::{Context, Result, bail};
use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Parser)]
#[command(name = "scribewatch", version, about)]
pub struct Config {
    #[arg(long, env = "SCRIBEWATCH_HOST", default_value = "0.0.0.0")]
    pub host: String,
    #[arg(long, env = "SCRIBEWATCH_PORT", default_value_t = 3000)]
    pub port: u16,
    #[arg(long, env = "SCRIBEWATCH_CONFIG_DIR", default_value = "/config")]
    pub config_dir: PathBuf,
    #[arg(long, env = "SCRIBEWATCH_DIST_DIR", default_value = "/app/frontend")]
    pub dist_dir: PathBuf,
    #[arg(long, env = "SCRIBEWATCH_DATA_DIR", default_value = "/data")]
    pub data_dir: PathBuf,
    #[arg(long, env = "SCRIBEWATCH_ALLOWED_ROOTS", value_delimiter = ':')]
    pub allowed_roots: Vec<PathBuf>,
    #[arg(long, env = "SCRIBEWATCH_SCAN_SECONDS", default_value_t = 5)]
    pub scan_seconds: u64,
    #[arg(long, env = "SCRIBEWATCH_FILE_STABILITY_MS", default_value_t = 2000)]
    pub file_stability_ms: u64,
    #[arg(long, env = "SCRIBEWATCH_MAX_TRANSCRIPTION_JOBS", default_value_t = 2)]
    pub max_transcription_jobs: usize,
    #[arg(
        long,
        env = "SCRIBEWATCH_MAX_UPLOAD_BYTES",
        default_value_t = 2_147_483_648
    )]
    pub max_upload_bytes: u64,
    #[arg(
        long,
        env = "SCRIBEWATCH_QUICK_RESULT_RETENTION_HOURS",
        default_value_t = 24
    )]
    pub quick_result_retention_hours: u64,
}

impl Config {
    pub fn init(&mut self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)
            .with_context(|| format!("create {}", self.config_dir.display()))?;
        self.config_dir = std::fs::canonicalize(&self.config_dir)?;
        std::fs::create_dir_all(&self.data_dir)
            .with_context(|| format!("create {}", self.data_dir.display()))?;
        self.data_dir = std::fs::canonicalize(&self.data_dir)?;
        std::fs::create_dir_all(self.quick_upload_dir())?;
        std::fs::create_dir_all(self.quick_result_dir())?;
        reset_ephemeral_dir(&self.normalized_dir())?;
        reset_ephemeral_dir(&self.export_dir())?;
        if self.allowed_roots.is_empty() {
            bail!("SCRIBEWATCH_ALLOWED_ROOTS must contain at least one existing directory");
        }
        self.allowed_roots = self
            .allowed_roots
            .iter()
            .map(|p| {
                std::fs::canonicalize(p)
                    .with_context(|| format!("canonicalize allowed root {}", p.display()))
            })
            .collect::<Result<_>>()?;
        ensure_sqlite_local(&self.config_dir)?;
        Ok(())
    }

    pub fn db_file(&self) -> PathBuf {
        self.config_dir.join("scribewatch.sqlite3")
    }

    pub fn quick_upload_dir(&self) -> PathBuf {
        self.data_dir.join("quick-uploads")
    }

    pub fn quick_result_dir(&self) -> PathBuf {
        self.data_dir.join("quick-results")
    }

    pub fn normalized_dir(&self) -> PathBuf {
        self.data_dir.join("normalized-audio")
    }

    pub fn export_dir(&self) -> PathBuf {
        self.data_dir.join("exports")
    }

    pub fn resolve_allowed_path(&self, path: &Path) -> Result<PathBuf> {
        let canon = std::fs::canonicalize(path)
            .with_context(|| format!("canonicalize {}", path.display()))?;
        if !self
            .allowed_roots
            .iter()
            .any(|root| canon.starts_with(root))
        {
            bail!(
                "path is outside SCRIBEWATCH_ALLOWED_ROOTS: {}",
                canon.display()
            );
        }
        Ok(canon)
    }

    pub fn resolve_allowed_file(&self, path: &Path) -> Result<PathBuf> {
        let path = self.resolve_allowed_path(path)?;
        if !path.is_file() {
            bail!("path is not a file: {}", path.display());
        }
        Ok(path)
    }

    pub fn resolve_allowed_dir(&self, path: &Path) -> Result<PathBuf> {
        let path = self.resolve_allowed_path(path)?;
        if !path.is_dir() {
            bail!("path is not a directory: {}", path.display());
        }
        Ok(path)
    }
}

fn reset_ephemeral_dir(path: &Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("clear ephemeral directory {}", path.display()))?;
    }
    std::fs::create_dir_all(path)
        .with_context(|| format!("create ephemeral directory {}", path.display()))?;
    Ok(())
}

fn unescape_mount(value: &str) -> String {
    value
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

pub fn ensure_sqlite_local(config_dir: &Path) -> Result<()> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = config_dir;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        let canon = std::fs::canonicalize(config_dir)?;
        let mounts = std::fs::read_to_string("/proc/self/mountinfo")?;
        let mut best: Option<(usize, String, String)> = None;
        for line in mounts.lines() {
            let Some((left, right)) = line.split_once(" - ") else {
                continue;
            };
            let fields: Vec<_> = left.split_whitespace().collect();
            let rfields: Vec<_> = right.split_whitespace().collect();
            if fields.len() < 5 || rfields.is_empty() {
                continue;
            }
            let mount_point = PathBuf::from(unescape_mount(fields[4]));
            if canon.starts_with(&mount_point) {
                let len = mount_point.as_os_str().len();
                if best.as_ref().is_none_or(|current| len > current.0) {
                    best = Some((
                        len,
                        mount_point.display().to_string(),
                        rfields[0].to_string(),
                    ));
                }
            }
        }
        if let Some((_, mount, fs)) = best {
            let lower = fs.to_ascii_lowercase();
            if [
                "nfs",
                "cifs",
                "smb",
                "fuse.sshfs",
                "sshfs",
                "9p",
                "ceph",
                "glusterfs",
            ]
            .iter()
            .any(|name| lower == *name || lower.starts_with(&format!("{name}.")))
            {
                bail!(
                    "SCRIBEWATCH_CONFIG_DIR={} is on network filesystem {} mounted at {}; SQLite must use local storage",
                    canon.display(),
                    fs,
                    mount
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(root: &Path, allowed: &Path) -> Config {
        Config {
            host: "127.0.0.1".into(),
            port: 3000,
            config_dir: root.join("config"),
            dist_dir: root.join("dist"),
            data_dir: root.join("data"),
            allowed_roots: vec![allowed.to_path_buf()],
            scan_seconds: 1,
            file_stability_ms: 50,
            max_transcription_jobs: 1,
            max_upload_bytes: 2_147_483_648,
            quick_result_retention_hours: 24,
        }
    }

    #[test]
    fn init_creates_quick_data_directories() {
        let temp = tempfile::tempdir().unwrap();
        let allowed = temp.path().join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        let mut config = config(temp.path(), &allowed);
        config.data_dir = temp.path().join("data");
        config.init().unwrap();
        assert!(config.quick_upload_dir().is_dir());
        assert!(config.quick_result_dir().is_dir());
        assert_eq!(config.max_upload_bytes, 2_147_483_648);
        assert_eq!(config.quick_result_retention_hours, 24);
    }

    #[cfg(unix)]
    #[test]
    fn path_validation_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let allowed = temp.path().join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        let outside = temp.path().join("outside.m4a");
        std::fs::write(&outside, b"outside").unwrap();
        symlink(&outside, allowed.join("escape.m4a")).unwrap();
        let mut config = config(temp.path(), &allowed);
        config.init().unwrap();
        assert!(
            config
                .resolve_allowed_file(&allowed.join("escape.m4a"))
                .is_err()
        );
    }
}
