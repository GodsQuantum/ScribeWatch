//! Persistent clinical drafts kept in the local private SQLite database.
use crate::{db::Database, jobs::now_ms};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

pub const MAX_REPORT_BYTES: usize = 1024 * 1024;
const MAX_PRIOR_REVISIONS: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportRevision {
    pub revision: u64,
    pub markdown: String,
    pub saved_at_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SavedReport {
    pub job_id: String,
    /// The initial generated note and source transcript never change.
    pub original_markdown: String,
    pub markdown: String,
    pub revision: u64,
    pub created_at_ms: u128,
    pub updated_at_ms: u128,
    #[serde(default)]
    pub history: Vec<ReportRevision>,
}

/// Encounter data is stored only for the individual report, never in reusable
/// model prompts, presets, browser localStorage or the initial ASR transcript.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportEncounter {
    #[serde(default)]
    pub surname: String,
    #[serde(default)]
    pub forename: String,
    #[serde(default)]
    pub date_of_birth: String,
    #[serde(default)]
    pub consultation_date: String,
    #[serde(default)]
    pub practitioner: String,
    #[serde(default)]
    pub practice_address: String,
    #[serde(default)]
    pub practice_contact: String,
    #[serde(default)]
    pub session_fields: Vec<ReportSessionField>,
    #[serde(default)]
    pub medical_report_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportSessionField {
    pub label: String,
    pub value: String,
}

impl ReportEncounter {
    pub fn valid(&self) -> bool {
        [
            &self.surname,
            &self.forename,
            &self.date_of_birth,
            &self.consultation_date,
            &self.practitioner,
            &self.practice_address,
            &self.practice_contact,
        ]
        .iter()
        .all(|text| text.len() <= 512)
            && self.session_fields.len() <= 24
            && self
                .session_fields
                .iter()
                .all(|field| field.label.len() <= 100 && field.value.len() <= 2000)
    }
}

pub fn retain_encounter(db: &Database, job_id: &str, encounter: &ReportEncounter) -> Result<bool> {
    if !encounter.valid() {
        bail!("Encounter metadata exceeds allowed field size");
    }
    db.insert_if_absent("report_encounter", job_id, encounter)
}

pub fn get_encounter(db: &Database, job_id: &str) -> Result<Option<ReportEncounter>> {
    db.get("report_encounter", job_id)
}

fn validate(markdown: &str) -> Result<()> {
    if markdown.trim().is_empty() {
        bail!("A report cannot be empty");
    }
    if markdown.len() > MAX_REPORT_BYTES {
        bail!("A report cannot exceed 1 MiB");
    }
    Ok(())
}

pub fn retain_generated(db: &Database, job_id: &str, markdown: &str) -> Result<bool> {
    validate(markdown)?;
    let now = now_ms();
    let report = SavedReport {
        job_id: job_id.to_owned(),
        original_markdown: markdown.to_owned(),
        markdown: markdown.to_owned(),
        revision: 0,
        created_at_ms: now,
        updated_at_ms: now,
        history: Vec::new(),
    };
    db.insert_if_absent("report", job_id, &report)
}

pub fn get(db: &Database, job_id: &str) -> Result<Option<SavedReport>> {
    db.get("report", job_id)
}

/// Returns None for concurrent modifications or stale expected revisions.
pub fn save_edit(
    db: &Database,
    existing: &SavedReport,
    expected_revision: u64,
    markdown: &str,
) -> Result<Option<SavedReport>> {
    validate(markdown)?;
    if existing.revision != expected_revision {
        return Ok(None);
    }
    // Structured notes can change; original Parakeet transcription cannot.
    const TRANSCRIPT_SEPARATOR: &str = "\n## Transcript\n";
    if let Some(pos) = existing.original_markdown.rfind(TRANSCRIPT_SEPARATOR) {
        let source = &existing.original_markdown[pos..];
        let updated_source = markdown
            .rfind(TRANSCRIPT_SEPARATOR)
            .map(|index| &markdown[index..]);
        if updated_source != Some(source) {
            bail!("Original Parakeet transcript must remain unchanged");
        }
    }
    if existing.markdown == markdown {
        return Ok(Some(existing.clone()));
    }
    let mut next = existing.clone();
    next.history.push(ReportRevision {
        revision: existing.revision,
        markdown: existing.markdown.clone(),
        saved_at_ms: existing.updated_at_ms,
    });
    if next.history.len() > MAX_PRIOR_REVISIONS {
        next.history
            .drain(..next.history.len() - MAX_PRIOR_REVISIONS);
    }
    next.revision = existing.revision.saturating_add(1);
    next.markdown = markdown.to_owned();
    next.updated_at_ms = now_ms();
    if db.compare_and_swap("report", &existing.job_id, existing, &next)? {
        Ok(Some(next))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editing_is_durable_preserves_original_and_detects_stale_writers() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("private.sqlite3");
        let original = "# Rapport original\n\nTranscription.";
        {
            let db = Database::open(&path).unwrap();
            assert!(retain_generated(&db, "job-a", original).unwrap());
            let initial = get(&db, "job-a").unwrap().unwrap();
            let updated = save_edit(&db, &initial, 0, "# Rapport corrigé")
                .unwrap()
                .unwrap();
            assert_eq!(updated.revision, 1);
            assert_eq!(updated.original_markdown, original);
            assert_eq!(updated.history.len(), 1);
            assert!(!retain_generated(&db, "job-a", "regen").unwrap());
            assert!(save_edit(&db, &initial, 0, "stale").unwrap().is_none());
            assert_eq!(get(&db, "job-a").unwrap().unwrap(), updated);
            assert!(
                save_edit(&db, &updated, 1, &updated.markdown)
                    .unwrap()
                    .is_some()
            );
        }
        let db = Database::open(&path).unwrap();
        let restored = get(&db, "job-a").unwrap().unwrap();
        assert_eq!(restored.markdown, "# Rapport corrigé");
        assert_eq!(restored.original_markdown, original);
    }

    #[test]
    fn generated_transcript_is_immutable_through_editing_api() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Database::open(&tmp.path().join("db.sqlite3")).unwrap();
        let initial =
            "## Structured notes\n\nOriginal conclusion.\n\n## Transcript\n\nCanonical ASR.";
        retain_generated(&db, "session", initial).unwrap();
        let saved = get(&db, "session").unwrap().unwrap();
        assert!(
            save_edit(
                &db,
                &saved,
                0,
                "## Structured notes\n\nCorrected.\n\n## Transcript\n\nTampered ASR."
            )
            .is_err()
        );
        assert!(
            save_edit(
                &db,
                &saved,
                0,
                "## Structured notes\n\nCorrected.\n\n## Transcript\n\nCanonical ASR."
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn encounter_is_private_persistent_and_not_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Database::open(&tmp.path().join("db.sqlite3")).unwrap();
        let metadata = ReportEncounter {
            surname: "Patient".into(),
            forename: "Test".into(),
            date_of_birth: String::new(),
            consultation_date: "2026-11-12".into(),
            practitioner: "Dr A".into(),
            practice_address: String::new(),
            practice_contact: String::new(),
            session_fields: Vec::new(),
            medical_report_enabled: true,
        };
        assert!(retain_encounter(&db, "session", &metadata).unwrap());
        assert!(!retain_encounter(&db, "session", &metadata).unwrap());
        assert_eq!(get_encounter(&db, "session").unwrap(), Some(metadata));
    }

    #[test]
    fn invalid_and_excessively_long_edits_are_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Database::open(&tmp.path().join("db.sqlite3")).unwrap();
        retain_generated(&db, "job-b", "Initial").unwrap();
        let mut report = get(&db, "job-b").unwrap().unwrap();
        assert!(save_edit(&db, &report, 0, "  ").is_err());
        assert!(save_edit(&db, &report, 0, &"x".repeat(MAX_REPORT_BYTES + 1)).is_err());
        for number in 1..=15 {
            report = save_edit(&db, &report, number - 1, &format!("edit {number}"))
                .unwrap()
                .unwrap();
        }
        assert_eq!(report.history.len(), MAX_PRIOR_REVISIONS);
        assert_eq!(report.original_markdown, "Initial");
    }
}
