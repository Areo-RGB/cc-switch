use std::path::PathBuf;

/// Grok Build's native configuration directory, used by its MCP writer.
pub fn get_grok_config_dir() -> PathBuf {
    crate::settings::get_grok_override_dir()
        .unwrap_or_else(|| crate::config::get_home_dir().join(".grok"))
}

pub fn get_grok_config_path() -> PathBuf {
    get_grok_config_dir().join("config.toml")
}
