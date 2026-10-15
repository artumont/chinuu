use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::vault::{error::VaultError, ignore::IgnoreRules};

const UUID_NAMESPACE: Uuid = Uuid::NAMESPACE_DNS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsNode {
    File {
        id: String,
        size: u64,                 // Size of the file used to allocate buffers
        name: String,              // The file name without the file type
        path: PathBuf,             // Relative to Index root
        file_type: Option<String>, // The type of the determined file (.*)
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

impl FsNode {
    /// Get current node id.
    pub fn id(&self) -> &str {
        match self {
            FsNode::File { id, .. } | FsNode::Folder { id, .. } => id,
        }
    }

    /// Get a folder's children nodes.
    pub fn children(&self) -> &[FsNode] {
        match self {
            FsNode::Folder { children, .. } => children,
            FsNode::File { .. } => &[],
        }
    }
}

impl FsIndex {
    /// Index a new directory.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, VaultError> {
        let path = path.as_ref();

        if !path.is_dir() {
            return Err(VaultError::NotADirectory(path.to_path_buf()));
        }

        let rules = IgnoreRules::build(path)?;

        let root_node = build_tree(path, path, &rules)?;
        let mut flat_index = HashMap::new();

        flatten(&root_node, &mut flat_index);

        Ok(FsIndex {
            root_path: path.to_path_buf(),
            flat_index,
        })
    }

    /// Get a file or folder via UUIDv5.
    pub fn find(&self, id: String) -> Option<FsNode> {
        self.flat_index.get(&id).cloned()
    }
}

fn flatten(node: &FsNode, index: &mut HashMap<String, FsNode>) {
    index.insert(node.id().to_owned(), node.clone());

    for child in node.children() {
        flatten(child, index);
    }
}

fn build_tree(abs: &Path, rel: &Path, rules: &IgnoreRules) -> Result<FsNode, VaultError> {
    let metadata = fs::metadata(abs).map_err(|e| VaultError::io(abs, e))?;
    let name = abs
        .file_name()
        .map(|os_str| os_str.to_string_lossy().into_owned())
        .unwrap_or_default();
    let rel_path = abs
        .strip_prefix(rel)
        .map_err(|e| VaultError::InvalidIgnorePattern(e.to_string()))?;
    let uuid = Uuid::new_v5(&UUID_NAMESPACE, rel_path.to_string_lossy().as_bytes());

    if metadata.is_dir() {
        let mut children_nodes = Vec::new();

        for entry in read_dir_sorted(abs)? {
            let entry_path = entry.path();
            let entry_rel = entry_path.strip_prefix(rel).unwrap_or(&entry_path);

            if rules.is_ignored(entry_rel, entry_path.is_dir()) {
                continue;
            }

            children_nodes.push(build_tree(&entry_path, rel, rules)?);
        }

        Ok(FsNode::Folder {
            id: uuid.to_string(),
            name,
            path: rel_path.to_path_buf(),
            children: children_nodes,
        })
    } else {
        Ok(FsNode::File {
            id: uuid.to_string(),
            size: metadata.len(),
            name,
            path: rel_path.to_path_buf(),
            file_type: abs.extension().and_then(|s| s.to_str()).map(String::from),
        })
    }
}

fn read_dir_sorted(path: &Path) -> Result<Vec<fs::DirEntry>, VaultError> {
    let mut entries = fs::read_dir(path)
        .map_err(|e| VaultError::io(path, e))?
        .collect::<io::Result<Vec<_>>>()
        .map_err(|e| VaultError::io(path, e))?;

    entries.sort_by_key(|entry| entry.file_name());

    Ok(entries)
}
