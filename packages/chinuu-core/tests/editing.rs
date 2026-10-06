//! The contract the editor depends on.
//!
//! `packages/editor` hands text to and from the host as a plain `string`, and
//! its README claims the file on disk "stays plain markdown" and that the text is
//! "the text in the file, byte for byte, at all times". Core cannot enforce that
//! claim, but it can avoid being the thing that breaks it, and that is what these
//! tests pin: reading and writing must not normalise, translate or decorate the
//! bytes in between.
//!
//! The one normalisation that does exist is on the editor's side, not here:
//! CodeMirror splits a document on `/\r\n?|\n/` and rejoins with `"\n"` unless the
//! `lineSeparator` facet is set, so a CRLF file comes back as LF through the
//! editor. That is the app's decision to make; core keeps the bytes it was given.

use std::{fs, path::Path, path::PathBuf};

use chinuu_core::{
    files::{
        create_folder, move_path, read_bytes, read_file, write_bytes, write_file, TEMP_PREFIX,
    },
    CoreError,
};

/// Payloads chosen for the ways text can be changed without anyone noticing.
const ROUND_TRIP_CASES: &[(&str, &str)] = &[
    ("no trailing newline", "a"),
    ("one trailing newline", "a\n"),
    ("two trailing newlines", "a\n\n"),
    ("empty", ""),
    ("only a newline", "\n"),
    ("crlf", "a\r\nb\r\n"),
    ("lone carriage return", "a\rb"),
    ("mixed line endings", "a\nb\r\nc\rd"),
    ("trailing spaces", "a   \n"),
    ("tabs", "a\tb\n"),
    ("byte order mark", "\u{feff}# a\n"),
    ("unicode", "héllo → 世界 🎉\n"),
    ("combining marks", "e\u{301}\n"),
    ("internal nul", "a\0b\n"),
    ("markdown that looks like markup", "**a** _b_ `c` | d |\n"),
    ("a very long line", "x\n"),
];

/// Any file left behind by the atomic write machinery.
fn temp_leftovers(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(TEMP_PREFIX) {
                out.push(path);
            } else if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                walk(&path, out);
            }
        }
    }

    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

// ------------------------------------------------------------- round tripping

#[test]
fn reading_then_writing_leaves_the_file_identical() {
    for (label, content) in ROUND_TRIP_CASES {
        let dir = tempfile::tempdir().unwrap();
        let note = dir.path().join("note.md");

        write_file(&note, content).unwrap();
        let read_back = read_file(&note).unwrap();
        assert_eq!(&read_back, content, "{label}: read changed the text");

        write_file(&note, &read_back).unwrap();
        let bytes = fs::read(&note).unwrap();
        assert_eq!(
            bytes,
            content.as_bytes(),
            "{label}: the second write changed the bytes"
        );
    }
}

#[test]
fn writing_does_not_add_a_trailing_newline() {
    let dir = tempfile::tempdir().unwrap();
    let note = dir.path().join("note.md");

    write_file(&note, "no newline here").unwrap();

    assert_eq!(fs::read(&note).unwrap(), b"no newline here");
}

#[test]
fn carriage_returns_survive_a_read_and_write() {
    let dir = tempfile::tempdir().unwrap();
    let note = dir.path().join("crlf.md");

    write_file(&note, "a\r\nb\r\n").unwrap();

    assert_eq!(read_file(&note).unwrap(), "a\r\nb\r\n");
    assert_eq!(fs::read(&note).unwrap(), b"a\r\nb\r\n");
}

#[test]
fn a_byte_order_mark_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let note = dir.path().join("bom.md");

    write_file(&note, "\u{feff}# a\n").unwrap();

    // Stripping it would silently change the file, so it is left alone.
    assert!(read_file(&note).unwrap().starts_with('\u{feff}'));
    assert_eq!(fs::read(&note).unwrap(), "\u{feff}# a\n".as_bytes());
}

#[test]
fn an_empty_file_stays_empty() {
    let dir = tempfile::tempdir().unwrap();
    let note = dir.path().join("empty.md");

    write_file(&note, "").unwrap();

    assert_eq!(fs::read(&note).unwrap(), b"");
    assert_eq!(read_file(&note).unwrap(), "");
}

#[test]
fn overwriting_replaces_the_content_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let note = dir.path().join("note.md");

    write_file(&note, "a much longer original line of text\n").unwrap();
    write_file(&note, "short\n").unwrap();

    // A truncate-then-write would leave tail bytes behind if it were wrong.
    assert_eq!(fs::read(&note).unwrap(), b"short\n");
    assert!(temp_leftovers(dir.path()).is_empty());
}

// ------------------------------------------------------------------- binaries

#[test]
fn read_bytes_round_trips_binary_content() {
    let dir = tempfile::tempdir().unwrap();
    let blob = dir.path().join("img.png");
    let content: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x00, 0xff, 0xfe, 0x0d, 0x0a];

    write_bytes(&blob, &content).unwrap();

    assert_eq!(read_bytes(&blob).unwrap(), content);
    assert_eq!(fs::read(&blob).unwrap(), content);
}

