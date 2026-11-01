//! Paths used by Kimi Code's retained MCP, skill, and session integrations.

use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    crate::config::get_home_dir().join(".kimi")
}
