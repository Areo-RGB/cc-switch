use std::path::PathBuf;

pub fn get_gemini_dir() -> PathBuf {
    crate::settings::get_gemini_override_dir()
        .unwrap_or_else(|| crate::config::get_home_dir().join(".gemini"))
}

pub fn get_gemini_settings_path() -> PathBuf {
    get_gemini_dir().join("settings.json")
}
