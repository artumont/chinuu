//! The JSON shapes a frontend receives.
//!
//! These are worth pinning down, because a rename of a field or a change of tag
//! style is a breaking change for the TypeScript side and nothing else would
//! catch it. Gated on the feature, so they run with
//! `cargo test -p chinuu-core --features serde`.

#![cfg(feature = "serde")]

use std::path::PathBuf;

use chinuu_core::{
    files::{index_directory, write_file, WatchEvent, WatchEventKind, WatchFailure, WatchUpdate},
    git::{ChangeKind, CommitInfo, FileStatus, PullOutcome, RepoStatus},
    CoreError,
};
use serde_json::json;

// ------------------------------------------------------------------ the vault

#[test]
fn a_vault_index_serializes_with_a_kind_tag() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path().join("a.md"), "# a\n").unwrap();
    write_file(dir.path().join("notes/b.md"), "# b\n").unwrap();

    let index = index_directory(dir.path()).unwrap();
    let value = serde_json::to_value(&index).unwrap();

    let root = &value["root_node"];
    assert_eq!(root["kind"], "folder");
    assert_eq!(root["id"], "", "the root is the empty id");
    assert!(root["path"].is_string(), "the on-disk path is included");

    let children = root["children"].as_array().unwrap();
    assert_eq!(children.len(), 2, "got {children:#?}");

    let file = children.iter().find(|c| c["id"] == "a.md").unwrap();
    assert_eq!(file["kind"], "file");
    assert_eq!(file["name"], "a.md");
    assert!(file["size"].is_number());
    assert!(
        file.get("children").is_none(),
        "a file must not carry a children key: {file:#?}"
    );

    let folder = children.iter().find(|c| c["id"] == "notes").unwrap();
    assert_eq!(folder["kind"], "folder");
    assert_eq!(folder["children"].as_array().unwrap().len(), 1);

    // The flat map is there too, keyed by id.
    assert_eq!(value["flat_index"]["notes/b.md"]["kind"], "file");
}

// ------------------------------------------------------------------ watching

#[test]
fn a_watch_event_serializes_as_a_plain_object() {
    let event = WatchEvent {
        id: "notes/a.md".to_owned(),
        kind: WatchEventKind::Renamed,
        from_id: Some("notes/old.md".to_owned()),
    };

    assert_eq!(
        serde_json::to_value(&event).unwrap(),
        json!({
            "id": "notes/a.md",
            "kind": "renamed",
            "from_id": "notes/old.md",
        })
    );
}

#[test]
fn a_watch_event_without_a_previous_id_serializes_null() {
    let event = WatchEvent {
        id: "a.md".to_owned(),
        kind: WatchEventKind::Created,
        from_id: None,
    };

    assert_eq!(
        serde_json::to_value(&event).unwrap()["from_id"],
        json!(null)
    );
}

#[test]
fn a_watch_update_is_adjacently_tagged() {
    let changed = WatchUpdate::Changed(vec![WatchEvent {
        id: "a.md".to_owned(),
        kind: WatchEventKind::Modified,
        from_id: None,
    }]);
    let value = serde_json::to_value(&changed).unwrap();
    assert_eq!(value["kind"], "changed");
    assert_eq!(value["data"][0]["id"], "a.md");

    let failed = WatchUpdate::Failed(WatchFailure {
        messages: vec!["inotify watch limit reached".to_owned()],
    });
    let value = serde_json::to_value(&failed).unwrap();
    assert_eq!(value["kind"], "failed");
    assert_eq!(value["data"]["messages"][0], "inotify watch limit reached");
}

#[test]
fn watch_event_kind_serializes_like_its_as_str() {
    for kind in [
        WatchEventKind::Created,
        WatchEventKind::Modified,
        WatchEventKind::Removed,
        WatchEventKind::Renamed,
    ] {
        assert_eq!(
            serde_json::to_value(kind).unwrap(),
            json!(kind.as_str()),
            "serde and as_str disagree for {kind:?}"
        );
    }
}

// ------------------------------------------------------------------ git shapes

#[test]
fn change_kind_serializes_like_its_as_str() {
    for kind in [
        ChangeKind::Added,
        ChangeKind::Modified,
        ChangeKind::Deleted,
        ChangeKind::Renamed,
        ChangeKind::TypeChange,
        ChangeKind::Untracked,
        ChangeKind::Conflicted,
    ] {
        assert_eq!(
            serde_json::to_value(kind).unwrap(),
            json!(kind.as_str()),
            "serde and as_str disagree for {kind:?}"
        );
    }
}

#[test]
fn repo_status_serializes_its_entries() {
    let status = RepoStatus {
        branch: Some("main".to_owned()),
        detached: false,
        entries: vec![FileStatus {
            id: "a.md".to_owned(),
            staged: Some(ChangeKind::Modified),
            unstaged: None,
        }],
        ahead: 2,
        behind: 1,
    };

    assert_eq!(
        serde_json::to_value(&status).unwrap(),
        json!({
            "branch": "main",
            "detached": false,
            "entries": [{ "id": "a.md", "staged": "modified", "unstaged": null }],
            "ahead": 2,
            "behind": 1,
        })
    );
}

#[test]
fn commit_info_serializes_its_fields() {
    let commit = CommitInfo {
        id: "abc123".to_owned(),
        summary: "first".to_owned(),
        message: "first\n\nbody".to_owned(),
        author_name: "Ada".to_owned(),
        author_email: "ada@example.com".to_owned(),
        time: 1_700_000_000,
        offset_minutes: 60,
        parents: vec!["def456".to_owned()],
    };

    let value = serde_json::to_value(&commit).unwrap();
    assert_eq!(value["summary"], "first");
    assert_eq!(value["time"], 1_700_000_000);
    assert_eq!(value["parents"][0], "def456");
}

#[test]
fn outcomes_serialize_in_snake_case() {
    assert_eq!(
        serde_json::to_value(PullOutcome::FastForward).unwrap(),
        json!("fast_forward")
    );
    assert_eq!(
        serde_json::to_value(PullOutcome::UpToDate).unwrap(),
        json!("up_to_date")
    );
    assert_eq!(
        serde_json::to_value(chinuu_core::git::PushOutcome::Pushed).unwrap(),
        json!("pushed")
    );
}

// ------------------------------------------------------------------- errors

#[test]
fn an_error_serializes_with_a_code_message_and_path() {
    let error = CoreError::NotAFile(PathBuf::from("/vault/notes"));

    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        json!({
            "code": "not-a-file",
            "message": "path is not a file: /vault/notes",
            "path": "/vault/notes",
        })
    );
}

#[test]
fn an_error_without_a_path_serializes_null() {
    let value = serde_json::to_value(CoreError::Conflicts).unwrap();

    assert_eq!(value["code"], "conflicts");
    assert_eq!(value["path"], json!(null));
}
