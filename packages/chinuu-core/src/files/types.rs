use std::{collections::HashMap, path::PathBuf};

/// Identifier of the vault root node.
///
/// Ids are vault-relative paths rendered with `/` separators, so the root has
/// the empty relative path. Keeping the separator platform-independent means a
/// vault indexed on Linux and opened on Windows produces identical ids, which
/// the git layer depends on.
pub const ROOT_ID: &str = "";

/// A single entry in the vault tree.
///
/// `id` is the vault-relative path (see [`ROOT_ID`]). It is stable across
/// sessions and machines, unlike an inode number, which is reused by the
/// filesystem and only unique within one mounted device.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        children: Vec<FsNode>,
        size: u64,
        path: PathBuf,
    },
}

impl FsNode {
    pub fn id(&self) -> &str {
        match self {
            FsNode::File { id, .. } | FsNode::Folder { id, .. } => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            FsNode::File { name, .. } | FsNode::Folder { name, .. } => name,
        }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            FsNode::File { path, .. } | FsNode::Folder { path, .. } => path,
        }
    }

    /// Size reported by the filesystem for this entry.
    ///
    /// For folders this is the directory record size, not the recursive total.
    pub fn size(&self) -> u64 {
        match self {
            FsNode::File { size, .. } | FsNode::Folder { size, .. } => *size,
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, FsNode::Folder { .. })
    }

    pub fn children(&self) -> &[FsNode] {
        match self {
            FsNode::Folder { children, .. } => children,
            FsNode::File { .. } => &[],
        }
    }
}

/// Result of indexing a vault.
#[derive(Debug, Clone)]
pub struct FsIndex {
    pub root_node: FsNode,
    /// `id` -> node, for direct lookup without walking the tree.
    ///
    /// Cost note: a folder value embeds its whole subtree, so indexing the
    /// tree this way duplicates descendants once per ancestor folder. Total
    /// memory is roughly O(nodes * depth), not O(nodes). Fine for shallow
    /// vaults; revisit with `Rc` or index handles if deep trees appear.
    pub flat_index: HashMap<String, FsNode>,
}

impl FsIndex {
    /// Look up a node by its vault-relative path id.
    pub fn get(&self, id: &str) -> Option<&FsNode> {
        self.flat_index.get(id)
    }

    /// Number of indexed nodes, root included.
    pub fn len(&self) -> usize {
        self.flat_index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.flat_index.is_empty()
    }
}
