use std::{fs, path::Path};

use crate::error::{CoreError, Result};

/// Read a file as UTF-8 text.
pub fn read_file(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();

    if !path.is_file() {
        return Err(CoreError::NotAFile(path.to_path_buf()));
    }

    fs::read_to_string(path).map_err(|e| CoreError::io(path, e))
}
