use crate::domain::{MarkdownOptions, Provider, Workflow};
use anyhow::{Context, Result, bail};
use std::{
    collections::HashSet,
    fs::OpenOptions,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

const DEFAULT_TAGS: [&str; 3] = ["voice-note", "transcription", "scribewatch"];
const MAX_TITLE_CHARS: usize = 80;
const MAX_TITLE_WORDS: usize = 10;

pub struct NoteContext<'a> {
    pub title: &'a str,
    pub source_name: &'a str,
    pub workflow_name: Option<&'a str>,
    pub provider_name: &'a str,
    pub model: &'a str,
    pub language: Option<&'a str>,
    pub tags: &'a [String],
    pub frontmatter: bool,
    pub created_at: OffsetDateTime,
}

fn yaml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

pub fn derive_title(transcript: &str, source_name: &str) -> String {
    let normalized = normalize_whitespace(transcript);
    let candidate = if normalized.is_empty() {
        Path::new(source_name)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Untitled transcription")
            .to_owned()
    } else {
        let sentence = normalized
            .char_indices()
            .find(|(_, c)| matches!(c, '.' | '!' | '?'))
            .map(|(index, c)| normalized[..index + c.len_utf8()].trim().to_owned());
        match sentence.filter(|value| value.chars().count() <= MAX_TITLE_CHARS) {
            Some(value) if !value.is_empty() => value,
            _ => normalized
                .split_whitespace()
                .take(MAX_TITLE_WORDS)
                .collect::<Vec<_>>()
                .join(" "),
        }
    };
    let candidate = normalize_whitespace(&candidate);
    if candidate.is_empty() {
        "Untitled transcription".into()
    } else {
        truncate_chars(&candidate, MAX_TITLE_CHARS)
    }
}

pub fn sanitize_note_filename(title: &str) -> String {
    let mut clean = String::with_capacity(title.len());
    for ch in title.chars() {
        if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            clean.push(' ');
        } else {
            clean.push(ch);
        }
    }
    let clean = normalize_whitespace(&clean)
        .trim_matches(|c: char| c == '.' || c.is_whitespace())
        .to_owned();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if clean.is_empty() || reserved.iter().any(|name| clean.eq_ignore_ascii_case(name)) {
        "Untitled transcription".into()
    } else {
        truncate_chars(&clean, MAX_TITLE_CHARS)
    }
}

fn normalize_tag(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('#').trim();
    if value.is_empty() {
        return None;
    }
    let mut out = String::new();
    let mut dash = false;
    for ch in value.to_lowercase().chars() {
        if ch.is_alphanumeric() || matches!(ch, '-' | '_' | '/') {
            out.push(ch);
            dash = false;
        } else if (ch.is_whitespace() || ch.is_ascii_punctuation()) && !out.is_empty() && !dash {
            out.push('-');
            dash = true;
        }
    }
    let out = out.trim_matches('-').to_owned();
    (!out.is_empty()).then_some(out)
}

pub fn normalize_custom_tags(custom: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    custom
        .iter()
        .filter_map(|tag| normalize_tag(tag))
        .filter(|tag| seen.insert(tag.to_lowercase()))
        .collect()
}

fn note_tags(custom: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    DEFAULT_TAGS
        .iter()
        .map(|tag| (*tag).to_owned())
        .chain(normalize_custom_tags(custom))
        .filter(|tag| seen.insert(tag.to_lowercase()))
        .collect()
}

pub fn render_note(context: &NoteContext<'_>, transcript: &str) -> Result<String> {
    let mut out = String::new();
    if context.frontmatter {
        out.push_str("---\n");
        out.push_str(&format!("title: {}\n", yaml_string(context.title)));
        out.push_str(&format!(
            "created: {}\n",
            context.created_at.format(&Rfc3339)?
        ));
        out.push_str("tags:\n");
        for tag in note_tags(context.tags) {
            out.push_str(&format!("  - {tag}\n"));
        }
        out.push_str(&format!("source: {}\n", yaml_string(context.source_name)));
        if let Some(workflow) = context.workflow_name {
            out.push_str(&format!("workflow: {}\n", yaml_string(workflow)));
        }
        out.push_str(&format!(
            "provider: {}\n",
            yaml_string(context.provider_name)
        ));
        out.push_str(&format!("model: {}\n", yaml_string(context.model)));
        out.push_str(&format!(
            "language: {}\n",
            yaml_string(context.language.unwrap_or("auto"))
        ));
        out.push_str("generated_by: ScribeWatch\n---\n\n");
    }
    out.push_str("# ");
    out.push_str(context.title.trim());
    out.push_str("\n\n");
    out.push_str(transcript.trim());
    out.push('\n');
    Ok(out)
}

