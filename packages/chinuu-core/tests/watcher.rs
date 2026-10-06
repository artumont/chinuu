//! Integration tests for the vault watcher.
//!
//! These watch a real temporary directory and wait for real inotify events, so
//! they are timing-dependent by nature. Each test uses its own directory, a
//! short debounce, and a generous ceiling on waiting, so a slow machine slows
//! them down rather than breaking them.

use std::{
    fs,
    path::Path,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

use chinuu_core::{
    files::{VaultWatcher, WatchEvent, WatchEventKind, WatchOptions, WatchUpdate},
    CoreError,
};

/// Debounce used by the tests. Short, so the suite stays quick.
const DEBOUNCE: Duration = Duration::from_millis(50);

/// Upper bound on waiting for an expected event. Generous on purpose.
const PATIENCE: Duration = Duration::from_secs(10);

/// Start a watcher, handing back a receiver the callback feeds.
fn watch(root: &Path) -> (VaultWatcher, Receiver<WatchUpdate>) {
    watch_with(root, WatchOptions::default())
}

fn watch_with(root: &Path, options: WatchOptions) -> (VaultWatcher, Receiver<WatchUpdate>) {
    let (tx, rx) = mpsc::channel();
    let watcher = VaultWatcher::start_with(root, options.with_debounce(DEBOUNCE), move |update| {
        let _ = tx.send(update);
    })
    .expect("watcher should start");

    (watcher, rx)
}

/// Gather changed events until `done` is satisfied, or the patience runs out.
fn wait_for(
    rx: &Receiver<WatchUpdate>,
    what: &str,
    mut done: impl FnMut(&[WatchEvent]) -> bool,
) -> Vec<WatchEvent> {
    let deadline = Instant::now() + PATIENCE;
    let mut collected: Vec<WatchEvent> = Vec::new();

    while Instant::now() < deadline {
        let remaining = deadline - Instant::now();
        match rx.recv_timeout(remaining) {
            Ok(WatchUpdate::Changed(events)) => {
                collected.extend(events);
                if done(&collected) {
                    return collected;
                }
            }
            Ok(WatchUpdate::Failed(failure)) => panic!("watcher reported failure: {failure}"),
            Err(err) => panic!("timed out waiting for {what}: {err}; saw {collected:?}"),
        }
    }

    panic!("timed out waiting for {what}; saw {collected:?}");
}

/// Assert that nothing arrives within a window comfortably past the debounce.
fn expect_no_events(rx: &Receiver<WatchUpdate>, window: Duration) {
    match rx.recv_timeout(window) {
        Ok(WatchUpdate::Changed(events)) => panic!("expected no events, but got {events:?}"),
        Ok(WatchUpdate::Failed(failure)) => panic!("watcher reported failure: {failure}"),
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("watcher stopped unexpectedly"),
    }
}

fn find<'a>(events: &'a [WatchEvent], id: &str) -> Option<&'a WatchEvent> {
    events.iter().find(|event| event.id == id)
}

// ------------------------------------------------------------------- lifecycle

#[test]
fn start_rejects_a_directory_that_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope");

    let err = VaultWatcher::start(&missing, |_| {}).unwrap_err();
    assert!(matches!(err, CoreError::NotADirectory(_)), "got {err:?}");
}

#[test]
fn start_rejects_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.md");
    fs::write(&file, "# a\n").unwrap();

    let err = VaultWatcher::start(&file, |_| {}).unwrap_err();
    assert!(matches!(err, CoreError::NotADirectory(_)), "got {err:?}");
}

#[test]
fn root_is_absolute_and_options_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let (watcher, _rx) = watch(dir.path());

    assert!(watcher.root().is_absolute());
    assert!(watcher.root().is_dir());
    assert!(watcher.is_recursive());
    assert_eq!(watcher.options().debounce, DEBOUNCE);
}

#[test]
fn dropping_the_watcher_stops_delivery() {
    let dir = tempfile::tempdir().unwrap();
    let (watcher, rx) = watch(dir.path());
    drop(watcher);

    fs::write(dir.path().join("a.md"), "# a\n").unwrap();

    // The sender is dropped once the debouncer thread stops, so the receiver
    // disconnects instead of hanging.
    match rx.recv_timeout(Duration::from_secs(5)) {
        Err(mpsc::RecvTimeoutError::Disconnected) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("watcher thread kept sending after the handle was dropped")
        }
        Ok(update) => panic!("expected silence after drop, got {update:?}"),
    }
}

// --------------------------------------------------------------------- events

#[test]
fn creating_a_file_is_reported_as_created() {
    let dir = tempfile::tempdir().unwrap();
    let (_watcher, rx) = watch(dir.path());

    fs::write(dir.path().join("a.md"), "# a\n").unwrap();

    let events = wait_for(&rx, "a.md to be created", |events| {
        find(events, "a.md").is_some()
    });

    let event = find(&events, "a.md").unwrap();
    // A write emits a create followed by a data change; the created kind wins,
    // because the path is new and that is what the UI needs to know.
    assert_eq!(event.kind, WatchEventKind::Created);
    assert_eq!(event.from_id, None);
}

#[test]
fn editing_an_existing_file_is_reported_as_modified() {
    let dir = tempfile::tempdir().unwrap();
    // Created before watching, so this is an edit rather than a create.
    fs::write(dir.path().join("a.md"), "# a\n").unwrap();
    let (_watcher, rx) = watch(dir.path());

    fs::write(dir.path().join("a.md"), "# a edited\n").unwrap();

    let events = wait_for(&rx, "a.md to be modified", |events| {
        find(events, "a.md").is_some()
    });

    assert_eq!(
        find(&events, "a.md").unwrap().kind,
        WatchEventKind::Modified
    );
}

