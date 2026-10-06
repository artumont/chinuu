use std::{fs, path::Path};

use crate::error::{CoreError, Result};

/// Read a file as raw bytes.
///
/// Nothing is decoded or normalised, so this is the right call for an attachment
/// and the wrong one for a note.
pub fn read_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path = path.as_ref();

    if !path.is_file() {
        return Err(CoreError::NotAFile(path.to_path_buf()));
    }

    fs::read(path).map_err(|e| CoreError::io(path, e))
}

/// Read a file as UTF-8 text.
///
/// The text comes back exactly as it is on disk: no newline translation, no
/// trailing newline added or removed, and a byte order mark is kept rather than
/// stripped. Writing the result back with [`write_file`] therefore leaves the
/// file byte for byte identical, which is the contract the editor depends on.
///
/// A file that is not valid UTF-8 is reported as [`CoreError::NotUtf8`] rather
/// than as an io failure, so a caller can offer to open it another way instead
/// of treating it as something that went wrong.
///
/// [`write_file`]: crate::files::write_file
pub fn read_file(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let bytes = read_bytes(path)?;

    String::from_utf8(bytes).map_err(|_| CoreError::NotUtf8(path.to_path_buf()))
}
