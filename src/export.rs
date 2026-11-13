use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Text,
    Html,
    Docx,
    Odt,
    Pdf,
}

impl ExportFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "md" | "markdown" => Ok(Self::Markdown),
            "txt" | "text" => Ok(Self::Text),
            "html" | "htm" => Ok(Self::Html),
            "docx" => Ok(Self::Docx),
            "odt" => Ok(Self::Odt),
            "pdf" => Ok(Self::Pdf),
            other => bail!("unsupported export format: {other}"),
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Text => "txt",
            Self::Html => "html",
            Self::Docx => "docx",
            Self::Odt => "odt",
            Self::Pdf => "pdf",
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Markdown => "text/markdown; charset=utf-8",
            Self::Text => "text/plain; charset=utf-8",
            Self::Html => "text/html; charset=utf-8",
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Odt => "application/vnd.oasis.opendocument.text",
            Self::Pdf => "application/pdf",
        }
    }
}

fn neutralize_remote_images(markdown: &str) -> String {
    let chars = markdown.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(markdown.len());
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == '!'
            && index + 1 < chars.len()
            && chars[index + 1] == '['
            && let Some(alt_end_rel) = chars[index + 2..].iter().position(|ch| *ch == ']')
        {
            let alt_end = index + 2 + alt_end_rel;
            if alt_end + 1 < chars.len()
                && chars[alt_end + 1] == '('
                && let Some(target_end_rel) = chars[alt_end + 2..].iter().position(|ch| *ch == ')')
            {
                let target_end = alt_end + 2 + target_end_rel;
                let alt = chars[index + 2..alt_end].iter().collect::<String>();
                out.push_str("[Image");
                if !alt.trim().is_empty() {
                    out.push_str(": ");
                    out.push_str(alt.trim());
                }
                out.push(']');
                index = target_end + 1;
                continue;
            }
        }
        out.push(chars[index]);
        index += 1;
    }
    out
}

/// Prefer the self-contained desktop bin/ directory. When explicitly
/// configured, refuse any fallback to globally installed host executables.
/// Docker distributions may use their image-installed tools.
fn find_portable_export_tool(
    name: &str,
    configured_dir: Option<&Path>,
    server_executable: Option<&Path>,
) -> Result<PathBuf> {
    let executable = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    if let Some(directory) = configured_dir {
        let candidate = directory.join(&executable);
        if !candidate.is_file() {
            bail!(
                "ScribeWatch portable exporter missing: {}. Repair/reinstall the application package.",
                candidate.display()
            );
        }
        return Ok(candidate);
    }
    if let Some(candidate) = server_executable
        .and_then(Path::parent)
        .map(|parent| parent.join(&executable))
        && candidate.is_file()
    {
        return Ok(candidate);
    }
    // Legacy Docker containers include pandoc and WeasyPrint in the image.
    // The desktop packages always ship their own executables beside the server.
    Ok(PathBuf::from(executable))
}

fn located_on_path(binary: &Path) -> bool {
    if binary.is_file() {
        return true;
    }
    std::env::var_os("PATH").is_some_and(|value| {
        std::env::split_paths(&value).any(|directory| directory.join(binary).is_file())
    })
}

async fn run_pandoc(input: &Path, output: &Path, format: ExportFormat) -> Result<()> {
    let work_dir = output
        .parent()
        .ok_or_else(|| anyhow::anyhow!("export output has no parent directory"))?;
    let configured = std::env::var_os("SCRIBEWATCH_EXPORT_TOOLS_DIR").map(PathBuf::from);
    let current_executable = std::env::current_exe().ok();
    let pandoc = find_portable_export_tool(
        "pandoc",
        configured.as_deref(),
        current_executable.as_deref(),
    )?;
    let typst = find_portable_export_tool(
        "typst",
        configured.as_deref(),
        current_executable.as_deref(),
    )?;
    if !located_on_path(&pandoc) {
        bail!("Pandoc exporter not available; repair the ScribeWatch installation.");
    }
    let mut command = Command::new(pandoc);
    command
        .kill_on_drop(true)
        .current_dir(work_dir)
        .env("TMPDIR", work_dir)
        .arg("--sandbox")
        .arg("--from=gfm-raw_html")
        .arg("--standalone")
        .arg(input)
        .arg("-o")
        .arg(output);
    match format {
        ExportFormat::Text => {
            command.arg("--to=plain");
        }
        ExportFormat::Html => {
            command.arg("--to=html5");
        }
        ExportFormat::Docx => {
            command.arg("--to=docx");
        }
        ExportFormat::Odt => {
            command.arg("--to=odt");
        }
        ExportFormat::Pdf => {
            command.arg("--variable=papersize:a4");
            if located_on_path(&typst) {
                command
                    .arg("--to=typst")
                    .arg(format!("--pdf-engine={}", typst.display()));
                if let Some(font_directory) = typst
                    .parent()
                    .filter(|path| path.is_absolute())
                    .map(|path| path.join("fonts"))
                    && font_directory.is_dir()
                {
                    command.arg(format!(
                        "--pdf-engine-opt=--font-path={}",
                        font_directory.display()
                    ));
                }
            } else if configured.is_none() && located_on_path(Path::new("weasyprint")) {
                // Backward-compatible Docker PDF output: the image already has
                // WeasyPrint. Never depend on it for packaged desktop installs.
                command.arg("--pdf-engine=weasyprint");
            } else {
                bail!("Native Typst PDF exporter missing; repair the ScribeWatch installation.");
            }
        }
        ExportFormat::Markdown => unreachable!("Markdown does not require pandoc"),
    }
    let output_result = command.output().await.context("run pandoc export")?;
    if !output_result.status.success() {
        bail!(
            "pandoc export failed: {}",
            String::from_utf8_lossy(&output_result.stderr)
                .trim()
                .chars()
                .take(1600)
                .collect::<String>()
        );
    }
    Ok(())
}

