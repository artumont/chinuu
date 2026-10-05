use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone)]
pub enum FsNode {
    File {
        id: String,
        name: String,
        size: u64,
        path: PathBuf,
    },
    Folder {
        id: String,
        name: String,
        children: Vec<FsNode>, // Support subfolders
        size: u64,
        path: PathBuf,
    },
}

pub struct FsIndex {
    // Store direct lookup pointers to speed up search
    pub root_node: FsNode,
    pub flat_index: HashMap<String, FsNode>,
}
