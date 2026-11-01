#![allow(non_snake_case)]

/// Read the retained application settings.
#[tauri::command]
pub async fn get_settings() -> Result<crate::settings::AppSettings, String> {
    Ok(crate::settings::get_settings_for_frontend())
}

/// Persist settings that belong to Desk.
#[tauri::command]
pub async fn save_settings(settings: crate::settings::AppSettings) -> Result<bool, String> {
    crate::settings::update_settings(settings).map_err(|e| e.to_string())?;
    Ok(true)
}
