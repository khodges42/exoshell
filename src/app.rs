use std::path::PathBuf;
use std::time::Duration;

use crate::commands::{CommandSuggestion, parse_command_suggestions_with_policy};
use crate::config::Config;
use crate::context::{
    ContextError, ContextPriority, ContextProviderRegistry, ContextProviderRequest,
    SessionContextStore, budget_warning, prune_context, register_default_context_providers,
    render_context_details, render_context_list, render_context_stats,
};
use crate::formatting::render_assistant_output_with_policy;
use crate::keybindings::render_keybindings;
use crate::prompts::{Stance, assemble_prompt, render_prompt_estimate};
use crate::providers::{ChatMessage, ChatRequest, ChatResponse, ChatRole, Provider, ProviderError};
use crate::repl::ReplError;
use crate::shell::ShellFamily;
use crate::transcripts::{Transcript, TranscriptError};

pub struct App {
    config: Config,
    provider: Box<dyn Provider>,
    conversation: Vec<ChatMessage>,
    transcript: Transcript,
    context_store: SessionContextStore,
    context_registry: ContextProviderRegistry,
    last_command_suggestions: Vec<CommandSuggestion>,
}

impl App {
    pub fn new(config: Config, provider: Box<dyn Provider>) -> Self {
        let transcript = Transcript::new(
            "openai-compatible".into(),
            config.provider.model.clone(),
            config.shell.family,
            config.interaction.stance,
        );
        let mut context_registry = ContextProviderRegistry::new();
        register_default_context_providers(&mut context_registry)
            .expect("default context providers should register");

        Self {
            config,
            provider,
            conversation: Vec::new(),
            transcript,
            context_store: SessionContextStore::new(),
            context_registry,
            last_command_suggestions: Vec::new(),
        }
    }

    pub async fn send(&mut self, input: String) -> Result<String, AppError> {
        self.conversation
            .push(ChatMessage::new(ChatRole::User, input.clone()));
        self.transcript.record_user(&input);

        let request_messages = self.assembled_messages()?;
        let request = ChatRequest {
            messages: request_messages,
            stream: false,
        };

        let timeout = Duration::from_secs(self.config.provider.request_timeout_seconds);
        let response = match tokio::time::timeout(timeout, self.provider.chat(request)).await {
            Err(_) => {
                let message = format!(
                    "provider request timed out after {} seconds",
                    self.config.provider.request_timeout_seconds
                );
                self.transcript.record_error(&message);
                return Err(AppError::Provider(ProviderError::Network(message)));
            }
            Ok(Ok(ChatResponse::Complete(response))) => response,
            Ok(Ok(ChatResponse::Stream(chunks))) => chunks.concat(),
            Ok(Err(error)) => {
                if let Some(route) = self.provider.last_model_route() {
                    self.transcript.record_model_route(&route);
                }
                self.transcript.record_error(&error.to_string());
                return Err(error.into());
            }
        };
        self.conversation
            .push(ChatMessage::new(ChatRole::Assistant, response.clone()));
        self.transcript.record_assistant(&response);
        if let Some(route) = self.provider.last_model_route() {
            self.transcript.record_model_route(&route);
        }
        self.last_command_suggestions =
            parse_command_suggestions_with_policy(&response, &self.config.commands.risk);
        for suggestion in &self.last_command_suggestions {
            self.transcript.record_command_suggestion(suggestion);
        }

        Ok(response)
    }

