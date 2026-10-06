use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};

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
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    // Internally tagged, so the frontend reads `{"kind":"file","id":...}`
    // rather than the default `{"File":{...}}`.
    serde(tag = "kind", rename_all = "lowercase")
)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
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

/// Join a vault-relative parent id with a child name, using `/`.
///
/// The single authority for building ids, shared by the indexer and the watcher
/// so the two cannot drift apart.
pub(crate) fn join_rel(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

/// The vault id for an absolute path inside the vault.
///
/// Returns `None` when `path` is outside `root`, and `Some(ROOT_ID)` when it is
/// the root itself. Separators are normalised to `/`, matching [`join_rel`], so
/// an id produced here is the same as one produced by the indexer.
pub(crate) fn id_from_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut id = String::new();

    for component in relative.components() {
        if let Component::Normal(part) = component {
            if !id.is_empty() {
                id.push('/');
            }
            id.push_str(&part.to_string_lossy());
        }
    }

    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_from_path_is_relative_and_uses_forward_slashes() {
        let root = Path::new("/vault");

        assert_eq!(id_from_path(root, Path::new("/vault")).as_deref(), Some(""));
        assert_eq!(
            id_from_path(root, Path::new("/vault/a.md")).as_deref(),
            Some("a.md")
        );
        assert_eq!(
            id_from_path(root, Path::new("/vault/notes/deep/b.md")).as_deref(),
            Some("notes/deep/b.md")
        );
    }

    #[test]
    fn id_from_path_rejects_paths_outside_the_root() {
        let root = Path::new("/vault");

        assert_eq!(id_from_path(root, Path::new("/elsewhere/a.md")), None);
        // A sibling whose name merely starts with the root's name is outside.
        assert_eq!(id_from_path(root, Path::new("/vault-other/a.md")), None);
    }

    #[test]
    fn id_from_path_agrees_with_join_rel() {
        let root = Path::new("/vault");

        let joined = join_rel(&join_rel(ROOT_ID, "notes"), "b.md");
        let from_path = id_from_path(root, Path::new("/vault/notes/b.md")).unwrap();

        assert_eq!(joined, from_path);
    }
}
