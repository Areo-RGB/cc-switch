//! Project inspection, script discovery, and desktop launchers.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::process;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedScript {
    pub id: String,
    pub name: String,
    pub command: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectInspection {
    pub frameworks: Vec<String>,
    pub languages: Vec<String>,
    pub package_manager: Option<String>,
    pub scripts: Vec<DetectedScript>,
    pub markers: Vec<String>,
}

pub fn project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("project")
        .to_owned()
}

pub fn package_manager(path: &Path) -> Option<String> {
    if path.join("pnpm-lock.yaml").exists() {
        Some("pnpm".to_owned())
    } else if path.join("yarn.lock").exists() {
        Some("yarn".to_owned())
    } else if path.join("bun.lock").exists() || path.join("bun.lockb").exists() {
        Some("bun".to_owned())
    } else if path.join("package-lock.json").exists() || path.join("package.json").exists() {
        Some("npm".to_owned())
    } else {
        None
    }
}

fn script_id(name: &str) -> String {
    let mut id = String::with_capacity(name.len());
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
            id.push(character);
        } else {
            id.push('-');
        }
    }
    id.trim_matches('-').to_owned()
}

fn push_script(
    scripts: &mut Vec<DetectedScript>,
    name: impl Into<String>,
    command: impl Into<String>,
) {
    let name = name.into();
    let id = script_id(&name);
    if id.is_empty() || scripts.iter().any(|script| script.id == id) {
        return;
    }
    scripts.push(DetectedScript {
        id,
        name,
        command: command.into(),
    });
}

fn package_scripts(
    path: &Path,
    scripts: &mut Vec<DetectedScript>,
    frameworks: &mut BTreeSet<String>,
    languages: &mut BTreeSet<String>,
) {
    let Ok(text) = fs::read_to_string(path.join("package.json")) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    let manager = package_manager(path).unwrap_or_else(|| "npm".to_owned());
    languages.insert(if path.join("tsconfig.json").exists() {
        "TypeScript".to_owned()
    } else {
        "JavaScript".to_owned()
    });
    let mut dependencies = HashSet::new();
    for section in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(object) = value.get(section).and_then(Value::as_object) {
            dependencies.extend(object.keys().cloned());
        }
    }
    for (dependency, framework) in [
        ("react", "React"),
        ("next", "Next.js"),
        ("vue", "Vue"),
        ("nuxt", "Nuxt"),
        ("svelte", "Svelte"),
        ("@sveltejs/kit", "SvelteKit"),
        ("@angular/core", "Angular"),
        ("astro", "Astro"),
        ("solid-js", "SolidJS"),
        ("electron", "Electron"),
        ("@tauri-apps/api", "Tauri"),
    ] {
        if dependencies.contains(dependency) {
            frameworks.insert(framework.to_owned());
        }
    }
    for (dependency, tool) in [
        ("vite", "Vite"),
        ("webpack", "Webpack"),
        ("esbuild", "esbuild"),
    ] {
        if dependencies.contains(dependency) {
            frameworks.insert(tool.to_owned());
        }
    }
    let Some(entries) = value.get("scripts").and_then(Value::as_object) else {
        return;
    };
    let mut names = entries.keys().cloned().collect::<Vec<_>>();
    names.sort();
    for name in names {
        push_script(scripts, name.clone(), format!("{manager} run {name}"));
    }
}

fn cargo_scripts(path: &Path, scripts: &mut Vec<DetectedScript>, languages: &mut BTreeSet<String>) {
    if !path.join("Cargo.toml").exists() {
        return;
    }
    languages.insert("Rust".to_owned());
    push_script(scripts, "cargo-run", "cargo run");
    push_script(scripts, "cargo-build", "cargo build");
    push_script(scripts, "cargo-test", "cargo test");
    push_script(scripts, "cargo-check", "cargo check");
}

fn just_scripts(path: &Path, scripts: &mut Vec<DetectedScript>) {
    let file = ["justfile", "Justfile", "JUSTFILE"]
        .into_iter()
        .map(|name| path.join(name))
        .find(|candidate| candidate.is_file());
    let Some(file) = file else { return };
    let Ok(text) = fs::read_to_string(file) else {
        return;
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("set ") {
            continue;
        }
        let Some((name, _)) = trimmed.split_once(':') else {
            continue;
        };
        let name = name.split_whitespace().next().unwrap_or("");
        if name.is_empty() || name.starts_with('@') || name.contains('=') {
            continue;
        }
        push_script(scripts, name, format!("just {name}"));
    }
}

