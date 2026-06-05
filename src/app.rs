use std::path::PathBuf;

use crate::config::Config;
use crate::context::{
    ContextError, ContextPriority, ContextProviderRegistry, ContextProviderRequest,
    SessionContextStore, budget_warning, prune_context, register_default_context_providers,
    render_context_details, render_context_list, render_context_stats, render_prompt_context,
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
    context_store: SessionContextStore,
    context_registry: ContextProviderRegistry,
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
            context_store: SessionContextStore::new(),
            context_registry,
        }
    }

    pub async fn send(&mut self, input: String) -> Result<String, AppError> {
        self.messages
            .push(ChatMessage::new(ChatRole::User, input.clone()));
        self.transcript.record_user(&input);

        let request_messages = self.assembled_messages()?;
        let request = ChatRequest {
            messages: request_messages,
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

    pub fn handle_command(&mut self, input: &str) -> Result<String, AppError> {
        let trimmed = input.trim();
        if trimmed == "/context" {
            return Ok(render_context_list(self.context_store.entries()));
        }

        if trimmed == "/context stats" {
            return Ok(render_context_stats(
                self.context_store.stats(),
                self.config.context.budget(),
            ));
        }

        if let Some(id) = trimmed.strip_prefix("/context show ") {
            let id = id.trim();
            let entry = self
                .context_store
                .get(id)
                .ok_or_else(|| ContextError::NotFound(id.to_string()))?;
            return Ok(render_context_details(entry));
        }

        if let Some(id) = trimmed.strip_prefix("/context remove ") {
            let id = id.trim();
            let entry = self
                .context_store
                .remove(id)
                .ok_or_else(|| ContextError::NotFound(id.to_string()))?;
            self.transcript
                .record_context_event("remove", &entry, "removed from session context");
            return Ok(format!("removed {}", entry.id));
        }

        if let Some(id) = trimmed.strip_prefix("/context enable ") {
            return self.set_enabled(id.trim(), true);
        }

        if let Some(id) = trimmed.strip_prefix("/context disable ") {
            return self.set_enabled(id.trim(), false);
        }

        if let Some(id) = trimmed.strip_prefix("/context pin ") {
            return self.set_pinned(id.trim(), true);
        }

        if let Some(id) = trimmed.strip_prefix("/context unpin ") {
            return self.set_pinned(id.trim(), false);
        }

        if let Some(rest) = trimmed.strip_prefix("/context priority ") {
            let (id, priority) = parse_context_priority_args(rest)?;
            self.context_store.set_priority(id, priority)?;
            let entry = self
                .context_store
                .get(id)
                .ok_or_else(|| ContextError::NotFound(id.to_string()))?;
            self.transcript.record_context_event(
                "priority",
                entry,
                &format!("priority set to {priority}"),
            );
            return Ok(format!("{} priority: {}", id, priority));
        }

        if let Some(content) = trimmed.strip_prefix("/add-note ") {
            return self.add_context(
                "manual",
                ContextProviderRequest {
                    content: Some(content.trim().to_string()),
                    ..ContextProviderRequest::default()
                },
            );
        }

        if let Some(path) = trimmed.strip_prefix("/add-file ") {
            return self.add_context(
                "file",
                ContextProviderRequest {
                    path: Some(PathBuf::from(path.trim())),
                    ..ContextProviderRequest::default()
                },
            );
        }

        if let Some(path) = trimmed.strip_prefix("/add-dir ") {
            return self.add_context(
                "directory_summary",
                ContextProviderRequest {
                    path: Some(PathBuf::from(path.trim())),
                    ..ContextProviderRequest::default()
                },
            );
        }

        Err(ContextError::InvalidInput(format!("unknown context command: {trimmed}")).into())
    }

    pub fn add_command_output(
        &mut self,
        stdout: String,
        command: Option<String>,
        exit_code: Option<i32>,
    ) -> Result<String, AppError> {
        self.add_context(
            "command_output",
            ContextProviderRequest {
                stdout: Some(stdout),
                command,
                exit_code,
                cwd: std::env::current_dir().ok(),
                ..ContextProviderRequest::default()
            },
        )
    }

    pub fn add_stdin_context(&mut self, content: String) -> Result<String, AppError> {
        self.add_context(
            "stdin",
            ContextProviderRequest {
                content: Some(content),
                cwd: std::env::current_dir().ok(),
                ..ContextProviderRequest::default()
            },
        )
    }

    pub fn add_note_context(&mut self, content: String) -> Result<String, AppError> {
        self.add_context(
            "manual",
            ContextProviderRequest {
                content: Some(content),
                ..ContextProviderRequest::default()
            },
        )
    }

    pub fn add_file_context(&mut self, path: PathBuf) -> Result<String, AppError> {
        self.add_context(
            "file",
            ContextProviderRequest {
                path: Some(path),
                ..ContextProviderRequest::default()
            },
        )
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

    fn assembled_messages(&mut self) -> Result<Vec<ChatMessage>, AppError> {
        let size = self.context_store.total_size();
        let budget = self.config.context.budget();
        if budget.is_over_budget(size) {
            let prune_result = prune_context(self.context_store.entries(), budget);
            let warning = budget_warning(size, budget, &prune_result);
            self.transcript.record_budget_warning(&warning);
            return Err(ContextError::TooLarge(warning).into());
        }

        let context = render_prompt_context(self.context_store.entries());
        if context.is_empty() {
            return Ok(self.messages.clone());
        }

        let mut messages = self.messages.clone();
        let insert_at = messages.len().saturating_sub(1);
        messages.insert(
            insert_at,
            ChatMessage::new(
                ChatRole::User,
                format!(
                    "Explicit session context selected by the operator follows.\n\n{}",
                    context
                ),
            ),
        );
        Ok(messages)
    }

    fn add_context(
        &mut self,
        provider_name: &str,
        request: ContextProviderRequest,
    ) -> Result<String, AppError> {
        let provider = self
            .context_registry
            .get(provider_name)
            .ok_or_else(|| ContextError::NotFound(format!("context provider '{provider_name}'")))?;
        let entry = provider.collect(request)?;
        let id = self.context_store.add(entry);
        let entry = self
            .context_store
            .get(&id)
            .ok_or_else(|| ContextError::NotFound(id.clone()))?;
        self.transcript
            .record_context_event("add", entry, "added to session context");
        Ok(format!("added {} ({})", entry.id, entry.title))
    }

    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<String, AppError> {
        self.context_store.set_enabled(id, enabled)?;
        let entry = self
            .context_store
            .get(id)
            .ok_or_else(|| ContextError::NotFound(id.to_string()))?;
        self.transcript.record_context_event(
            if enabled { "enable" } else { "disable" },
            entry,
            if enabled {
                "enabled for model requests"
            } else {
                "disabled for model requests"
            },
        );
        Ok(format!(
            "{} {}",
            id,
            if enabled { "enabled" } else { "disabled" }
        ))
    }

    fn set_pinned(&mut self, id: &str, pinned: bool) -> Result<String, AppError> {
        self.context_store.set_pinned(id, pinned)?;
        let entry = self
            .context_store
            .get(id)
            .ok_or_else(|| ContextError::NotFound(id.to_string()))?;
        self.transcript.record_context_event(
            if pinned { "pin" } else { "unpin" },
            entry,
            if pinned {
                "pinned for pruning"
            } else {
                "unpinned for pruning"
            },
        );
        Ok(format!(
            "{} {}",
            id,
            if pinned { "pinned" } else { "unpinned" }
        ))
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
    #[error(transparent)]
    Context(#[from] ContextError),
}

fn parse_context_priority_args(input: &str) -> Result<(&str, ContextPriority), ContextError> {
    let mut parts = input.split_whitespace();
    let id = parts
        .next()
        .ok_or_else(|| ContextError::InvalidInput("context ID is required".into()))?;
    let priority = parts
        .next()
        .ok_or_else(|| ContextError::InvalidInput("priority is required".into()))?
        .parse::<ContextPriority>()?;
    if parts.next().is_some() {
        return Err(ContextError::InvalidInput(
            "too many arguments for context priority".into(),
        ));
    }
    Ok((id, priority))
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CliOptions {
    pub config_path: Option<PathBuf>,
    pub shell_family: Option<ShellFamily>,
    pub transcript_enabled: Option<bool>,
    pub transcript_directory: Option<PathBuf>,
    pub context_notes: Vec<String>,
    pub context_files: Vec<PathBuf>,
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
                "--context-note" => {
                    let value = args.next().ok_or_else(|| {
                        crate::config::ConfigError::Invalid("--context-note requires text".into())
                    })?;
                    options.context_notes.push(value);
                }
                "--context-file" => {
                    let value = args.next().ok_or_else(|| {
                        crate::config::ConfigError::Invalid("--context-file requires a path".into())
                    })?;
                    options.context_files.push(PathBuf::from(value));
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
        "Usage: exoshell [--config <path>] [--shell powershell|posix] [--context-note <text>] [--context-file <path>] [--no-transcript] [--transcript-dir <path>] [--no-color]\n\nStarts the Exoshell interactive model chat. Exoshell suggests commands; it does not execute them."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProviderConfig, ShellConfig, TranscriptConfig};
    use std::sync::{Arc, Mutex};

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
            "--context-note".to_string(),
            "note".to_string(),
            "--context-file".to_string(),
            "Cargo.toml".to_string(),
            "--no-color".to_string(),
        ])
        .expect("options parse");

        assert_eq!(options.shell_family, Some(ShellFamily::Posix));
        assert_eq!(options.transcript_enabled, Some(true));
        assert_eq!(options.transcript_directory, Some(PathBuf::from("out")));
        assert_eq!(options.context_notes, vec!["note".to_string()]);
        assert_eq!(options.context_files, vec![PathBuf::from("Cargo.toml")]);
        assert!(options.no_color);
    }

    #[test]
    fn app_registers_default_context_providers_on_startup() {
        let app = App::new(test_config(), Box::new(NoopProvider));

        let provider_names: Vec<String> = app
            .context_registry
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
                "stdin".to_string(),
                "directory_summary".to_string()
            ]
        );
        assert_eq!(app.context_store.total_size().characters, 0);
    }

    #[test]
    fn context_commands_add_and_mutate_entries() {
        let mut app = App::new(test_config(), Box::new(NoopProvider));

        assert_eq!(
            app.handle_command("/add-note inspect Cargo.toml")
                .expect("add note"),
            "added ctx-001 (manual context)"
        );
        assert!(
            app.handle_command("/context")
                .expect("list")
                .contains("ctx-001")
        );
        assert!(
            app.handle_command("/context show ctx-001")
                .expect("show")
                .contains("inspect Cargo.toml")
        );
        assert_eq!(
            app.handle_command("/context priority ctx-001 high")
                .expect("priority"),
            "ctx-001 priority: high"
        );
        assert_eq!(
            app.handle_command("/context disable ctx-001")
                .expect("disable"),
            "ctx-001 disabled"
        );
        assert_eq!(
            app.handle_command("/context pin ctx-001").expect("pin"),
            "ctx-001 pinned"
        );
        assert!(
            app.handle_command("/context stats")
                .expect("stats")
                .contains("total_entries: 1")
        );
        assert_eq!(
            app.handle_command("/context remove ctx-001")
                .expect("remove"),
            "removed ctx-001"
        );
    }

    #[tokio::test]
    async fn enabled_context_is_inserted_before_current_user_prompt() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let provider = CapturingProvider { seen: seen.clone() };
        let mut app = App::new(test_config(), Box::new(provider));
        app.handle_command("/add-note repo uses cargo")
            .expect("add note");

        app.send("what should I inspect?".into())
            .await
            .expect("send");

        let messages = seen.lock().expect("seen lock").clone();
        assert_eq!(
            messages.last().expect("last").content,
            "what should I inspect?"
        );
        assert!(
            messages
                .iter()
                .any(|message| message.content.contains("[Context: ctx-001]")
                    && message.content.contains("repo uses cargo"))
        );
    }

    #[tokio::test]
    async fn disabled_context_is_omitted_from_provider_request() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let provider = CapturingProvider { seen: seen.clone() };
        let mut app = App::new(test_config(), Box::new(provider));
        app.handle_command("/add-note hidden context")
            .expect("add note");
        app.handle_command("/context disable ctx-001")
            .expect("disable");

        app.send("hello".into()).await.expect("send");

        let messages = seen.lock().expect("seen lock").clone();
        assert!(
            !messages
                .iter()
                .any(|message| message.content.contains("hidden context"))
        );
    }

    #[tokio::test]
    async fn over_budget_context_fails_before_provider_request() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let provider = CapturingProvider { seen: seen.clone() };
        let mut config = test_config();
        config.context.max_characters = Some(3);
        let mut app = App::new(config, Box::new(provider));
        app.handle_command("/add-note too much context")
            .expect("add note");

        let error = app
            .send("hello".into())
            .await
            .expect_err("budget should stop request");

        assert!(error.to_string().contains("context budget exceeded"));
        assert!(seen.lock().expect("seen lock").is_empty());
    }

    #[test]
    fn stdin_context_uses_default_provider_path() {
        let mut app = App::new(test_config(), Box::new(NoopProvider));

        let message = app
            .add_stdin_context("from pipe".into())
            .expect("stdin context");

        assert_eq!(message, "added ctx-001 (piped stdin)");
        let entry = app.context_store.get("ctx-001").expect("entry");
        assert_eq!(entry.provenance.origin.to_string(), "stdin");
        assert_eq!(entry.content, "from pipe");
    }

    struct NoopProvider;

    #[async_trait::async_trait]
    impl Provider for NoopProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            Ok(ChatResponse::Complete("noop".into()))
        }
    }

    struct CapturingProvider {
        seen: Arc<Mutex<Vec<ChatMessage>>>,
    }

    #[async_trait::async_trait]
    impl Provider for CapturingProvider {
        async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            *self.seen.lock().expect("seen lock") = request.messages;
            Ok(ChatResponse::Complete("captured".into()))
        }
    }

    fn test_config() -> Config {
        Config {
            provider: ProviderConfig {
                base_url: "http://localhost:11434/v1".into(),
                api_key: "test-key".into(),
                api_key_env: "EXOSHELL_TEST_KEY".into(),
                model: "test-model".into(),
                request_timeout_seconds: 120,
            },
            shell: ShellConfig {
                family: ShellFamily::PowerShell,
            },
            transcript: TranscriptConfig {
                directory: PathBuf::from("transcripts"),
                enabled: false,
            },
            context: crate::config::ContextConfig::default(),
        }
    }
}
