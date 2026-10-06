use std::path::Path;

use chinuu_core::{
    files::{delete_file, delete_folder, index_directory, read_file, write_file, FsNode, ROOT_ID},
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

// --------------------------------------------------------------------- delete

#[test]
fn delete_file_removes_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("a.md");
    write_file(&target, "# a\n").unwrap();

    delete_file(&target).unwrap();

    assert!(!target.exists());
    assert!(read_file(&target).is_err());
}

#[test]
fn delete_file_keeps_its_siblings() {
    let vault = sample_vault();

    delete_file(vault.path().join("notes/b.md")).unwrap();

    assert!(!vault.path().join("notes/b.md").exists());
    assert!(vault.path().join("notes/deep/c.md").exists());
    assert!(vault.path().join("a.md").exists());
}

#[test]
fn delete_file_rejects_a_directory() {
    let vault = sample_vault();

    let err = delete_file(vault.path().join("notes")).unwrap_err();

    assert!(matches!(err, CoreError::NotAFile(_)), "got {err:?}");
    assert!(
        vault.path().join("notes").is_dir(),
        "the folder must survive"
    );
}

#[test]
fn delete_file_reports_a_missing_path() {
    let dir = tempfile::tempdir().unwrap();

    let err = delete_file(dir.path().join("gone.md")).unwrap_err();

    assert!(matches!(err, CoreError::Io { .. }), "got {err:?}");
}

#[test]
fn delete_folder_removes_the_whole_tree() {
    let vault = sample_vault();

    delete_folder(vault.path().join("notes")).unwrap();

    assert!(!vault.path().join("notes").exists());
    assert!(vault.path().join("a.md").exists());
}

#[test]
fn delete_folder_rejects_a_file() {
    let vault = sample_vault();

    let err = delete_folder(vault.path().join("a.md")).unwrap_err();

    assert!(matches!(err, CoreError::NotADirectory(_)), "got {err:?}");
    assert!(vault.path().join("a.md").exists(), "the file must survive");
}

#[test]
fn delete_folder_reports_a_missing_path() {
    let dir = tempfile::tempdir().unwrap();

    let err = delete_folder(dir.path().join("gone")).unwrap_err();

    assert!(matches!(err, CoreError::Io { .. }), "got {err:?}");
}

#[test]
fn a_deleted_file_disappears_from_a_reindex() {
    let vault = sample_vault();
    assert!(index_directory(vault.path())
        .unwrap()
        .get("notes/b.md")
        .is_some());

    delete_file(vault.path().join("notes/b.md")).unwrap();

    let index = index_directory(vault.path()).unwrap();
    assert!(index.get("notes/b.md").is_none());
    // The folder and its other contents are untouched.
    assert!(index.get("notes").is_some());
    assert!(index.get("notes/deep/c.md").is_some());
}

#[cfg(unix)]
#[test]
fn delete_file_unlinks_a_symlink_and_spares_its_target() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real.md");
    let link = dir.path().join("link.md");
    write_file(&target, "# real\n").unwrap();
    symlink(&target, &link).unwrap();

    delete_file(&link).unwrap();

    // exists() follows the link, so it would still report true here.
    assert!(link.symlink_metadata().is_err(), "the link is gone");
    assert!(target.is_file(), "the target survives");
    assert_eq!(read_file(&target).unwrap(), "# real\n");
}

#[cfg(unix)]
#[test]
fn delete_file_unlinks_a_symlink_to_a_directory() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real");
    let link = dir.path().join("link");

    std::fs::create_dir_all(&target).unwrap();
    write_file(target.join("inner.md"), "# inner\n").unwrap();
    symlink(&target, &link).unwrap();

    // A link is not a directory, so this unlinks the link rather than refusing.
    delete_file(&link).unwrap();

    assert!(link.symlink_metadata().is_err(), "the link is gone");
    assert!(
        target.join("inner.md").is_file(),
        "the target folder survives"
    );
}

#[cfg(unix)]
#[test]
fn delete_folder_refuses_a_symlink_to_a_directory() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real");
    let link = dir.path().join("link");

    std::fs::create_dir_all(&target).unwrap();
    write_file(target.join("inner.md"), "# inner\n").unwrap();
    symlink(&target, &link).unwrap();

    let err = delete_folder(&link).unwrap_err();

    assert!(matches!(err, CoreError::NotADirectory(_)), "got {err:?}");
    assert!(
        target.join("inner.md").is_file(),
        "the target folder survives"
    );
}

#[cfg(unix)]
#[test]
fn delete_folder_does_not_follow_a_symlink_inside_it() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside");
    let vault = dir.path().join("vault");

    std::fs::create_dir_all(&outside).unwrap();
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    write_file(outside.join("keep.md"), "# keep\n").unwrap();
    symlink(&outside, vault.join("notes/escape")).unwrap();

    delete_folder(vault.join("notes")).unwrap();

    assert!(!vault.join("notes").exists());
    assert!(
        outside.join("keep.md").is_file(),
        "a link inside the folder must not take its target with it"
    );
}
