use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::commands::CommandSuggestion;
use crate::context::{ContextEntry, redacted_provider_details};
use crate::prompts::Stance;
use crate::shell::ShellFamily;

#[derive(Debug, Clone)]
pub struct Transcript {
    started_at_epoch_ms: u128,
    provider: String,
    model: String,
    shell_family: ShellFamily,
    stance: Stance,
    entries: Vec<TranscriptEntry>,
}

impl Transcript {
    pub fn new(provider: String, model: String, shell_family: ShellFamily, stance: Stance) -> Self {
        Self {
            started_at_epoch_ms: unix_millis(),
            provider,
            model,
            shell_family,
            stance,
            entries: Vec::new(),
        }
    }

    pub fn record_user(&mut self, content: &str) {
        self.entries
            .push(TranscriptEntry::User(content.to_string()));
    }

    pub fn record_assistant(&mut self, content: &str) {
        self.entries
            .push(TranscriptEntry::Assistant(content.to_string()));
    }

    pub fn record_error(&mut self, content: &str) {
        self.entries
            .push(TranscriptEntry::Error(content.to_string()));
    }

    pub fn record_context_event(&mut self, action: &str, entry: &ContextEntry, note: &str) {
        self.entries.push(TranscriptEntry::ContextEvent {
            action: action.to_string(),
            id: entry.id.clone(),
            kind: entry.kind.to_string(),
            title: entry.title.clone(),
            enabled: entry.enabled,
            pinned: entry.pinned,
            priority: entry.priority.to_string(),
            characters: entry.size.characters,
            estimated_tokens: entry.size.estimated_tokens,
            origin: entry.provenance.origin.to_string(),
            provider_details: redacted_provider_details(&entry.provenance),
            note: note.to_string(),
        });
    }

    pub fn record_budget_warning(&mut self, warning: &str) {
        self.entries
            .push(TranscriptEntry::BudgetWarning(warning.to_string()));
    }

    pub fn record_stance_change(&mut self, previous: Stance, current: Stance) {
        self.stance = current;
        self.entries.push(TranscriptEntry::StanceChange {
            previous: previous.to_string(),
            current: current.to_string(),
        });
    }

    pub fn record_command_suggestion(&mut self, suggestion: &CommandSuggestion) {
        self.entries.push(TranscriptEntry::CommandSuggestion {
            id: suggestion.id.clone(),
            shell: suggestion.shell.to_string(),
            command: suggestion.command.clone(),
            model_risk: suggestion.model_risk.map(|risk| risk.to_string()),
            detected_risk: suggestion.detected_risk.level.to_string(),
            risk_reasons: suggestion.detected_risk.reasons.clone(),
        });
    }

    pub fn record_command_action(&mut self, id: &str, action: &str, note: &str) {
        self.entries.push(TranscriptEntry::CommandAction {
            id: id.to_string(),
            action: action.to_string(),
            note: note.to_string(),
        });
    }

    pub fn write_to_dir(&self, directory: &Path) -> Result<PathBuf, TranscriptError> {
        fs::create_dir_all(directory).map_err(|error| TranscriptError::CreateDir {
            path: directory.to_path_buf(),
            error,
        })?;

        let path = directory.join(format!(
            "session-{}-{}.md",
            self.started_at_epoch_ms,
            std::process::id()
        ));
        fs::write(&path, self.to_markdown()).map_err(|error| TranscriptError::Write {
            path: path.clone(),
            error,
        })?;

        Ok(path)
    }

