//! Paths and MCP document access for Oh My Pi.

use crate::config::get_home_dir;
use crate::error::AppError;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
static TEST_AGENT_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

pub(crate) fn get_omp_agent_dir() -> Result<PathBuf, AppError> {
    #[cfg(test)]
    if let Some(path) = TEST_AGENT_DIR.lock()?.clone() {
        return Ok(path);
    }
    if let Some(dir) = crate::settings::get_omp_override_dir() {
        return Ok(dir);
    }
    get_omp_native_agent_dir()
}

pub(crate) fn get_omp_active_profile() -> Result<Option<String>, AppError> {
    let selected = std::env::var_os("OMP_PROFILE").or_else(|| std::env::var_os("PI_PROFILE"));
    normalize_profile(
        selected
            .as_deref()
            .map(|name| name.to_string_lossy())
            .as_deref(),
    )
}

fn normalize_profile(raw: Option<&str>) -> Result<Option<String>, AppError> {
    let name = raw.unwrap_or_default().trim();
    if name.is_empty() || name == "default" {
        return Ok(None);
    }
    let reserved = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved_device = matches!(reserved.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (reserved.len() == 4
            && (reserved.starts_with("COM") || reserved.starts_with("LPT"))
            && reserved.as_bytes()[3].is_ascii_digit());
    if name.len() > 64
        || name.is_empty()
        || (!name.as_bytes()[0].is_ascii_lowercase() && !name.as_bytes()[0].is_ascii_digit())
        || !name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._-".contains(&c))
        || name.ends_with('.')
        || reserved_device
    {
        return Err(AppError::Config(format!(
            "Invalid Oh My Pi profile: {name}"
        )));
    }
    Ok(Some(name.to_string()))
}

fn native_agent_dir(
    home: &Path,
    config_dir: Option<&str>,
    profile: Option<&str>,
    inherited_profile: Option<&str>,
    agent_dir: Option<&Path>,
) -> Result<PathBuf, AppError> {
    let config_dir = config_dir.filter(|name| !name.is_empty()).unwrap_or(".omp");
    let root = home.join(config_dir.trim_start_matches(std::path::MAIN_SEPARATOR));
    if let Some(profile) = profile {
        return Ok(root.join("profiles").join(profile).join("agent"));
    }
    let agent_dir = agent_dir.filter(|agent| {
        inherited_profile
            .is_none_or(|profile| *agent != root.join("profiles").join(profile).join("agent"))
    });
    if let Some(agent) = agent_dir.filter(|path| !path.as_os_str().is_empty()) {
        return if agent.is_absolute() {
            Ok(agent.to_path_buf())
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(agent))
                .map_err(|error| {
                    AppError::Config(format!("Cannot resolve Oh My Pi agent directory: {error}"))
                })
        };
    }
    Ok(root.join("agent"))
}

pub(crate) fn get_omp_native_agent_dir() -> Result<PathBuf, AppError> {
    let profile = get_omp_active_profile()?;
    let config_dir = std::env::var("PI_CONFIG_DIR").ok();
    let agent_dir = std::env::var_os("PI_CODING_AGENT_DIR").map(PathBuf::from);
    let inherited_profile = normalize_profile(std::env::var("PI_PROFILE").ok().as_deref())
        .ok()
        .flatten();
    native_agent_dir(
        &get_home_dir(),
        config_dir.as_deref(),
        profile.as_deref(),
        inherited_profile.as_deref(),
        agent_dir.as_deref(),
    )
}

pub(crate) fn get_omp_data_dir() -> Result<PathBuf, AppError> {
    let agent = get_omp_agent_dir()?;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let profile = get_omp_active_profile()?;
        let config_dir = std::env::var("PI_CONFIG_DIR").ok();
        let default_agent = native_agent_dir(
            &get_home_dir(),
            config_dir.as_deref(),
            profile.as_deref(),
            None,
            None,
        )?;
        if agent == default_agent {
            if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
                let mut root = PathBuf::from(xdg).join("omp");
                if let Some(profile) = profile {
                    root = root.join("profiles").join(profile);
                }
                if root.exists() {
                    return Ok(root);
                }
            }
        }
    }
    Ok(agent)
}

