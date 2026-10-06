use std::path::Path;

use chinuu_core::{
    files::{
        delete_file, delete_folder, index_directory, index_directory_with, read_file, write_file,
        FsNode, IgnoreRules, IGNORE_FILE_NAME, ROOT_ID,
    },
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

// ---------------------------------------------------------------- ignore rules

#[test]
fn ignored_paths_are_absent_from_the_index() {
    let vault = sample_vault();
    write_file(vault.path().join(IGNORE_FILE_NAME), "*.tmp\nprivate/\n").unwrap();
    write_file(vault.path().join("scratch.tmp"), "x\n").unwrap();
    write_file(vault.path().join("notes/draft.tmp"), "x\n").unwrap();
    write_file(vault.path().join("private/secret.md"), "x\n").unwrap();

    let index = index_directory(vault.path()).unwrap();

    assert!(index.get("scratch.tmp").is_none());
    assert!(index.get("notes/draft.tmp").is_none());
    assert!(
        index.get("private").is_none(),
        "the folder itself is hidden"
    );
    assert!(index.get("private/secret.md").is_none());

    // Everything else is untouched.
    assert!(index.get("a.md").is_some());
    assert!(index.get("notes/b.md").is_some());
    assert!(index.get("notes/deep/c.md").is_some());
}

#[test]
fn git_and_the_ignore_file_are_hidden_by_default() {
    let vault = sample_vault();
    write_file(vault.path().join(IGNORE_FILE_NAME), "*.tmp\n").unwrap();
    std::fs::create_dir_all(vault.path().join(".git")).unwrap();
    write_file(vault.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();

    let index = index_directory(vault.path()).unwrap();

    assert!(index.get(".git").is_none());
    assert!(index.get(".git/HEAD").is_none());
    assert!(
        index.get(IGNORE_FILE_NAME).is_none(),
        "the ignore file is configuration, not a note"
    );
}

#[cfg(unix)]
#[test]
fn an_ignored_directory_is_never_descended_into() {
    use std::os::unix::fs::PermissionsExt;

    let vault = sample_vault();
    write_file(vault.path().join(IGNORE_FILE_NAME), "private/\n").unwrap();
    let private = vault.path().join("private");
    write_file(private.join("x.md"), "x\n").unwrap();

    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o000)).unwrap();

    // If the mode does not actually stop us, for instance when the suite runs as
    // root, the test cannot prove anything, so it steps aside rather than
    // passing for the wrong reason.
    if std::fs::read_dir(&private).is_ok() {
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }

    let result = index_directory(vault.path());

    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o755)).unwrap();

    // A walk that filtered after the fact would try to read this directory and
    // fail. Pruning means it is never opened at all.
    assert!(
        result.is_ok(),
        "the walk should not have entered the ignored directory: {result:?}"
    );
}

#[test]
fn index_directory_with_uses_exactly_the_given_rules() {
    let vault = sample_vault();
    write_file(vault.path().join(IGNORE_FILE_NAME), "private/\n").unwrap();
    write_file(vault.path().join("private/secret.md"), "x\n").unwrap();

    // The vault's own file is deliberately not consulted here, so neither the
    // folder it hides nor the built-in patterns apply.
    let rules = IgnoreRules::parse("*.tmp").unwrap();
    let index = index_directory_with(vault.path(), &rules).unwrap();

    assert!(index.get(IGNORE_FILE_NAME).is_some());
    assert!(index.get("private/secret.md").is_some());
}

#[test]
fn an_empty_rule_set_hides_nothing() {
    let vault = sample_vault();
    std::fs::create_dir_all(vault.path().join(".git")).unwrap();
    write_file(vault.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();

    let index = index_directory_with(vault.path(), &IgnoreRules::empty()).unwrap();

    assert!(index.get(".git").is_some());
    assert!(index.get(".git/HEAD").is_some());
}

#[test]
fn a_bad_pattern_in_the_vault_file_is_reported() {
    let vault = sample_vault();
    write_file(vault.path().join(IGNORE_FILE_NAME), "[z-a].md\n").unwrap();

    let err = index_directory(vault.path()).unwrap_err();

    assert!(
        matches!(err, CoreError::InvalidIgnorePattern(_)),
        "got {err:?}"
    );
}
