//! Deep-link import support for MCP servers, prompts, and skills.

mod mcp;
mod parser;
mod prompt;
mod skill;
mod utils;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub use mcp::import_mcp_from_deeplink;
pub use parser::parse_deeplink_url;
pub use prompt::import_prompt_from_deeplink;
pub use skill::import_skill_from_deeplink;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PendingDeepLink {
    Import { request: Box<DeepLinkImportRequest> },
    Error { url: String, error: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct PendingDeepLinkEntry {
    pub id: u64,
    pub link: PendingDeepLink,
}

static PENDING_DEEPLINK: Mutex<Option<PendingDeepLinkEntry>> = Mutex::new(None);
static NEXT_PENDING_ID: AtomicU64 = AtomicU64::new(1);

fn pending_slot() -> std::sync::MutexGuard<'static, Option<PendingDeepLinkEntry>> {
    PENDING_DEEPLINK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn set_pending_deeplink(link: PendingDeepLink) {
    let id = NEXT_PENDING_ID.fetch_add(1, Ordering::Relaxed);
    *pending_slot() = Some(PendingDeepLinkEntry { id, link });
}

pub fn peek_pending_deeplink() -> Option<PendingDeepLinkEntry> {
    pending_slot().clone()
}

pub fn ack_pending_deeplink(id: u64) {
    let mut slot = pending_slot();
    if slot.as_ref().is_some_and(|entry| entry.id == id) {
        *slot = None;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepLinkImportRequest {
    pub version: String,
    pub resource: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apps: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_format: Option<String>,
}
