//! Paths and MCP-facing readers for Codex configuration.

use std::path::PathBuf;

pub fn get_codex_config_dir() -> PathBuf {
    crate::config::get_home_dir().join(".codex")
}

pub fn get_codex_config_path() -> PathBuf {
    get_codex_config_dir().join("config.toml")
}

pub fn get_codex_auth_path() -> PathBuf {
    get_codex_config_dir().join("auth.json")
}

pub fn read_and_validate_codex_config_text() -> Result<String, crate::error::AppError> {
    let path = get_codex_config_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(crate::error::AppError::io(&path, error)),
    }
}

pub fn read_codex_config_text() -> Result<String, crate::error::AppError> {
    read_and_validate_codex_config_text()
}
