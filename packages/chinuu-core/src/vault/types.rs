use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsNode {
    File {
        id: String,
        size: u64,              // Size of the file used to allocate buffers
        name: String,           // The file name without the file type
        path: impl AsRef<Path>, // Relative to Index root
        file_type: String,      // The type of the determined file (.*)
    },
    Folder {
        id: String,
        name: String,           // Just the folder name
        path: impl AsRef<Path>, // Relative to index root
        children: Vec<FsNode>,  // The children of the folder (FsNode::File or FsNode::Folder)
    },
}

pub struct FsIndex {
    root_path: impl AsRef<Path>,
    flat_index: impl HashMap<String, FsNode>,
}

pub struct Vault {
    index: FsIndex,
}
