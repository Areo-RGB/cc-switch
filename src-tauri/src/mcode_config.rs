//! MiniMax Code native paths and transactional file updates.
use crate::config::{atomic_write_private, get_home_dir};
use crate::error::AppError;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn data_dir() -> PathBuf {
    explicit_data_dir(
        std::env::var("MINIMAX_DATA_DIR").ok().as_deref(),
        std::env::var("MAVIS_DATA_DIR").ok().as_deref(),
    )
    .unwrap_or_else(|| get_home_dir().join(".minimax"))
}

fn explicit_data_dir(minimax: Option<&str>, mavis: Option<&str>) -> Option<PathBuf> {
    [minimax, mavis]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|path| !path.is_empty())
        .map(PathBuf::from)
}

/// Write a native file, run the database commit, and restore the original file on failure.
pub(crate) fn write_and_commit<T>(
    path: &Path,
    write: impl FnOnce() -> Result<(), AppError>,
    commit: impl FnOnce() -> Result<T, AppError>,
) -> Result<T, AppError> {
    let previous = match fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(AppError::io(path, error)),
    };
    write()?;
    match commit() {
        Ok(result) => Ok(result),
        Err(error) => {
            let rollback = match previous {
                Some(bytes) => atomic_write_private(path, &bytes),
                None => match fs::remove_file(path) {
                    Ok(()) => Ok(()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(error) => Err(AppError::io(path, error)),
                },
            };
            if let Err(rollback_error) = rollback {
                return Err(AppError::Message(format!(
                    "Config update failed ({error}); restoring {} also failed: {rollback_error}",
                    path.display()
                )));
            }
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_data_directory_precedence_and_blank_values() {
        assert_eq!(
            explicit_data_dir(Some(" /primary "), Some("/legacy")),
            Some("/primary".into())
        );
        assert_eq!(
            explicit_data_dir(Some(" \t"), Some(" /legacy ")),
            Some("/legacy".into())
        );
        assert_eq!(explicit_data_dir(None, Some("")), None);
        assert_eq!(explicit_data_dir(None, None), None);
    }

    #[test]
    fn write_and_commit_restores_a_new_file_when_commit_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        let result: Result<(), AppError> = write_and_commit(
            &path,
            || fs::write(&path, b"new").map_err(|error| AppError::io(&path, error)),
            || Err::<(), _>(AppError::Message("reject".into())),
        );
        assert!(result.is_err());
        assert!(!path.exists());
    }
}
