//! Paths and session resume commands for the retained Antigravity integrations.

use std::path::{Path, PathBuf};

pub fn agy_dir() -> PathBuf {
    crate::config::get_home_dir()
        .join(".gemini")
        .join("antigravity-cli")
}

pub fn customizations_dir() -> PathBuf {
    crate::config::get_home_dir().join(".gemini").join("config")
}

pub fn key_profiles_root() -> PathBuf {
    crate::config::get_app_config_dir().join("agy")
}

pub fn resume_command(gemini_dir: Option<&Path>, conversation: &str) -> String {
    match gemini_dir {
        Some(dir) => format!(
            "agy {} --conversation={conversation}",
            shell_word(&format!("--gemini_dir={}", dir.to_string_lossy()))
        ),
        None => format!("agy --conversation={conversation}"),
    }
}

fn shell_word(value: &str) -> String {
    let plain = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '='));
    if plain {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', r"'\''"))
    }
}
