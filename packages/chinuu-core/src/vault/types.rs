use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsNode {
    File {
        id: String,
        size: u64,         // Size of the file used to allocate buffers
        name: String,      // The file name without the file type
        path: PathBuf,     // Relative to Index root
        file_type: String, // The type of the determined file (.*)
    },
    Folder {
        id: String,
        name: String,          // Just the folder name
        path: PathBuf,         // Relative to index root
        children: Vec<FsNode>, // The children of the folder (FsNode::File or FsNode::Folder)
    },
}

pub struct FsIndex {
    root_path: PathBuf,
    flat_index: HashMap<String, FsNode>,
}

pub struct Vault {
    index: FsIndex,
}
