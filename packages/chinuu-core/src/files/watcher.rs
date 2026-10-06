use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use notify::{
    event::{ModifyKind, RenameMode},
    EventKind, RecursiveMode,
};
use notify_debouncer_full::{
    new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache,
};

use crate::{
    error::{CoreError, Result},
    files::types::{id_from_path, join_rel},
};

/// Default quiet period before a batch is delivered.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(200);

/// What happened to a path inside the vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchEventKind {
    Created,
    Modified,
    Removed,
    /// A rename or move. The new id is [`WatchEvent::id`], the old one is
    /// [`WatchEvent::from_id`].
    Renamed,
}

impl WatchEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            WatchEventKind::Created => "created",
            WatchEventKind::Modified => "modified",
            WatchEventKind::Removed => "removed",
            WatchEventKind::Renamed => "renamed",
        }
    }
}

/// One coalesced change inside the vault.
///
/// `id` is a vault-relative path, identical in form to [`FsNode`]'s ids:
/// `/` separated, so it matches what the index and the git layer use.
///
/// [`FsNode`]: crate::files::FsNode
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchEvent {
    /// Vault-relative id of the path that changed.
    pub id: String,
    pub kind: WatchEventKind,
    /// Previous id, set only for [`WatchEventKind::Renamed`].
    pub from_id: Option<String>,
}

/// Why the watcher stopped being reliable.
///
/// This is the "your tree is now stale" signal. Notify reports problems such as
/// exhausted watch descriptors here because they happen long after the watch was
/// established, so they cannot be returned from [`VaultWatcher::start`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchFailure {
    pub messages: Vec<String>,
}

impl std::fmt::Display for WatchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.messages.join("; "))
    }
}

/// One delivery to the watcher callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchUpdate {
    /// The listed paths changed. Never empty, and never contains two entries
    /// for the same id.
    Changed(Vec<WatchEvent>),
    /// The watcher itself failed. Treat the tree as stale and reindex.
    Failed(WatchFailure),
}

impl WatchUpdate {
    pub fn events(&self) -> &[WatchEvent] {
        match self {
            WatchUpdate::Changed(events) => events,
            WatchUpdate::Failed(_) => &[],
        }
    }

    pub fn failure(&self) -> Option<&WatchFailure> {
        match self {
            WatchUpdate::Changed(_) => None,
            WatchUpdate::Failed(failure) => Some(failure),
        }
    }
}

/// How to watch a vault.
#[derive(Debug, Clone)]
pub struct WatchOptions {
    /// How long the filesystem must stay quiet before a batch is delivered.
    ///
    /// This is a quiet-period debounce: continuous writes keep pushing the
    /// delivery back until they stop. That is what stops a large edit from
    /// arriving as a flicker of partial states.
    pub debounce: Duration,
    /// Watch subdirectories too. On by default; a flat vault can turn it off to
    /// save one kernel watch per directory.
    pub recursive: bool,
    /// Vault-relative ids to ignore, matched on the id itself or as a parent
    /// directory.
    ///
    /// Defaults to `.git`, which matters because our own git operations churn
    /// it and would otherwise feed our changes straight back to us.
    pub ignore: Vec<String>,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            debounce: DEFAULT_DEBOUNCE,
            recursive: true,
            ignore: vec![".git".to_owned()],
        }
    }
}

impl WatchOptions {
    /// Ignore additional vault-relative ids.
    pub fn ignoring(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.ignore.extend(ids.into_iter().map(Into::into));
        self
    }

    /// Set the quiet period.
    pub fn with_debounce(mut self, debounce: Duration) -> Self {
        self.debounce = debounce;
        self
    }

    /// Watch only the top level of the vault.
    pub fn shallow(mut self) -> Self {
        self.recursive = false;
        self
    }
}

/// A running watch. Dropping it stops watching.
pub struct VaultWatcher {
    /// Held only to keep the watch alive; dropping it stops the debouncer
    /// thread. Never read.
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
    root: PathBuf,
    options: WatchOptions,
}

