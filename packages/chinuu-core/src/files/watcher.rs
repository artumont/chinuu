use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use notify::{
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
    EventKind, RecursiveMode,
};
use notify_debouncer_full::{
    new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache,
};

use crate::{
    error::{CoreError, Result},
    files::{
        ignore::IgnoreRules,
        types::{id_from_path, join_rel},
    },
};

/// Default quiet period before a batch is delivered.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(200);

/// What happened to a path inside the vault.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    // Matches `WatchEventKind::as_str`, so the string a frontend compares
    // against is the same one the Rust side reports.
    serde(rename_all = "lowercase")
)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
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
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    // Adjacently tagged: `{"kind":"changed","data":[...]}`. An internally
    // tagged enum cannot hold a newtype variant like `Changed(Vec<_>)`.
    serde(tag = "kind", content = "data", rename_all = "lowercase")
)]
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
    /// Rules deciding which vault paths are hidden.
    ///
    /// The same rules the indexer uses, so a path hidden from the tree cannot
    /// still arrive as an event. Defaults to [`IgnoreRules::defaults`], which
    /// covers `.git` and the ignore file itself.
    pub rules: IgnoreRules,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            debounce: DEFAULT_DEBOUNCE,
            recursive: true,
            rules: IgnoreRules::defaults(),
        }
    }
}

impl WatchOptions {
    /// Ignore additional patterns, in gitignore syntax.
    ///
    /// An error is returned for a pattern that will not compile, rather than
    /// dropping it and leaving a caller to wonder why a path still shows up.
    pub fn ignoring<I, S>(mut self, patterns: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.rules.add_patterns(patterns)?;
        Ok(self)
    }

    /// Replace the rules outright, for instance with [`IgnoreRules::for_vault`].
    pub fn with_rules(mut self, rules: IgnoreRules) -> Self {
        self.rules = rules;
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
    ///
    /// Honours the vault's own `.chinuuignore` on top of the built-in defaults,
    /// the same way [`index_directory`] does. Use
    /// [`VaultWatcher::start_with`] to supply the rules yourself.
    ///
    /// [`index_directory`]: crate::files::index_directory
    pub fn start<F>(root: impl AsRef<Path>, callback: F) -> Result<Self>
    where
        F: FnMut(WatchUpdate) + Send + 'static,
    {
        let root = root.as_ref();

        // Checked before reading the ignore file, so a file path reports
        // `NotADirectory` rather than an io error from looking for a file inside
        // it.
        if !root.is_dir() {
            return Err(CoreError::NotADirectory(root.to_path_buf()));
        }

        let options = WatchOptions::default().with_rules(IgnoreRules::for_vault(root)?);
        Self::start_with(root, options, callback)
    }

    /// Watch `root` with explicit options, using exactly the rules they carry.
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
        let event_kind = debounced.event.kind;
        let Some(kind) = kind_of(event_kind) else {
            continue;
        };
        let hint = dir_hint(event_kind);

        // A correlated rename carries both paths, (from, to) in that order.
        if kind == WatchEventKind::Renamed {
            let ids = ids_for(root, options, &debounced.event.paths, hint);
            if let [from, to] = ids.as_slice() {
                merged
                    .entry(to.clone())
                    .or_default()
                    .apply(kind, Some(from.clone()));
                continue;
            }

            // One half was filtered out, which is the normal shape of an atomic
            // save: the temporary file is ignored, so the rename has nothing to
            // pair with. Report only what the half we can see supports, rather
            // than claiming a rename whose origin the UI never knew about.
            let to_id = debounced
                .event
                .paths
                .get(1)
                .and_then(|path| visible_id(root, options, path, hint));
            let from_id = debounced
                .event
                .paths
                .first()
                .and_then(|path| visible_id(root, options, path, hint));

            if let Some(id) = to_id {
                merged
                    .entry(id)
                    .or_default()
                    .apply(WatchEventKind::Created, None);
            } else if let Some(id) = from_id {
                merged
                    .entry(id)
                    .or_default()
                    .apply(WatchEventKind::Removed, None);
            }
            continue;
        }

        for id in ids_for(root, options, &debounced.event.paths, hint) {
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
        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);

        // Checked before the entry is reported, so an ignored directory is never
        // announced and is never descended into.
        if options.rules.is_ignored(&child_id, is_dir) {
            continue;
        }

        merged
            .entry(child_id.clone())
            .or_default()
            .apply(WatchEventKind::Created, None);

        if is_dir {
            scan_new_directory(root, options, &child_id, merged);
        }
    }
}

/// Resolve one event path to a visible vault id.
///
/// `None` when the path is outside the vault, is the vault root itself, or is
/// hidden by the ignore rules. `dir_hint` is what the event said the path was,
/// when it said anything.
fn visible_id(
    root: &Path,
    options: &WatchOptions,
    path: &Path,
    dir_hint: Option<bool>,
) -> Option<String> {
    let id = id_from_path(root, path)?;

    // An event on the root says nothing that a child event does not, and the
    // empty id is not a path the UI can act on.
    if id.is_empty() {
        return None;
    }

    // The filesystem answers whether the path is a directory, except for a
    // removal: the path is already gone there, so only the event knows whether
    // a `dir/` pattern should have matched it.
    let is_dir = dir_hint.unwrap_or_else(|| root.join(&id).is_dir());
    if options.rules.is_ignored(&id, is_dir) {
        return None;
    }

    Some(id)
}

/// Map an event's paths to vault ids, dropping anything filtered out.
fn ids_for(
    root: &Path,
    options: &WatchOptions,
    paths: &[PathBuf],
    dir_hint: Option<bool>,
) -> Vec<String> {
    let mut ids = Vec::new();

    for path in paths {
        if let Some(id) = visible_id(root, options, path, dir_hint) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }

    ids
}

