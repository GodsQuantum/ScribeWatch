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

async fn run_pandoc(input: &Path, output: &Path, format: ExportFormat) -> Result<()> {
    let work_dir = output
        .parent()
        .ok_or_else(|| anyhow::anyhow!("export output has no parent directory"))?;
    let mut command = Command::new("pandoc");
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
            command.arg("--pdf-engine=weasyprint");
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
    fn supported_formats_are_explicit() {
        for format in ["md", "txt", "html", "docx", "odt", "pdf"] {
            assert!(ExportFormat::parse(format).is_ok());
        }
        assert!(ExportFormat::parse("exe").is_err());
    }
}
