use std::{fs, path::Path};

use crate::error::{CoreError, Result};

/// Write text to a file, creating missing parent directories.
pub fn write_file(path: impl AsRef<Path>, content: &str) -> Result<()> {
    let path = path.as_ref();

    if path.is_dir() {
        return Err(CoreError::NotAFile(path.to_path_buf()));
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
    }

    fs::write(path, content).map_err(|e| CoreError::io(path, e))
}