impl std::fmt::Debug for VaultWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VaultWatcher")
            .field("root", &self.root)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl VaultWatcher {
    /// Watch `root`, delivering debounced batches to `callback`.
    ///
    /// The callback runs on the watcher's thread, once per quiet period that
    /// saw at least one change.
    pub fn start<F>(root: impl AsRef<Path>, callback: F) -> Result<Self>
    where
        F: FnMut(WatchUpdate) + Send + 'static,
    {
        Self::start_with(root, WatchOptions::default(), callback)
    }

    /// Watch `root` with explicit options.
    pub fn start_with<F>(
        root: impl AsRef<Path>,
        options: WatchOptions,
        mut callback: F,
    ) -> Result<Self>
    where
        F: FnMut(WatchUpdate) + Send + 'static,
    {
        let root = root.as_ref();
        if !root.is_dir() {
            return Err(CoreError::NotADirectory(root.to_path_buf()));
        }

        // Canonicalise so that event paths share a prefix with `root` even when
        // the caller passed a path through a symlink, which is the usual case
        // for a temporary directory on macOS.
        let root = root.canonicalize().map_err(|e| CoreError::io(root, e))?;

        let watch_root = root.clone();
        let filter_root = root.clone();
        let filter_options = options.clone();

        let mut debouncer = new_debouncer(
            options.debounce,
            None,
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        let batch = coalesce(&filter_root, &filter_options, &events);
                        // A quiet period that produced nothing worth reporting says
                        // nothing, rather than waking the UI with an empty batch.
                        if !batch.is_empty() {
                            callback(WatchUpdate::Changed(batch));
                        }
                    }
                    Err(errors) => callback(WatchUpdate::Failed(WatchFailure {
                        messages: errors.iter().map(ToString::to_string).collect(),
                    })),
                }
            },
        )?;

        let mode = if options.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        debouncer.watch(watch_root.as_path(), mode)?;

        Ok(Self {
            _debouncer: debouncer,
            root,
            options,
        })
    }

    /// The canonical root being watched.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn options(&self) -> &WatchOptions {
        &self.options
    }

    pub fn is_recursive(&self) -> bool {
        self.options.recursive
    }
}

/// Accumulates what happened to one path inside a single quiet window.
///
/// Last-wins would be wrong here. Saving a new file emits a create and then a
/// data change, and folding that into a modification loses the fact that the UI
/// has to insert a row rather than update one.
#[derive(Default)]
struct PendingEvent {
    kind: Option<WatchEventKind>,
    from_id: Option<String>,
    /// Set once this path was seen created, or renamed into place.
    is_new: bool,
}

impl PendingEvent {
    fn apply(&mut self, kind: WatchEventKind, from_id: Option<String>) {
        match kind {
            // A removal stands until a later create supersedes it.
            WatchEventKind::Removed => {
                self.is_new = false;
                self.kind = Some(WatchEventKind::Removed);
                self.from_id = None;
            }
            WatchEventKind::Created => {
                self.is_new = true;
                self.kind = Some(WatchEventKind::Created);
                self.from_id = None;
            }
            WatchEventKind::Renamed => {
                self.is_new = true;
                self.kind = Some(WatchEventKind::Renamed);
                self.from_id = from_id;
            }
            // An edit to a path that appeared in this same window is still a new
            // path as far as the UI is concerned.
            WatchEventKind::Modified => {
                if !self.is_new {
                    self.kind = Some(WatchEventKind::Modified);
                    self.from_id = None;
                }
            }
        }
    }

    fn into_event(self, id: String) -> Option<WatchEvent> {
        self.kind.map(|kind| WatchEvent {
            id,
            kind,
            from_id: self.from_id,
        })
    }
}

/// Turn a debounced burst into one event per changed id.
///
/// Folded through a `BTreeMap`, so the result is sorted by id and holds no
/// duplicates.
fn coalesce(root: &Path, options: &WatchOptions, events: &[DebouncedEvent]) -> Vec<WatchEvent> {
    let mut merged: BTreeMap<String, PendingEvent> = BTreeMap::new();

    for debounced in events {
        let Some(kind) = kind_of(debounced.event.kind) else {
            continue;
        };

        // A correlated rename carries both paths, (from, to) in that order.
        if kind == WatchEventKind::Renamed {
            let ids = ids_for(root, options, &debounced.event.paths);
            if let [from, to] = ids.as_slice() {
                merged
                    .entry(to.clone())
                    .or_default()
                    .apply(kind, Some(from.clone()));
                continue;
            }
        }

        for id in ids_for(root, options, &debounced.event.paths) {
            merged.entry(id).or_default().apply(kind, None);
        }
    }

    if options.recursive {
        let created: Vec<String> = merged
            .iter()
            .filter(|(_, pending)| pending.kind == Some(WatchEventKind::Created))
            .map(|(id, _)| id.clone())
            .filter(|id| root.join(id).is_dir())
            .collect();

        for id in created {
            scan_new_directory(root, options, &id, &mut merged);
        }
    }

    merged
        .into_iter()
        .filter_map(|(id, pending)| pending.into_event(id))
        .collect()
}

/// Fold a newly created directory's contents into the batch.
///
/// A recursive watch only starts covering a directory once the kernel has
/// reported it, so a directory created and then filled immediately can lose the
/// events for its contents. That is not hypothetical: unpacking a folder of
/// notes, or a checkout landing a directory tree, does exactly that. Walking the
/// new directory closes the window.
///
/// Only meaningful for a recursive watch; a shallow one deliberately reports
/// nothing below the top level.
fn scan_new_directory(
    root: &Path,
    options: &WatchOptions,
    id: &str,
    merged: &mut BTreeMap<String, PendingEvent>,
) {
    let Ok(entries) = fs::read_dir(root.join(id)) else {
        // Gone again already, which the batch's removal event will cover.
        return;
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let child_id = join_rel(id, &name);
        if is_ignored(options, &child_id) {
            continue;
        }

        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        merged
            .entry(child_id.clone())
            .or_default()
            .apply(WatchEventKind::Created, None);

        if is_dir {
            scan_new_directory(root, options, &child_id, merged);
        }
    }
}

