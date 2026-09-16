use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub transcription_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_timeout() -> u64 {
    600
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    pub id: String,
    pub name: String,
    pub transcription_url: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub enabled: bool,
    pub has_api_key: bool,
}

impl From<&Provider> for ProviderView {
    fn from(p: &Provider) -> Self {
        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            transcription_url: p.transcription_url.clone(),
            model: p.model.clone(),
            timeout_seconds: p.timeout_seconds,
            enabled: p.enabled,
            has_api_key: !p.api_key.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownOptions {
    #[serde(default = "default_true")]
    pub frontmatter: bool,
    #[serde(default = "default_heading")]
    pub transcript_heading: String,
}

fn default_heading() -> String {
    "Transcript".into()
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            frontmatter: true,
            transcript_heading: default_heading(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub watch_dir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    pub archive_dir: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub provider_id: String,
    #[serde(default)]
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default)]
    pub markdown: MarkdownOptions,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    #[default]
    Workflow,
    Quick,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuickSourceKind {
    Server,
    Upload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuickOutputKind {
    Server,
    Client,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuickJobMeta {
    pub source_kind: QuickSourceKind,
    pub output_kind: QuickOutputKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_name: Option<String>,
    #[serde(default = "default_true")]
    pub frontmatter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    #[default]
    Pending,
    Transcribing,
    Publishing,
    Archiving,
    Done,
    Error,
    Cancelled,
    Interrupted,
}

impl JobStatus {
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Pending | Self::Transcribing | Self::Publishing | Self::Archiving
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    #[serde(default)]
    pub kind: JobKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quick: Option<QuickJobMeta>,
    pub provider_id: String,
    pub original_name: String,
    pub source_path: PathBuf,
    pub source_size: u64,
    pub source_mtime_ns: i128,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub status: JobStatus,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markdown_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_path: Option<PathBuf>,
    #[serde(default)]
    pub markdown_published: bool,
    pub created_at_ms: u128,
    pub updated_at_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JobEvent {
    pub id: String,
    pub status: JobStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub workflows: usize,
    pub enabled_workflows: usize,
    pub active_jobs: usize,
    pub completed_jobs: usize,
    pub failed_jobs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v01_workflow_and_job_json_deserialize_with_v02_defaults() {
        let workflow_json = r#"{"id":"w","name":"old","watchDir":"/w","archiveDir":"/a","providerId":"p","model":"","markdown":{"frontmatter":true,"transcriptHeading":"Transcript"},"enabled":true}"#;
        let workflow: Workflow = serde_json::from_str(workflow_json).unwrap();
        assert_eq!(workflow.output_dir, None);
        assert!(workflow.tags.is_empty());

        let job_json = r#"{"id":"j","workflowId":"w","providerId":"p","originalName":"a.m4a","sourcePath":"/w/a.m4a","sourceSize":1,"sourceMtimeNs":1,"model":"m","status":"done","attempts":1,"markdownPublished":true,"createdAtMs":1,"updatedAtMs":1}"#;
        let job: Job = serde_json::from_str(job_json).unwrap();
        assert_eq!(job.kind, JobKind::Workflow);
        assert_eq!(job.workflow_id.as_deref(), Some("w"));
        assert!(job.quick.is_none());
    }

    #[test]
    fn provider_view_never_serializes_api_secret() {
        let provider = Provider {
            id: "p".into(),
            name: "Provider".into(),
            transcription_url: "http://localhost/transcriptions".into(),
            model: "model".into(),
            api_key: "super-secret".into(),
            timeout_seconds: 30,
            enabled: true,
        };
        let json = serde_json::to_string(&ProviderView::from(&provider)).unwrap();
        assert!(!json.contains("super-secret"));
        assert!(json.contains("\"hasApiKey\":true"));
    }
}
