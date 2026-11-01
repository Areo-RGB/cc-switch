#![allow(non_snake_case)]

mod auto_launch;
mod deeplink;
mod import_export;
mod mcp;
mod prompt;
mod session_manager;
mod settings;
pub mod skill;
mod sync_support;

mod s3_sync;
mod webdav_sync;

// projects-github: Phase 3 project detail, Git, GitHub, launchers, and scripts.
mod projects_github;

pub use auto_launch::*;
pub use deeplink::*;
pub use import_export::*;
pub use mcp::*;
pub use prompt::*;
pub use session_manager::*;
pub use settings::*;
pub use skill::*;

pub use projects_github::*;
pub use s3_sync::*;
pub use webdav_sync::*;
