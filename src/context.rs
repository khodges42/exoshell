use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContextEntry {
    pub id: String,
    pub kind: ContextKind,
    pub title: String,
    pub enabled: bool,
    pub pinned: bool,
    pub priority: ContextPriority,
    pub created_at_epoch_ms: u128,
    pub provenance: ContextProvenance,
    pub content: String,
    pub size: ContextSize,
}

impl ContextEntry {
    pub fn new(
        id: impl Into<String>,
        kind: ContextKind,
        title: impl Into<String>,
        provenance: ContextProvenance,
        content: impl Into<String>,
    ) -> Self {
        let content = content.into();
        Self {
            id: id.into(),
            kind,
            title: title.into(),
            enabled: true,
            pinned: false,
            priority: ContextPriority::default(),
            created_at_epoch_ms: unix_millis(),
            provenance,
            size: ContextSize::from_content(&content),
            content,
        }
    }

    pub fn with_priority(mut self, priority: ContextPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextKind {
    File,
    CommandOutput,
    DirectorySummary,
    GitDiff,
    GitHistory,
    GitStatus,
    Log,
    Note,
    SearchResult,
    NotebookEntry,
    Manual,
    Unknown(String),
}

impl fmt::Display for ContextKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File => formatter.write_str("file"),
            Self::CommandOutput => formatter.write_str("command_output"),
            Self::DirectorySummary => formatter.write_str("directory_summary"),
            Self::GitDiff => formatter.write_str("git_diff"),
            Self::GitHistory => formatter.write_str("git_history"),
            Self::GitStatus => formatter.write_str("git_status"),
            Self::Log => formatter.write_str("log"),
            Self::Note => formatter.write_str("note"),
            Self::SearchResult => formatter.write_str("search_result"),
            Self::NotebookEntry => formatter.write_str("notebook_entry"),
            Self::Manual => formatter.write_str("manual"),
            Self::Unknown(value) => write!(formatter, "unknown:{value}"),
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

impl fmt::Display for ContextPriority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => formatter.write_str("low"),
            Self::Normal => formatter.write_str("normal"),
            Self::High => formatter.write_str("high"),
            Self::Critical => formatter.write_str("critical"),
        }
    }
}

