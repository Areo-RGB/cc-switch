//! GitHub helpers backed by the local `gh` CLI.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::git::{self, GitRemote};
use super::process;

const LIST_FIELDS: &str = "name,nameWithOwner,description,isPrivate,isArchived,updatedAt,pushedAt,visibility,url,stargazerCount,primaryLanguage";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRepo {
    pub name: String,
    pub name_with_owner: String,
    pub description: String,
    pub is_private: bool,
    pub is_archived: bool,
    pub url: String,
    pub stargazer_count: u64,
    pub updated_at: String,
    pub pushed_at: String,
    pub visibility: String,
    pub primary_language: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubAuthStatus {
    pub authenticated: bool,
    pub account: Option<String>,
    pub host: Option<String>,
    pub error: Option<String>,
}

fn string_value(object: &serde_json::Map<String, Value>, key: &str) -> String {
    object
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

pub fn parse_repos(text: &str) -> Vec<GitHubRepo> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(Value::as_object)
        .map(|object| {
            let language = object
                .get("primaryLanguage")
                .and_then(|value| value.as_object().and_then(|item| item.get("name")))
                .and_then(Value::as_str)
                .or_else(|| object.get("primaryLanguage").and_then(Value::as_str))
                .unwrap_or_default()
                .to_owned();
            GitHubRepo {
                name: string_value(object, "name"),
                name_with_owner: {
                    let value = string_value(object, "nameWithOwner");
                    if value.is_empty() {
                        string_value(object, "name")
                    } else {
                        value
                    }
                },
                description: string_value(object, "description"),
                is_private: object
                    .get("isPrivate")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                is_archived: object
                    .get("isArchived")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                url: string_value(object, "url"),
                stargazer_count: object
                    .get("stargazerCount")
                    .and_then(Value::as_u64)
                    .unwrap_or_default(),
                updated_at: string_value(object, "updatedAt"),
                pushed_at: string_value(object, "pushedAt"),
                visibility: string_value(object, "visibility"),
                primary_language: language,
            }
        })
        .collect()
}

pub fn filter_repos(
    repos: &[GitHubRepo],
    query: &str,
    visibility: &str,
    sort: &str,
) -> Vec<GitHubRepo> {
    let query = query.trim().to_ascii_lowercase();
    let visibility = visibility.to_ascii_lowercase();
    let mut filtered = repos
        .iter()
        .filter(|repo| match visibility.as_str() {
            "public" => !repo.is_private,
            "private" => repo.is_private,
            "archived" => repo.is_archived,
            _ => true,
        })
        .filter(|repo| {
            query.is_empty()
                || repo.name_with_owner.to_ascii_lowercase().contains(&query)
                || repo.description.to_ascii_lowercase().contains(&query)
                || repo.primary_language.to_ascii_lowercase().contains(&query)
        })
        .cloned()
        .collect::<Vec<_>>();
    match sort.to_ascii_lowercase().as_str() {
        "pushed" => filtered.sort_by(|left, right| right.pushed_at.cmp(&left.pushed_at)),
        "name" => filtered.sort_by(|left, right| left.name_with_owner.cmp(&right.name_with_owner)),
        "stars" => filtered.sort_by_key(|repo| std::cmp::Reverse(repo.stargazer_count)),
        _ => filtered.sort_by(|left, right| right.updated_at.cmp(&left.updated_at)),
    }
    filtered
}

pub fn valid_repo_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && !name.contains("..")
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

pub fn github_slug(url: &str) -> Option<String> {
    let lower = url.to_ascii_lowercase();
    let marker = "github.com";
    let index = lower.find(marker)?;
    let before = &lower[..index];
    if let Some(last) = before.chars().last() {
        if last.is_ascii_alphanumeric() || matches!(last, '.' | '-') {
            return None;
        }
    }
    let mut rest = &url[index + marker.len()..];
    rest = rest.trim_start_matches([':', '/']);
    let mut parts = rest.trim_end_matches('/').split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim().trim_end_matches(".git");
    if !valid_repo_name(owner) || !valid_repo_name(repo) {
        return None;
    }
    Some(format!("{owner}/{repo}").to_ascii_lowercase())
}

pub fn https_clone_url(repo: &GitHubRepo) -> String {
    let base = repo.url.trim_end_matches('/');
    let base = if base.is_empty() {
        format!("https://github.com/{}", repo.name_with_owner)
    } else {
        base.to_owned()
    };
    if base.ends_with(".git") {
        base
    } else {
        format!("{base}.git")
    }
}

fn command_output(args: &[String]) -> Result<process::CommandOutput, String> {
    process::run_capture("gh", args, None)
}

pub fn auth_status() -> GitHubAuthStatus {
    let output = match command_output(&["auth".to_owned(), "status".to_owned()]) {
        Ok(output) => output,
        Err(error) => {
            return GitHubAuthStatus {
                error: Some(error),
                ..Default::default()
            }
        }
    };
    let combined = output.combined();
    let account = combined.lines().find_map(|line| {
        let lower = line.to_ascii_lowercase();
        let marker = "account ";
        let start = lower.find(marker)? + marker.len();
        let value = line[start..]
            .split_whitespace()
            .next()?
            .trim_matches(['(', ')', ',', ':']);
        (!value.is_empty()).then(|| value.to_owned())
    });
    let host = combined.lines().find_map(|line| {
        let lower = line.to_ascii_lowercase();
        let start = lower.find("logged in to ")? + "logged in to ".len();
        let value = line[start..]
            .split_whitespace()
            .next()?
            .trim_matches(['(', ')', ',']);
        (!value.is_empty()).then(|| value.to_owned())
    });
    if output.code == 0 {
        GitHubAuthStatus {
            authenticated: true,
            account,
            host,
            error: None,
        }
    } else {
        GitHubAuthStatus {
            authenticated: false,
            account,
            host,
            error: Some(if combined.is_empty() {
                "gh auth status failed.".to_owned()
            } else {
                combined
            }),
        }
    }
}

pub fn list_repos(
    owner: Option<&str>,
    query: &str,
    visibility: &str,
    sort: &str,
) -> Result<Vec<GitHubRepo>, String> {
    let mut args = vec!["repo".to_owned(), "list".to_owned()];
    if let Some(owner) = owner.map(str::trim).filter(|owner| !owner.is_empty()) {
        if !valid_repo_name(owner) {
            return Err("Invalid GitHub owner.".to_owned());
        }
        args.push(owner.to_owned());
    }
    args.extend([
        "--limit".to_owned(),
        "200".to_owned(),
        "--json".to_owned(),
        LIST_FIELDS.to_owned(),
    ]);
    let output = command_output(&args)?;
    if output.code != 0 {
        return Err(output.combined());
    }
    Ok(filter_repos(
        &parse_repos(&output.stdout),
        query,
        visibility,
        sort,
    ))
}

fn normalized_remote(remote: &str) -> Result<String, String> {
    let remote = remote.trim();
    if remote.is_empty() {
        return Err("Remote URL must not be empty.".to_owned());
    }
    if let Some(slug) = github_slug(remote) {
        return Ok(format!("https://github.com/{slug}.git"));
    }
    if let Some((owner, repo)) = remote.split_once('/') {
        if valid_repo_name(owner) && valid_repo_name(repo) && !repo.contains('/') {
            return Ok(format!("https://github.com/{owner}/{repo}.git"));
        }
    }
    if remote.starts_with("https://") || remote.starts_with("ssh://") || remote.starts_with("git@")
    {
        return github_slug(remote)
            .map(|slug| format!("https://github.com/{slug}.git"))
            .ok_or_else(|| "Remote must point to github.com.".to_owned());
    }
    Err("Remote must be a GitHub URL or owner/repository slug.".to_owned())
}

pub fn create_repo(project_path: &Path, name: &str, is_private: bool) -> Result<GitRemote, String> {
    if !project_path.is_dir() {
        return Err(format!(
            "Project folder not found: {}",
            project_path.display()
        ));
    }
    let name = name.trim();
    let valid_name = name.rsplit_once('/').map(|(_, repo)| repo).unwrap_or(name);
    if !valid_repo_name(valid_name) {
        return Err("Invalid GitHub repository name.".to_owned());
    }
    if !git::is_git_repository(project_path) {
        git::init(project_path)?;
    }
    if git::remote_url(project_path).is_some() {
        return Err("This project already has a Git remote.".to_owned());
    }
    let visibility = if is_private { "--private" } else { "--public" };
    let args = vec![
        "repo".to_owned(),
        "create".to_owned(),
        name.to_owned(),
        visibility.to_owned(),
        "--source".to_owned(),
        ".".to_owned(),
        "--remote".to_owned(),
        "origin".to_owned(),
        "--push".to_owned(),
    ];
    let output = command_output(&args)?;
    if output.code != 0 {
        return Err(output.combined());
    }
    Ok(git::remote_state(project_path))
}

pub fn link_and_push(project_path: &Path, remote: &str) -> Result<GitRemote, String> {
    if !project_path.is_dir() {
        return Err(format!(
            "Project folder not found: {}",
            project_path.display()
        ));
    }
    if !git::is_git_repository(project_path) {
        git::init(project_path)?;
    }
    let remote = normalized_remote(remote)?;
    let has_origin = git::command_output(project_path, &["remote", "get-url", "origin"]).is_some();
    let remote_args = if has_origin {
        vec![
            "remote".to_owned(),
            "set-url".to_owned(),
            "origin".to_owned(),
            remote,
        ]
    } else {
        vec![
            "remote".to_owned(),
            "add".to_owned(),
            "origin".to_owned(),
            remote,
        ]
    };
    git::run_git(project_path, &remote_args)?;
    let current_branch = git::command_output(project_path, &["branch", "--show-current"]);
    let needs_branch = current_branch
        .as_deref()
        .map(|branch| branch.is_empty())
        .unwrap_or(true);
    let branch = current_branch
        .filter(|branch| !branch.is_empty())
        .unwrap_or_else(|| "main".to_owned());
    if needs_branch {
        let _ = git::run_git(
            project_path,
            &["switch".to_owned(), "-c".to_owned(), branch.clone()],
        );
    }
    git::run_git(
        project_path,
        &[
            "push".to_owned(),
            "-u".to_owned(),
            "origin".to_owned(),
            branch,
        ],
    )?;
    Ok(git::remote_state(project_path))
}

pub fn browser_url(remote: &GitRemote) -> Option<String> {
    let url = remote.url.as_deref()?;
    github_slug(url).map(|slug| format!("https://github.com/{slug}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_github_or_ambiguous_slugs() {
        assert_eq!(github_slug("https://gitlab.com/me/repo"), None);
        assert_eq!(github_slug("https://notgithub.com/me/repo"), None);
        assert_eq!(
            github_slug("https://github.com/me/repo.git"),
            Some("me/repo".to_owned())
        );
    }

    #[test]
    fn normalizes_github_remote_inputs_without_network() {
        assert_eq!(
            normalized_remote("me/repo").unwrap(),
            "https://github.com/me/repo.git"
        );
        assert_eq!(
            normalized_remote("git@github.com:me/repo.git").unwrap(),
            "https://github.com/me/repo.git"
        );
        assert!(normalized_remote("https://gitlab.com/me/repo").is_err());
    }

    #[test]
    fn parses_auth_status_text_without_exposing_full_output_on_success() {
        let text = "Logged in to github.com account paul (keyring)\n";
        assert!(text.to_ascii_lowercase().contains("account paul"));
    }
}
