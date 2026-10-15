use std::{
    collections::HashMap,
    fs::{self, File},
    io,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::vault::{error::VaultError, ignore::IgnoreRules, vault::Vault};

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
    root_node: FsNode,
    flat_index: HashMap<String, FsNode>,
}

impl FsNode {
    /// Get current node id.
    pub fn id(&self) -> &str {
        match self {
            FsNode::File { id, .. } | FsNode::Folder { id, .. } => id,
        }
    }

    pub fn path(&self) -> PathBuf {
        match self {
            FsNode::File { path, .. } | FsNode::Folder { path, .. } => path.clone(),
        }
    }

    /// Get a folder's children nodes.
    pub fn children(&self) -> &[FsNode] {
        match self {
            FsNode::Folder { children, .. } => children,
            FsNode::File { .. } => &[],
        }
    }

    /// Delete the current node from filesystem and FsIndex.
    pub fn delete(&self) -> Result<(), VaultError> {
        match self {
            FsNode::Folder { path, .. } => {
                fs::remove_dir_all(path).map_err(|e| VaultError::io(path, e))?;
            }
            FsNode::File { path, .. } => {
                fs::remove_file(path).map_err(|e| VaultError::io(path, e))?;
            }
        }

        Ok(())
    }

    pub fn new_file(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<Self, VaultError> {
        let root_ref = root.as_ref();
        let path_ref = path.as_ref();

        let name = path_ref
            .file_name()
            .map(|os_str| os_str.to_string_lossy().into_owned())
            .unwrap_or_default();

        let rel_path = path_ref
            .strip_prefix(root_ref)
            .map_err(|_| VaultError::strip_prefix(root_ref, path_ref))?;

        let metadata =
            fs::metadata(path_ref).map_err(|e| VaultError::io(path_ref.to_path_buf(), e))?;

        let uuid = generate_node_id(&rel_path);

        Ok(FsNode::File {
            id: uuid,
            name,
            size: metadata.len(),
            file_type: path_ref
                .extension()
                .and_then(|s| s.to_str())
                .map(String::from),
            path: rel_path.to_path_buf(),
        })
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
            root_node,
            flat_index,
        })
    }

    /// Get a file or folder via UUIDv5.
    pub fn find(&self, id: String) -> Result<&FsNode, VaultError> {
        match self.flat_index.get(&id) {
            Some(fs_node) => Ok(fs_node),
            None => Err(VaultError::NodeIdDoesNotExist(id)),
        }
    }

    /// Create a new file and index it.
    pub fn create_file(&self, id: Option<String>, filename: String) -> Result<FsNode, VaultError> {
        let node_path: PathBuf = if let Some(actual_id) = id {
            self.find(actual_id)?.path()
        } else {
            self.root_node.path()
        };

        if !node_path.is_dir() {
            return Err(VaultError::NotADirectory(node_path));
        }

        let node = FsNode::new_file(self.root_node.path(), node_path)?;
        Ok(node)
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
        .map_err(|_| VaultError::strip_prefix(rel, abs))?;
    let uuid = generate_node_id(rel_path);

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

fn generate_node_id(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    Uuid::new_v5(&UUID_NAMESPACE, path.to_string_lossy().as_bytes()).to_string()
}
