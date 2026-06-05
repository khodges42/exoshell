use std::path::PathBuf;

use crate::config::Config;
use crate::context::{
    ContextProviderRegistry, SessionContextStore, register_default_context_providers,
};
use crate::prompts::phase1_system_prompt;
use crate::providers::{ChatMessage, ChatRequest, ChatResponse, ChatRole, Provider, ProviderError};
use crate::repl::ReplError;
use crate::shell::ShellFamily;
use crate::transcripts::{Transcript, TranscriptError};

pub struct App {
    config: Config,
    provider: Box<dyn Provider>,
    messages: Vec<ChatMessage>,
    transcript: Transcript,
    _context_store: SessionContextStore,
    _context_registry: ContextProviderRegistry,
}

impl App {
    pub fn new(config: Config, provider: Box<dyn Provider>) -> Self {
        let transcript = Transcript::new(
            "openai-compatible".into(),
            config.provider.model.clone(),
            config.shell.family,
        );
        let messages = vec![ChatMessage::new(
            ChatRole::System,
            phase1_system_prompt(config.shell.family),
        )];
        let mut context_registry = ContextProviderRegistry::new();
        register_default_context_providers(&mut context_registry)
            .expect("default context providers should register");

        Self {
            config,
            provider,
            messages,
            transcript,
            _context_store: SessionContextStore::new(),
            _context_registry: context_registry,
        }
    }

    pub async fn send(&mut self, input: String) -> Result<String, AppError> {
        self.messages
            .push(ChatMessage::new(ChatRole::User, input.clone()));
        self.transcript.record_user(&input);

        let request = ChatRequest {
            messages: self.messages.clone(),
            stream: false,
        };

        let response = match self.provider.chat(request).await {
            Ok(ChatResponse::Complete(response)) => response,
            Ok(ChatResponse::Stream(chunks)) => chunks.concat(),
            Err(error) => {
                self.transcript.record_error(&error.to_string());
                return Err(error.into());
            }
        };
        self.messages
            .push(ChatMessage::new(ChatRole::Assistant, response.clone()));
        self.transcript.record_assistant(&response);

        Ok(response)
    }

    pub fn save_transcript(&self) -> Result<Option<PathBuf>, AppError> {
        if !self.config.transcript.enabled {
            return Ok(None);
        }

        let path = self
            .transcript
            .write_to_dir(&self.config.transcript.directory)?;
        Ok(Some(path))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error(transparent)]
    Repl(#[from] ReplError),
    #[error(transparent)]
    Transcript(#[from] TranscriptError),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CliOptions {
    pub config_path: Option<PathBuf>,
    pub shell_family: Option<ShellFamily>,
    pub transcript_enabled: Option<bool>,
    pub transcript_directory: Option<PathBuf>,
    pub no_color: bool,
    pub show_help: bool,
}

impl CliOptions {
    pub fn parse<I>(args: I) -> Result<Self, AppError>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = CliOptions::default();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => options.show_help = true,
                "--config" => {
                    let value = args.next().ok_or_else(|| {
                        crate::config::ConfigError::Invalid("--config requires a path".into())
                    })?;
                    options.config_path = Some(PathBuf::from(value));
                }
                "--shell" => {
                    let value = args.next().ok_or_else(|| {
                        crate::config::ConfigError::Invalid("--shell requires a value".into())
                    })?;
                    options.shell_family = Some(value.parse().map_err(
                        |error: crate::shell::ShellFamilyError| {
                            crate::config::ConfigError::Invalid(error.to_string())
                        },
                    )?);
                }
                "--no-transcript" => options.transcript_enabled = Some(false),
                "--transcript-dir" => {
                    let value = args.next().ok_or_else(|| {
                        crate::config::ConfigError::Invalid(
                            "--transcript-dir requires a path".into(),
                        )
                    })?;
                    options.transcript_directory = Some(PathBuf::from(value));
                    options.transcript_enabled = Some(true);
                }
                "--no-color" => options.no_color = true,
                other => {
                    return Err(crate::config::ConfigError::Invalid(format!(
                        "unknown argument: {other}"
                    ))
                    .into());
                }
            }
        }

        Ok(options)
    }

    pub fn help() -> &'static str {
        "Usage: exoshell [--config <path>] [--shell powershell|posix] [--no-transcript] [--transcript-dir <path>] [--no-color]\n\nStarts the Exoshell Phase 1 interactive model chat. Exoshell suggests commands; it does not execute them."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProviderConfig, ShellConfig, TranscriptConfig};

    #[test]
    fn parses_config_path() {
        let options = CliOptions::parse(["--config".to_string(), "config.toml".to_string()])
            .expect("options parse");

        assert_eq!(options.config_path, Some(PathBuf::from("config.toml")));
    }

    #[test]
    fn help_flag_is_supported() {
        let options = CliOptions::parse(["--help".to_string()]).expect("options parse");

        assert!(options.show_help);
    }

    #[test]
    fn parses_phase1_cli_overrides() {
        let options = CliOptions::parse([
            "--shell".to_string(),
            "posix".to_string(),
            "--no-transcript".to_string(),
            "--transcript-dir".to_string(),
            "out".to_string(),
            "--no-color".to_string(),
        ])
        .expect("options parse");

        assert_eq!(options.shell_family, Some(ShellFamily::Posix));
        assert_eq!(options.transcript_enabled, Some(true));
        assert_eq!(options.transcript_directory, Some(PathBuf::from("out")));
        assert!(options.no_color);
    }

    #[test]
    fn app_registers_default_context_providers_on_startup() {
        let app = App::new(test_config(), Box::new(NoopProvider));

        let provider_names: Vec<String> = app
            ._context_registry
            .list()
            .into_iter()
            .map(|metadata| metadata.name)
            .collect();

        assert_eq!(
            provider_names,
            vec![
                "manual".to_string(),
                "file".to_string(),
                "command_output".to_string(),
                "directory_summary".to_string()
            ]
        );
        assert_eq!(app._context_store.total_size().characters, 0);
    }

    struct NoopProvider;

    #[async_trait::async_trait]
    impl Provider for NoopProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            Ok(ChatResponse::Complete("noop".into()))
        }
    }

    fn test_config() -> Config {
        Config {
            provider: ProviderConfig {
                base_url: "http://localhost:11434/v1".into(),
                api_key: "test-key".into(),
                api_key_env: "EXOSHELL_TEST_KEY".into(),
                model: "test-model".into(),
            },
            shell: ShellConfig {
                family: ShellFamily::PowerShell,
            },
            transcript: TranscriptConfig {
                directory: PathBuf::from("transcripts"),
                enabled: false,
            },
        }
    }
}
