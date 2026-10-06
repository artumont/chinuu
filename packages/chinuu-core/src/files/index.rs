use std::{collections::HashMap, env, fs, io, path::Path};

use crate::{
    error::{CoreError, Result},
    files::types::{FsIndex, FsNode, ROOT_ID},
};

/// Index the current working directory.
pub fn index_cwd() -> Result<FsIndex> {
    let current_dir = env::current_dir()?;
    index_directory(current_dir)
}

/// Index the given directory as a vault root.
pub fn index_directory(directory: impl AsRef<Path>) -> Result<FsIndex> {
    let directory = directory.as_ref();

    if !directory.is_dir() {
        return Err(CoreError::NotADirectory(directory.to_path_buf()));
    }

    let root_node = build_tree(directory, ROOT_ID)?;
    let mut flat_index = HashMap::new();

    flatten(&root_node, &mut flat_index);

    Ok(FsIndex {
        root_node,
        flat_index,
    })
}

/// Insert every node of the in-memory tree into the lookup map.
fn flatten(node: &FsNode, index: &mut HashMap<String, FsNode>) {
    index.insert(node.id().to_owned(), node.clone());

    for child in node.children() {
        flatten(child, index);
    }
}

/// Recursively read the directory tree from disk.
///
/// `abs` is the on-disk path, `rel` is the vault-relative id being built for
/// the same entry. Ids are joined with `/` regardless of platform so a vault
/// indexed on one OS resolves to the same ids on another.
fn build_tree(abs: &Path, rel: &str) -> Result<FsNode> {
    let metadata = fs::metadata(abs).map_err(|e| CoreError::io(abs, e))?;
    let name = abs
        .file_name()
        .map(|os_str| os_str.to_string_lossy().into_owned())
        .unwrap_or_default();

    if metadata.is_dir() {
        let mut children_nodes = Vec::new();

        for entry in read_dir_sorted(abs)? {
            let entry_path = entry.path();
            let child_name = entry.file_name().to_string_lossy().into_owned();
            let child_rel = join_rel(rel, &child_name);
            children_nodes.push(build_tree(&entry_path, &child_rel)?);
        }

        Ok(FsNode::Folder {
            id: rel.to_owned(),
            name,
            children: children_nodes,
            size: metadata.len(),
            path: abs.to_path_buf(),
        })
    } else {
        Ok(FsNode::File {
            id: rel.to_owned(),
            name,
            size: metadata.len(),
            path: abs.to_path_buf(),
        })
    }
}

/// Join a vault-relative parent id with a child name using `/`.
fn join_rel(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

/// Read a directory, sorted by file name for a deterministic tree.
fn read_dir_sorted(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(path)
        .map_err(|e| CoreError::io(path, e))?
        .collect::<io::Result<Vec<_>>>()
        .map_err(|e| CoreError::io(path, e))?;

    entries.sort_by_key(|entry| entry.file_name());

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_rel_uses_forward_slashes() {
        assert_eq!(join_rel(ROOT_ID, "notes"), "notes");
        assert_eq!(join_rel("notes", "a.md"), "notes/a.md");
        assert_eq!(join_rel("notes/a", "b.md"), "notes/a/b.md");
    }

    #[test]
    fn missing_directory_is_reported() {
        let err = index_directory("/definitely/not/here/xyzzy").unwrap_err();
        assert!(matches!(err, CoreError::NotADirectory(_)));
    }
}
