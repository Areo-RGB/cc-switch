use crate::config::atomic_write;
use crate::error::AppError;
use crate::jsonc_document::JsoncDocument;
use crate::settings::get_opencode_override_dir;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub fn get_opencode_dir() -> PathBuf {
    get_opencode_override_dir().unwrap_or_else(|| {
        crate::config::get_home_dir()
            .join(".config")
            .join("opencode")
    })
}

pub fn get_opencode_config_path() -> Result<PathBuf, AppError> {
    let dir = get_opencode_dir();
    for name in ["opencode.jsonc", "opencode.json"] {
        let path = dir.join(name);
        if path.try_exists().map_err(|e| AppError::io(&path, e))? {
            return Ok(path);
        }
    }
    Ok(dir.join("opencode.json"))
}

/// OpenCode stores its session database in the XDG data directory.
pub fn get_opencode_db_path() -> PathBuf {
    if let Ok(custom_path) = std::env::var("OPENCODE_DB") {
        if !custom_path.is_empty() {
            let path = PathBuf::from(custom_path);
            return if path.is_absolute() {
                path
            } else {
                get_opencode_data_dir().join(path)
            };
        }
    }
    get_opencode_data_dir().join("opencode.db")
}

fn get_opencode_data_dir() -> PathBuf {
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        if !xdg_data.is_empty() {
            return PathBuf::from(xdg_data).join("opencode");
        }
    }
    crate::config::get_home_dir()
        .join(".local")
        .join("share")
        .join("opencode")
}

struct OpenCodeDocument {
    path: PathBuf,
    previous_contents: Option<Vec<u8>>,
    document: JsoncDocument,
}

impl OpenCodeDocument {
    fn load(path: &Path) -> Result<Self, AppError> {
        let previous_contents = match std::fs::read(path) {
            Ok(contents) => Some(contents),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(AppError::io(path, error)),
        };
        let source = match &previous_contents {
            Some(contents) => std::str::from_utf8(contents).map_err(|error| {
                AppError::Config(format!(
                    "Invalid OpenCode config {}: {error}",
                    path.display()
                ))
            })?,
            None => "{\n  \"$schema\": \"https://opencode.ai/config.json\"\n}\n",
        };
        let document = JsoncDocument::parse(source).map_err(|error| {
            AppError::Config(format!(
                "Invalid OpenCode config {}: {error}",
                path.display()
            ))
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            previous_contents,
            document,
        })
    }

    fn save(self) -> Result<(), AppError> {
        let source = self.document.validated_source()?;
        let current = match std::fs::read(&self.path) {
            Ok(contents) => Some(contents),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(AppError::io(&self.path, error)),
        };
        if current != self.previous_contents {
            return Err(AppError::Config(format!(
                "OpenCode config changed on disk. Please reload and try again: {}",
                self.path.display()
            )));
        }
        if self.previous_contents.as_deref() != Some(source.as_bytes()) {
            atomic_write(&self.path, source.as_bytes())?;
        }
        Ok(())
    }
}

fn config_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn read_config() -> Result<Value, AppError> {
    Ok(OpenCodeDocument::load(&get_opencode_config_path()?)?
        .document
        .value()
        .clone())
}

fn edit_config(edit: impl FnOnce(&mut Value)) -> Result<(), AppError> {
    let _guard = config_lock().lock()?;
    let path = get_opencode_config_path()?;
    let mut document = OpenCodeDocument::load(&path)?;
    let mut value = document.document.value().clone();
    edit(&mut value);
    if document.document.apply(&value)? {
        document.save()?;
    }
    Ok(())
}

pub fn get_mcp_servers() -> Result<Map<String, Value>, AppError> {
    Ok(read_config()?
        .get("mcp")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default())
}

pub fn set_mcp_server(id: &str, config: Value) -> Result<(), AppError> {
    edit_config(|full_config| {
        if !full_config.get("mcp").is_some_and(Value::is_object) {
            full_config["mcp"] = json!({});
        }
        full_config["mcp"][id] = config;
    })
}

pub fn remove_mcp_server(id: &str) -> Result<(), AppError> {
    edit_config(|config| {
        if let Some(mcp) = config.get_mut("mcp").and_then(Value::as_object_mut) {
            mcp.remove(id);
        }
    })
}
