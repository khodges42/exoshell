use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::shell::ShellFamily;

#[derive(Debug, Clone)]
pub struct Transcript {
    started_at_epoch_ms: u128,
    provider: String,
    model: String,
    shell_family: ShellFamily,
    entries: Vec<TranscriptEntry>,
}

impl Transcript {
    pub fn new(provider: String, model: String, shell_family: ShellFamily) -> Self {
        Self {
            started_at_epoch_ms: unix_millis(),
            provider,
            model,
            shell_family,
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
            "# Exoshell Session {}\n\n- started_at_epoch_ms: `{}`\n- provider: `{}`\n- model: `{}`\n- shell_family: `{}`\n\n",
            self.started_at_epoch_ms,
            self.started_at_epoch_ms,
            self.provider,
            self.model,
            self.shell_family
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

    #[test]
    fn writes_markdown_transcript() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut transcript = Transcript::new(
            "test-provider".into(),
            "test-model".into(),
            ShellFamily::Posix,
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
        assert!(contents.contains("## User"));
        assert!(contents.contains("hello"));
        assert!(contents.contains("## Assistant"));
        assert!(contents.contains("## Error"));
    }
}
