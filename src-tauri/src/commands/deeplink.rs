use crate::deeplink::{
    import_mcp_from_deeplink, import_prompt_from_deeplink, import_skill_from_deeplink,
    parse_deeplink_url, DeepLinkImportRequest, PendingDeepLinkEntry,
};
use crate::store::AppState;
use tauri::State;

#[tauri::command]
pub fn peek_pending_deeplink() -> Option<PendingDeepLinkEntry> {
    crate::deeplink::peek_pending_deeplink()
}

#[tauri::command]
pub fn ack_pending_deeplink(id: u64) {
    crate::deeplink::ack_pending_deeplink(id);
}

#[tauri::command]
pub fn parse_deeplink(url: String) -> Result<DeepLinkImportRequest, String> {
    parse_deeplink_url(&url).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_from_deeplink_unified(
    state: State<'_, AppState>,
    request: DeepLinkImportRequest,
) -> Result<serde_json::Value, String> {
    match request.resource.as_str() {
        "prompt" => Ok(serde_json::json!({
            "type": "prompt",
            "id": import_prompt_from_deeplink(&state, request).map_err(|e| e.to_string())?,
        })),
        "mcp" => {
            let result = import_mcp_from_deeplink(&state, request).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "type": "mcp",
                "importedCount": result.imported_count,
                "importedIds": result.imported_ids,
                "failed": result.failed,
            }))
        }
        "skill" => Ok(serde_json::json!({
            "type": "skill",
            "key": import_skill_from_deeplink(&state, request).map_err(|e| e.to_string())?,
        })),
        other => Err(format!("Unsupported resource type: {other}")),
    }
}
