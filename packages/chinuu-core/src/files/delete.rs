use std::{fs, path::Path};

use crate::error::{CoreError, Result};

/// Delete a single file.
///
/// A symbolic link is unlinked and whatever it points at is left untouched,
/// whether that is a file or a directory.
///
/// Fails with [`CoreError::NotAFile`] for a directory, and with
/// [`CoreError::Io`] when the path does not exist: deleting something that is
/// already gone is an error rather than a quiet success, so a caller cannot
/// mistake a typo for a completed delete.
pub fn delete_file(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    // symlink_metadata does not follow links, which is what makes a link to a
    // directory show up as a link rather than as a directory.
    let metadata = fs::symlink_metadata(path).map_err(|e| CoreError::io(path, e))?;

    if metadata.is_dir() {
        return Err(CoreError::NotAFile(path.to_path_buf()));
    }

    fs::remove_file(path).map_err(|e| CoreError::io(path, e))
}

/// Delete a folder and everything inside it.
///
/// Links inside the folder are unlinked rather than followed, so a link to a
/// directory elsewhere does not take that directory with it. The same is true of
/// the path itself: a link to a directory is refused rather than followed, so
/// this cannot be aimed at a folder outside the vault by way of a link.
///
/// Fails with [`CoreError::NotADirectory`] for a file or a link, and with
/// [`CoreError::Io`] when the path does not exist.
pub fn delete_folder(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let metadata = fs::symlink_metadata(path).map_err(|e| CoreError::io(path, e))?;

    if !metadata.is_dir() {
        return Err(CoreError::NotADirectory(path.to_path_buf()));
    }

    fs::remove_dir_all(path).map_err(|e| CoreError::io(path, e))
}
