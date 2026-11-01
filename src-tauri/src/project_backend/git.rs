//! Git operations used by the Projects detail page.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::process;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileStatus {
    pub path: String,
    pub index_status: String,
    pub work_tree_status: String,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
    pub conflicted: bool,
}

impl GitFileStatus {
    pub fn is_deleted(&self) -> bool {
        self.index_status == "D" || self.work_tree_status == "D"
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitRepositoryStatus {
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub remote_url: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub files: Vec<GitFileStatus>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRemote {
    pub connected: bool,
    pub url: Option<String>,
    pub name: Option<String>,
    pub default_branch: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitCommit {
    pub hash: String,
    pub message: String,
    pub date: String,
}

pub fn parse_git_status_entries(raw: &str) -> Vec<GitFileStatus> {
    let entries = raw
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();
    let mut files = Vec::new();
    let mut index = 0;
    while index < entries.len() {
        let entry = entries[index];
        let bytes = entry.as_bytes();
        if bytes.len() < 3 {
            index += 1;
            continue;
        }
        let index_status = bytes[0] as char;
        let work_tree_status = bytes[1] as char;
        let mut path = entry[3..].to_owned();
        let pair = format!("{index_status}{work_tree_status}");
        let conflicted = matches!(
            pair.as_str(),
            "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU"
        );
        let untracked = pair == "??";
        if matches!(index_status, 'R' | 'C') && index + 1 < entries.len() {
            path = entries[index + 1].to_owned();
        }
        files.push(GitFileStatus {
            path,
            index_status: index_status.to_string(),
            work_tree_status: work_tree_status.to_string(),
            staged: !untracked && index_status != ' ' && index_status != '?',
            unstaged: !untracked && work_tree_status != ' ' && work_tree_status != '?',
            untracked,
            conflicted,
        });
        index += 1;
        if matches!(index_status, 'R' | 'C') && index < entries.len() {
            index += 1;
        }
    }
    files.sort_by_key(|file| file.path.to_lowercase());
    files
}

pub fn safe_repo_relative_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("Invalid repository file path.".to_owned());
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("Could not verify project path: {error}"))?;
    let target = canonical_root.join(relative_path);
    let mut existing = target.parent().unwrap_or(&canonical_root);
    while !existing.exists() {
        existing = existing
            .parent()
            .ok_or_else(|| "Could not verify file path.".to_owned())?;
    }
    let canonical_existing = existing
        .canonicalize()
        .map_err(|error| format!("Could not verify file path: {error}"))?;
    if !canonical_existing.starts_with(&canonical_root) {
        return Err("The file path points outside the project.".to_owned());
    }
    if target.exists()
        && !target
            .canonicalize()
            .map_err(|error| format!("Could not verify file path: {error}"))?
            .starts_with(&canonical_root)
    {
        return Err("The file resolves to a path outside the project.".to_owned());
    }
    Ok(target)
}

fn output_text(output: process::CommandOutput) -> Result<String, String> {
    if output.code == 0 {
        let stdout = output.stdout.trim();
        if stdout.is_empty() {
            Ok(output.stderr.trim().to_owned())
        } else {
            Ok(stdout.to_owned())
        }
    } else {
        let text = output.combined();
        Err(if text.is_empty() {
            format!("Git exited with status {}.", output.code)
        } else {
            text
        })
    }
}

pub fn run_git(root: &Path, args: &[String]) -> Result<String, String> {
    output_text(process::run_capture("git", args, Some(root))?)
}

pub fn command_output(root: &Path, args: &[&str]) -> Option<String> {
    let args = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    process::run_capture("git", &args, Some(root))
        .ok()
        .filter(|output| output.code == 0)
        .map(|output| output.stdout.trim().to_owned())
        .filter(|text| !text.is_empty())
}

pub fn git_project_root(project_path: &str) -> Result<PathBuf, String> {
    let root = PathBuf::from(project_path.trim());
    if !root.is_dir() {
        return Err(format!("Project folder not found: {project_path}"));
    }
    if command_output(&root, &["rev-parse", "--is-inside-work-tree"]).as_deref() != Some("true") {
        return Err("The project folder is not a Git repository.".to_owned());
    }
    Ok(root)
}

pub fn is_git_repository(project_path: &Path) -> bool {
    project_path.is_dir()
        && command_output(project_path, &["rev-parse", "--is-inside-work-tree"]).as_deref()
            == Some("true")
}

pub fn git_status(project_path: &Path) -> Result<GitRepositoryStatus, String> {
    let root = git_project_root(&project_path.to_string_lossy())?;
    let branch = command_output(&root, &["branch", "--show-current"])
        .or_else(|| command_output(&root, &["rev-parse", "--short", "HEAD"]));
    let upstream = command_output(
        &root,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    );
    let (ahead, behind) = upstream
        .as_ref()
        .and_then(|_| {
            command_output(
                &root,
                &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
            )
        })
        .and_then(|value| {
            let mut parts = value.split_whitespace();
            Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
        })
        .unwrap_or_default();
    let raw = run_git(
        &root,
        &[
            "status".to_owned(),
            "--short".to_owned(),
            "-z".to_owned(),
            "--untracked-files=all".to_owned(),
        ],
    )?;
    Ok(GitRepositoryStatus {
        branch,
        upstream,
        remote_url: remote_url(&root),
        ahead,
        behind,
        files: parse_git_status_entries(&raw),
    })
}

pub fn branches(project_path: &Path) -> Result<Vec<(String, Option<String>, bool)>, String> {
    let root = git_project_root(&project_path.to_string_lossy())?;
    let output = run_git(
        &root,
        &[
            "for-each-ref".to_owned(),
            "--format=%(refname:short)%09%(upstream:short)%09%(HEAD)".to_owned(),
            "refs/heads".to_owned(),
        ],
    )?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let name = parts.next()?.trim();
            if name.is_empty() {
                return None;
            }
            let upstream = parts
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_owned);
            let current = parts.next().map(str::trim) == Some("*");
            Some((name.to_owned(), upstream, current))
        })
        .collect())
}