fn pyproject_scripts(
    path: &Path,
    scripts: &mut Vec<DetectedScript>,
    frameworks: &mut BTreeSet<String>,
    languages: &mut BTreeSet<String>,
) {
    let pyproject = path.join("pyproject.toml");
    let Ok(text) = fs::read_to_string(&pyproject) else {
        return;
    };
    languages.insert("Python".to_owned());
    let lower = text.to_ascii_lowercase();
    for (needle, framework) in [
        ("fastapi", "FastAPI"),
        ("flask", "Flask"),
        ("django", "Django"),
    ] {
        if lower.contains(needle) {
            frameworks.insert(framework.to_owned());
        }
    }
    let Ok(value) = text.parse::<toml::Value>() else {
        return;
    };
    let mut names = Vec::new();
    if let Some(table) = value
        .get("project")
        .and_then(|value| value.get("scripts"))
        .and_then(toml::Value::as_table)
    {
        names.extend(table.keys().cloned());
    }
    if let Some(table) = value
        .get("tool")
        .and_then(|value| value.get("poetry"))
        .and_then(|value| value.get("scripts"))
        .and_then(toml::Value::as_table)
    {
        names.extend(table.keys().cloned());
    }
    names.sort();
    for name in names {
        push_script(scripts, name.clone(), format!("python -m {name}"));
    }
}

fn make_scripts(path: &Path, scripts: &mut Vec<DetectedScript>) {
    let file = ["Makefile", "makefile", "GNUmakefile"]
        .into_iter()
        .map(|name| path.join(name))
        .find(|candidate| candidate.is_file());
    let Some(file) = file else { return };
    let Ok(text) = fs::read_to_string(file) else {
        return;
    };
    for line in text.lines() {
        if line.starts_with([' ', '\t']) || line.trim_start().starts_with('#') {
            continue;
        }
        let Some((target, _)) = line.split_once(':') else {
            continue;
        };
        let target = target.trim();
        if target.is_empty() || target.contains(['%', '=']) || target.starts_with('.') {
            continue;
        }
        push_script(scripts, format!("make-{target}"), format!("make {target}"));
    }
}

pub fn detect_scripts(path: &Path) -> Vec<DetectedScript> {
    let mut scripts = Vec::new();
    let mut frameworks = BTreeSet::new();
    let mut languages = BTreeSet::new();
    package_scripts(path, &mut scripts, &mut frameworks, &mut languages);
    cargo_scripts(path, &mut scripts, &mut languages);
    just_scripts(path, &mut scripts);
    pyproject_scripts(path, &mut scripts, &mut frameworks, &mut languages);
    make_scripts(path, &mut scripts);
    if path.join("go.mod").is_file() {
        languages.insert("Go".to_owned());
        push_script(&mut scripts, "go-run", "go run .");
        push_script(&mut scripts, "go-test", "go test ./...");
    }
    if path.join("pom.xml").is_file() {
        languages.insert("Java".to_owned());
        push_script(&mut scripts, "maven-test", "mvn test");
        push_script(&mut scripts, "maven-package", "mvn package");
    }
    let gradle = if path.join("gradlew").is_file() {
        "./gradlew"
    } else if path.join("gradlew.bat").is_file() {
        "gradlew.bat"
    } else {
        "gradle"
    };
    if path.join("build.gradle").is_file() || path.join("build.gradle.kts").is_file() {
        languages.insert("Java".to_owned());
        push_script(&mut scripts, "gradle-build", format!("{gradle} build"));
        push_script(&mut scripts, "gradle-test", format!("{gradle} test"));
    }
    if (path.join("main.py").is_file() || path.join("app.py").is_file())
        && !scripts.iter().any(|script| script.name == "python-run")
    {
        languages.insert("Python".to_owned());
        let entry = if path.join("main.py").is_file() {
            "main.py"
        } else {
            "app.py"
        };
        push_script(&mut scripts, "python-run", format!("python {entry}"));
    }
    scripts
}

