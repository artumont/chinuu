use std::path::Path;

use chinuu_core::{
    files::{index_directory, read_file, write_file, FsNode, ROOT_ID},
    CoreError,
};

/// Build a small vault on disk and return the tempdir guard.
fn sample_vault() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write_file(root.join("a.md"), "# a\n").unwrap();
    write_file(root.join("notes/b.md"), "# b\n").unwrap();
    write_file(root.join("notes/deep/c.md"), "# c\n").unwrap();

    dir
}

fn ids_of(node: &FsNode, out: &mut Vec<String>) {
    out.push(node.id().to_owned());
    for child in node.children() {
        ids_of(child, out);
    }
}

#[test]
fn ids_are_vault_relative_paths() {
    let vault = sample_vault();
    let index = index_directory(vault.path()).unwrap();

    let mut ids = Vec::new();
    ids_of(&index.root_node, &mut ids);
    ids.sort();

    assert_eq!(
        ids,
        vec![
            "",
            "a.md",
            "notes",
            "notes/b.md",
            "notes/deep",
            "notes/deep/c.md",
        ]
    );
}

#[test]
fn root_node_uses_root_id() {
    let vault = sample_vault();
    let index = index_directory(vault.path()).unwrap();

    assert_eq!(index.root_node.id(), ROOT_ID);
    assert!(index.root_node.is_folder());
}

#[test]
fn flat_index_resolves_by_relative_id() {
    let vault = sample_vault();
    let index = index_directory(vault.path()).unwrap();

    let file = index
        .get("notes/deep/c.md")
        .expect("c.md should be indexed");
    assert!(matches!(file, FsNode::File { .. }));
    assert_eq!(file.name(), "c.md");
    assert_eq!(file.path(), &vault.path().join("notes/deep/c.md"));

    let folder = index.get("notes").expect("notes should be indexed");
    assert!(folder.is_folder());
    assert_eq!(folder.children().len(), 2);
}

#[test]
fn index_has_one_entry_per_node() {
    let vault = sample_vault();
    let index = index_directory(vault.path()).unwrap();

    // root, a.md, notes, notes/b.md, notes/deep, notes/deep/c.md
    assert_eq!(index.len(), 6);
}

#[test]
fn children_are_sorted_by_name() {
    let vault = sample_vault();
    let index = index_directory(vault.path()).unwrap();

    let names: Vec<_> = index
        .root_node
        .children()
        .iter()
        .map(|n| n.name().to_owned())
        .collect();

    assert_eq!(names, vec!["a.md", "notes"]);
}

#[test]
fn ids_do_not_contain_backslashes() {
    let vault = sample_vault();
    let index = index_directory(vault.path()).unwrap();

    for id in index.flat_index.keys() {
        assert!(!id.contains('\\'), "id should use forward slashes: {id}");
    }
}

#[test]
fn write_file_creates_missing_parents() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("x/y/z.md");

    write_file(&target, "hello").unwrap();

    assert_eq!(read_file(&target).unwrap(), "hello");
}

#[test]
fn read_file_rejects_directory() {
    let dir = tempfile::tempdir().unwrap();

    let err = read_file(dir.path()).unwrap_err();
    assert!(matches!(err, CoreError::NotAFile(_)));
}

#[test]
fn write_file_rejects_directory() {
    let dir = tempfile::tempdir().unwrap();

    let err = write_file(dir.path(), "nope").unwrap_err();
    assert!(matches!(err, CoreError::NotAFile(_)));
}

#[test]
fn index_directory_rejects_a_file() {
    let vault = sample_vault();
    let err = index_directory(vault.path().join("a.md")).unwrap_err();

    assert!(matches!(err, CoreError::NotADirectory(_)));
}

#[test]
fn io_errors_carry_the_path() {
    let missing = Path::new("/definitely/not/here/missing.md");

    let err = read_file(missing).unwrap_err();
    assert!(matches!(err, CoreError::NotAFile(_)));
}