pub async fn render_export(
    source_markdown: &Path,
    export_dir: &Path,
    job_id: &str,
    format: ExportFormat,
) -> Result<PathBuf> {
    if format == ExportFormat::Markdown {
        return Ok(source_markdown.to_path_buf());
    }
    std::fs::create_dir_all(export_dir)?;
    let sanitized = export_dir.join(format!("{job_id}.source.md"));
    let destination = export_dir.join(format!("{job_id}.{}", format.extension()));
    let markdown = tokio::fs::read_to_string(source_markdown)
        .await
        .with_context(|| format!("read {}", source_markdown.display()))?;
    let markdown = neutralize_remote_images(&markdown);
    tokio::fs::write(&sanitized, markdown).await?;
    let _ = tokio::fs::remove_file(&destination).await;
    let result = run_pandoc(&sanitized, &destination, format).await;
    let _ = tokio::fs::remove_file(&sanitized).await;
    result?;
    let metadata = tokio::fs::metadata(&destination).await?;
    if metadata.len() == 0 {
        let _ = tokio::fs::remove_file(&destination).await;
        bail!("export produced an empty file");
    }
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_targets_are_removed_before_document_conversion() {
        let input =
            "Text ![diagram](https://example.com/tracker.png) end.\n![local](file:///etc/passwd)";
        let sanitized = neutralize_remote_images(input);
        assert_eq!(sanitized, "Text [Image: diagram] end.\n[Image: local]");
        assert!(!sanitized.contains("https://"));
        assert!(!sanitized.contains("file://"));
    }

    #[test]
    fn native_exporters_are_resolved_without_global_host_installation() {
        let root = tempfile::tempdir().unwrap();
        let tools = root.path().join("export-tools");
        std::fs::create_dir_all(&tools).unwrap();
        let binary = if cfg!(windows) {
            "pandoc.exe"
        } else {
            "pandoc"
        };
        std::fs::write(tools.join(binary), b"portable").unwrap();
        assert_eq!(
            find_portable_export_tool("pandoc", Some(&tools), None).unwrap(),
            tools.join(binary)
        );
        assert!(find_portable_export_tool("typst", Some(&tools), None).is_err());
        let exe = root.path().join("bin").join("scribewatch");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(exe.parent().unwrap().join(binary), b"portable").unwrap();
        assert_eq!(
            find_portable_export_tool("pandoc", None, Some(&exe)).unwrap(),
            exe.parent().unwrap().join(binary)
        );
    }

    #[tokio::test]
    async fn all_formats_render_with_explicit_native_export_tools() {
        // The build gate supplies SCRIBEWATCH_EXPORT_TOOLS_DIR. In a normal
        // unit-test job without staged native assets we do not download tools.
        if std::env::var_os("SCRIBEWATCH_EXPORT_TOOLS_DIR").is_none() {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.md");
        std::fs::write(
            &source,
            "# Compte rendu\n\n- Réunion lundi.\n- Document préparé.\n- 牙科咨询.\n",
        )
        .unwrap();
        for (index, format) in [
            ExportFormat::Markdown,
            ExportFormat::Text,
            ExportFormat::Html,
            ExportFormat::Docx,
            ExportFormat::Odt,
            ExportFormat::Pdf,
        ]
        .into_iter()
        .enumerate()
        {
            let actual = render_export(&source, directory.path(), &format!("test-{index}"), format)
                .await
                .unwrap();
            let bytes = std::fs::read(&actual).unwrap();
            assert!(!bytes.is_empty(), "empty {format:?}");
            if format == ExportFormat::Pdf {
                assert!(bytes.starts_with(b"%PDF-"), "not a real PDF");
            }
            if matches!(format, ExportFormat::Docx | ExportFormat::Odt) {
                assert!(bytes.starts_with(b"PK"), "not a ZIP document");
            }
        }
    }

    #[test]
    fn supported_formats_are_explicit() {
        for format in ["md", "txt", "html", "docx", "odt", "pdf"] {
            assert!(ExportFormat::parse(format).is_ok());
        }
        assert!(ExportFormat::parse("exe").is_err());
    }
}