/// Map an event's paths to vault ids, dropping anything filtered out.
fn ids_for(root: &Path, options: &WatchOptions, paths: &[PathBuf]) -> Vec<String> {
    let mut ids = Vec::new();

    for path in paths {
        let Some(id) = id_from_path(root, path) else {
            continue;
        };

        // An event on the root itself says nothing that a child event does not,
        // and the empty id is not a path the UI can act on.
        if id.is_empty() || is_ignored(options, &id) || ids.contains(&id) {
            continue;
        }

        ids.push(id);
    }

    ids
}

/// Whether `id` is one of the ignored ids, or sits underneath one.
fn is_ignored(options: &WatchOptions, id: &str) -> bool {
    options.ignore.iter().any(|prefix| {
        id.strip_prefix(prefix.as_str())
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

/// Group notify's event kinds into the four things a vault UI acts on.
fn kind_of(kind: EventKind) -> Option<WatchEventKind> {
    match kind {
        EventKind::Create(_) => Some(WatchEventKind::Created),
        EventKind::Remove(_) => Some(WatchEventKind::Removed),

        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => Some(WatchEventKind::Renamed),
        // A rename whose other half could not be found: report the two sides
        // honestly rather than inventing a pairing.
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => Some(WatchEventKind::Removed),
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => Some(WatchEventKind::Created),
        EventKind::Modify(_) => Some(WatchEventKind::Modified),

        // Opening or reading a file is not a change, and reporting it would
        // make the UI churn every time a note is displayed.
        EventKind::Access(_) => None,

        // Unknown events are reported as changes rather than dropped, so a
        // platform that classifies imprecisely does not go silent.
        EventKind::Any | EventKind::Other => Some(WatchEventKind::Modified),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> WatchOptions {
        WatchOptions::default()
    }

    #[test]
    fn path_builders_are_chainable() {
        let opts = WatchOptions::default()
            .with_debounce(Duration::from_millis(50))
            .ignoring([".obsidian", "node_modules"])
            .shallow();

        assert_eq!(opts.debounce, Duration::from_millis(50));
        assert!(!opts.recursive);
        assert!(opts.ignore.contains(&".git".to_owned()));
        assert!(opts.ignore.contains(&".obsidian".to_owned()));
    }

    #[test]
    fn ignore_matches_the_id_and_its_children() {
        let opts = options();

        assert!(is_ignored(&opts, ".git"));
        assert!(is_ignored(&opts, ".git/config"));
        assert!(is_ignored(&opts, ".git/objects/ab/cdef"));

        assert!(!is_ignored(&opts, "notes/.gitignore"));
        assert!(!is_ignored(&opts, ".github/workflows"));
        assert!(!is_ignored(&opts, "a.md"));
    }

    #[test]
    fn access_events_are_dropped() {
        assert_eq!(
            kind_of(EventKind::Access(notify::event::AccessKind::Any)),
            None
        );
    }

    #[test]
    fn create_and_remove_map_to_their_kinds() {
        assert_eq!(
            kind_of(EventKind::Create(notify::event::CreateKind::File)),
            Some(WatchEventKind::Created)
        );
        assert_eq!(
            kind_of(EventKind::Remove(notify::event::RemoveKind::File)),
            Some(WatchEventKind::Removed)
        );
    }

    #[test]
    fn rename_modes_map_to_rename_remove_and_create() {
        use notify::event::RenameMode;

        assert_eq!(
            kind_of(EventKind::Modify(ModifyKind::Name(RenameMode::Both))),
            Some(WatchEventKind::Renamed)
        );
        assert_eq!(
            kind_of(EventKind::Modify(ModifyKind::Name(RenameMode::From))),
            Some(WatchEventKind::Removed)
        );
        assert_eq!(
            kind_of(EventKind::Modify(ModifyKind::Name(RenameMode::To))),
            Some(WatchEventKind::Created)
        );
    }

    #[test]
    fn event_kind_renames_are_named_in_snake_case() {
        assert_eq!(WatchEventKind::Created.as_str(), "created");
        assert_eq!(WatchEventKind::Renamed.as_str(), "renamed");
    }

    #[test]
    fn ids_for_skips_the_root_and_ignored_paths() {
        let root = Path::new("/vault");
        let opts = options();
        let paths = vec![
            PathBuf::from("/vault"),
            PathBuf::from("/vault/.git/index"),
            PathBuf::from("/vault/notes/a.md"),
            PathBuf::from("/elsewhere/outside.md"),
        ];

        assert_eq!(ids_for(root, &opts, &paths), vec!["notes/a.md"]);
    }
}
