use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    pub root: PathBuf,
    pub git_dir: PathBuf,
    pub head: ProjectHead,
}

impl ProjectInfo {
    pub fn branch_label(&self) -> String {
        self.head.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectHead {
    Branch(String),
    Detached(String),
    Unknown,
}

impl fmt::Display for ProjectHead {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Branch(branch) => formatter.write_str(branch),
            Self::Detached(commit) => write!(formatter, "detached at {commit}"),
            Self::Unknown => formatter.write_str("unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLocalConfig {
    pub honor_gitignore: bool,
    pub ignore: Vec<String>,
}

impl Default for ProjectLocalConfig {
    fn default() -> Self {
        Self {
            honor_gitignore: true,
            ignore: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfigReport {
    pub root: PathBuf,
    pub shared_path: PathBuf,
    pub local_path: PathBuf,
    pub shared_loaded: bool,
    pub local_loaded: bool,
    pub config: ProjectLocalConfig,
    pub warnings: Vec<String>,
}

pub fn detect_project(
    start: &Path,
    root_override: Option<&Path>,
) -> Result<Option<ProjectInfo>, ProjectError> {
    let search_start = resolve_start(start, root_override);
    let Some((root, git_dir)) = find_git_root(&search_start)? else {
        return Ok(None);
    };
    let head = read_project_head(&git_dir)?;

    Ok(Some(ProjectInfo {
        root,
        git_dir,
        head,
    }))
}

pub fn render_project_status(project: Option<&ProjectInfo>) -> String {
    let Some(project) = project else {
        return "Project\nstatus: not detected".into();
    };

    format!(
        "Project\nroot: {}\nbranch: {}\ngit_dir: {}",
        project.root.display(),
        project.branch_label(),
        project.git_dir.display()
    )
}

pub fn load_project_config(root: &Path) -> Result<ProjectConfigReport, ProjectError> {
    let shared_path = root.join(".exoshell.toml");
    let local_path = root.join(".exoshell.local.toml");
    let mut config = ProjectLocalConfig::default();
    let mut warnings = Vec::new();
    let mut shared_loaded = false;
    let mut local_loaded = false;

    if shared_path.exists() {
        let loaded = read_project_config_file(&shared_path)?;
        config = merge_project_config(config, loaded.config);
        warnings.extend(loaded.warnings);
        shared_loaded = true;
    }

    if local_path.exists() {
        let loaded = read_project_config_file(&local_path)?;
        config = merge_project_config(config, loaded.config);
        warnings.extend(loaded.warnings);
        local_loaded = true;
    }

    Ok(ProjectConfigReport {
        root: root.to_path_buf(),
        shared_path,
        local_path,
        shared_loaded,
        local_loaded,
        config,
        warnings,
    })
}

pub fn render_project_config(report: &ProjectConfigReport) -> String {
    let mut rendered = String::new();
    rendered.push_str("Project Config\n");
    rendered.push_str(&format!("root: {}\n", report.root.display()));
    rendered.push_str(&format!(
        "shared: {} ({})\n",
        report.shared_path.display(),
        if report.shared_loaded {
            "loaded"
        } else {
            "missing"
        }
    ));
    rendered.push_str(&format!(
        "local: {} ({})\n",
        report.local_path.display(),
        if report.local_loaded {
            "loaded"
        } else {
            "missing"
        }
    ));
    rendered.push_str(&format!(
        "honor_gitignore: {}\n",
        report.config.honor_gitignore
    ));
    rendered.push_str("ignore:\n");
    render_lines(&mut rendered, &report.config.ignore);
    rendered.push_str("warnings:\n");
    render_lines(&mut rendered, &report.warnings);
    rendered.trim_end().to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryIgnoreRules {
    root: PathBuf,
    patterns: Vec<IgnorePattern>,
}

impl RepositoryIgnoreRules {
    pub fn from_project_config(
        root: &Path,
        config: &ProjectLocalConfig,
    ) -> Result<Self, ProjectError> {
        Self::from_parts(root, config.honor_gitignore, config.ignore.clone())
    }

    pub fn from_parts(
        root: &Path,
        honor_gitignore: bool,
        exoshell_patterns: Vec<String>,
    ) -> Result<Self, ProjectError> {
        let mut patterns = Vec::new();
        if honor_gitignore {
            patterns.extend(read_gitignore_patterns(root)?);
        }
        patterns.extend(
            exoshell_patterns
                .into_iter()
                .filter_map(IgnorePattern::parse),
        );

        Ok(Self {
            root: root.to_path_buf(),
            patterns,
        })
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        let relative = relative_display(&self.root, path);
        if relative.is_empty() {
            return false;
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default();
        let mut ignored = is_noisy_project_path(&name);
        for pattern in &self.patterns {
            if pattern.matches(&relative) {
                ignored = !pattern.negated;
            }
        }
        ignored
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IgnorePattern {
    pattern: String,
    negated: bool,
    directory_only: bool,
    anchored: bool,
}

impl IgnorePattern {
    fn parse(raw: String) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('#') {
            return None;
        }
        let (negated, raw) = raw
            .strip_prefix('!')
            .map(|pattern| (true, pattern))
            .unwrap_or((false, raw));
        let anchored = raw.starts_with('/');
        let raw = raw.trim_start_matches('/');
        let directory_only = raw.ends_with('/');
        let pattern = raw.trim_end_matches('/').trim();
        if pattern.is_empty() {
            return None;
        }

        Some(Self {
            pattern: pattern.replace('\\', "/"),
            negated,
            directory_only,
            anchored,
        })
    }

    fn matches(&self, relative: &str) -> bool {
        let relative = relative.replace('\\', "/");
        if self.directory_only {
            return relative == self.pattern || relative.starts_with(&format!("{}/", self.pattern));
        }
        if self.anchored || self.pattern.contains('/') {
            return wildcard_match(&self.pattern, &relative);
        }
        relative
            .split('/')
            .any(|segment| wildcard_match(&self.pattern, segment))
    }
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    if pattern == value {
        return true;
    }
    if !pattern.contains('*') {
        return false;
    }

    let mut remaining = value;
    let anchored_start = !pattern.starts_with('*');
    let anchored_end = !pattern.ends_with('*');
    let parts = pattern
        .split('*')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return true;
    }

    for (index, part) in parts.iter().enumerate() {
        let Some(position) = remaining.find(part) else {
            return false;
        };
        if index == 0 && anchored_start && position != 0 {
            return false;
        }
        remaining = &remaining[position + part.len()..];
    }

    !anchored_end || remaining.is_empty()
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
struct RawProjectConfigFile {
    project: Option<RawProjectLocalConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
struct RawProjectLocalConfig {
    honor_gitignore: Option<bool>,
    ignore: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoadedProjectConfig {
    config: RawProjectLocalConfig,
    warnings: Vec<String>,
}

fn read_project_config_file(path: &Path) -> Result<LoadedProjectConfig, ProjectError> {
    let contents = fs::read_to_string(path).map_err(|error| ProjectError::Read {
        path: path.to_path_buf(),
        error: error.to_string(),
    })?;
    let raw: RawProjectConfigFile =
        toml::from_str(&contents).map_err(|error| ProjectError::ConfigParse {
            path: path.to_path_buf(),
            error: error.to_string(),
        })?;
    let value: toml::Value =
        toml::from_str(&contents).map_err(|error| ProjectError::ConfigParse {
            path: path.to_path_buf(),
            error: error.to_string(),
        })?;
    let mut warnings = Vec::new();
    collect_secret_warnings(path, &value, "", &mut warnings);

    Ok(LoadedProjectConfig {
        config: raw.project.unwrap_or_default(),
        warnings,
    })
}

fn merge_project_config(
    mut base: ProjectLocalConfig,
    override_config: RawProjectLocalConfig,
) -> ProjectLocalConfig {
    if let Some(honor_gitignore) = override_config.honor_gitignore {
        base.honor_gitignore = honor_gitignore;
    }
    if let Some(ignore) = override_config.ignore {
        base.ignore = ignore;
    }
    base
}

fn read_gitignore_patterns(root: &Path) -> Result<Vec<IgnorePattern>, ProjectError> {
    let path = root.join(".gitignore");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(&path).map_err(|error| ProjectError::Read {
        path,
        error: error.to_string(),
    })?;
    Ok(contents
        .lines()
        .filter_map(|line| IgnorePattern::parse(line.to_string()))
        .collect())
}

fn collect_secret_warnings(
    path: &Path,
    value: &toml::Value,
    prefix: &str,
    warnings: &mut Vec<String>,
) {
    let Some(table) = value.as_table() else {
        return;
    };
    for (key, value) in table {
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}.{key}")
        };
        if looks_secret_key(key) {
            warnings.push(format!(
                "possible secret key '{full_key}' in {}; project config should not store secrets",
                path.file_name()
                    .map(|name| name.to_string_lossy())
                    .unwrap_or_else(|| path.as_os_str().to_string_lossy())
            ));
        }
        collect_secret_warnings(path, value, &full_key, warnings);
    }
}

fn looks_secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("secret")
        || key.contains("token")
        || key.contains("password")
        || key.contains("api_key")
        || key.contains("apikey")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
    pub root: PathBuf,
    pub branch: String,
    pub major_directories: Vec<String>,
    pub languages: Vec<LanguageSummary>,
    pub entry_points: Vec<String>,
    pub build_files: Vec<String>,
    pub files_seen: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSummary {
    pub language: String,
    pub files: usize,
}

pub fn summarize_project(
    start: &Path,
    root_override: Option<&Path>,
) -> Result<ProjectSummary, ProjectError> {
    let project = detect_project(start, root_override)?.ok_or(ProjectError::NotDetected)?;
    let config = load_project_config(&project.root)?;
    let ignore_rules = RepositoryIgnoreRules::from_project_config(&project.root, &config.config)?;
    Ok(summarize_project_root(&project, &ignore_rules))
}

pub fn render_project_summary(summary: &ProjectSummary) -> String {
    let mut rendered = String::new();
    rendered.push_str(&format!(
        "Project Summary\nroot: {}\n",
        summary.root.display()
    ));
    rendered.push_str(&format!("branch: {}\n", summary.branch));
    rendered.push_str(&format!("files_seen: {}\n", summary.files_seen));
    rendered.push_str(&format!("truncated: {}\n\n", summary.truncated));

    rendered.push_str("major_directories:\n");
    render_lines(&mut rendered, &summary.major_directories);
    rendered.push_str("\nlanguages:\n");
    if summary.languages.is_empty() {
        rendered.push_str("- none\n");
    } else {
        for language in &summary.languages {
            rendered.push_str(&format!(
                "- {}: {} files\n",
                language.language, language.files
            ));
        }
    }
    rendered.push_str("\nentry_points:\n");
    render_lines(&mut rendered, &summary.entry_points);
    rendered.push_str("\nbuild_files:\n");
    render_lines(&mut rendered, &summary.build_files);

    rendered.trim_end().to_string()
}

fn summarize_project_root(
    project: &ProjectInfo,
    ignore_rules: &RepositoryIgnoreRules,
) -> ProjectSummary {
    const MAX_FILES: usize = 1_000;
    let major_directories = major_directories(&project.root, ignore_rules);
    let mut languages = BTreeMap::<String, usize>::new();
    let mut entry_points = Vec::new();
    let mut build_files = Vec::new();
    let mut files_seen = 0;
    let mut truncated = false;
    let mut stack = vec![project.root.clone()];

    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let entry_path = entry.path();
            if ignore_rules.is_ignored(&entry_path) {
                continue;
            }
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                stack.push(entry_path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            files_seen += 1;
            if files_seen > MAX_FILES {
                truncated = true;
                break;
            }

            let relative = relative_display(&project.root, &entry_path);
            if let Some(language) = language_for_path(&entry_path) {
                *languages.entry(language.into()).or_insert(0) += 1;
            }
            if is_likely_entry_point(&relative) {
                entry_points.push(relative.clone());
            }
            if is_build_file(&relative) {
                build_files.push(relative);
            }
        }

        if truncated {
            break;
        }
    }

    let mut languages = languages
        .into_iter()
        .map(|(language, files)| LanguageSummary { language, files })
        .collect::<Vec<_>>();
    languages.sort_by(|left, right| {
        right
            .files
            .cmp(&left.files)
            .then(left.language.cmp(&right.language))
    });
    entry_points.sort();
    entry_points.dedup();
    build_files.sort();
    build_files.dedup();

    ProjectSummary {
        root: project.root.clone(),
        branch: project.branch_label(),
        major_directories,
        languages,
        entry_points,
        build_files,
        files_seen: files_seen.min(MAX_FILES),
        truncated,
    }
}

fn major_directories(root: &Path, ignore_rules: &RepositoryIgnoreRules) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut directories = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if ignore_rules.is_ignored(&entry.path()) || !entry.file_type().ok()?.is_dir() {
                None
            } else {
                Some(format!("{name}/"))
            }
        })
        .collect::<Vec<_>>();
    directories.sort();
    directories
}

fn render_lines(rendered: &mut String, lines: &[String]) {
    if lines.is_empty() {
        rendered.push_str("- none\n");
    } else {
        for line in lines {
            rendered.push_str(&format!("- {line}\n"));
        }
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_noisy_project_path(name: &str) -> bool {
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

fn language_for_path(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_string_lossy().as_ref() {
        "rs" => Some("Rust"),
        "toml" => Some("TOML"),
        "md" => Some("Markdown"),
        "js" | "jsx" => Some("JavaScript"),
        "ts" | "tsx" => Some("TypeScript"),
        "py" => Some("Python"),
        "ps1" => Some("PowerShell"),
        "sh" | "bash" | "zsh" => Some("Shell"),
        "json" => Some("JSON"),
        "yaml" | "yml" => Some("YAML"),
        _ => None,
    }
}

fn is_likely_entry_point(relative: &str) -> bool {
    matches!(
        relative,
        "src/main.rs"
            | "src/lib.rs"
            | "main.py"
            | "app.py"
            | "index.js"
            | "index.ts"
            | "src/index.js"
            | "src/index.ts"
    )
}

fn is_build_file(relative: &str) -> bool {
    matches!(
        relative,
        "Cargo.toml"
            | "package.json"
            | "pyproject.toml"
            | "Makefile"
            | "justfile"
            | "go.mod"
            | "pom.xml"
            | "build.gradle"
    )
}

fn resolve_start(start: &Path, root_override: Option<&Path>) -> PathBuf {
    match root_override {
        Some(root) if root.is_absolute() => root.to_path_buf(),
        Some(root) => start.join(root),
        None => start.to_path_buf(),
    }
}

fn find_git_root(start: &Path) -> Result<Option<(PathBuf, PathBuf)>, ProjectError> {
    let mut current = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        let git_marker = current.join(".git");
        if git_marker.exists() {
            let git_dir = resolve_git_dir(&git_marker)?;
            if git_dir.join("HEAD").exists() {
                return Ok(Some((current, git_dir)));
            }
        }

        if !current.pop() {
            return Ok(None);
        }
    }
}

fn resolve_git_dir(git_marker: &Path) -> Result<PathBuf, ProjectError> {
    if git_marker.is_dir() {
        return Ok(git_marker.to_path_buf());
    }

    let contents = fs::read_to_string(git_marker).map_err(|error| ProjectError::Read {
        path: git_marker.to_path_buf(),
        error: error.to_string(),
    })?;
    let git_dir = contents
        .trim()
        .strip_prefix("gitdir:")
        .map(str::trim)
        .ok_or_else(|| ProjectError::InvalidGitFile(git_marker.to_path_buf()))?;
    let git_dir = PathBuf::from(git_dir);
    if git_dir.is_absolute() {
        Ok(git_dir)
    } else {
        Ok(git_marker
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(git_dir))
    }
}

fn read_project_head(git_dir: &Path) -> Result<ProjectHead, ProjectError> {
    let head_path = git_dir.join("HEAD");
    let contents = fs::read_to_string(&head_path).map_err(|error| ProjectError::Read {
        path: head_path,
        error: error.to_string(),
    })?;
    let head = contents.trim();
    if head.is_empty() {
        return Ok(ProjectHead::Unknown);
    }

    if let Some(reference) = head.strip_prefix("ref:") {
        let branch = reference
            .trim()
            .strip_prefix("refs/heads/")
            .unwrap_or_else(|| reference.trim());
        if branch.is_empty() {
            Ok(ProjectHead::Unknown)
        } else {
            Ok(ProjectHead::Branch(branch.to_string()))
        }
    } else {
        Ok(ProjectHead::Detached(short_commit(head)))
    }
}

fn short_commit(value: &str) -> String {
    value.chars().take(12).collect()
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProjectError {
    #[error("failed to read project metadata at {path}: {error}")]
    Read { path: PathBuf, error: String },
    #[error("failed to parse project config at {path}: {error}")]
    ConfigParse { path: PathBuf, error: String },
    #[error("invalid git file at {0}")]
    InvalidGitFile(PathBuf),
    #[error("project not detected")]
    NotDetected,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn detects_nested_git_repository_root_and_branch() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let outer = tempdir.path().join("outer");
        let nested = outer.join("nested");
        let source = nested.join("src");
        write_head(&outer, "main");
        write_head(&nested, "feature/branch");
        fs::create_dir_all(&source).expect("source dir");

        let project = detect_project(&source, None)
            .expect("project detection")
            .expect("project");

        assert_eq!(project.root, nested);
        assert_eq!(
            project.head,
            ProjectHead::Branch("feature/branch".to_string())
        );
    }

    #[test]
    fn override_root_selects_requested_repository() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let outer = tempdir.path().join("outer");
        let nested = outer.join("nested");
        let source = nested.join("src");
        write_head(&outer, "main");
        write_head(&nested, "feature");
        fs::create_dir_all(&source).expect("source dir");

        let project = detect_project(&source, Some(&outer))
            .expect("project detection")
            .expect("project");

        assert_eq!(project.root, outer);
        assert_eq!(project.head, ProjectHead::Branch("main".to_string()));
    }

    #[test]
    fn detached_head_is_rendered_gracefully() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let repo = tempdir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).expect("git dir");
        fs::write(
            repo.join(".git").join("HEAD"),
            "d34db33fd34db33fd34db33fd34db33fd34db33f\n",
        )
        .expect("head");

        let project = detect_project(&repo, None)
            .expect("project detection")
            .expect("project");

        assert_eq!(
            project.head,
            ProjectHead::Detached("d34db33fd34d".to_string())
        );
        assert!(render_project_status(Some(&project)).contains("detached at d34db33fd34d"));
    }

    #[test]
    fn returns_none_without_git_repository() {
        let tempdir = tempfile::tempdir().expect("tempdir");

        let project = detect_project(tempdir.path(), None).expect("project detection");

        assert_eq!(project, None);
    }

    #[test]
    fn summarizes_project_without_full_indexing() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let repo = tempdir.path().join("repo");
        write_head(&repo, "main");
        fs::create_dir_all(repo.join("src")).expect("src dir");
        fs::create_dir_all(repo.join("target")).expect("target dir");
        fs::write(repo.join("Cargo.toml"), "[package]\nname = \"demo\"\n").expect("cargo");
        fs::write(repo.join("src").join("main.rs"), "fn main() {}\n").expect("main");
        fs::write(repo.join("README.md"), "# demo\n").expect("readme");
        fs::write(repo.join("target").join("ignored.rs"), "ignored").expect("ignored");

        let summary = summarize_project(&repo, None).expect("summary");
        let rendered = render_project_summary(&summary);

        assert_eq!(summary.branch, "main");
        assert!(summary.major_directories.contains(&"src/".to_string()));
        assert!(!summary.major_directories.contains(&"target/".to_string()));
        assert!(summary.entry_points.contains(&"src/main.rs".to_string()));
        assert!(summary.build_files.contains(&"Cargo.toml".to_string()));
        assert!(rendered.contains("languages:"));
        assert!(rendered.contains("Rust"));
    }

    #[test]
    fn project_config_loads_shared_and_local_with_local_precedence() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let repo = tempdir.path().join("repo");
        write_head(&repo, "main");
        fs::write(
            repo.join(".exoshell.toml"),
            "[project]\nhonor_gitignore = true\nignore = [\"generated\"]\n",
        )
        .expect("shared config");
        fs::write(
            repo.join(".exoshell.local.toml"),
            "[project]\nhonor_gitignore = false\nignore = [\"local-only\"]\napi_token = \"bad\"\n",
        )
        .expect("local config");

        let report = load_project_config(&repo).expect("project config");
        let rendered = render_project_config(&report);

        assert!(report.shared_loaded);
        assert!(report.local_loaded);
        assert!(!report.config.honor_gitignore);
        assert_eq!(report.config.ignore, vec!["local-only".to_string()]);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("api_token"))
        );
        assert!(rendered.contains("honor_gitignore: false"));
        assert!(rendered.contains("- local-only"));
        assert!(rendered.contains("possible secret key"));
    }

