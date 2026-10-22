use std::{
    fs,
    io::{self, Error},
    path::Path,
};

use uuid::Uuid;

use crate::fileio::types::UUID_NAMESPACE;

fn generate_path_id(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    Uuid::new_v5(&UUID_NAMESPACE, path.to_string_lossy().as_bytes()).to_string()
}

fn read_dir_sorted(path: impl AsRef<Path>) -> Result<Vec<fs::DirEntry>, Error> {
    let mut entries = fs::read_dir(path)?.collect::<io::Result<Vec<_>>>()?;

    entries.sort_by_key(|entry| entry.file_name());

    Ok(entries)
}
