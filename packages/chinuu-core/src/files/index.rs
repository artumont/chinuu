use std::{
    collections::HashMap,
    env::{self},
    fs, io,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use crate::files::types::{FsIndex, FsNode};

/// Index the current working directory
pub fn index_cwd() -> Result<FsIndex, io::Error> {
    let current_dir = env::current_dir().unwrap();
    index_directory(current_dir)
}

/// Index the especified directory
pub fn index_directory(directory: PathBuf) -> Result<FsIndex, io::Error> {
    let root_node = build_tree_from_disk(&directory)?;
    let mut flat_index = HashMap::new();

    create_index_from_disk(&root_node, &mut flat_index);

    Ok(FsIndex {
        root_node: root_node,
        flat_index: flat_index,
    })
}

/// Create final HashMap index from the FsNode tree
fn create_index_from_disk(node: &FsNode, index: &mut HashMap<String, FsNode>) {
    match node {
        FsNode::Folder {
            id,
            children,
            name: _,
            size: _,
            path: _,
        } => {
            index.insert(id.clone(), node.clone());

            for node in children {
                create_index_from_disk(node, index);
            }
        }
        FsNode::File {
            id,
            name: _,
            size: _,
            path: _,
        } => {
            index.insert(id.clone(), node.clone());
        }
    }
}

/// Recursively get the dir tree from disk
///
/// Current implementation is Linux only due to the size
/// and the ino function a fix would be to use another crate
/// to get the id and to use `metadata.len()` instead of size
fn build_tree_from_disk(path: &Path) -> Result<FsNode, io::Error> {
    let metadata = fs::metadata(path)?;
    let name = path
        .file_name()
        .map(|os_str| os_str.to_string_lossy().into_owned())
        .unwrap_or_else(|| "".to_string());

    if path.is_dir() {
        let mut children_nodes = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?; // Unwraps the DirEntry result safely
            let entry_path = entry.path();
            let child_node = build_tree_from_disk(&entry_path)?;
            children_nodes.push(child_node);
        }

        return Ok(FsNode::Folder {
            id: metadata.ino().to_string(),
            name: name,
            children: children_nodes,
            size: metadata.size(),
            path: path.to_path_buf(),
        });
    } else {
        return Ok(FsNode::File {
            id: metadata.ino().to_string(),
            name: name,
            size: metadata.size(),
            path: path.to_path_buf(),
        });
    }
}
