#![allow(non_snake_case)]

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

use crate::project_backend::git;
use crate::project_backend::github;
use crate::project_backend::process::{self, ProcessHandle, StreamEvent};
use crate::project_backend::{self, AgentsSummary, ProjectDetails, ProjectScript};

#[derive(Clone)]
struct RunningScript {
    project_id: String,
    script_id: String,
    handle: ProcessHandle,
}

fn running_scripts() -> &'static Mutex<HashMap<String, RunningScript>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, RunningScript>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn script_is_running(project_id: &str, script_id: &str) -> bool {
    running_scripts()
        .lock()
        .map(|scripts| {
            scripts
                .values()
                .any(|script| script.project_id == project_id && script.script_id == script_id)
        })
        .unwrap_or(false)
}

pub fn shutdown_processes() {
    if let Ok(mut scripts) = running_scripts().lock() {
        let running = scripts
            .drain()
            .map(|(_, script)| script)
            .collect::<Vec<_>>();
        for script in running {
            script.handle.kill();
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScriptOutputEvent {
    process_id: String,
    project_id: String,
    script_id: String,
    stream: &'static str,
    line: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScriptExitEvent {
    process_id: String,
    project_id: String,
    script_id: String,
    exit_code: i32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProcess {
    pub id: String,
    pub status: &'static str,
}

fn stream_script_output(
    app: AppHandle,
    process_id: String,
    project_id: String,
    script_id: String,
    receiver: std::sync::mpsc::Receiver<StreamEvent>,
) {
    std::thread::spawn(move || {
        for event in receiver {
            match event {
                StreamEvent::Stdout(line) => {
                    let _ = app.emit(
                        "project-script-output",
                        ScriptOutputEvent {
                            process_id: process_id.clone(),
                            project_id: project_id.clone(),
                            script_id: script_id.clone(),
                            stream: "stdout",
                            line,
                        },
                    );
                }
                StreamEvent::Stderr(line) => {
                    let _ = app.emit(
                        "project-script-output",
                        ScriptOutputEvent {
                            process_id: process_id.clone(),
                            project_id: project_id.clone(),
                            script_id: script_id.clone(),
                            stream: "stderr",
                            line,
                        },
                    );
                }
                StreamEvent::Exit(exit_code) => {
                    if let Ok(mut scripts) = running_scripts().lock() {
                        scripts.remove(&process_id);
                    }
                    let _ = app.emit(
                        "project-script-exit",
                        ScriptExitEvent {
                            process_id: process_id.clone(),
                            project_id: project_id.clone(),
                            script_id: script_id.clone(),
                            exit_code,
                        },
                    );
                    break;
                }
            }
        }
    });
}

#[tauri::command]
pub async fn projects_get(projectId: String) -> Result<ProjectDetails, String> {
    let path = project_backend::project_path(&projectId)?;
    project_backend::project_details(&path, |script_id| script_is_running(&projectId, script_id))
}

#[tauri::command]
pub async fn projects_open_folder(app: AppHandle, projectId: String) -> Result<(), String> {
    let path = project_backend::project_path(&projectId)?;
    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<String>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn projects_open_editor(projectId: String) -> Result<(), String> {
    let path = project_backend::project_path(&projectId)?;
    project_backend::launchers::launch_editor(&path)
}

#[tauri::command]
pub async fn projects_open_terminal(projectId: String) -> Result<(), String> {
    let path = project_backend::project_path(&projectId)?;
    project_backend::launchers::launch_terminal(&path)
}

#[tauri::command]
pub async fn projects_scripts(projectId: String) -> Result<Vec<ProjectScript>, String> {
    let path = project_backend::project_path(&projectId)?;
    Ok(project_backend::project_scripts(&path, |script_id| {
        script_is_running(&projectId, script_id)
    }))
}

#[tauri::command]
pub async fn projects_run_script(
    app: AppHandle,
    projectId: String,
    scriptId: String,
) -> Result<ProjectProcess, String> {
    let path = project_backend::project_path(&projectId)?;
    let script = project_backend::script_definition(&path, &scriptId)?;
    if script_is_running(&projectId, &script.id) {
        return Err(format!("Script is already running: {}", script.name));
    }
    let stream = process::start_shell(&script.command, &path)?;
    let process_id = stream.handle.pid().to_string();
    if let Ok(mut scripts) = running_scripts().lock() {
        scripts.insert(
            process_id.clone(),
            RunningScript {
                project_id: projectId.clone(),
                script_id: script.id.clone(),
                handle: stream.handle,
            },
        );
    } else {
        return Err("Could not register project process.".to_owned());
    }
    stream_script_output(app, process_id.clone(), projectId, script.id, stream.events);
    Ok(ProjectProcess {
        id: process_id,
        status: "running",
    })
}

#[tauri::command]
pub async fn projects_stop_script(projectId: String, processId: String) -> Result<(), String> {
    if let Ok(mut scripts) = running_scripts().lock() {
        let actual_process_id = if scripts
            .get(&processId)
            .is_some_and(|script| script.project_id == projectId)
        {
            Some(processId)
        } else {
            scripts
                .iter()
                .find(|(_, script)| script.project_id == projectId && script.script_id == processId)
                .map(|(id, _)| id.clone())
        };
        if let Some(actual_process_id) = actual_process_id {
            if let Some(script) = scripts.remove(&actual_process_id) {
                script.handle.kill();
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn projects_agents_summary(projectId: String) -> Result<AgentsSummary, String> {
    let path = project_backend::project_path(&projectId)?;
    Ok(project_backend::agents_summary(&path))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusResponse {
    branch: String,
    ahead: usize,
    behind: usize,
    entries: Vec<GitStatusEntry>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusEntry {
    path: String,
    status: &'static str,
    staged: bool,
}

fn status_label(entry: &git::GitFileStatus) -> &'static str {
    if entry.conflicted || entry.is_deleted() {
        "deleted"
    } else if entry.index_status == "R" || entry.work_tree_status == "R" {
        "renamed"
    } else if entry.untracked || entry.index_status == "A" {
        "new"
    } else {
        "modified"
    }
}

#[tauri::command]
pub async fn git_status(projectId: String) -> Result<GitStatusResponse, String> {
    let path = project_backend::project_path(&projectId)?;
    let status = git::git_status(&path)?;
    Ok(GitStatusResponse {
        branch: status.branch.unwrap_or_default(),
        ahead: status.ahead,
        behind: status.behind,
        entries: status
            .files
            .iter()
            .map(|entry| GitStatusEntry {
                path: entry.path.clone(),
                status: status_label(entry),
                staged: entry.staged,
            })
            .collect(),
    })
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchResponse {
    current: String,
    branches: Vec<GitBranchEntry>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchEntry {
    name: String,
    remote: Option<String>,
    current: bool,
}

#[tauri::command]
pub async fn git_branches(projectId: String) -> Result<GitBranchResponse, String> {
    let path = project_backend::project_path(&projectId)?;
    let branches = git::branches(&path)?;
    let current = branches
        .iter()
        .find(|(_, _, current)| *current)
        .map(|(name, _, _)| name.clone())
        .unwrap_or_default();
    Ok(GitBranchResponse {
        current,
        branches: branches
            .into_iter()
            .map(|(name, remote, current)| GitBranchEntry {
                name,
                remote,
                current,
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn git_diff(
    projectId: String,
    path: String,
    staged: bool,
    unstaged: bool,
    untracked: bool,
) -> Result<String, String> {
    let project = project_backend::project_path(&projectId)?;
    git::diff(&project, &path, staged, unstaged, untracked)
}

#[tauri::command]
pub async fn git_stage(projectId: String, paths: Vec<String>) -> Result<(), String> {
    let project = project_backend::project_path(&projectId)?;
    git::stage(&project, &paths)
}

#[tauri::command]
pub async fn git_unstage(projectId: String, paths: Vec<String>) -> Result<(), String> {
    let project = project_backend::project_path(&projectId)?;
    git::unstage(&project, &paths)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitResponse {
    hash: String,
    message: String,
}

#[tauri::command]
pub async fn git_commit(projectId: String, message: String) -> Result<GitCommitResponse, String> {
    let project = project_backend::project_path(&projectId)?;
    let commit = git::commit(&project, &message)?;
    Ok(GitCommitResponse {
        hash: commit.hash,
        message: commit.message,
    })
}

#[tauri::command]
pub async fn git_checkout(projectId: String, branch: String) -> Result<(), String> {
    let project = project_backend::project_path(&projectId)?;
    git::checkout(&project, &branch)
}

#[tauri::command]
pub async fn git_remote(projectId: String) -> Result<git::GitRemote, String> {
    let project = project_backend::project_path(&projectId)?;
    Ok(git::remote_state(&project))
}

#[tauri::command]
pub async fn github_open(app: AppHandle, projectId: String) -> Result<(), String> {
    let project = project_backend::project_path(&projectId)?;
    let remote = git::remote_state(&project);
    let url =
        github::browser_url(&remote).ok_or_else(|| "No GitHub remote is configured.".to_owned())?;
    app.opener()
        .open_url(url, None::<String>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn github_create_repo(
    projectId: String,
    name: String,
    isPrivate: bool,
) -> Result<git::GitRemote, String> {
    let project = project_backend::project_path(&projectId)?;
    github::create_repo(&project, &name, isPrivate)
}

#[tauri::command]
pub async fn github_link_push(
    projectId: String,
    remoteUrl: String,
) -> Result<git::GitRemote, String> {
    let project = project_backend::project_path(&projectId)?;
    github::link_and_push(&project, &remoteUrl)
}

#[tauri::command]
pub async fn github_auth_status() -> Result<github::GitHubAuthStatus, String> {
    Ok(github::auth_status())
}

#[tauri::command]
pub async fn github_repo_list(
    owner: Option<String>,
    query: Option<String>,
    visibility: Option<String>,
    sort: Option<String>,
) -> Result<Vec<github::GitHubRepo>, String> {
    github::list_repos(
        owner.as_deref(),
        query.as_deref().unwrap_or_default(),
        visibility.as_deref().unwrap_or("all"),
        sort.as_deref().unwrap_or("updated"),
    )
}