    pub fn handle_command(&mut self, input: &str) -> Result<String, AppError> {
        let trimmed = input.trim();
        if trimmed == "/context" {
            return Ok(render_context_list(self.context_store.entries()));
        }

        if trimmed == "/panel" {
            return Ok(self.render_panel());
        }

        if trimmed == "/help" {
            return Ok(help_overview().into());
        }

        if trimmed == "/keys" {
            return Ok(render_keybindings());
        }

        if let Some(topic) = trimmed.strip_prefix("/help ") {
            return Ok(help_topic(topic.trim()).into());
        }

        if trimmed == "/context stats" {
            let prompt_estimate = self.prompt_budget_estimate();
            return Ok(render_context_stats(
                self.context_store.stats(),
                self.config.context.budget(),
            ) + "\n\nPrompt estimate:\n"
                + &render_prompt_estimate(prompt_estimate));
        }

        if trimmed == "/stance" {
            return Ok(self.render_stance_status());
        }

        if let Some(stance) = trimmed.strip_prefix("/stance ") {
            return self.set_stance(stance.trim());
        }

        if let Some(id) = trimmed.strip_prefix("/copy ") {
            return self.copy_command(id.trim());
        }

        if let Some(id) = trimmed.strip_prefix("/explain ") {
            return self.explain_command(id.trim());
        }

        if let Some(id) = trimmed.strip_prefix("/discard ") {
            return self.discard_command(id.trim());
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

    pub fn render_assistant_output(&self, response: &str) -> String {
        render_assistant_output_with_policy(response, &self.config.commands.risk)
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

        Ok(assemble_prompt(
            self.config.shell.family,
            self.config.interaction.stance,
            &self.conversation,
            self.context_store.entries(),
            budget,
        )
        .messages)
    }

    fn prompt_budget_estimate(&self) -> crate::prompts::PromptBudgetEstimate {
        assemble_prompt(
            self.config.shell.family,
            self.config.interaction.stance,
            &self.conversation,
            self.context_store.entries(),
            self.config.context.budget(),
        )
        .estimate
    }

    pub fn stance(&self) -> Stance {
        self.config.interaction.stance
    }

    fn render_stance_status(&self) -> String {
        format!(
            "current stance: {}\navailable stances: {}",
            self.config.interaction.stance,
            Stance::names()
        )
    }

    fn render_panel(&self) -> String {
        format!(
            "Exoshell session\nstance: {}\nshell: {}\nprovider: openai-compatible\nmodel: {}\nrouter: {}\ntranscript: {}\n\nContext\n{}\n\nPrompt estimate\n{}",
            self.config.interaction.stance,
            self.config.shell.family,
            self.config.provider.model,
            self.render_router_status(),
            if self.config.transcript.enabled {
                "enabled"
            } else {
                "disabled"
            },
            render_context_list(self.context_store.entries()),
            render_prompt_estimate(self.prompt_budget_estimate())
        )
    }

    fn render_router_status(&self) -> String {
        if !self.config.router.enabled {
            return "disabled".into();
        }

        let roles = self
            .config
            .router
            .roles
            .iter()
            .map(|role| format!("{}={}", role.name, role.model))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "enabled model={} fallback={} roles=[{}]",
            self.config.router.model, self.config.router.fallback_role, roles
        )
    }

    fn set_stance(&mut self, input: &str) -> Result<String, AppError> {
        let stance = input
            .parse::<Stance>()
            .map_err(|error| crate::config::ConfigError::Invalid(error.to_string()))?;
        let previous = self.config.interaction.stance;
        self.config.interaction.stance = stance;
        self.transcript.record_stance_change(previous, stance);
        Ok(format!("stance: {stance}"))
    }

    fn copy_command(&mut self, id: &str) -> Result<String, AppError> {
        let command = self.command_suggestion(id)?.command.clone();
        self.transcript.record_command_action(
            id,
            "copy",
            "printed command; clipboard support unavailable",
        );
        Ok(format!("clipboard unavailable; command {id}:\n{}", command))
    }

    fn explain_command(&mut self, id: &str) -> Result<String, AppError> {
        let suggestion = self.command_suggestion(id)?.clone();
        let mut explanation = format!(
            "{} is a {} command suggestion.\n\nCommand:\n{}\n\n",
            suggestion.id, suggestion.shell, suggestion.command
        );
        if let Some(summary) = &suggestion.explanation {
            explanation.push_str(&format!("Model note: {summary}\n"));
        }
        if let Some(warning) = suggestion.detected_risk.warning() {
            explanation.push_str(&format!("{warning}\n"));
        } else {
            explanation
                .push_str("No obvious destructive pattern was detected. Review before running.\n");
        }
        self.transcript
            .record_command_action(id, "explain", "operator requested explanation");
        Ok(explanation.trim_end().to_string())
    }

    fn discard_command(&mut self, id: &str) -> Result<String, AppError> {
        {
            let suggestion = self.command_suggestion_mut(id)?;
            suggestion.discarded = true;
        }
        self.transcript
            .record_command_action(id, "discard", "operator discarded suggestion");
        Ok(format!("{id} discarded"))
    }

    fn command_suggestion(&self, id: &str) -> Result<&CommandSuggestion, AppError> {
        self.last_command_suggestions
            .iter()
            .find(|suggestion| suggestion.id == id)
            .ok_or_else(|| ContextError::NotFound(format!("command suggestion '{id}'")).into())
    }

    fn command_suggestion_mut(&mut self, id: &str) -> Result<&mut CommandSuggestion, AppError> {
        self.last_command_suggestions
            .iter_mut()
            .find(|suggestion| suggestion.id == id)
            .ok_or_else(|| ContextError::NotFound(format!("command suggestion '{id}'")).into())
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

fn help_overview() -> &'static str {
    "Commands:
/context                       list attached context
/context stats                 show context and prompt budget estimates
/context show <id>             inspect a context entry
/context enable|disable <id>   control model inclusion
/context pin|unpin <id>        control pruning preference
/context priority <id> <value> set low, normal, high, or critical priority
/add-note <text>               attach manual context
/add-file <path>               attach a UTF-8 file
/add-dir <path>                attach a shallow directory summary
/add-output                    paste command output as explicit context
/stance [name]                 show or set operator, audit, teach, or quiet
/copy <cmd-id>                 print a suggested command; does not execute it
/explain <cmd-id>              explain a suggested command
/discard <cmd-id>              mark a suggested command as discarded
/panel                         show session, stance, provider, and context state
/keys                          show keybinding fallbacks for the line REPL
/multi                         enter multi-line input
/exit                          quit and write transcript if enabled

Exoshell suggests commands. It does not execute them."
}

fn help_topic(topic: &str) -> &'static str {
    match topic {
        "context" => {
            "Context is explicit and session-scoped. Use /add-note, /add-file, /add-dir, or /add-output to attach material. Use /context stats before requests to inspect attached context size and prompt estimates."
        }
        "stance" => {
            "Stances change the compact prompt fragment used for the next request: operator is concise and action-oriented, audit focuses on risks, teach explains more, and quiet minimizes prose while keeping safety warnings."
        }
        "commands" => {
            "Suggested commands appear as fenced shell blocks and get IDs such as cmd-001. Use /copy, /explain, or /discard by ID. Copy prints the command when clipboard support is unavailable and never runs it."
        }
        "keys" => {
            "The current line REPL does not install advanced terminal keybindings. Use /keys to see the predictable slash-command fallbacks for copy, explain, discard, context, and stance actions."
        }
        _ => "Unknown help topic. Try /help context, /help stance, /help commands, or /help keys.",
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CliOptions {
    pub config_path: Option<PathBuf>,
    pub shell_family: Option<ShellFamily>,
    pub stance: Option<Stance>,
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
                "--stance" => {
                    let value = args.next().ok_or_else(|| {
                        crate::config::ConfigError::Invalid("--stance requires a value".into())
                    })?;
                    options.stance = Some(value.parse().map_err(
                        |error: crate::prompts::StanceError| {
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
        "Usage: exoshell [--config <path>] [--shell powershell|posix] [--stance operator|audit|teach|quiet] [--context-note <text>] [--context-file <path>] [--no-transcript] [--transcript-dir <path>] [--no-color]\n\nStarts the Exoshell interactive model chat. Exoshell suggests commands; it does not execute them."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        CommandConfig, InteractionConfig, ProviderConfig, ShellConfig, TranscriptConfig,
    };
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
            "--stance".to_string(),
            "audit".to_string(),
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
        assert_eq!(options.stance, Some(Stance::Audit));
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
        assert!(
            app.handle_command("/context stats")
                .expect("stats")
                .contains("Prompt estimate:")
        );
        assert_eq!(
            app.handle_command("/context remove ctx-001")
                .expect("remove"),
            "removed ctx-001"
        );
    }

    #[test]
    fn stance_command_shows_and_changes_current_stance() {
        let mut app = App::new(test_config(), Box::new(NoopProvider));

        assert_eq!(app.stance(), Stance::Operator);
        assert!(
            app.handle_command("/stance")
                .expect("stance")
                .contains("operator, audit, teach, quiet")
        );
        assert_eq!(
            app.handle_command("/stance audit").expect("set stance"),
            "stance: audit"
        );
        assert_eq!(app.stance(), Stance::Audit);
        assert!(app.handle_command("/stance unknown").is_err());
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
    async fn command_suggestion_actions_use_last_response_ids() {
        let mut app = App::new(
            test_config(),
            Box::new(StaticProvider {
                response: "Inspect:\n```powershell\nGet-ChildItem\n```".into(),
            }),
        );

        app.send("suggest a command".into()).await.expect("send");

        assert!(
            app.handle_command("/copy cmd-001")
                .expect("copy")
                .contains("Get-ChildItem")
        );
        assert!(
            app.handle_command("/explain cmd-001")
                .expect("explain")
                .contains("powershell")
        );
        assert_eq!(
            app.handle_command("/discard cmd-001").expect("discard"),
            "cmd-001 discarded"
        );
        assert!(app.handle_command("/copy cmd-999").is_err());
    }

    #[test]
    fn panel_and_help_render_phase2_controls() {
        let mut app = App::new(test_config(), Box::new(NoopProvider));
        app.handle_command("/add-note repo uses cargo")
            .expect("add note");

        assert!(
            app.handle_command("/panel")
                .expect("panel")
                .contains("stance: operator")
        );
        assert!(
            app.handle_command("/help commands")
                .expect("help")
                .contains("/copy")
        );
        assert!(
            app.handle_command("/keys")
                .expect("keys")
                .contains("/discard <cmd-id>")
        );
        assert!(
            app.handle_command("/help keys")
                .expect("help keys")
                .contains("line REPL")
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

    #[tokio::test]
    async fn provider_request_times_out() {
        let mut config = test_config();
        config.provider.request_timeout_seconds = 0;
        let mut app = App::new(config, Box::new(SlowProvider));

        let error = app
            .send("hello".into())
            .await
            .expect_err("request should time out");

        assert!(error.to_string().contains("timed out"));
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

    struct SlowProvider;

    #[async_trait::async_trait]
    impl Provider for SlowProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            Ok(ChatResponse::Complete("late".into()))
        }
    }

    struct StaticProvider {
        response: String,
    }

    #[async_trait::async_trait]
    impl Provider for StaticProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            Ok(ChatResponse::Complete(self.response.clone()))
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
            router: crate::providers::router::ModelRouterConfig::default(),
            shell: ShellConfig {
                family: ShellFamily::PowerShell,
            },
            interaction: InteractionConfig {
                stance: Stance::Operator,
            },
            commands: CommandConfig {
                risk: crate::commands::CommandRiskPolicy::default(),
            },
            transcript: TranscriptConfig {
                directory: PathBuf::from("transcripts"),
                enabled: false,
            },
            context: crate::config::ContextConfig::default(),
        }
    }
}