pub(crate) fn get_omp_mcp_path() -> Result<PathBuf, AppError> {
    Ok(get_omp_agent_dir()?.join("mcp.json"))
}

pub(crate) fn update_omp_mcp_document(
    update: impl FnOnce(&mut Value) -> Result<bool, AppError>,
) -> Result<(), AppError> {
    let path = get_omp_mcp_path()?;
    let before = match fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(AppError::io(&path, error)),
    };
    let mut document = match &before {
        Some(bytes) => serde_json::from_slice(bytes)
            .map_err(|error| AppError::Config(format!("Invalid Oh My Pi MCP file: {error}")))?,
        None => Value::Object(Default::default()),
    };
    if !document.is_object() || document.get("mcpServers").is_some_and(|v| !v.is_object()) {
        return Err(AppError::Config(
            "Oh My Pi MCP document must be an object".into(),
        ));
    }
    if !update(&mut document)? {
        return Ok(());
    }
    let mut after = serde_json::to_vec_pretty(&document)
        .map_err(|error| AppError::JsonSerialize { source: error })?;
    after.push(b'\n');
    if fs::read(&path).ok() != before {
        return Err(AppError::Conflict("omp.errors.externalChange".into()));
    }
    crate::config::atomic_write_private(&path, &after)
}

/// Detect YAML anchors, aliases, tags, or merge keys before a text edit.
pub(crate) fn has_yaml_reference_tokens(line: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;
    let mut node_start = true;
    let mut after_space = true;
    let mut previous = [' '; 2];
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote == Some('"') && c == '\\' {
            escaped = true;
            continue;
        }
        if quote == Some('\'') && c == '\'' && chars.peek() == Some(&'\'') {
            chars.next();
            continue;
        }
        if quote == Some(c) {
            quote = None;
            node_start = false;
            after_space = false;
            continue;
        }
        if quote.is_some() {
            continue;
        }
        if c == ':' && previous == ['<', '<'] {
            return true;
        }
        match c {
            '#' if after_space => break,
            '"' | '\'' if node_start => quote = Some(c),
            '&' | '*' | '!' if node_start => return true,
            _ => {}
        }
        previous = [previous[1], c];
        after_space = c.is_whitespace();
        if !c.is_whitespace() {
            let entry_marker = node_start
                && matches!(c, '-' | '?')
                && chars.peek().is_none_or(|next| next.is_whitespace());
            node_start = entry_marker || matches!(c, ':' | '[' | '{' | ',');
        }
    }
    false
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::{Path, PathBuf};

    pub(crate) struct TestAgentDir {
        _dir: Option<tempfile::TempDir>,
        previous: Option<PathBuf>,
    }

    impl TestAgentDir {
        pub(crate) fn new() -> Self {
            let dir = tempfile::tempdir().expect("create Oh My Pi test directory");
            let agent_dir = dir.path().join("agent");
            std::fs::create_dir_all(&agent_dir).expect("create Oh My Pi agent directory");
            Self::set(agent_dir, Some(dir))
        }

        pub(crate) fn at(agent_dir: &Path) -> Self {
            Self::set(agent_dir.to_path_buf(), None)
        }

        fn set(agent_dir: PathBuf, dir: Option<tempfile::TempDir>) -> Self {
            let previous = super::TEST_AGENT_DIR.lock().unwrap().replace(agent_dir);
            Self {
                _dir: dir,
                previous,
            }
        }
    }

    impl Drop for TestAgentDir {
        fn drop(&mut self) {
            *super::TEST_AGENT_DIR.lock().unwrap() = self.previous.take();
        }
    }
}