pub fn inspect_project(path: &Path) -> ProjectInspection {
    if !path.is_dir() {
        return ProjectInspection::default();
    }
    let mut frameworks = BTreeSet::new();
    let mut languages = BTreeSet::new();
    let mut scripts = Vec::new();
    package_scripts(path, &mut scripts, &mut frameworks, &mut languages);
    cargo_scripts(path, &mut scripts, &mut languages);
    just_scripts(path, &mut scripts);
    pyproject_scripts(path, &mut scripts, &mut frameworks, &mut languages);
    make_scripts(path, &mut scripts);
    if path.join("src-tauri").is_dir() {
        frameworks.insert("Tauri".to_owned());
    }
    let scripts_from_all = detect_scripts(path);
    for script in scripts_from_all {
        if !scripts.iter().any(|existing| existing.id == script.id) {
            scripts.push(script);
        }
    }
    let markers = [
        ".git",
        "package.json",
        "Cargo.toml",
        "pyproject.toml",
        "justfile",
        "Justfile",
        "Makefile",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "Dockerfile",
    ]
    .into_iter()
    .filter(|name| path.join(name).exists())
    .map(str::to_owned)
    .collect();
    ProjectInspection {
        frameworks: frameworks.into_iter().collect(),
        languages: languages.into_iter().collect(),
        package_manager: package_manager(path),
        scripts,
        markers,
    }
}

pub fn find_script(path: &Path, id: &str) -> Option<DetectedScript> {
    detect_scripts(path)
        .into_iter()
        .find(|script| script.id == id || script.name == id)
}

fn split_command(command: &str) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for character in command.chars() {
        match quote {
            Some(active) if character == active => quote = None,
            Some(_) => current.push(character),
            None if matches!(character, '\'' | '"') => quote = Some(character),
            None if character.is_whitespace() => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }
            None => current.push(character),
        }
    }
    if quote.is_some() {
        return Err("Unterminated launcher quote.".to_owned());
    }
    if !current.is_empty() {
        result.push(current);
    }
    Ok(result)
}

pub fn launch_editor(path: &Path) -> Result<(), String> {
    let executable = std::env::var("CC_SWITCH_EDITOR")
        .ok()
        .or_else(|| std::env::var("VISUAL").ok())
        .or_else(|| std::env::var("EDITOR").ok())
        .or_else(|| {
            ["code", "cursor", "zed", "windsurf", "idea", "subl"]
                .into_iter()
                .find(|candidate| process::which(candidate).is_some())
                .map(str::to_owned)
        })
        .ok_or_else(|| "No supported editor was found.".to_owned())?;
    let mut parts = split_command(&executable)?;
    let program = parts
        .first()
        .cloned()
        .ok_or_else(|| "Editor command is empty.".to_owned())?;
    parts.remove(0);
    parts.push(path.display().to_string());
    process::spawn_detached(&program, &parts, path)
}

pub fn launch_terminal(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        return process::spawn_detached(
            "open",
            &["-a".into(), "Terminal".into(), path.display().to_string()],
            path,
        );
    }
    #[cfg(target_os = "windows")]
    {
        return process::spawn_detached(
            "cmd.exe",
            &["/K".into(), format!("cd /d {}", path.display())],
            path,
        );
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let candidates = [
            (
                "x-terminal-emulator",
                vec!["--working-directory".to_owned(), path.display().to_string()],
            ),
            (
                "gnome-terminal",
                vec!["--working-directory".to_owned(), path.display().to_string()],
            ),
            (
                "konsole",
                vec!["--workdir".to_owned(), path.display().to_string()],
            ),
            (
                "xfce4-terminal",
                vec!["--working-directory".to_owned(), path.display().to_string()],
            ),
        ];
        for (program, args) in candidates {
            if process::which(program).is_some() {
                return process::spawn_detached(program, &args, path);
            }
        }
        return Err("No supported terminal was found.".to_owned());
    }
    #[allow(unreachable_code)]
    Err("No supported terminal was found.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_quoted_editor_commands() {
        assert_eq!(
            split_command("/Applications/Visual Studio Code.app/code --reuse-window"),
            Ok(vec![
                "/Applications/Visual".to_owned(),
                "Studio".to_owned(),
                "Code.app/code".to_owned(),
                "--reuse-window".to_owned()
            ])
        );
        assert!(split_command("code \"unterminated").is_err());
    }
}
