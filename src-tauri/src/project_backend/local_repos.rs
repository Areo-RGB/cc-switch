//! Bounded scan of local folders whose GitHub remotes match `owner/repo`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub type LocalRepos = BTreeMap<String, Vec<String>>;

const MAX_DEPTH: usize = 4;
const SKIP: [&str; 6] = ["node_modules", "target", "build", "dist", ".venv", "vendor"];

fn git_config(directory: &Path) -> Option<PathBuf> {
    let dot_git = directory.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git.join("config"));
    }
    let text = std::fs::read_to_string(&dot_git).ok()?;
    let relative = text.strip_prefix("gitdir:")?.trim();
    let gitdir = directory.join(relative);
    let common = std::fs::read_to_string(gitdir.join("commondir"))
        .map(|value| gitdir.join(value.trim()))
        .unwrap_or(gitdir);
    Some(common.join("config"))
}

pub fn github_slug(url: &str) -> Option<String> {
    super::github::github_slug(url)
}

fn remote_slugs(config: &str) -> Vec<String> {
    config
        .lines()
        .filter_map(|line| line.trim().strip_prefix("url"))
        .filter_map(|value| value.trim_start().strip_prefix('='))
        .filter_map(|value| github_slug(value.trim()))
        .collect()
}

pub fn scan(root: &Path) -> LocalRepos {
    let mut output = LocalRepos::new();
    walk(root, 0, &mut output);
    for paths in output.values_mut() {
        paths.sort();
        paths.dedup();
    }
    output
}

fn walk(directory: &Path, depth: usize, output: &mut LocalRepos) {
    if let Some(config) = git_config(directory) {
        if let Ok(text) = std::fs::read_to_string(config) {
            for slug in remote_slugs(&text) {
                output
                    .entry(slug)
                    .or_default()
                    .push(directory.display().to_string());
            }
        }
    }
    if depth >= MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || SKIP.contains(&name.as_str()) {
            continue;
        }
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            walk(&entry.path(), depth + 1, output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_remote_urls() {
        let config = "[remote \"origin\"]\n\turl = git@github.com:a/b.git\n[remote \"upstream\"]\n\turl=https://github.com/c/d\n";
        assert_eq!(remote_slugs(config), ["a/b", "c/d"]);
    }
}