    #[test]
    fn project_summary_honors_gitignore_and_exoshell_ignore_rules() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let repo = tempdir.path().join("repo");
        write_head(&repo, "main");
        fs::create_dir_all(repo.join("src")).expect("src dir");
        fs::create_dir_all(repo.join("generated")).expect("generated dir");
        fs::write(repo.join(".gitignore"), "ignored.py\n").expect("gitignore");
        fs::write(
            repo.join(".exoshell.toml"),
            "[project]\nignore = [\"generated/\"]\n",
        )
        .expect("project config");
        fs::write(repo.join("src").join("main.rs"), "fn main() {}\n").expect("main");
        fs::write(repo.join("ignored.py"), "print('ignored')\n").expect("ignored");
        fs::write(repo.join("generated").join("ignored.ts"), "ignored\n").expect("generated");

        let summary = summarize_project(&repo, None).expect("summary");

        assert!(summary.entry_points.contains(&"src/main.rs".to_string()));
        assert!(
            summary
                .languages
                .iter()
                .any(|language| language.language == "Rust")
        );
        assert!(
            !summary
                .languages
                .iter()
                .any(|language| language.language == "Python")
        );
        assert!(
            !summary
                .languages
                .iter()
                .any(|language| language.language == "TypeScript")
        );
        assert!(
            !summary
                .major_directories
                .contains(&"generated/".to_string())
        );
    }

    #[test]
    fn repository_ignore_rules_support_negation_and_wildcards() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let rules = RepositoryIgnoreRules::from_parts(
            tempdir.path(),
            false,
            vec!["*.log".into(), "!keep.log".into(), "cache/".into()],
        )
        .expect("rules");

        assert!(rules.is_ignored(&tempdir.path().join("debug.log")));
        assert!(!rules.is_ignored(&tempdir.path().join("keep.log")));
        assert!(rules.is_ignored(&tempdir.path().join("cache").join("data.txt")));
    }

    fn write_head(root: &Path, branch: &str) {
        let git_dir = root.join(".git");
        fs::create_dir_all(&git_dir).expect("git dir");
        fs::write(git_dir.join("HEAD"), format!("ref: refs/heads/{branch}\n")).expect("head");
    }
}
