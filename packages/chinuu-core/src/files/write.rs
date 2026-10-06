//! Creating, moving and writing paths.
//!
//! Everything here changes the vault, and the write is the one worth reading the
//! code for. `write_file` and `write_bytes` replace the file through a temporary
//! file and a rename, which is what keeps a save from ever being observed half
//! finished.

use std::{
    fs::{self, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::error::{CoreError, Result};

/// Prefix of the temporary file an atomic write goes through.
///
/// The default ignore patterns cover this, so a write in flight never shows up
/// in the tree and never arrives as a watch event.
pub const TEMP_PREFIX: &str = ".chinuu-tmp-";

/// Separates two writes within one process, so two threads cannot pick the same
/// temporary path.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Create a folder, and any missing parents.
///
/// Succeeds when the folder is already there. Refuses a path that is a file.
pub fn create_folder(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    if path.is_file() {
        return Err(CoreError::NotADirectory(path.to_path_buf()));
    }

    fs::create_dir_all(path).map_err(|e| CoreError::io(path, e))
}

/// Move or rename a file or a folder.
///
/// Missing parent directories of the destination are created. An existing
/// destination is refused rather than overwritten, so a rename can never
/// quietly destroy a note; a caller that really means to replace something has
/// to delete it first.
pub fn move_path(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();

    // Reported against the source, because a missing source is the mistake a
    // caller usually made.
    fs::symlink_metadata(from).map_err(|e| CoreError::io(from, e))?;

    if to.symlink_metadata().is_ok() {
        return Err(CoreError::AlreadyExists(to.to_path_buf()));
    }

    if let Some(parent) = to.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
    }

    fs::rename(from, to).map_err(|e| CoreError::io(to, e))
}

/// Write text to a file, creating missing parent directories.
///
/// The bytes are written exactly as given. Nothing is normalised, no newline is
/// added or removed, and no encoding is applied, so reading a note and writing
/// it back unchanged leaves the file byte for byte identical.
///
/// The write is atomic: the content goes to a temporary file in the same
/// directory, is flushed to disk, and only then is renamed over the target. A
/// reader therefore sees either the whole old file or the whole new one, and a
/// crash part way through a save cannot leave a truncated note behind.
pub fn write_file(path: impl AsRef<Path>, content: &str) -> Result<()> {
    write_bytes(path, content.as_bytes())
}

/// Write bytes to a file. Atomic and byte for byte, as [`write_file`].
///
/// Use this for attachments, which are not text.
pub fn write_bytes(path: impl AsRef<Path>, content: &[u8]) -> Result<()> {
    let path = path.as_ref();

    if path.is_dir() {
        return Err(CoreError::NotAFile(path.to_path_buf()));
    }

    // Writing through a symbolic link must not replace the link, which renaming
    // onto it would do. Resolve it and write to what it points at instead.
    let target = write_target(path)?;

    if target.is_dir() {
        return Err(CoreError::NotAFile(target));
    }

    let parent = match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;

    // The temporary file has to share a directory with the target: a rename is
    // only atomic within one filesystem.
    let temp = temp_path(parent);
    prepare_temp(&temp, content)?;

    // A rename replaces the target wholesale, permissions included, so the old
    // ones are carried over first. Without this a 0644 note becomes 0600.
    #[cfg(unix)]
    if let Ok(metadata) = fs::symlink_metadata(&target) {
        if let Err(e) = fs::set_permissions(&temp, metadata.permissions()) {
            let _ = fs::remove_file(&temp);
            return Err(CoreError::io(&temp, e));
        }
    }

    if let Err(e) = fs::rename(&temp, &target) {
        let _ = fs::remove_file(&temp);
        return Err(CoreError::io(&target, e));
    }

    Ok(())
}

/// Where a write should land, following a symbolic link when there is one.
fn write_target(path: &Path) -> Result<PathBuf> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            if let Ok(resolved) = fs::canonicalize(path) {
                return Ok(resolved);
            }

            // A broken link. `fs::write` would create what it points at, so
            // follow it one level and let the write do the same.
            let link = fs::read_link(path).map_err(|e| CoreError::io(path, e))?;
            Ok(if link.is_absolute() {
                link
            } else {
                path.parent().unwrap_or(Path::new(".")).join(link)
            })
        }
        // Missing, or an ordinary file: write where we were told.
        _ => Ok(path.to_path_buf()),
    }
}

/// Write the bytes and flush them to disk before the caller renames.
fn prepare_temp(temp: &Path, content: &[u8]) -> Result<()> {
    // `create_new` rather than `create`: a name collision must fail instead of
    // truncating whatever is already there.
    let mut file = match OpenOptions::new().write(true).create_new(true).open(temp) {
        Ok(file) => file,
        Err(e) => return Err(CoreError::io(temp, e)),
    };

    // Flushing before the rename is what makes the swap meaningful. A crash
    // after this leaves the new content; a crash before it leaves the old file
    // untouched.
    let written = file.write_all(content).and_then(|()| file.sync_all());

    if let Err(e) = written {
        drop(file);
        let _ = fs::remove_file(temp);
        return Err(CoreError::io(temp, e));
    }

    Ok(())
}

/// A unique temporary path inside `dir`.
///
/// No randomness dependency: the process id separates processes, the counter
/// separates writers inside one process, and the timestamp separates runs of the
/// same process.
fn temp_path(dir: &Path) -> PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    dir.join(format!(
        "{TEMP_PREFIX}{}-{nanos}-{counter}",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_paths_are_unique_and_prefixed() {
        let dir = Path::new("/vault");

        let first = temp_path(dir);
        let second = temp_path(dir);

        assert_ne!(first, second);
        for path in [&first, &second] {
            assert!(path.starts_with(dir));
            assert!(
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(TEMP_PREFIX),
                "got {path:?}"
            );
        }
    }

    #[test]
    fn temp_paths_are_ignored_by_the_default_rules() {
        // The two constants live in different modules, so this is what keeps
        // them from drifting apart: a temp file must never surface in the tree.
        let rules = crate::files::IgnoreRules::defaults();
        let name = temp_path(Path::new("/vault"))
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        assert!(rules.is_ignored(&name, false), "{name} is not ignored");
    }
}
