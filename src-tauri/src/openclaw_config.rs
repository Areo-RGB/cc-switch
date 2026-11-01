use std::path::PathBuf;

/// OpenClaw's native data directory, used by session discovery and prompts.
pub fn get_openclaw_dir() -> PathBuf {
    crate::config::get_home_dir().join(".openclaw")
}
