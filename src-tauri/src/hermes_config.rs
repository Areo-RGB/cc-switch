//! Minimal Hermes configuration access retained for MCP synchronization and sessions.

use crate::config::atomic_write;
use crate::error::AppError;
use crate::settings::get_hermes_override_dir;
use serde_yaml::{Mapping, Value as YamlValue};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub fn get_hermes_dir() -> PathBuf {
    if let Some(override_dir) = get_hermes_override_dir() {
        return override_dir;
    }
    if let Some(raw) = std::env::var_os("HERMES_HOME") {
        let value = raw.to_string_lossy();
        if !value.trim().is_empty() {
            return PathBuf::from(value.trim());
        }
    }
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("LOCALAPPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| crate::config::get_home_dir().join("AppData").join("Local"))
            .join("hermes");
    }
    #[cfg(not(target_os = "windows"))]
    crate::config::get_home_dir().join(".hermes")
}

pub fn get_hermes_config_path() -> PathBuf {
    get_hermes_dir().join("config.yaml")
}

pub fn read_hermes_config() -> Result<YamlValue, AppError> {
    let path = get_hermes_config_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(empty_map()),
        Err(error) => return Err(AppError::io(&path, error)),
    };
    if text.trim().is_empty() {
        return Ok(empty_map());
    }
    serde_yaml::from_str(&text).map_err(|error| {
        AppError::Config(format!("Failed to parse Hermes config as YAML: {error}"))
    })
}

fn empty_map() -> YamlValue {
    YamlValue::Mapping(Mapping::new())
}

pub fn get_mcp_servers_yaml() -> Result<Mapping, AppError> {
    Ok(read_hermes_config()?
        .get("mcp_servers")
        .and_then(YamlValue::as_mapping)
        .cloned()
        .unwrap_or_default())
}

pub fn update_mcp_servers_yaml<F>(updater: F) -> Result<(), AppError>
where
    F: FnOnce(&mut Mapping) -> Result<(), AppError>,
{
    let lock = mcp_lock().lock()?;
    let mut config = read_hermes_config()?;
    let root = config
        .as_mapping_mut()
        .ok_or_else(|| AppError::Config("Hermes config root must be a mapping".into()))?;
    let mut servers = root
        .get("mcp_servers")
        .and_then(YamlValue::as_mapping)
        .cloned()
        .unwrap_or_default();
    updater(&mut servers)?;
    root.insert(
        YamlValue::String("mcp_servers".into()),
        YamlValue::Mapping(servers),
    );
    let output = serde_yaml::to_string(&config)
        .map_err(|error| AppError::Config(format!("Failed to serialize Hermes config: {error}")))?;
    let path = get_hermes_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
    }
    atomic_write(&path, output.as_bytes())?;
    drop(lock);
    Ok(())
}

pub(crate) fn yaml_to_json(value: &YamlValue) -> Result<serde_json::Value, AppError> {
    let text = serde_yaml::to_string(value)
        .map_err(|error| AppError::Config(format!("Failed to serialize YAML value: {error}")))?;
    serde_yaml::from_str(&text)
        .map_err(|error| AppError::Config(format!("Failed to convert YAML to JSON: {error}")))
}

pub(crate) fn json_to_yaml(value: &serde_json::Value) -> Result<YamlValue, AppError> {
    let text = serde_json::to_string(value)
        .map_err(|error| AppError::Config(format!("Failed to serialize JSON value: {error}")))?;
    serde_yaml::from_str(&text)
        .map_err(|error| AppError::Config(format!("Failed to convert JSON to YAML: {error}")))
}

fn mcp_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