fn binary_untracked_diff(path: &str) -> String {
    format!("diff --git a/{path} b/{path}\nnew file mode 100644\nBinary files /dev/null and b/{path} differ")
}

pub fn untracked_file_diff(path: &str, bytes: &[u8]) -> String {
    if bytes.contains(&0) {
        return binary_untracked_diff(path);
    }
    let Ok(content) = String::from_utf8(bytes.to_vec()) else {
        return binary_untracked_diff(path);
    };
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.is_empty() {
        return format!(
            "diff --git a/{path} b/{path}\nnew file mode 100644\n--- /dev/null\n+++ b/{path}"
        );
    }
    let lines = normalized.lines().collect::<Vec<_>>();
    let mut diff = format!(
        "diff --git a/{path} b/{path}\nnew file mode 100644\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n",
        lines.len()
    );
    for line in lines {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    if !normalized.ends_with('\n') {
        diff.push_str("\\ No newline at end of file\n");
    }
    diff.trim_end_matches('\n').to_owned()
}

pub fn diff(
    project_path: &Path,
    path: &str,
    staged: bool,
    unstaged: bool,
    untracked: bool,
) -> Result<String, String> {
    let root = git_project_root(&project_path.to_string_lossy())?;
    let target = safe_repo_relative_path(&root, path)?;
    if untracked {
        return fs::read(&target)
            .map(|bytes| untracked_file_diff(path, &bytes))
            .map_err(|error| format!("Untracked file could not be read: {error}"));
    }
    let mut sections = Vec::new();
    for cached in [
        Some(true).filter(|_| staged),
        Some(false).filter(|_| unstaged),
    ]
    .into_iter()
    .flatten()
    {
        let mut args = vec![
            "diff".to_owned(),
            "--no-ext-diff".to_owned(),
            "--no-color".to_owned(),
        ];
        if cached {
            args.push("--cached".to_owned());
        }
        args.extend(["--".to_owned(), path.to_owned()]);
        let value = run_git(&root, &args)?;
        if !value.is_empty() {
            sections.push(value);
        }
    }
    Ok(sections.join("\n\n"))
}

fn validate_paths(root: &Path, paths: &[String]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("Select at least one file.".to_owned());
    }
    for path in paths {
        safe_repo_relative_path(root, path)?;
    }
    Ok(())
}

pub fn stage(project_path: &Path, paths: &[String]) -> Result<(), String> {
    let root = git_project_root(&project_path.to_string_lossy())?;
    validate_paths(&root, paths)?;
    let mut args = vec!["add".to_owned(), "--".to_owned()];
    args.extend(paths.iter().cloned());
    run_git(&root, &args).map(|_| ())
}