    fn to_markdown(&self) -> String {
        let mut markdown = format!(
            "# Exoshell Session {}\n\n- started_at_epoch_ms: `{}`\n- provider: `{}`\n- model: `{}`\n- shell_family: `{}`\n- stance: `{}`\n\n",
            self.started_at_epoch_ms,
            self.started_at_epoch_ms,
            self.provider,
            self.model,
            self.shell_family,
            self.stance
        );

        for entry in &self.entries {
            match entry {
                TranscriptEntry::User(content) => {
                    markdown.push_str("## User\n\n");
                    markdown.push_str(content);
                    markdown.push_str("\n\n");
                }
                TranscriptEntry::Assistant(content) => {
                    markdown.push_str("## Assistant\n\n");
                    markdown.push_str(content);
                    markdown.push_str("\n\n");
                }
                TranscriptEntry::Error(content) => {
                    markdown.push_str("## Error\n\n");
                    markdown.push_str(content);
                    markdown.push_str("\n\n");
                }
                TranscriptEntry::ContextEvent {
                    action,
                    id,
                    kind,
                    title,
                    enabled,
                    pinned,
                    priority,
                    characters,
                    estimated_tokens,
                    origin,
                    provider_details,
                    note,
                } => {
                    markdown.push_str("## Context Event\n\n");
                    markdown.push_str(&format!("- action: `{action}`\n"));
                    markdown.push_str(&format!("- id: `{id}`\n"));
                    markdown.push_str(&format!("- type: `{kind}`\n"));
                    markdown.push_str(&format!("- title: `{title}`\n"));
                    markdown.push_str(&format!("- enabled: `{enabled}`\n"));
                    markdown.push_str(&format!("- pinned: `{pinned}`\n"));
                    markdown.push_str(&format!("- priority: `{priority}`\n"));
                    markdown.push_str(&format!(
                        "- size: `{characters}` chars / `~{estimated_tokens}` tokens\n"
                    ));
                    markdown.push_str(&format!("- origin: `{origin}`\n"));
                    for (key, value) in provider_details {
                        markdown.push_str(&format!("- {key}: `{value}`\n"));
                    }
                    markdown.push_str(&format!("- note: `{note}`\n\n"));
                }
                TranscriptEntry::BudgetWarning(content) => {
                    markdown.push_str("## Context Budget Warning\n\n");
                    markdown.push_str(content);
                    markdown.push_str("\n\n");
                }
                TranscriptEntry::StanceChange { previous, current } => {
                    markdown.push_str("## Stance Change\n\n");
                    markdown.push_str(&format!("- previous: `{previous}`\n"));
                    markdown.push_str(&format!("- current: `{current}`\n\n"));
                }
                TranscriptEntry::CommandSuggestion {
                    id,
                    shell,
                    command,
                    model_risk,
                    detected_risk,
                    risk_reasons,
                } => {
                    markdown.push_str("## Command Suggestion\n\n");
                    markdown.push_str(&format!("- id: `{id}`\n"));
                    markdown.push_str(&format!("- shell: `{shell}`\n"));
                    if let Some(model_risk) = model_risk {
                        markdown.push_str(&format!("- model_risk: `{model_risk}`\n"));
                    }
                    markdown.push_str(&format!("- detected_risk: `{detected_risk}`\n"));
                    for reason in risk_reasons {
                        markdown.push_str(&format!("- risk_reason: `{reason}`\n"));
                    }
                    markdown.push_str("\n```text\n");
                    markdown.push_str(command);
                    markdown.push_str("\n```\n\n");
                }
                TranscriptEntry::CommandAction { id, action, note } => {
                    markdown.push_str("## Command Action\n\n");
                    markdown.push_str(&format!("- id: `{id}`\n"));
                    markdown.push_str(&format!("- action: `{action}`\n"));
                    markdown.push_str(&format!("- note: `{note}`\n\n"));
                }
            }
        }

        markdown
    }
}

#[derive(Debug, Clone)]
enum TranscriptEntry {
    User(String),
    Assistant(String),
    Error(String),
    ContextEvent {
        action: String,
        id: String,
        kind: String,
        title: String,
        enabled: bool,
        pinned: bool,
        priority: String,
        characters: usize,
        estimated_tokens: usize,
        origin: String,
        provider_details: Vec<(String, String)>,
        note: String,
    },
    BudgetWarning(String),
    StanceChange {
        previous: String,
        current: String,
    },
    CommandSuggestion {
        id: String,
        shell: String,
        command: String,
        model_risk: Option<String>,
        detected_risk: String,
        risk_reasons: Vec<String>,
    },
    CommandAction {
        id: String,
        action: String,
        note: String,
    },
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX epoch")
        .as_millis()
}

#[derive(Debug, thiserror::Error)]
pub enum TranscriptError {
    #[error("failed to create transcript directory {path}: {error}")]
    CreateDir {
        path: PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error("failed to write transcript {path}: {error}")]
    Write {
        path: PathBuf,
        #[source]
        error: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ContextEntry, ContextKind, ContextProvenance};

    #[test]
    fn writes_markdown_transcript() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut transcript = Transcript::new(
            "test-provider".into(),
            "test-model".into(),
            ShellFamily::Posix,
            Stance::Operator,
        );
        transcript.record_user("hello");
        transcript.record_assistant("hi");
        transcript.record_error("provider failed");

        let path = transcript
            .write_to_dir(tempdir.path())
            .expect("transcript writes");
        let contents = fs::read_to_string(path).expect("read transcript");

        assert!(contents.contains("test-model"));
        assert!(contents.contains("test-provider"));
        assert!(contents.contains("shell_family: `posix`"));
        assert!(contents.contains("stance: `operator`"));
        assert!(contents.contains("## User"));
        assert!(contents.contains("hello"));
        assert!(contents.contains("## Assistant"));
        assert!(contents.contains("## Error"));
    }

    #[test]
    fn context_events_record_metadata_and_redact_provider_details() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut transcript = Transcript::new(
            "test-provider".into(),
            "test-model".into(),
            ShellFamily::Posix,
            Stance::Operator,
        );
        let mut provenance = ContextProvenance::manual();
        provenance
            .provider_details
            .insert("api_key".into(), "secret".into());
        let entry = ContextEntry::new(
            "ctx-001",
            ContextKind::Manual,
            "manual",
            provenance,
            "payload should not appear in context event",
        );

        transcript.record_context_event("add", &entry, "added");
        transcript.record_budget_warning("context budget exceeded");
        transcript.record_stance_change(Stance::Operator, Stance::Audit);

        let path = transcript
            .write_to_dir(tempdir.path())
            .expect("transcript writes");
        let contents = fs::read_to_string(path).expect("read transcript");

        assert!(contents.contains("## Context Event"));
        assert!(contents.contains("action: `add`"));
        assert!(contents.contains("id: `ctx-001`"));
        assert!(contents.contains("api_key: `[redacted]`"));
        assert!(!contents.contains("payload should not appear"));
        assert!(contents.contains("## Context Budget Warning"));
        assert!(contents.contains("## Stance Change"));
        assert!(contents.contains("current: `audit`"));
    }
}