pub fn publish_unique(output_dir: &Path, title: &str, content: &str) -> Result<PathBuf> {
    if !output_dir.is_dir() {
        bail!(
            "Markdown output directory is unavailable: {}",
            output_dir.display()
        );
    }
    let base = sanitize_note_filename(title);
    let temp = output_dir.join(format!(".scribewatch.partial-{}.md", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    drop(file);
    for n in 1usize..10_001 {
        let name = if n == 1 {
            format!("{base}.md")
        } else {
            format!("{base} ({n}).md")
        };
        let destination = output_dir.join(name);
        match std::fs::hard_link(&temp, &destination) {
            Ok(()) => {
                std::fs::remove_file(&temp)?;
                #[cfg(unix)]
                if let Ok(dir) = std::fs::File::open(output_dir) {
                    let _ = dir.sync_all();
                }
                return Ok(destination);
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                let _ = std::fs::remove_file(&temp);
                return Err(error)
                    .with_context(|| format!("publish Markdown to {}", destination.display()));
            }
        }
    }
    let _ = std::fs::remove_file(&temp);
    bail!("could not reserve a unique Markdown filename")
}

// v0.1 compatibility helpers. The workflow pipeline migrates to the APIs above in v0.2.
pub fn render(
    workflow: &Workflow,
    provider: &Provider,
    source_name: &str,
    model: &str,
    language: Option<&str>,
    transcript: &str,
) -> String {
    let MarkdownOptions {
        frontmatter,
        transcript_heading,
    } = &workflow.markdown;
    let mut out = String::new();
    if *frontmatter {
        out.push_str("---\n");
        out.push_str(&format!("source: {}\n", yaml_string(source_name)));
        out.push_str(&format!("workflow: {}\n", yaml_string(&workflow.name)));
        out.push_str(&format!("provider: {}\n", yaml_string(&provider.name)));
        out.push_str(&format!("model: {}\n", yaml_string(model)));
        out.push_str(&format!(
            "language: {}\n",
            yaml_string(language.unwrap_or("auto"))
        ));
        out.push_str("generated_by: ScribeWatch\n---\n\n");
    }
    let heading = transcript_heading.trim();
    out.push_str("# ");
    out.push_str(if heading.is_empty() {
        "Transcript"
    } else {
        heading
    });
    out.push_str("\n\n");
    out.push_str(transcript.trim());
    out.push('\n');
    out
}

pub fn output_path(source: &Path) -> PathBuf {
    source.with_extension("md")
}

pub fn publish_atomic_no_overwrite(destination: &Path, content: &str) -> Result<()> {
    if destination.exists() {
        bail!("Markdown output already exists: {}", destination.display());
    }
    let parent = destination
        .parent()
        .context("Markdown output has no parent directory")?;
    let temp = parent.join(format!(".scribewatch.partial-{}.md", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    let publish = std::fs::hard_link(&temp, destination).with_context(|| {
        format!(
            "publish Markdown without overwrite to {}",
            destination.display()
        )
    });
    if let Err(error) = publish {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    std::fs::remove_file(&temp)?;
    #[cfg(unix)]
    if let Ok(dir) = std::fs::File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    #[test]
    fn title_prefers_short_first_sentence() {
        assert_eq!(
            derive_title("Acheter du lait demain. Puis appeler Paul.", "memo.m4a"),
            "Acheter du lait demain."
        );
    }

    #[test]
    fn title_falls_back_to_first_ten_words_and_unicode_safe_cap() {
        let transcript = "Voilà une très longue note vocale sans ponctuation qui continue encore avec énormément de détails supplémentaires inutiles ici";
        let title = derive_title(transcript, "memo.m4a");
        assert_eq!(title.split_whitespace().count(), 10);
        assert!(title.chars().count() <= 80);
        assert!(std::str::from_utf8(title.as_bytes()).is_ok());
    }

    #[test]
    fn empty_transcript_uses_source_stem() {
        assert_eq!(derive_title("   ", "Recording 42.m4a"), "Recording 42");
    }

    #[test]
    fn filename_sanitization_preserves_unicode_and_removes_reserved_characters() {
        assert_eq!(
            sanitize_note_filename("  Idée: demain / café? *important*  "),
            "Idée demain café important"
        );
    }

    #[test]
    fn note_is_obsidian_friendly_and_deduplicates_tags() {
        let custom = vec!["#Projet".into(), "Voice-Note".into(), "idée rapide".into()];
        let ctx = NoteContext {
            title: "Acheter du lait demain.",
            source_name: "memo \"test\".m4a",
            workflow_name: Some("Voice notes"),
            provider_name: "Provider",
            model: "whisper",
            language: Some("fr"),
            tags: &custom,
            frontmatter: true,
            created_at: OffsetDateTime::from_unix_timestamp(1_789_501_200).unwrap(),
        };
        let note = render_note(&ctx, "Acheter du lait demain.").unwrap();
        assert!(note.contains("title: \"Acheter du lait demain.\""));
        assert!(note.contains("created: 2026-09-15T19:40:00Z"));
        assert_eq!(note.matches("  - voice-note\n").count(), 1);
        assert!(note.contains("  - projet\n"));
        assert!(note.contains("  - idée-rapide\n"));
        assert!(note.contains("source: \"memo \\\"test\\\".m4a\""));
        assert!(note.contains("# Acheter du lait demain.\n\nAcheter du lait demain.\n"));
    }

    #[test]
    fn frontmatter_can_be_disabled() {
        let ctx = NoteContext {
            title: "Titre",
            source_name: "memo.m4a",
            workflow_name: None,
            provider_name: "Provider",
            model: "whisper",
            language: None,
            tags: &[],
            frontmatter: false,
            created_at: OffsetDateTime::from_unix_timestamp(1_789_501_200).unwrap(),
        };
        assert_eq!(render_note(&ctx, "Texte").unwrap(), "# Titre\n\nTexte\n");
    }

    #[test]
    fn unique_publication_never_overwrites() {
        let temp = tempfile::tempdir().unwrap();
        let first = publish_unique(temp.path(), "Même idée", "one").unwrap();
        let second = publish_unique(temp.path(), "Même idée", "two").unwrap();
        assert_eq!(first.file_name().unwrap(), "Même idée.md");
        assert_eq!(second.file_name().unwrap(), "Même idée (2).md");
        assert_eq!(std::fs::read_to_string(first).unwrap(), "one");
        assert_eq!(std::fs::read_to_string(second).unwrap(), "two");
    }
}