#[test]
fn binary_content_is_reported_as_not_utf8() {
    let dir = tempfile::tempdir().unwrap();
    let blob = dir.path().join("img.png");
    write_bytes(&blob, &[0xff, 0xfe, 0x00]).unwrap();

    let err = read_file(&blob).unwrap_err();

    // Distinct from an io failure: the file is fine, it is just not text.
    assert!(matches!(err, CoreError::NotUtf8(_)), "got {err:?}");
    assert_eq!(err.code(), "not-utf8");
    assert_eq!(err.path(), Some(blob.as_path()));
    // And the bytes are still reachable.
    assert_eq!(read_bytes(&blob).unwrap(), vec![0xff, 0xfe, 0x00]);
}

#[test]
fn reading_a_directory_as_text_is_refused() {
    let dir = tempfile::tempdir().unwrap();

    let err = read_file(dir.path()).unwrap_err();

    assert!(matches!(err, CoreError::NotAFile(_)), "got {err:?}");
}

// ------------------------------------------------------------------ the swap

#[test]
fn a_write_leaves_no_temporary_file_behind() {
    let dir = tempfile::tempdir().unwrap();
    let note = dir.path().join("notes/deep/note.md");

    write_file(&note, "first\n").unwrap();
    write_file(&note, "second\n").unwrap();
    write_file(dir.path().join("other.md"), "third\n").unwrap();

    assert!(
        temp_leftovers(dir.path()).is_empty(),
        "left behind {:?}",
        temp_leftovers(dir.path())
    );
}

#[cfg(unix)]
#[test]
fn a_write_keeps_the_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let note = dir.path().join("note.md");
    write_file(&note, "one\n").unwrap();

    for mode in [0o600, 0o644, 0o664] {
        fs::set_permissions(&note, fs::Permissions::from_mode(mode)).unwrap();
        write_file(&note, "two\n").unwrap();

        let after = fs::metadata(&note).unwrap().permissions().mode() & 0o777;
        assert_eq!(after, mode, "mode changed to {after:o}");
    }
}

#[cfg(unix)]
#[test]
fn a_write_through_a_symlink_keeps_the_link() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real.md");
    let link = dir.path().join("link.md");
    write_file(&real, "before\n").unwrap();
    symlink(&real, &link).unwrap();

    write_file(&link, "after\n").unwrap();

    // Replacing the link would have turned it into an ordinary file, which is
    // the opposite of what saving through a link means.
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the link was replaced"
    );
    assert_eq!(read_file(&real).unwrap(), "after\n");
    assert_eq!(read_file(&link).unwrap(), "after\n");
}

// ---------------------------------------------------------- other path changes

#[test]
fn move_path_renames_a_note() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("old.md");
    let to = dir.path().join("notes/new.md");
    write_file(&from, "content\n").unwrap();

    move_path(&from, &to).unwrap();

    assert!(!from.exists());
    assert_eq!(read_file(&to).unwrap(), "content\n");
}

#[test]
fn move_path_moves_a_folder() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path().join("notes/deep/a.md"), "a\n").unwrap();

    move_path(dir.path().join("notes"), dir.path().join("archive/notes")).unwrap();

    assert!(!dir.path().join("notes").exists());
    assert_eq!(
        read_file(dir.path().join("archive/notes/deep/a.md")).unwrap(),
        "a\n"
    );
}

#[test]
fn move_path_refuses_to_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("a.md");
    let to = dir.path().join("b.md");
    write_file(&from, "a\n").unwrap();
    write_file(&to, "b\n").unwrap();

    let err = move_path(&from, &to).unwrap_err();

    assert!(matches!(err, CoreError::AlreadyExists(_)), "got {err:?}");
    // Neither file was touched, so nothing was lost.
    assert_eq!(read_file(&from).unwrap(), "a\n");
    assert_eq!(read_file(&to).unwrap(), "b\n");
}

#[test]
fn move_path_reports_a_missing_source() {
    let dir = tempfile::tempdir().unwrap();

    let err = move_path(dir.path().join("gone.md"), dir.path().join("new.md")).unwrap_err();

    assert_eq!(err.code(), "io");
    assert_eq!(err.io_kind(), Some(std::io::ErrorKind::NotFound));
    assert_eq!(err.path(), Some(dir.path().join("gone.md").as_path()));
}

#[test]
fn create_folder_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("notes/deep");

    create_folder(&folder).unwrap();
    create_folder(&folder).unwrap();

    assert!(folder.is_dir());
}

#[test]
fn create_folder_refuses_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.md");
    write_file(&file, "a\n").unwrap();

    let err = create_folder(&file).unwrap_err();

    assert!(matches!(err, CoreError::NotADirectory(_)), "got {err:?}");
    assert_eq!(read_file(&file).unwrap(), "a\n");
}

// ---------------------------------------------------------------- integration

#[test]
fn a_renamed_note_takes_its_new_id_in_the_index() {
    use chinuu_core::files::index_directory;

    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path().join("draft.md"), "# draft\n").unwrap();

    move_path(
        dir.path().join("draft.md"),
        dir.path().join("notes/final.md"),
    )
    .unwrap();

    let index = index_directory(dir.path()).unwrap();
    assert!(index.get("draft.md").is_none());
    assert!(index.get("notes/final.md").is_some());
}
