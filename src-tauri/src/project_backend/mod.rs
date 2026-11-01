//! Backend primitives for the Projects + GitHub page.

pub mod git;
pub mod github;
pub mod launchers;
pub mod local_repos;
pub mod process;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use git::GitRemote;
use launchers::DetectedScript;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectScript {
    pub id: String,
    pub name: String,
    pub command: String,
    pub running: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentClientSummary {
    pub client: String,
    pub mcp_servers: usize,
    pub skills: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentsSummary {
    pub mcp_servers: usize,
    pub skills: usize,
    pub clients: Vec<AgentClientSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub path: String,
    pub name: String,
    pub kind: String,
    pub frameworks: Vec<String>,
    pub branch: String,
    pub last_commit: String,
    pub last_commit_at: String,
    pub changed_files: usize,
    pub hidden: bool,
    pub remote: GitRemote,
    pub scripts: Vec<ProjectScript>,
    pub agents: AgentsSummary,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetails {
    pub project: Project,
    pub readme: Option<String>,
    pub package_manager: Option<String>,
}

pub fn project_path(project_id: &str) -> Result<std::path::PathBuf, String> {
    let path = std::path::PathBuf::from(project_id.trim());
    if !path.is_dir() {
        return Err(format!("Project folder not found: {project_id}"));
    }
    Ok(path)
}

pub fn project_scripts(path: &Path, running: impl Fn(&str) -> bool) -> Vec<ProjectScript> {
    launchers::detect_scripts(path)
        .into_iter()
        .map(|script| ProjectScript {
            running: running(&script.id),
            id: script.id,
            name: script.name,
            command: script.command,
        })
        .collect()
}

fn git_metadata(path: &Path) -> (String, usize, String, String) {
    if !git::is_git_repository(path) {
        return (String::new(), 0, String::new(), String::new());
    }
    let branch = git::command_output(path, &["branch", "--show-current"]).unwrap_or_default();
    let changed_files = git::command_output(path, &["status", "--porcelain"])
        .map(|value| value.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or_default();
    let commit = git::last_commit(path);
    (
        branch,
        changed_files,
        commit
            .as_ref()
            .map(|value| value.message.clone())
            .unwrap_or_default(),
        commit.map(|value| value.date).unwrap_or_default(),
    )
}

pub fn build_project(path: &Path, running: impl Fn(&str) -> bool) -> Result<Project, String> {
    if !path.is_dir() {
        return Err(format!("Project folder not found: {}", path.display()));
    }
    let inspection = launchers::inspect_project(path);
    let (branch, changed_files, last_commit, last_commit_at) = git_metadata(path);
    Ok(Project {
        id: path.display().to_string(),
        path: path.display().to_string(),
        name: launchers::project_name(path),
        kind: if git::is_git_repository(path) {
            "git"
        } else {
            "folder"
        }
        .to_owned(),
        frameworks: inspection.frameworks,
        branch,
        last_commit,
        last_commit_at,
        changed_files,
        hidden: false,
        remote: git::remote_state(path),
        scripts: project_scripts(path, running),
        agents: agents_summary(path),
    })
}

fn read_json_object(path: &Path, key: &str) -> BTreeSet<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return BTreeSet::new();
    };
    value
        .get(key)
        .and_then(Value::as_object)
        .map(|object| object.keys().cloned().collect())
        .unwrap_or_default()
}

fn toml_mcp_servers(path: &Path) -> BTreeSet<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    let Ok(value) = text.parse::<toml::Value>() else {
        return BTreeSet::new();
    };
    value
        .get("mcp_servers")
        .and_then(toml::Value::as_table)
        .map(|table| table.keys().cloned().collect())
        .unwrap_or_default()
}

fn opencode_mcp_servers(path: &Path) -> BTreeSet<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return BTreeSet::new();
    };
    let Some(object) = value.get("mcp").and_then(Value::as_object) else {
        return BTreeSet::new();
    };
    let object = object
        .get("servers")
        .and_then(Value::as_object)
        .unwrap_or(object);
    object.keys().cloned().collect()
}

fn skill_dirs(path: &Path) -> BTreeSet<String> {
    let mut output = BTreeSet::new();
    for relative in [
        ".agents/skills",
        ".claude/skills",
        ".codex/skills",
        ".pi/skills",
        ".opencode/skills",
    ] {
        let Ok(entries) = fs::read_dir(path.join(relative)) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.path().join("SKILL.md").is_file() {
                output.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    output
}

fn skill_count(path: &Path) -> usize {
    fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().join("SKILL.md").is_file())
                .count()
        })
        .unwrap_or_default()
}

pub fn agents_summary(path: &Path) -> AgentsSummary {
    let client_mcp = [
        (
            "Claude",
            read_json_object(&path.join(".mcp.json"), "mcpServers"),
        ),
        ("Codex", toml_mcp_servers(&path.join(".codex/config.toml"))),
        (
            "OpenCode",
            opencode_mcp_servers(&path.join("opencode.json")),
        ),
        (
            "Pi",
            read_json_object(&path.join(".pi/mcp.json"), "mcpServers"),
        ),
        (
            "Gemini",
            read_json_object(&path.join(".gemini/settings.json"), "mcpServers"),
        ),
    ];
    let skills = skill_dirs(path);
    let mut all_mcp = BTreeSet::new();
    for (_, servers) in &client_mcp {
        all_mcp.extend(servers.iter().cloned());
    }
    let clients = client_mcp
        .into_iter()
        .map(|(client, servers)| AgentClientSummary {
            client: client.to_owned(),
            mcp_servers: servers.len(),
            skills: if client == "Claude" {
                skill_count(&path.join(".claude/skills"))
            } else if client == "Codex" {
                skill_count(&path.join(".codex/skills"))
            } else if client == "OpenCode" {
                skill_count(&path.join(".opencode/skills"))
            } else if client == "Pi" {
                skill_count(&path.join(".pi/skills"))
            } else {
                0
            },
        })
        .collect();
    AgentsSummary {
        mcp_servers: all_mcp.len(),
        skills: skills.len(),
        clients,
    }
}

pub fn project_details(
    path: &Path,
    running: impl Fn(&str) -> bool,
) -> Result<ProjectDetails, String> {
    let project = build_project(path, running)?;
    let readme = ["README.md", "README", "readme.md"]
        .into_iter()
        .map(|name| path.join(name))
        .find_map(|candidate| fs::read_to_string(candidate).ok());
    let package_manager = launchers::package_manager(path);
    Ok(ProjectDetails {
        project,
        readme,
        package_manager,
    })
}

pub fn script_definition(path: &Path, id: &str) -> Result<DetectedScript, String> {
    launchers::find_script(path, id).ok_or_else(|| format!("Script not found: {id}"))
}