impl FromStr for ContextPriority {
    type Err = ContextError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "low" => Ok(Self::Low),
            "normal" => Ok(Self::Normal),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            other => Err(ContextError::InvalidInput(format!(
                "unsupported context priority '{other}', expected low, normal, high, or critical"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContextProvenance {
    pub origin: ContextOrigin,
    pub source_path: Option<PathBuf>,
    pub command: Option<String>,
    pub cwd: Option<PathBuf>,
    pub provider_details: HashMap<String, String>,
    pub sensitive_provider_fields: Vec<String>,
}

impl ContextProvenance {
    pub fn new(origin: ContextOrigin) -> Self {
        Self {
            origin,
            source_path: None,
            command: None,
            cwd: None,
            provider_details: HashMap::new(),
            sensitive_provider_fields: Vec::new(),
        }
    }

    pub fn manual() -> Self {
        Self::new(ContextOrigin::Manual)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextOrigin {
    Manual,
    File,
    CommandOutput,
    Notebook,
    Git,
    Search,
    Generated,
    Stdin,
}

impl fmt::Display for ContextOrigin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manual => formatter.write_str("manual"),
            Self::File => formatter.write_str("file"),
            Self::CommandOutput => formatter.write_str("command_output"),
            Self::Notebook => formatter.write_str("notebook"),
            Self::Git => formatter.write_str("git"),
            Self::Search => formatter.write_str("search"),
            Self::Generated => formatter.write_str("generated"),
            Self::Stdin => formatter.write_str("stdin"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContextSize {
    pub characters: usize,
    pub estimated_tokens: usize,
}

impl ContextSize {
    pub fn from_content(content: &str) -> Self {
        let characters = content.chars().count();
        Self {
            characters,
            estimated_tokens: estimate_tokens(characters),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextProviderMetadata {
    pub name: String,
    pub kind: ContextKind,
    pub description: String,
}

pub trait ContextProvider: Send + Sync {
    fn metadata(&self) -> ContextProviderMetadata;
    fn collect(&self, request: ContextProviderRequest) -> Result<ContextEntry, ContextError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContextProviderRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_code: Option<i32>,
    pub path: Option<PathBuf>,
    pub command: Option<String>,
    pub cwd: Option<PathBuf>,
    pub provider_options: HashMap<String, String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ContextError {
    #[error("context not found: {0}")]
    NotFound(String),
    #[error("permission denied while reading context: {0}")]
    PermissionDenied(String),
    #[error("invalid context input: {0}")]
    InvalidInput(String),
    #[error("unsupported context content: {0}")]
    UnsupportedContent(String),
    #[error("context is too large: {0}")]
    TooLarge(String),
    #[error("context provider failed: {0}")]
    InternalFailure(String),
}

pub struct ContextProviderRegistry {
    providers: HashMap<String, Box<dyn ContextProvider>>,
    order: Vec<String>,
}

impl ContextProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn register(&mut self, provider: Box<dyn ContextProvider>) -> Result<(), ContextError> {
        let metadata = provider.metadata();
        if metadata.name.trim().is_empty() {
            return Err(ContextError::InvalidInput(
                "context provider name is empty".into(),
            ));
        }

        if self.providers.contains_key(&metadata.name) {
            return Err(ContextError::InvalidInput(format!(
                "context provider '{}' is already registered",
                metadata.name
            )));
        }

        self.order.push(metadata.name.clone());
        self.providers.insert(metadata.name, provider);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&dyn ContextProvider> {
        self.providers.get(name).map(|provider| provider.as_ref())
    }

    pub fn list(&self) -> Vec<ContextProviderMetadata> {
        self.order
            .iter()
            .filter_map(|name| self.providers.get(name))
            .map(|provider| provider.metadata())
            .collect()
    }
}

impl Default for ContextProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn register_default_context_providers(
    registry: &mut ContextProviderRegistry,
) -> Result<(), ContextError> {
    registry.register(Box::new(ManualContextProvider))?;
    registry.register(Box::new(FileContextProvider::default()))?;
    registry.register(Box::new(CommandOutputContextProvider))?;
    registry.register(Box::new(StdinContextProvider))?;
    registry.register(Box::new(DirectorySummaryContextProvider::default()))?;
    registry.register(Box::new(GitStatusContextProvider))?;
    registry.register(Box::new(GitDiffContextProvider::default()))?;
    registry.register(Box::new(GitCommitContextProvider))?;
    Ok(())
}

pub struct ManualContextProvider;

impl ContextProvider for ManualContextProvider {
    fn metadata(&self) -> ContextProviderMetadata {
        ContextProviderMetadata {
            name: "manual".into(),
            kind: ContextKind::Manual,
            description: "adds user-provided text as explicit context".into(),
        }
    }

    fn collect(&self, request: ContextProviderRequest) -> Result<ContextEntry, ContextError> {
        let content = required_non_empty_content(request.content)?;
        Ok(ContextEntry::new(
            "",
            ContextKind::Manual,
            request.title.unwrap_or_else(|| "manual context".into()),
            ContextProvenance::manual(),
            content,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct FileContextProvider {
    pub max_bytes: usize,
}

impl Default for FileContextProvider {
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024,
        }
    }
}

impl ContextProvider for FileContextProvider {
    fn metadata(&self) -> ContextProviderMetadata {
        ContextProviderMetadata {
            name: "file".into(),
            kind: ContextKind::File,
            description: "loads a UTF-8 text file as explicit context".into(),
        }
    }

    fn collect(&self, request: ContextProviderRequest) -> Result<ContextEntry, ContextError> {
        let path = request
            .path
            .ok_or_else(|| ContextError::InvalidInput("file path is required".into()))?;
        let metadata = fs::metadata(&path).map_err(|error| file_read_error(&path, error))?;

        if !metadata.is_file() {
            return Err(ContextError::InvalidInput(format!(
                "{} is not a file",
                path.display()
            )));
        }

        let byte_len = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
        if byte_len > self.max_bytes {
            return Err(ContextError::TooLarge(format!(
                "{} is {} bytes; limit is {} bytes",
                path.display(),
                byte_len,
                self.max_bytes
            )));
        }

        let bytes = fs::read(&path).map_err(|error| file_read_error(&path, error))?;
        if bytes.contains(&0) {
            return Err(ContextError::UnsupportedContent(format!(
                "{} appears to be binary",
                path.display()
            )));
        }

        let content = String::from_utf8(bytes).map_err(|error| {
            ContextError::UnsupportedContent(format!(
                "{} is not valid UTF-8: {error}",
                path.display()
            ))
        })?;

        let mut provenance = ContextProvenance::new(ContextOrigin::File);
        provenance.source_path = Some(path.clone());
        if let Ok(modified) = metadata.modified()
            && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
        {
            provenance.provider_details.insert(
                "modified_at_epoch_ms".into(),
                duration.as_millis().to_string(),
            );
        }
        provenance
            .provider_details
            .insert("byte_size".into(), byte_len.to_string());

        Ok(ContextEntry::new(
            "",
            ContextKind::File,
            request
                .title
                .unwrap_or_else(|| file_title(&path).unwrap_or_else(|| path.display().to_string())),
            provenance,
            content,
        ))
    }
}

pub struct CommandOutputContextProvider;

impl ContextProvider for CommandOutputContextProvider {
    fn metadata(&self) -> ContextProviderMetadata {
        ContextProviderMetadata {
            name: "command_output".into(),
            kind: ContextKind::CommandOutput,
            description: "adds user-provided command output without executing commands".into(),
        }
    }

    fn collect(&self, request: ContextProviderRequest) -> Result<ContextEntry, ContextError> {
        let content = command_output_content(&request)?;
        let mut provenance = ContextProvenance::new(ContextOrigin::CommandOutput);
        provenance.command = request.command;
        provenance.cwd = request.cwd;
        provenance
            .provider_details
            .insert("provided_by_user".into(), "true".into());
        if let Some(exit_code) = request.exit_code {
            provenance
                .provider_details
                .insert("exit_code".into(), exit_code.to_string());
        }

        Ok(ContextEntry::new(
            "",
            ContextKind::CommandOutput,
            request.title.unwrap_or_else(|| "command output".into()),
            provenance,
            content,
        ))
    }
}

pub struct StdinContextProvider;

impl ContextProvider for StdinContextProvider {
    fn metadata(&self) -> ContextProviderMetadata {
        ContextProviderMetadata {
            name: "stdin".into(),
            kind: ContextKind::CommandOutput,
            description: "adds piped stdin as explicit context".into(),
        }
    }

    fn collect(&self, request: ContextProviderRequest) -> Result<ContextEntry, ContextError> {
        let content = required_non_empty_content(request.content)?;
        let mut provenance = ContextProvenance::new(ContextOrigin::Stdin);
        provenance.cwd = request.cwd;
        provenance
            .provider_details
            .insert("provided_by_user".into(), "true".into());
        provenance.provider_details.insert(
            "upstream_command_known".into(),
            request.command.is_some().to_string(),
        );
        provenance.command = request.command;

        Ok(ContextEntry::new(
            "",
            ContextKind::CommandOutput,
            request.title.unwrap_or_else(|| "piped stdin".into()),
            provenance,
            content,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct DirectorySummaryContextProvider {
    pub max_depth: usize,
    pub max_entries: usize,
}

impl Default for DirectorySummaryContextProvider {
    fn default() -> Self {
        Self {
            max_depth: 2,
            max_entries: 200,
        }
    }
}

impl ContextProvider for DirectorySummaryContextProvider {
    fn metadata(&self) -> ContextProviderMetadata {
        ContextProviderMetadata {
            name: "directory_summary".into(),
            kind: ContextKind::DirectorySummary,
            description: "summarizes directory names without reading file contents".into(),
        }
    }

    fn collect(&self, request: ContextProviderRequest) -> Result<ContextEntry, ContextError> {
        let path = request
            .path
            .ok_or_else(|| ContextError::InvalidInput("directory path is required".into()))?;
        let metadata = fs::metadata(&path).map_err(|error| file_read_error(&path, error))?;
        if !metadata.is_dir() {
            return Err(ContextError::InvalidInput(format!(
                "{} is not a directory",
                path.display()
            )));
        }

        let mut summary = DirectorySummary::default();
        summarize_directory(&path, 0, self.max_depth, self.max_entries, &mut summary)?;

        let mut provenance = ContextProvenance::new(ContextOrigin::Generated);
        provenance.source_path = Some(path.clone());
        provenance
            .provider_details
            .insert("max_depth".into(), self.max_depth.to_string());
        provenance
            .provider_details
            .insert("max_entries".into(), self.max_entries.to_string());
        provenance
            .provider_details
            .insert("truncated".into(), summary.truncated.to_string());

        Ok(ContextEntry::new(
            "",
            ContextKind::DirectorySummary,
            request
                .title
                .unwrap_or_else(|| format!("directory summary: {}", path.display())),
            provenance,
            summary.lines.join("\n"),
        ))
    }
}

pub struct GitStatusContextProvider;

impl ContextProvider for GitStatusContextProvider {
    fn metadata(&self) -> ContextProviderMetadata {
        ContextProviderMetadata {
            name: "git_status".into(),
            kind: ContextKind::GitStatus,
            description: "captures read-only Git branch and working tree status".into(),
        }
    }

    fn collect(&self, request: ContextProviderRequest) -> Result<ContextEntry, ContextError> {
        let path = request
            .path
            .or(request.cwd)
            .unwrap_or_else(|| PathBuf::from("."));
        let status = git_output(&path, &["status", "--porcelain=v1", "-b"], "git status")?;
        let parsed = parse_git_status_porcelain(&status);

        let mut provenance = ContextProvenance::new(ContextOrigin::Git);
        provenance.source_path = Some(path.clone());
        provenance
            .provider_details
            .insert("branch".into(), parsed.branch.clone());
        provenance
            .provider_details
            .insert("staged_count".into(), parsed.staged.len().to_string());
        provenance
            .provider_details
            .insert("modified_count".into(), parsed.modified.len().to_string());
        provenance
            .provider_details
            .insert("untracked_count".into(), parsed.untracked.len().to_string());

        Ok(ContextEntry::new(
            "",
            ContextKind::GitStatus,
            request.title.unwrap_or_else(|| "git status".into()),
            provenance,
            render_git_status_context(&parsed),
        ))
    }
}

pub struct GitCommitContextProvider;

impl ContextProvider for GitCommitContextProvider {
    fn metadata(&self) -> ContextProviderMetadata {
        ContextProviderMetadata {
            name: "git_commits".into(),
            kind: ContextKind::GitHistory,
            description: "captures recent Git commits and changed files".into(),
        }
    }

    fn collect(&self, request: ContextProviderRequest) -> Result<ContextEntry, ContextError> {
        let path = request
            .path
            .or(request.cwd)
            .unwrap_or_else(|| PathBuf::from("."));
        let count = parse_git_commit_count(request.provider_options.get("count"))?;
        let author = request.provider_options.get("author").cloned();
        let file = request.provider_options.get("file").cloned();
        let log = git_log_output(&path, count, author.as_deref(), file.as_deref())?;
        let content = if log.trim().is_empty() {
            "recent commits: none".to_string()
        } else {
            log
        };

        let mut provenance = ContextProvenance::new(ContextOrigin::Git);
        provenance.source_path = Some(path);
        provenance
            .provider_details
            .insert("count".into(), count.to_string());
        provenance.provider_details.insert(
            "author_filter".into(),
            author.unwrap_or_else(|| "none".into()),
        );
        provenance
            .provider_details
            .insert("path_filter".into(), file.unwrap_or_else(|| "none".into()));

        Ok(ContextEntry::new(
            "",
            ContextKind::GitHistory,
            request
                .title
                .unwrap_or_else(|| format!("recent git commits ({count})")),
            provenance,
            content,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct GitDiffContextProvider {
    pub max_characters: usize,
}

impl Default for GitDiffContextProvider {
    fn default() -> Self {
        Self {
            max_characters: 20_000,
        }
    }
}

impl ContextProvider for GitDiffContextProvider {
    fn metadata(&self) -> ContextProviderMetadata {
        ContextProviderMetadata {
            name: "git_diff".into(),
            kind: ContextKind::GitDiff,
            description: "captures read-only staged or unstaged Git diffs".into(),
        }
    }

    fn collect(&self, request: ContextProviderRequest) -> Result<ContextEntry, ContextError> {
        let path = request
            .path
            .or(request.cwd)
            .unwrap_or_else(|| PathBuf::from("."));
        let mode = request
            .provider_options
            .get("mode")
            .map(String::as_str)
            .unwrap_or("unstaged");
        let file = request.provider_options.get("file").cloned();

        let mut args = vec!["diff"];
        match mode {
            "staged" => args.push("--staged"),
            "unstaged" => {}
            other => {
                return Err(ContextError::InvalidInput(format!(
                    "unsupported git diff mode '{other}', expected staged or unstaged"
                )));
            }
        }
        if file.is_some() {
            args.push("--");
        }
        if let Some(file) = file.as_deref() {
            args.push(file);
        }

        let diff = git_output(&path, &args, "git diff")?;
        let truncated = truncate_visible(&diff, self.max_characters);

        let mut provenance = ContextProvenance::new(ContextOrigin::Git);
        provenance.source_path = Some(path.clone());
        provenance
            .provider_details
            .insert("mode".into(), mode.to_string());
        provenance
            .provider_details
            .insert("file".into(), file.clone().unwrap_or_else(|| "all".into()));
        provenance.provider_details.insert(
            "truncated".into(),
            (truncated.omitted_characters > 0).to_string(),
        );
        provenance.provider_details.insert(
            "omitted_characters".into(),
            truncated.omitted_characters.to_string(),
        );

        Ok(ContextEntry::new(
            "",
            ContextKind::GitDiff,
            request
                .title
                .unwrap_or_else(|| git_diff_title(mode, file.as_deref())),
            provenance,
            if truncated.content.trim().is_empty() {
                format!("mode: {mode}\ndiff: none")
            } else {
                format!("mode: {mode}\n{}", truncated.content)
            },
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedGitStatus {
    branch: String,
    staged: Vec<String>,
    modified: Vec<String>,
    untracked: Vec<String>,
}

fn parse_git_status_porcelain(output: &str) -> ParsedGitStatus {
    let mut parsed = ParsedGitStatus {
        branch: "unknown".into(),
        staged: Vec::new(),
        modified: Vec::new(),
        untracked: Vec::new(),
    };

    for line in output.lines() {
        if let Some(branch) = line.strip_prefix("## ") {
            parsed.branch = branch.trim().to_string();
            continue;
        }

        if line.len() < 3 {
            continue;
        }

        let mut chars = line.chars();
        let index_status = chars.next().unwrap_or(' ');
        let worktree_status = chars.next().unwrap_or(' ');
        let path = chars.as_str().trim().to_string();
        if path.is_empty() {
            continue;
        }

        if index_status == '?' && worktree_status == '?' {
            parsed.untracked.push(path);
            continue;
        }

        if index_status != ' ' {
            parsed.staged.push(path.clone());
        }
        if worktree_status != ' ' {
            parsed.modified.push(path);
        }
    }

    parsed
}

fn render_git_status_context(status: &ParsedGitStatus) -> String {
    let mut rendered = String::new();
    rendered.push_str(&format!("branch: {}\n", status.branch));
    rendered.push_str("staged:\n");
    render_path_list(&mut rendered, &status.staged);
    rendered.push_str("modified:\n");
    render_path_list(&mut rendered, &status.modified);
    rendered.push_str("untracked:\n");
    render_path_list(&mut rendered, &status.untracked);
    rendered.trim_end().to_string()
}

fn render_path_list(rendered: &mut String, paths: &[String]) {
    if paths.is_empty() {
        rendered.push_str("- none\n");
    } else {
        for path in paths {
            rendered.push_str(&format!("- {path}\n"));
        }
    }
}

fn parse_git_commit_count(value: Option<&String>) -> Result<usize, ContextError> {
    let Some(value) = value else {
        return Ok(5);
    };
    let count = value.parse::<usize>().map_err(|_| {
        ContextError::InvalidInput(format!(
            "git commit count must be a positive integer: {value}"
        ))
    })?;
    if count == 0 || count > 100 {
        return Err(ContextError::InvalidInput(
            "git commit count must be between 1 and 100".into(),
        ));
    }
    Ok(count)
}

fn git_log_output(
    path: &Path,
    count: usize,
    author: Option<&str>,
    file: Option<&str>,
) -> Result<String, ContextError> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(path)
        .arg("log")
        .arg(format!("--max-count={count}"))
        .arg("--date=iso-strict")
        .arg("--pretty=format:commit: %H%nshort: %h%nauthor: %an <%ae>%ndate: %ad%nsubject: %s")
        .arg("--name-only");
    if let Some(author) = author {
        command.arg(format!("--author={author}"));
    }
    if let Some(file) = file {
        command.arg("--").arg(file);
    }

    let output = command.output().map_err(|error| {
        ContextError::InternalFailure(format!("failed to run git log: {error}"))
    })?;
    if output.status.success() {
        return String::from_utf8(output.stdout).map_err(|error| {
            ContextError::UnsupportedContent(format!("git log output was not valid UTF-8: {error}"))
        });
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.contains("does not have any commits yet")
        || stderr.contains("your current branch") && stderr.contains("no commits")
    {
        return Ok(String::new());
    }

    Err(ContextError::InternalFailure(format!(
        "git log failed for {}: {}",
        path.display(),
        if stderr.is_empty() {
            output.status.to_string()
        } else {
            stderr
        }
    )))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TruncatedContent {
    content: String,
    omitted_characters: usize,
}

fn truncate_visible(content: &str, max_characters: usize) -> TruncatedContent {
    let total = content.chars().count();
    if total <= max_characters {
        return TruncatedContent {
            content: content.to_string(),
            omitted_characters: 0,
        };
    }

    let kept = content.chars().take(max_characters).collect::<String>();
    let omitted = total.saturating_sub(max_characters);
    TruncatedContent {
        content: format!("{kept}\n\n[truncated: omitted {omitted} characters]"),
        omitted_characters: omitted,
    }
}

fn git_diff_title(mode: &str, file: Option<&str>) -> String {
    match file {
        Some(file) => format!("git diff ({mode}): {file}"),
        None => format!("git diff ({mode})"),
    }
}

fn git_output(path: &Path, args: &[&str], label: &str) -> Result<String, ContextError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .map_err(|error| {
            ContextError::InternalFailure(format!("failed to run {label}: {error}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(ContextError::InternalFailure(format!(
            "{label} failed for {}: {}",
            path.display(),
            if stderr.is_empty() {
                output.status.to_string()
            } else {
                stderr
            }
        )));
    }

    String::from_utf8(output.stdout).map_err(|error| {
        ContextError::UnsupportedContent(format!("{label} output was not valid UTF-8: {error}"))
    })
}

#[derive(Debug, Clone, Default)]
pub struct SessionContextStore {
    entries: Vec<ContextEntry>,
    next_id: u64,
}

impl SessionContextStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add(&mut self, mut entry: ContextEntry) -> String {
        let id = self.next_context_id();
        entry.id = id.clone();
        entry.size = ContextSize::from_content(&entry.content);
        self.entries.push(entry);
        id
    }

    pub fn remove(&mut self, id: &str) -> Option<ContextEntry> {
        let index = self.entries.iter().position(|entry| entry.id == id)?;
        Some(self.entries.remove(index))
    }

    pub fn get(&self, id: &str) -> Option<&ContextEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut ContextEntry> {
        self.entries.iter_mut().find(|entry| entry.id == id)
    }

    pub fn entries(&self) -> &[ContextEntry] {
        &self.entries
    }

    pub fn iter(&self) -> impl Iterator<Item = &ContextEntry> {
        self.entries.iter()
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), ContextError> {
        let entry = self
            .get_mut(id)
            .ok_or_else(|| ContextError::NotFound(id.to_string()))?;
        entry.enabled = enabled;
        Ok(())
    }

    pub fn set_pinned(&mut self, id: &str, pinned: bool) -> Result<(), ContextError> {
        let entry = self
            .get_mut(id)
            .ok_or_else(|| ContextError::NotFound(id.to_string()))?;
        entry.pinned = pinned;
        Ok(())
    }

    pub fn set_priority(
        &mut self,
        id: &str,
        priority: ContextPriority,
    ) -> Result<(), ContextError> {
        let entry = self
            .get_mut(id)
            .ok_or_else(|| ContextError::NotFound(id.to_string()))?;
        entry.priority = priority;
        Ok(())
    }

    pub fn total_size(&self) -> ContextSize {
        self.entries.iter().filter(|entry| entry.enabled).fold(
            ContextSize {
                characters: 0,
                estimated_tokens: 0,
            },
            |total, entry| ContextSize {
                characters: total.characters + entry.size.characters,
                estimated_tokens: total.estimated_tokens + entry.size.estimated_tokens,
            },
        )
    }

    pub fn stats(&self) -> ContextStats {
        let total_entries = self.entries.len();
        let enabled_entries = self.entries.iter().filter(|entry| entry.enabled).count();
        let disabled_entries = total_entries.saturating_sub(enabled_entries);
        let pinned_entries = self.entries.iter().filter(|entry| entry.pinned).count();
        let size = self.total_size();

        ContextStats {
            total_entries,
            enabled_entries,
            disabled_entries,
            pinned_entries,
            enabled_size: size,
        }
    }

    fn next_context_id(&mut self) -> String {
        let id = format!("ctx-{:03}", self.next_id);
        self.next_id += 1;
        id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextStats {
    pub total_entries: usize,
    pub enabled_entries: usize,
    pub disabled_entries: usize,
    pub pinned_entries: usize,
    pub enabled_size: ContextSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContextBudget {
    pub max_characters: Option<usize>,
    pub max_estimated_tokens: Option<usize>,
}

impl ContextBudget {
    pub fn is_over_budget(&self, size: ContextSize) -> bool {
        self.max_characters.is_some_and(|max| size.characters > max)
            || self
                .max_estimated_tokens
                .is_some_and(|max| size.estimated_tokens > max)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPruneResult {
    pub included_ids: Vec<String>,
    pub removed_ids: Vec<String>,
    pub final_size: ContextSize,
    pub over_budget: bool,
}

pub fn prune_context(entries: &[ContextEntry], budget: ContextBudget) -> ContextPruneResult {
    let enabled: Vec<&ContextEntry> = entries.iter().filter(|entry| entry.enabled).collect();
    let mut included_ids: Vec<String> = enabled.iter().map(|entry| entry.id.clone()).collect();
    let mut final_size = combined_size(&enabled);

    if !budget.is_over_budget(final_size) {
        return ContextPruneResult {
            included_ids,
            removed_ids: Vec::new(),
            final_size,
            over_budget: false,
        };
    }

    let mut candidates = enabled;
    candidates.sort_by_key(|entry| {
        (
            entry.pinned,
            entry.priority,
            entries
                .iter()
                .position(|candidate| candidate.id == entry.id)
                .unwrap_or(usize::MAX),
        )
    });

    let mut removed_ids = Vec::new();
    for candidate in candidates {
        if !budget.is_over_budget(final_size) {
            break;
        }

        included_ids.retain(|id| id != &candidate.id);
        removed_ids.push(candidate.id.clone());
        final_size.characters = final_size
            .characters
            .saturating_sub(candidate.size.characters);
        final_size.estimated_tokens = final_size
            .estimated_tokens
            .saturating_sub(candidate.size.estimated_tokens);
    }

    ContextPruneResult {
        included_ids,
        removed_ids,
        final_size,
        over_budget: budget.is_over_budget(final_size),
    }
}

pub fn render_prompt_context(entries: &[ContextEntry]) -> String {
    let mut rendered = String::new();
    for entry in entries.iter().filter(|entry| entry.enabled) {
        rendered.push_str(&format!("[Context: {}]\n", entry.id));
        rendered.push_str(&format!("Type: {}\n", entry.kind));
        rendered.push_str(&format!("Title: {}\n", entry.title));
        rendered.push_str(&format!("Origin: {}\n", entry.provenance.origin));
        if let Some(path) = &entry.provenance.source_path {
            rendered.push_str(&format!("Path: {}\n", path.display()));
        }
        if let Some(command) = &entry.provenance.command {
            rendered.push_str(&format!("Command: {command}\n"));
        }
        if let Some(cwd) = &entry.provenance.cwd {
            rendered.push_str(&format!("Cwd: {}\n", cwd.display()));
        }
        if let Some(truncated) = entry.provenance.provider_details.get("truncated") {
            rendered.push_str(&format!("Truncated: {truncated}\n"));
        }
        rendered.push_str(&format!("Priority: {}\n", entry.priority));
        rendered.push('\n');
        rendered.push_str(&entry.content);
        rendered.push_str("\n\n");
    }

    rendered.trim_end().to_string()
}

pub fn render_context_list(entries: &[ContextEntry]) -> String {
    if entries.is_empty() {
        return "No context entries.".into();
    }

    let mut rendered = String::from("ID       Type               State     Pin  Priority  Size\n");
    for entry in entries {
        rendered.push_str(&format!(
            "{:<8} {:<18} {:<9} {:<4} {:<9} {} chars / ~{} tokens  {}\n",
            entry.id,
            entry.kind,
            if entry.enabled { "enabled" } else { "disabled" },
            if entry.pinned { "yes" } else { "no" },
            entry.priority,
            entry.size.characters,
            entry.size.estimated_tokens,
            entry.title
        ));
    }

    rendered.trim_end().to_string()
}

pub fn render_context_details(entry: &ContextEntry) -> String {
    let mut rendered = String::new();
    rendered.push_str(&format!("ID: {}\n", entry.id));
    rendered.push_str(&format!("Type: {}\n", entry.kind));
    rendered.push_str(&format!("Title: {}\n", entry.title));
    rendered.push_str(&format!("Enabled: {}\n", entry.enabled));
    rendered.push_str(&format!("Pinned: {}\n", entry.pinned));
    rendered.push_str(&format!("Priority: {}\n", entry.priority));
    rendered.push_str(&format!(
        "Size: {} chars / ~{} tokens\n",
        entry.size.characters, entry.size.estimated_tokens
    ));
    rendered.push_str(&format!("Origin: {}\n", entry.provenance.origin));
    if let Some(path) = &entry.provenance.source_path {
        rendered.push_str(&format!("Path: {}\n", path.display()));
    }
    if let Some(command) = &entry.provenance.command {
        rendered.push_str(&format!("Command: {command}\n"));
    }
    if let Some(cwd) = &entry.provenance.cwd {
        rendered.push_str(&format!("Cwd: {}\n", cwd.display()));
    }
    for (key, value) in redacted_provider_details(&entry.provenance) {
        rendered.push_str(&format!("{key}: {value}\n"));
    }
    rendered.push('\n');
    rendered.push_str(&entry.content);
    rendered
}

pub fn render_context_stats(stats: ContextStats, budget: ContextBudget) -> String {
    let mut rendered = String::new();
    rendered.push_str(&format!("total_entries: {}\n", stats.total_entries));
    rendered.push_str(&format!("enabled_entries: {}\n", stats.enabled_entries));
    rendered.push_str(&format!("disabled_entries: {}\n", stats.disabled_entries));
    rendered.push_str(&format!("pinned_entries: {}\n", stats.pinned_entries));
    rendered.push_str(&format!("characters: {}\n", stats.enabled_size.characters));
    rendered.push_str(&format!(
        "estimated_tokens: {}\n",
        stats.enabled_size.estimated_tokens
    ));
    if let Some(max) = budget.max_characters {
        rendered.push_str(&format!(
            "character_budget: {}/{}\n",
            stats.enabled_size.characters, max
        ));
    }
    if let Some(max) = budget.max_estimated_tokens {
        rendered.push_str(&format!(
            "token_budget: {}/{}\n",
            stats.enabled_size.estimated_tokens, max
        ));
    }

    rendered.trim_end().to_string()
}

pub fn render_prune_result(result: &ContextPruneResult) -> String {
    format!(
        "included: {}\nremoved: {}\nfinal_size: {} chars / ~{} tokens\nover_budget: {}",
        comma_list(&result.included_ids),
        comma_list(&result.removed_ids),
        result.final_size.characters,
        result.final_size.estimated_tokens,
        result.over_budget
    )
}

pub fn budget_warning(
    size: ContextSize,
    budget: ContextBudget,
    result: &ContextPruneResult,
) -> String {
    let mut exceeded = Vec::new();
    if let Some(max) = budget.max_characters
        && size.characters > max
    {
        exceeded.push(format!("characters {} > {}", size.characters, max));
    }
    if let Some(max) = budget.max_estimated_tokens
        && size.estimated_tokens > max
    {
        exceeded.push(format!(
            "estimated_tokens {} > {}",
            size.estimated_tokens, max
        ));
    }

    format!(
        "context budget exceeded: {}. No context was silently dropped.\n{}",
        exceeded.join(", "),
        render_prune_result(result)
    )
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SerializedContext {
    pub version: u32,
    pub entries: Vec<SerializedContextEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SerializedContextEntry {
    pub entry: ContextEntry,
    pub content_included: bool,
}

pub fn serialize_context(entries: &[ContextEntry], include_content: bool) -> SerializedContext {
    SerializedContext {
        version: 1,
        entries: entries
            .iter()
            .cloned()
            .map(|mut entry| {
                if !include_content {
                    entry.content.clear();
                    entry.size = ContextSize::from_content("");
                }
                SerializedContextEntry {
                    entry,
                    content_included: include_content,
                }
            })
            .collect(),
    }
}

pub fn redacted_provider_details(provenance: &ContextProvenance) -> Vec<(String, String)> {
    let mut details: Vec<(String, String)> = provenance
        .provider_details
        .iter()
        .map(|(key, value)| {
            if provenance.sensitive_provider_fields.contains(key) || looks_sensitive(key) {
                (key.clone(), "[redacted]".into())
            } else {
                (key.clone(), value.clone())
            }
        })
        .collect();
    details.sort_by(|left, right| left.0.cmp(&right.0));
    details
}

fn combined_size(entries: &[&ContextEntry]) -> ContextSize {
    entries.iter().fold(
        ContextSize {
            characters: 0,
            estimated_tokens: 0,
        },
        |total, entry| ContextSize {
            characters: total.characters + entry.size.characters,
            estimated_tokens: total.estimated_tokens + entry.size.estimated_tokens,
        },
    )
}

fn comma_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".into()
    } else {
        values.join(", ")
    }
}

fn looks_sensitive(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("secret")
        || key.contains("token")
        || key.contains("password")
        || key.contains("api_key")
        || key.contains("apikey")
}

fn estimate_tokens(characters: usize) -> usize {
    characters.div_ceil(4)
}

fn required_non_empty_content(content: Option<String>) -> Result<String, ContextError> {
    let content =
        content.ok_or_else(|| ContextError::InvalidInput("context content is required".into()))?;
    if content.trim().is_empty() {
        return Err(ContextError::InvalidInput(
            "context content cannot be empty".into(),
        ));
    }

    Ok(content)
}

fn command_output_content(request: &ContextProviderRequest) -> Result<String, ContextError> {
    if request.stdout.is_none() && request.stderr.is_none() {
        return required_non_empty_content(request.content.clone());
    }

    let mut parts = Vec::new();
    if let Some(stdout) = &request.stdout
        && !stdout.trim().is_empty()
    {
        parts.push(format!("stdout:\n{stdout}"));
    }
    if let Some(stderr) = &request.stderr
        && !stderr.trim().is_empty()
    {
        parts.push(format!("stderr:\n{stderr}"));
    }

    if parts.is_empty() {
        return Err(ContextError::InvalidInput(
            "command output cannot be empty".into(),
        ));
    }

    Ok(parts.join("\n\n"))
}

fn file_read_error(path: &Path, error: std::io::Error) -> ContextError {
    match error.kind() {
        std::io::ErrorKind::NotFound => ContextError::NotFound(path.display().to_string()),
        std::io::ErrorKind::PermissionDenied => {
            ContextError::PermissionDenied(path.display().to_string())
        }
        _ => ContextError::InternalFailure(format!("{}: {error}", path.display())),
    }
}

fn file_title(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
}

#[derive(Default)]
struct DirectorySummary {
    lines: Vec<String>,
    truncated: bool,
}

fn summarize_directory(
    path: &Path,
    depth: usize,
    max_depth: usize,
    max_entries: usize,
    summary: &mut DirectorySummary,
) -> Result<(), ContextError> {
    if depth > max_depth || summary.lines.len() >= max_entries {
        summary.truncated = true;
        return Ok(());
    }

    let mut entries = fs::read_dir(path)
        .map_err(|error| file_read_error(path, error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| file_read_error(path, error))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        if summary.lines.len() >= max_entries {
            summary.truncated = true;
            break;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        if is_noisy_path(&file_name) {
            continue;
        }

        let entry_path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| file_read_error(&entry_path, error))?;
        let suffix = if kind.is_dir() { "/" } else { "" };
        let indent = "  ".repeat(depth);
        summary.lines.push(format!("{indent}{file_name}{suffix}"));

        if kind.is_dir() {
            summarize_directory(&entry_path, depth + 1, max_depth, max_entries, summary)?;
        }
    }

    Ok(())
}

fn is_noisy_path(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "target"
            | "node_modules"
            | ".venv"
            | "venv"
            | "dist"
            | "build"
            | ".next"
            | ".cache"
    )
}

fn unix_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before UNIX epoch")
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_entry_serializes_round_trip() {
        let mut provenance = ContextProvenance::new(ContextOrigin::File);
        provenance.source_path = Some(PathBuf::from("Cargo.toml"));
        let entry = ContextEntry::new(
            "ctx-001",
            ContextKind::File,
            "Cargo.toml",
            provenance,
            "[package]\nname = \"exoshell\"",
        )
        .with_priority(ContextPriority::High)
        .with_pinned(true);

        let json = serde_json::to_string(&entry).expect("serialize entry");
        let decoded: ContextEntry = serde_json::from_str(&json).expect("deserialize entry");

        assert_eq!(decoded.id, "ctx-001");
        assert_eq!(decoded.kind, ContextKind::File);
        assert_eq!(decoded.priority, ContextPriority::High);
        assert!(decoded.pinned);
        assert_eq!(decoded.provenance.origin, ContextOrigin::File);
        assert_eq!(decoded.size.characters, entry.content.chars().count());
    }

    #[test]
    fn priority_defaults_to_normal_and_orders_for_pruning() {
        let entry = ContextEntry::new(
            "ctx-001",
            ContextKind::Manual,
            "note",
            ContextProvenance::manual(),
            "content",
        );

        assert_eq!(entry.priority, ContextPriority::Normal);
        assert!(ContextPriority::Low < ContextPriority::Normal);
        assert!(ContextPriority::Normal < ContextPriority::High);
        assert!(ContextPriority::High < ContextPriority::Critical);
    }

    #[test]
    fn registry_registers_lists_and_rejects_duplicates() {
        let mut registry = ContextProviderRegistry::new();
        registry
            .register(Box::new(FakeProvider::new("manual")))
            .expect("register provider");

        assert!(registry.get("manual").is_some());
        assert_eq!(registry.list()[0].name, "manual");

        let error = registry
            .register(Box::new(FakeProvider::new("manual")))
            .expect_err("duplicate should fail");

        assert!(matches!(error, ContextError::InvalidInput(_)));
    }

    #[test]
    fn provider_trait_returns_context_entries() {
        let provider = FakeProvider::new("manual");
        let entry = provider
            .collect(ContextProviderRequest {
                content: Some("operator note".into()),
                ..ContextProviderRequest::default()
            })
            .expect("collect entry");

        assert_eq!(entry.kind, ContextKind::Manual);
        assert_eq!(entry.content, "operator note");
    }

    #[test]
    fn store_generates_stable_human_readable_ids_and_mutates_state() {
        let mut store = SessionContextStore::new();
        let first = store.add(sample_entry("placeholder", ContextPriority::Normal, false));
        let second = store.add(sample_entry("placeholder", ContextPriority::Normal, false));

        assert_eq!(first, "ctx-001");
        assert_eq!(second, "ctx-002");
        assert_eq!(store.entries()[0].id, "ctx-001");

        store.set_enabled(&first, false).expect("disable");
        store.set_pinned(&first, true).expect("pin");
        store
            .set_priority(&first, ContextPriority::Critical)
            .expect("priority");

        let entry = store.get(&first).expect("entry exists");
        assert!(!entry.enabled);
        assert!(entry.pinned);
        assert_eq!(entry.priority, ContextPriority::Critical);

        assert_eq!(store.remove(&first).expect("removed").id, "ctx-001");
        assert!(store.get(&first).is_none());
        assert_eq!(store.get(&second).expect("second remains").id, "ctx-002");
    }

    #[test]
    fn budget_calculation_uses_enabled_entries_only() {
        let mut store = SessionContextStore::new();
        let enabled_id = store.add(sample_entry("a", ContextPriority::Normal, false));
        let disabled_id = store.add(sample_entry("12345678", ContextPriority::Normal, false));
        store.set_enabled(&disabled_id, false).expect("disable");

        let size = store.total_size();

        assert_eq!(store.get(&enabled_id).expect("enabled").size.characters, 1);
        assert_eq!(size.characters, 1);
        assert_eq!(size.estimated_tokens, 1);
    }

    #[test]
    fn pruning_removes_low_priority_unpinned_entries_first() {
        let entries = vec![
            sample_entry("low", ContextPriority::Low, false),
            sample_entry("critical", ContextPriority::Critical, false),
            sample_entry("pinned low", ContextPriority::Low, true),
        ];
        let result = prune_context(
            &entries,
            ContextBudget {
                max_characters: Some(18),
                max_estimated_tokens: None,
            },
        );

        assert_eq!(result.removed_ids, vec!["ctx-low"]);
        assert_eq!(
            result.included_ids,
            vec!["ctx-critical".to_string(), "ctx-pinned-low".to_string()]
        );
        assert!(!result.over_budget);
    }

    #[test]
    fn prompt_context_renderer_skips_disabled_entries_and_keeps_metadata() {
        let mut file = sample_entry("file body", ContextPriority::High, false);
        file.kind = ContextKind::File;
        file.title = "Cargo.toml".into();
        file.provenance = ContextProvenance::new(ContextOrigin::File);
        file.provenance.source_path = Some(PathBuf::from("/repo/Cargo.toml"));
        let disabled = sample_entry("disabled", ContextPriority::Normal, false).with_enabled(false);

        let rendered = render_prompt_context(&[file, disabled]);

        assert!(rendered.contains("[Context: ctx-file-body]"));
        assert!(rendered.contains("Type: file"));
        assert!(rendered.contains("Title: Cargo.toml"));
        assert!(rendered.contains("Origin: file"));
        assert!(rendered.contains("Path: /repo/Cargo.toml"));
        assert!(rendered.contains("file body"));
        assert!(!rendered.contains("disabled"));
    }

    #[test]
    fn user_facing_renderers_show_list_details_stats_and_pruning() {
        let mut entry = sample_entry("visible", ContextPriority::High, true);
        entry.kind = ContextKind::Note;
        entry.title = "operator note".into();
        let entries = vec![entry.clone()];

        let list = render_context_list(&entries);
        assert!(list.contains("ctx-visible"));
        assert!(list.contains("operator note"));
        assert!(list.contains("high"));

        let details = render_context_details(&entry);
        assert!(details.contains("ID: ctx-visible"));
        assert!(details.contains("Pinned: true"));
        assert!(details.contains("visible"));

        let stats = render_context_stats(
            ContextStats {
                total_entries: 1,
                enabled_entries: 1,
                disabled_entries: 0,
                pinned_entries: 1,
                enabled_size: entry.size,
            },
            ContextBudget {
                max_characters: Some(100),
                max_estimated_tokens: Some(25),
            },
        );
        assert!(stats.contains("total_entries: 1"));
        assert!(stats.contains("character_budget: 7/100"));

        let prune = render_prune_result(&ContextPruneResult {
            included_ids: vec!["ctx-visible".into()],
            removed_ids: vec!["ctx-old".into()],
            final_size: entry.size,
            over_budget: false,
        });
        assert!(prune.contains("included: ctx-visible"));
        assert!(prune.contains("removed: ctx-old"));
    }

    #[test]
    fn context_serialization_excludes_content_by_default() {
        let entry = sample_entry("secret-ish body", ContextPriority::Normal, false);

        let without_content = serialize_context(std::slice::from_ref(&entry), false);
        assert_eq!(without_content.version, 1);
        assert!(!without_content.entries[0].content_included);
        assert!(without_content.entries[0].entry.content.is_empty());

        let with_content = serialize_context(std::slice::from_ref(&entry), true);
        assert!(with_content.entries[0].content_included);
        assert_eq!(with_content.entries[0].entry.content, "secret-ish body");
    }

    #[test]
    fn provider_details_redact_sensitive_fields() {
        let mut provenance = ContextProvenance::manual();
        provenance
            .provider_details
            .insert("api_key".into(), "sk-test".into());
        provenance
            .provider_details
            .insert("plain".into(), "visible".into());
        provenance
            .provider_details
            .insert("custom".into(), "hidden".into());
        provenance.sensitive_provider_fields.push("custom".into());

        let details = redacted_provider_details(&provenance);

        assert!(details.contains(&("api_key".into(), "[redacted]".into())));
        assert!(details.contains(&("custom".into(), "[redacted]".into())));
        assert!(details.contains(&("plain".into(), "visible".into())));
    }

    #[test]
    fn budget_warning_identifies_exceeded_limits() {
        let warning = budget_warning(
            ContextSize {
                characters: 10,
                estimated_tokens: 3,
            },
            ContextBudget {
                max_characters: Some(5),
                max_estimated_tokens: Some(2),
            },
            &ContextPruneResult {
                included_ids: vec!["ctx-002".into()],
                removed_ids: vec!["ctx-001".into()],
                final_size: ContextSize {
                    characters: 4,
                    estimated_tokens: 1,
                },
                over_budget: false,
            },
        );

        assert!(warning.contains("characters 10 > 5"));
        assert!(warning.contains("estimated_tokens 3 > 2"));
        assert!(warning.contains("removed: ctx-001"));
    }

    #[test]
    fn context_errors_render_user_facing_messages() {
        assert_eq!(
            ContextError::NotFound("ctx-999".into()).to_string(),
            "context not found: ctx-999"
        );
        assert_eq!(
            ContextError::TooLarge("file exceeds limit".into()).to_string(),
            "context is too large: file exceeds limit"
        );
    }

    #[test]
    fn default_providers_register_in_order() {
        let mut registry = ContextProviderRegistry::new();
        register_default_context_providers(&mut registry).expect("register defaults");

        let names: Vec<String> = registry
            .list()
            .into_iter()
            .map(|metadata| metadata.name)
            .collect();

        assert_eq!(
            names,
            vec![
                "manual".to_string(),
                "file".to_string(),
                "command_output".to_string(),
                "stdin".to_string(),
                "directory_summary".to_string(),
                "git_status".to_string(),
                "git_diff".to_string(),
                "git_commits".to_string()
            ]
        );
    }

    #[test]
    fn parses_git_status_porcelain_into_context_sections() {
        let parsed = parse_git_status_porcelain(
            "## main...origin/main\n M src/app.rs\nM  Cargo.toml\nMM src/context.rs\n?? notes.md\n",
        );

        assert_eq!(parsed.branch, "main...origin/main");
        assert_eq!(
            parsed.staged,
            vec!["Cargo.toml".to_string(), "src/context.rs".to_string()]
        );
        assert_eq!(
            parsed.modified,
            vec!["src/app.rs".to_string(), "src/context.rs".to_string()]
        );
        assert_eq!(parsed.untracked, vec!["notes.md".to_string()]);

        let rendered = render_git_status_context(&parsed);
        assert!(rendered.contains("branch: main...origin/main"));
        assert!(rendered.contains("staged:"));
        assert!(rendered.contains("- Cargo.toml"));
        assert!(rendered.contains("untracked:"));
    }

    #[test]
    fn git_status_provider_metadata_is_git_context() {
        let metadata = GitStatusContextProvider.metadata();

        assert_eq!(metadata.name, "git_status");
        assert_eq!(metadata.kind, ContextKind::GitStatus);
    }

    #[test]
    fn git_diff_provider_metadata_is_git_context() {
        let metadata = GitDiffContextProvider::default().metadata();

        assert_eq!(metadata.name, "git_diff");
        assert_eq!(metadata.kind, ContextKind::GitDiff);
    }

    #[test]
    fn git_diff_truncation_is_visible() {
        let truncated = truncate_visible("abcdef", 3);

        assert_eq!(truncated.omitted_characters, 3);
        assert!(truncated.content.contains("abc"));
        assert!(
            truncated
                .content
                .contains("[truncated: omitted 3 characters]")
        );
    }

    #[test]
    fn git_commit_provider_metadata_is_git_context() {
        let metadata = GitCommitContextProvider.metadata();

        assert_eq!(metadata.name, "git_commits");
        assert_eq!(metadata.kind, ContextKind::GitHistory);
    }

    #[test]
    fn git_commit_count_is_bounded() {
        assert_eq!(parse_git_commit_count(None).expect("default"), 5);
        assert_eq!(
            parse_git_commit_count(Some(&"10".to_string())).expect("count"),
            10
        );
        assert!(parse_git_commit_count(Some(&"0".to_string())).is_err());
        assert!(parse_git_commit_count(Some(&"101".to_string())).is_err());
        assert!(parse_git_commit_count(Some(&"abc".to_string())).is_err());
    }

    #[test]
    fn git_commit_provider_handles_repositories_without_commits() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        std::process::Command::new("git")
            .arg("init")
            .arg(tempdir.path())
            .output()
            .expect("git init");

        let entry = GitCommitContextProvider
            .collect(ContextProviderRequest {
                path: Some(tempdir.path().to_path_buf()),
                ..ContextProviderRequest::default()
            })
            .expect("empty git log");

        assert_eq!(entry.kind, ContextKind::GitHistory);
        assert_eq!(entry.content, "recent commits: none");
    }

    #[test]
    fn manual_provider_rejects_empty_context() {
        let error = ManualContextProvider
            .collect(ContextProviderRequest {
                content: Some("   ".into()),
                ..ContextProviderRequest::default()
            })
            .expect_err("empty manual context should fail");

        assert!(matches!(error, ContextError::InvalidInput(_)));
    }

    #[test]
    fn file_provider_loads_utf8_text_with_provenance() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("note.txt");
        std::fs::write(&path, "hello file").expect("write file");

        let entry = FileContextProvider { max_bytes: 1024 }
            .collect(ContextProviderRequest {
                path: Some(path.clone()),
                ..ContextProviderRequest::default()
            })
            .expect("file context");

        assert_eq!(entry.kind, ContextKind::File);
        assert_eq!(entry.title, "note.txt");
        assert_eq!(entry.content, "hello file");
        assert_eq!(entry.provenance.origin, ContextOrigin::File);
        assert_eq!(entry.provenance.source_path, Some(path));
        assert_eq!(
            entry.provenance.provider_details.get("byte_size"),
            Some(&"10".to_string())
        );
    }

    #[test]
    fn file_provider_rejects_missing_binary_and_oversized_files() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let missing = tempdir.path().join("missing.txt");
        let missing_error = FileContextProvider { max_bytes: 1024 }
            .collect(ContextProviderRequest {
                path: Some(missing),
                ..ContextProviderRequest::default()
            })
            .expect_err("missing file should fail");
        assert!(matches!(missing_error, ContextError::NotFound(_)));

        let binary = tempdir.path().join("binary.bin");
        std::fs::write(&binary, [1, 0, 2]).expect("write binary");
        let binary_error = FileContextProvider { max_bytes: 1024 }
            .collect(ContextProviderRequest {
                path: Some(binary),
                ..ContextProviderRequest::default()
            })
            .expect_err("binary file should fail");
        assert!(matches!(binary_error, ContextError::UnsupportedContent(_)));

        let large = tempdir.path().join("large.txt");
        std::fs::write(&large, "abcdef").expect("write large");
        let large_error = FileContextProvider { max_bytes: 3 }
            .collect(ContextProviderRequest {
                path: Some(large),
                ..ContextProviderRequest::default()
            })
            .expect_err("large file should fail");
        assert!(matches!(large_error, ContextError::TooLarge(_)));
    }

    #[test]
    fn command_output_provider_labels_user_provided_output() {
        let entry = CommandOutputContextProvider
            .collect(ContextProviderRequest {
                title: Some("cargo test output".into()),
                stdout: Some("test result: ok".into()),
                stderr: Some("warning: slow test".into()),
                command: Some("cargo test".into()),
                cwd: Some(PathBuf::from("/repo")),
                exit_code: Some(0),
                ..ContextProviderRequest::default()
            })
            .expect("command output context");

        assert_eq!(entry.kind, ContextKind::CommandOutput);
        assert!(entry.content.contains("stdout:\ntest result: ok"));
        assert!(entry.content.contains("stderr:\nwarning: slow test"));
        assert_eq!(entry.provenance.origin, ContextOrigin::CommandOutput);
        assert_eq!(entry.provenance.command, Some("cargo test".into()));
        assert_eq!(entry.provenance.cwd, Some(PathBuf::from("/repo")));
        assert_eq!(
            entry.provenance.provider_details.get("provided_by_user"),
            Some(&"true".to_string())
        );
        assert_eq!(
            entry.provenance.provider_details.get("exit_code"),
            Some(&"0".to_string())
        );
    }

    #[test]
    fn command_output_provider_accepts_stdout_only_and_stderr_only() {
        let stdout = CommandOutputContextProvider
            .collect(ContextProviderRequest {
                stdout: Some("ok".into()),
                ..ContextProviderRequest::default()
            })
            .expect("stdout only");
        assert_eq!(stdout.content, "stdout:\nok");

        let stderr = CommandOutputContextProvider
            .collect(ContextProviderRequest {
                stderr: Some("failed".into()),
                ..ContextProviderRequest::default()
            })
            .expect("stderr only");
        assert_eq!(stderr.content, "stderr:\nfailed");
    }

    #[test]
    fn stdin_provider_records_stdin_provenance_without_guessing_command() {
        let entry = StdinContextProvider
            .collect(ContextProviderRequest {
                content: Some("piped text".into()),
                cwd: Some(PathBuf::from("/repo")),
                ..ContextProviderRequest::default()
            })
            .expect("stdin context");

        assert_eq!(entry.kind, ContextKind::CommandOutput);
        assert_eq!(entry.provenance.origin, ContextOrigin::Stdin);
        assert_eq!(entry.provenance.command, None);
        assert_eq!(entry.provenance.cwd, Some(PathBuf::from("/repo")));
        assert_eq!(
            entry
                .provenance
                .provider_details
                .get("upstream_command_known"),
            Some(&"false".to_string())
        );
    }

    #[test]
    fn directory_summary_provider_skips_noisy_paths_and_truncates() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(tempdir.path().join(".git")).expect("git dir");
        std::fs::create_dir(tempdir.path().join("src")).expect("src dir");
        std::fs::write(tempdir.path().join("src").join("main.rs"), "fn main() {}")
            .expect("main file");
        std::fs::write(tempdir.path().join("README.md"), "# readme").expect("readme");

        let entry = DirectorySummaryContextProvider {
            max_depth: 2,
            max_entries: 2,
        }
        .collect(ContextProviderRequest {
            path: Some(tempdir.path().to_path_buf()),
            ..ContextProviderRequest::default()
        })
        .expect("directory summary");

        assert_eq!(entry.kind, ContextKind::DirectorySummary);
        assert!(!entry.content.contains(".git"));
        assert!(entry.content.contains("README.md") || entry.content.contains("src/"));
        assert_eq!(
            entry.provenance.provider_details.get("truncated"),
            Some(&"true".to_string())
        );
    }

    struct FakeProvider {
        name: String,
    }

    impl FakeProvider {
        fn new(name: &str) -> Self {
            Self { name: name.into() }
        }
    }

    impl ContextProvider for FakeProvider {
        fn metadata(&self) -> ContextProviderMetadata {
            ContextProviderMetadata {
                name: self.name.clone(),
                kind: ContextKind::Manual,
                description: "fake manual provider".into(),
            }
        }

        fn collect(&self, request: ContextProviderRequest) -> Result<ContextEntry, ContextError> {
            let content = request
                .content
                .ok_or_else(|| ContextError::InvalidInput("content is required".into()))?;

            Ok(ContextEntry::new(
                "",
                ContextKind::Manual,
                request.title.unwrap_or_else(|| "manual".into()),
                ContextProvenance::manual(),
                content,
            ))
        }
    }

    fn sample_entry(content: &str, priority: ContextPriority, pinned: bool) -> ContextEntry {
        ContextEntry::new(
            format!("ctx-{}", content.replace(' ', "-")),
            ContextKind::Manual,
            content,
            ContextProvenance::manual(),
            content,
        )
        .with_priority(priority)
        .with_pinned(pinned)
    }
}
