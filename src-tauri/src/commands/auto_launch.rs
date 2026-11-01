#[tauri::command]
pub fn set_auto_launch(enabled: bool) -> Result<(), String> {
    if enabled {
        crate::auto_launch::enable_auto_launch()
    } else {
        crate::auto_launch::disable_auto_launch()
    }
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_auto_launch_status() -> Result<bool, String> {
    crate::auto_launch::is_auto_launch_enabled().map_err(|error| error.to_string())
}