#[test]
fn deleting_a_file_is_reported_as_removed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.md"), "# a\n").unwrap();
    let (_watcher, rx) = watch(dir.path());

    fs::remove_file(dir.path().join("a.md")).unwrap();

    let events = wait_for(&rx, "a.md to be removed", |events| {
        find(events, "a.md").is_some()
    });

    assert_eq!(find(&events, "a.md").unwrap().kind, WatchEventKind::Removed);
}

#[test]
fn nested_paths_use_vault_relative_ids() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("notes/deep")).unwrap();
    let (_watcher, rx) = watch(dir.path());

    fs::write(dir.path().join("notes/deep/b.md"), "# b\n").unwrap();

    let events = wait_for(&rx, "notes/deep/b.md to appear", |events| {
        find(events, "notes/deep/b.md").is_some()
    });

    let event = find(&events, "notes/deep/b.md").unwrap();
    assert_eq!(event.kind, WatchEventKind::Created);
    // Ids never carry the root prefix and never use a backslash.
    assert!(!event.id.contains('\\'));
    assert!(!event.id.starts_with('/'));
}

#[test]
fn a_batch_carries_one_entry_per_path() {
    let dir = tempfile::tempdir().unwrap();
    let (_watcher, rx) = watch(dir.path());

    // Several writes to the same path inside one quiet window.
    fs::write(dir.path().join("a.md"), "one\n").unwrap();
    fs::write(dir.path().join("a.md"), "two\n").unwrap();
    fs::write(dir.path().join("a.md"), "three\n").unwrap();

    let events = wait_for(&rx, "a.md", |events| find(events, "a.md").is_some());

    let matches = events.iter().filter(|e| e.id == "a.md").count();
    assert_eq!(matches, 1, "expected one entry per id, got {events:?}");
}

#[test]
fn a_rename_is_reported_with_its_previous_id() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.md"), "# a\n").unwrap();
    let (_watcher, rx) = watch(dir.path());

    fs::rename(dir.path().join("a.md"), dir.path().join("b.md")).unwrap();

    let events = wait_for(&rx, "a rename to arrive", |events| {
        events
            .iter()
            .any(|e| e.kind == WatchEventKind::Renamed && e.id == "b.md")
    });

    let renamed = events
        .iter()
        .find(|e| e.kind == WatchEventKind::Renamed && e.id == "b.md")
        .expect("a correlated rename");

    assert_eq!(renamed.from_id.as_deref(), Some("a.md"));
}

#[test]
fn a_new_subdirectory_is_watched() {
    let dir = tempfile::tempdir().unwrap();
    let (_watcher, rx) = watch(dir.path());

    // The directory is created after the watch starts, so this also checks that
    // a recursive watch picks up newly created subdirectories.
    fs::create_dir_all(dir.path().join("fresh")).unwrap();
    fs::write(dir.path().join("fresh/c.md"), "# c\n").unwrap();

    let events = wait_for(&rx, "fresh/c.md to appear", |events| {
        find(events, "fresh/c.md").is_some()
    });

    assert_eq!(
        find(&events, "fresh/c.md").unwrap().kind,
        WatchEventKind::Created
    );
}

// -------------------------------------------------------------------- filters

#[test]
fn git_is_ignored_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let (_watcher, rx) = watch(dir.path());

    fs::create_dir_all(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();

    expect_no_events(&rx, DEBOUNCE * 8);
}

#[test]
fn a_custom_ignored_directory_is_respected() {
    let dir = tempfile::tempdir().unwrap();
    let (_watcher, rx) = watch_with(dir.path(), WatchOptions::default().ignoring([".obsidian"]));

    fs::create_dir_all(dir.path().join(".obsidian")).unwrap();
    fs::write(dir.path().join(".obsidian/app.json"), "{}\n").unwrap();

    expect_no_events(&rx, DEBOUNCE * 8);
}

#[test]
fn a_similar_name_is_not_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let (_watcher, rx) = watch(dir.path());

    // Only the exact id `.git` and its children are ignored.
    fs::create_dir_all(dir.path().join(".github/workflows")).unwrap();
    fs::write(dir.path().join(".github/workflows/ci.yml"), "on: push\n").unwrap();

    let events = wait_for(&rx, "the .github path", |events| {
        events.iter().any(|e| e.id.starts_with(".github"))
    });

    assert!(
        events.iter().any(|e| e.id == ".github/workflows/ci.yml"),
        "got {events:?}"
    );
}

#[test]
fn a_shallow_watch_ignores_subdirectories() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("notes")).unwrap();
    let (_watcher, rx) = watch_with(dir.path(), WatchOptions::default().shallow());

    fs::write(dir.path().join("notes/a.md"), "# a\n").unwrap();

    expect_no_events(&rx, DEBOUNCE * 8);
}

#[test]
fn a_shallow_watch_still_reports_top_level_paths() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("notes")).unwrap();
    let (_watcher, rx) = watch_with(dir.path(), WatchOptions::default().shallow());

    fs::write(dir.path().join("a.md"), "# a\n").unwrap();

    let events = wait_for(&rx, "a.md", |events| find(events, "a.md").is_some());
    assert_eq!(find(&events, "a.md").unwrap().kind, WatchEventKind::Created);
}
