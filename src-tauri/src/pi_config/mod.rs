//! Path and native-settings helpers for Pi MCP, prompts, skills, and sessions.

use crate::config::get_home_dir;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
#[cfg(test)]
use std::sync::{LazyLock, Mutex};

#[cfg(test)]
static TEST_AGENT_DIR: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PiNativeDefaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_dir: Option<String>,
}

pub(crate) fn get_pi_agent_dir() -> Result<PathBuf, AppError> {
    #[cfg(test)]
    if let Some(path) = TEST_AGENT_DIR.lock()?.clone() {
        if !path.is_absolute() {
            return Err(AppError::InvalidInput(format!(
                "Pi agent directory must be absolute: {}",
                path.display()
            )));
        }
        return Ok(path);
    }
    let path = crate::settings::get_pi_override_dir()
        .or_else(|| std::env::var_os("PI_CODING_AGENT_DIR").map(PathBuf::from))
        .unwrap_or_else(|| get_home_dir().join(".pi").join("agent"));
    if !path.is_absolute() {
        return Err(AppError::InvalidInput(format!(
            "Pi agent directory must be absolute: {}",
            path.display()
        )));
    }
    Ok(path)
}

fn get_pi_settings_path() -> Result<PathBuf, AppError> {
    Ok(get_pi_agent_dir()?.join("settings.json"))
}

pub(crate) fn read_pi_native_defaults() -> Result<PiNativeDefaults, AppError> {
    let path = get_pi_settings_path()?;
    if !path.exists() {
        return Ok(PiNativeDefaults::default());
    }
    let text = fs::read_to_string(&path).map_err(|error| AppError::io(&path, error))?;
    let value: Value = json5::from_str(&text).map_err(|error| {
        AppError::Config(format!(
            "Pi settings file is not valid JSON5 ({}): {error}",
            path.display()
        ))
    })?;
    let object = value.as_object().ok_or_else(|| {
        AppError::Config(format!(
            "Pi settings root must be an object: {}",
            path.display()
        ))
    })?;
    let read_string = |key: &str| -> Result<Option<String>, AppError> {
        match object.get(key) {
            None => Ok(None),
            Some(Value::String(value)) => Ok(Some(value.clone())),
            Some(_) => Err(AppError::Config(format!(
                "Pi settings field '{key}' must be a string: {}",
                path.display()
            ))),
        }
    };
    Ok(PiNativeDefaults {
        default_provider: read_string("defaultProvider")?,
        default_model: read_string("defaultModel")?,
        session_dir: read_string("sessionDir")?,
    })
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
            let dir = tempfile::tempdir().expect("create Pi test directory");
            let agent_dir = dir.path().join("agent");
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
