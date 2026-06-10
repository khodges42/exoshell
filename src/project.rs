use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

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
    Ok(summarize_project_root(&project))
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

fn summarize_project_root(project: &ProjectInfo) -> ProjectSummary {
    const MAX_FILES: usize = 1_000;
    let major_directories = major_directories(&project.root);
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
            let name = entry.file_name().to_string_lossy().to_string();
            if is_noisy_project_path(&name) {
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

fn major_directories(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut directories = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if is_noisy_project_path(&name) || !entry.file_type().ok()?.is_dir() {
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

    fn write_head(root: &Path, branch: &str) {
        let git_dir = root.join(".git");
        fs::create_dir_all(&git_dir).expect("git dir");
        fs::write(git_dir.join("HEAD"), format!("ref: refs/heads/{branch}\n")).expect("head");
    }
}