pub fn unstage(project_path: &Path, paths: &[String]) -> Result<(), String> {
    let root = git_project_root(&project_path.to_string_lossy())?;
    validate_paths(&root, paths)?;
    let mut args = vec!["restore".to_owned(), "--staged".to_owned(), "--".to_owned()];
    args.extend(paths.iter().cloned());
    if run_git(&root, &args).is_ok() {
        return Ok(());
    }
    let mut fallback = vec!["rm".to_owned(), "--cached".to_owned(), "--".to_owned()];
    fallback.extend(paths.iter().cloned());
    run_git(&root, &fallback).map(|_| ())
}

pub fn commit(project_path: &Path, message: &str) -> Result<GitCommit, String> {
    let root = git_project_root(&project_path.to_string_lossy())?;
    let message = message.trim();
    if message.is_empty() {
        return Err("Commit message must not be empty.".to_owned());
    }
    run_git(
        &root,
        &["commit".to_owned(), "-m".to_owned(), message.to_owned()],
    )?;
    Ok(GitCommit {
        hash: command_output(&root, &["rev-parse", "--short", "HEAD"]).unwrap_or_default(),
        message: message.to_owned(),
        date: command_output(&root, &["show", "-s", "--format=%cI", "HEAD"]).unwrap_or_default(),
    })
}

pub fn checkout(project_path: &Path, branch: &str) -> Result<(), String> {
    let root = git_project_root(&project_path.to_string_lossy())?;
    let branch = branch.trim();
    if branch.is_empty() || branch.starts_with('-') || branch.chars().any(char::is_whitespace) {
        return Err("Invalid branch name.".to_owned());
    }
    run_git(&root, &["switch".to_owned(), branch.to_owned()]).map(|_| ())
}

pub fn init(project_path: &Path) -> Result<(), String> {
    if !project_path.is_dir() {
        return Err(format!(
            "Project folder not found: {}",
            project_path.display()
        ));
    }
    run_git(project_path, &["init".to_owned()]).map(|_| ())
}

pub fn remote_url(root: &Path) -> Option<String> {
    command_output(root, &["remote", "get-url", "origin"]).or_else(|| {
        let name = command_output(root, &["remote"])?
            .lines()
            .next()?
            .trim()
            .to_owned();
        command_output(root, &["remote", "get-url", &name])
    })
}

pub fn remote_state(project_path: &Path) -> GitRemote {
    if !is_git_repository(project_path) {
        return GitRemote::default();
    }
    let url = remote_url(project_path);
    let default_branch = command_output(
        project_path,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .and_then(|value| value.strip_prefix("origin/").map(str::to_owned))
    .or_else(|| command_output(project_path, &["branch", "--show-current"]));
    GitRemote {
        connected: url.is_some(),
        name: url.as_deref().and_then(remote_name),
        url,
        default_branch,
    }
}

fn remote_name(url: &str) -> Option<String> {
    let value = url.trim_end_matches('/').trim_end_matches(".git");
    let after = value.rsplit_once(['/', ':'])?.1;
    let owner = value.strip_suffix(after).and_then(|prefix| {
        prefix
            .trim_end_matches(['/', ':'])
            .rsplit_once(['/', ':'])
            .map(|(_, owner)| owner)
    })?;
    if owner.is_empty() || after.is_empty() {
        None
    } else {
        Some(format!("{owner}/{after}"))
    }
}

pub fn last_commit(project_path: &Path) -> Option<GitCommit> {
    if !is_git_repository(project_path) {
        return None;
    }
    let value = command_output(
        project_path,
        &[
            "log",
            "-1",
            "--date=iso-strict",
            "--pretty=format:%h%x1f%s%x1f%cI",
        ],
    )?;
    let mut parts = value.split('\u{1f}');
    Some(GitCommit {
        hash: parts.next()?.to_owned(),
        message: parts.next()?.to_owned(),
        date: parts.next()?.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_untracked_text_diff() {
        let diff = untracked_file_diff("notes.txt", b"one\ntwo");
        assert!(diff.contains("+++ b/notes.txt"));
        assert!(diff.contains("+one"));
        assert!(diff.contains("No newline"));
    }

    #[test]
    fn recognizes_github_remote_names() {
        assert_eq!(
            remote_name("git@github.com:owner/repo.git").as_deref(),
            Some("owner/repo")
        );
        assert_eq!(
            remote_name("https://github.com/owner/repo").as_deref(),
            Some("owner/repo")
        );
    }
}