/// Whether an event told us the path is a directory, when it said so.
///
/// Notify classifies this only for creations and removals. Everywhere else the
/// filesystem is asked instead.
fn dir_hint(kind: EventKind) -> Option<bool> {
    match kind {
        EventKind::Create(CreateKind::Folder) => Some(true),
        EventKind::Create(CreateKind::File) => Some(false),
        EventKind::Remove(RemoveKind::Folder) => Some(true),
        EventKind::Remove(RemoveKind::File) => Some(false),
        _ => None,
    }
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
            .unwrap()
            .shallow();

        assert_eq!(opts.debounce, Duration::from_millis(50));
        assert!(!opts.recursive);
        assert!(opts.rules.is_ignored(".git", true));
        assert!(opts.rules.is_ignored(".obsidian", true));
        assert!(opts.rules.is_ignored("node_modules", true));
    }

    #[test]
    fn an_unparsable_pattern_is_refused() {
        let err = WatchOptions::default().ignoring(["[z-a].md"]).unwrap_err();

        assert!(
            matches!(err, CoreError::InvalidIgnorePattern(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn default_rules_cover_git_and_its_children() {
        let opts = options();

        assert!(opts.rules.is_ignored(".git", true));
        assert!(opts.rules.is_ignored(".git/config", false));
        assert!(opts.rules.is_ignored(".git/objects/ab/cdef", false));

        assert!(!opts.rules.is_ignored("notes/.gitignore", false));
        assert!(!opts.rules.is_ignored(".github/workflows", true));
        assert!(!opts.rules.is_ignored("a.md", false));
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
    fn dir_hint_reports_what_the_event_knew() {
        assert_eq!(dir_hint(EventKind::Create(CreateKind::Folder)), Some(true));
        assert_eq!(dir_hint(EventKind::Create(CreateKind::File)), Some(false));
        assert_eq!(dir_hint(EventKind::Remove(RemoveKind::Folder)), Some(true));
        assert_eq!(dir_hint(EventKind::Remove(RemoveKind::File)), Some(false));
        assert_eq!(dir_hint(EventKind::Modify(ModifyKind::Any)), None);
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

        assert_eq!(ids_for(root, &opts, &paths, None), vec!["notes/a.md"]);
    }

    #[test]
    fn a_dir_only_rule_uses_the_hint_when_the_path_is_gone() {
        let root = Path::new("/vault");
        let opts = WatchOptions::default().ignoring(["private/"]).unwrap();
        let gone = vec![PathBuf::from("/vault/private")];

        // The path no longer exists, so only the event's own classification can
        // tell a `dir/` pattern that this was a directory.
        assert!(ids_for(root, &opts, &gone, Some(true)).is_empty());
        assert_eq!(ids_for(root, &opts, &gone, Some(false)), vec!["private"]);
    }

    /// The event shape an atomic save produces: the temporary file is hidden by
    /// the default rules, so only the destination survives filtering.
    fn atomic_save_rename() -> DebouncedEvent {
        let event = notify::Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(PathBuf::from("/vault/.chinuu-tmp-1-2-3"))
            .add_path(PathBuf::from("/vault/notes/a.md"));

        DebouncedEvent::new(event, std::time::Instant::now())
    }

    #[test]
    fn an_unpaired_rename_claims_no_origin() {
        let root = Path::new("/vault");
        let options = WatchOptions::default();

        let batch = coalesce(root, &options, &[atomic_save_rename()]);

        assert_eq!(batch.len(), 1, "got {batch:?}");
        assert_eq!(batch[0].id, "notes/a.md");
        assert_eq!(batch[0].kind, WatchEventKind::Created);
        assert_eq!(
            batch[0].from_id, None,
            "there is no origin the UI could act on"
        );
    }

    #[test]
    fn an_unpaired_rename_into_an_ignored_path_is_a_removal() {
        let root = Path::new("/vault");
        let options = WatchOptions::default();

        // Renamed into the ignored directory, so only the source survives.
        let event = notify::Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(PathBuf::from("/vault/notes/a.md"))
            .add_path(PathBuf::from("/vault/.git/a.md"));

        let batch = coalesce(
            root,
            &options,
            &[DebouncedEvent::new(event, std::time::Instant::now())],
        );

        assert_eq!(batch.len(), 1, "got {batch:?}");
        assert_eq!(batch[0].id, "notes/a.md");
        assert_eq!(batch[0].kind, WatchEventKind::Removed);
    }

    #[test]
    fn a_paired_rename_keeps_its_origin() {
        let root = Path::new("/vault");
        let options = WatchOptions::default();

        let event = notify::Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(PathBuf::from("/vault/a.md"))
            .add_path(PathBuf::from("/vault/b.md"));

        let batch = coalesce(
            root,
            &options,
            &[DebouncedEvent::new(event, std::time::Instant::now())],
        );

        assert_eq!(batch.len(), 1, "got {batch:?}");
        assert_eq!(batch[0].id, "b.md");
        assert_eq!(batch[0].kind, WatchEventKind::Renamed);
        assert_eq!(batch[0].from_id.as_deref(), Some("a.md"));
    }

    #[test]
    fn a_temporary_file_alone_produces_nothing() {
        let root = Path::new("/vault");
        let options = WatchOptions::default();

        let event = notify::Event::new(EventKind::Create(CreateKind::File))
            .add_path(PathBuf::from("/vault/.chinuu-tmp-1-2-3"));

        let batch = coalesce(
            root,
            &options,
            &[DebouncedEvent::new(event, std::time::Instant::now())],
        );

        assert!(batch.is_empty(), "got {batch:?}");
    }
}
