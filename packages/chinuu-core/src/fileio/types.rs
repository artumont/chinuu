use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use uuid::{uuid, Uuid};

/// Chinuu UUIDv5 namespace ("chinuu")
pub const UUID_NAMESPACE: Uuid = uuid!("8926cb79-a26c-44e4-a19f-878000dc3d0c");

/// Represents a file found within a filesystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: String, // UUIDv5 based on abs path
    pub name: String,
    pub path: PathBuf, // abs path
    pub size: u64,
}

/// Represents a folder found within a filesystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String, // UUIDv5 based on abs path
    pub name: String,
    pub path: PathBuf, // abs path
    pub children: Vec<File>,
}

/// Common type for both folder and file used in `flat_index`.
pub enum FsNode {
    File(File),
    Folder(Folder),
}

/// Represents a filesystem tree.
pub struct FsIndex {
    pub root_path: PathBuf,                  // abs path
    pub flat_index: HashMap<String, FsNode>, // id, File or Folder
}
