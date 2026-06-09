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

    fn write_head(root: &Path, branch: &str) {
        let git_dir = root.join(".git");
        fs::create_dir_all(&git_dir).expect("git dir");
        fs::write(git_dir.join("HEAD"), format!("ref: refs/heads/{branch}\n")).expect("head");
    }
}
