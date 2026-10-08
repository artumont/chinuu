# Chinuu-Core

The Rust side of chinuu: the vault on disk, addressed by stable ids, the git
operations that sync it, and the watcher that reports changes to it.

No actual application logic should live here, it should be only system 
interaction and internal logic

```rust
use chinuu_core::files::{index_directory, read_file, write_file};
use chinuu_core::Result;

fn show(vault: &str) -> Result<()> {
    let index = index_directory(vault)?;

    // Direct lookup by vault-relative path, no tree walk.
    let note = index.get("notes/daily.md").expect("indexed above");
    println!("{} ({} bytes)", note.name(), note.size());

    // Children of any folder come back sorted by name.
    for child in index.root_node.children() {
        println!("{} -> {}", child.name(), child.id());
    }

    let text = read_file(note.path())?;
    println!("{text}");

    write_file("notes/new.md", "# new\n")?;
    Ok(())
}
```

## The public surface

| Item | Signature | What it does |
| ---- | --------- | ------------ |
| `files::index_cwd` | `() -> Result<FsIndex>` | Index the process working directory. |
| `files::index_directory` | `(impl AsRef<Path>) -> Result<FsIndex>` | Index a directory as a vault root, honouring its `.chinuuignore`. |
| `files::index_directory_with` | `(impl AsRef<Path>, &IgnoreRules) -> Result<FsIndex>` | Index using exactly the rules given. |
| `files::read_file` | `(impl AsRef<Path>) -> Result<String>` | Read a file as UTF-8 text, exactly as stored. |
| `files::read_bytes` | `(impl AsRef<Path>) -> Result<Vec<u8>>` | Read a file as raw bytes, for attachments. |
| `files::write_file` | `(impl AsRef<Path>, &str) -> Result<()>` | Write text atomically, creating missing parent directories. |
| `files::write_bytes` | `(impl AsRef<Path>, &[u8]) -> Result<()>` | Write bytes atomically. |
| `files::create_folder` | `(impl AsRef<Path>) -> Result<()>` | Create a folder and any missing parents. |
| `files::move_path` | `(impl AsRef<Path>, impl AsRef<Path>) -> Result<()>` | Move or rename a file or folder. Refuses an existing destination. |
| `files::delete_file` | `(impl AsRef<Path>) -> Result<()>` | Delete one file, or unlink one symbolic link. |
| `files::delete_folder` | `(impl AsRef<Path>) -> Result<()>` | Delete a folder and everything inside it. Recursive, destructive, no undo. |
| `files::FsNode` | enum | One file or folder. |
| `files::FsIndex` | struct | `root_node` plus a flat `id -> node` map. |
| `files::ROOT_ID` | `&str` | The root node's id, which is the empty string. |
| `CoreError` | enum | Every failure this crate can produce. |
| `Result<T>` | alias | `std::result::Result<T, CoreError>`. |

`FsNode` is an enum with `File` and `Folder` variants, both carrying `id`, `name`,
`size` and `path`, and `Folder` additionally carrying `children`. Prefer the
accessors over matching, since they work for both variants:

| Accessor | Returns | Notes |
| -------- | ------- | ----- |
| `id()` | `&str` | Vault-relative path, see below. |
| `name()` | `&str` | Last path segment. |
| `path()` | `&PathBuf` | Absolute on-disk path. |
| `size()` | `u64` | Filesystem size. For folders this is the directory record, not a recursive total. |
| `is_folder()` | `bool` | |
| `children()` | `&[FsNode]` | Empty slice for files, so callers never match just to iterate. |

`FsIndex` exposes `get(id) -> Option<&FsNode>`, `len()` and `is_empty()`. Both
fields are public, so you can also reach `root_node` and `flat_index` directly.

### Reading and writing

`read_file` and `write_file` are byte transparent between them. Nothing is
normalised on the way in or out: no newline translation, no trailing newline
added or removed, and a byte order mark is kept rather than stripped. Reading a
note and writing it back unchanged leaves the file identical, which is what
`packages/editor` assumes when it says the text in the editor is the file, byte
for byte. `tests/editing.rs` pins that across the cases that usually break it,
including CRLF, a lone carriage return, a byte order mark, an empty file, and an
internal NUL.

`write_file` and `write_bytes` are atomic. The content goes to a temporary file
in the same directory, is flushed to disk, and only then is renamed over the
target, so a reader sees either the whole old file or the whole new one. A crash
part way through a save cannot leave a truncated note. Three consequences worth
knowing:

- The temporary file is named with `TEMP_PREFIX` and the default ignore rules
  cover it, so it never appears in the tree and never arrives as a watch event.
- A write through a symbolic link follows the link and keeps it, rather than
  replacing it with an ordinary file.
- The target's permissions are carried over, so a `0644` note does not become
  `0600` after a save.

A file that is not valid UTF-8 is `NotUtf8` rather than an io error, so a caller
can offer to open it another way instead of reporting a failure. `read_bytes`
reads it regardless.

`move_path` refuses an existing destination instead of overwriting it, and
`delete_folder` is the only recursive, unrecoverable operation here.

### Deleting

`delete_file` removes one file. `delete_folder` removes a folder and everything
inside it, which is destructive and has no undo, so a caller that can still be
talked out of it should confirm with the user first.

Neither follows a symbolic link. `delete_file` unlinks a link and leaves its
target alone, whether that target is a file or a directory. `delete_folder`
refuses a link outright with `NotADirectory`, and a link inside the folder being
removed is unlinked rather than followed. Both type checks use
`fs::symlink_metadata` rather than `fs::metadata`, so a link to a directory is
never mistaken for a directory. That is what stops "delete this folder" from
reaching a folder outside the vault.

Both report a missing path as an error rather than succeeding quietly, so a typo
cannot pass for a completed delete. A caller that would rather treat an already
missing path as success can match on the error and decide.

## Ids are vault-relative paths

The one decision worth reading the code for. A node's `id` is its path relative
to the vault root, always joined with `/`:

| Path on disk | `id` |
| ------------ | ---- |
| `/home/me/vault` | `""` (`ROOT_ID`) |
| `/home/me/vault/a.md` | `a.md` |
| `/home/me/vault/notes/b.md` | `notes/b.md` |
| `/home/me/vault/notes/deep/c.md` | `notes/deep/c.md` |

Two reasons it is a relative path rather than the obvious alternative.

**An inode number is not stable.** The first version keyed nodes by `st_ino`. An
inode is only unique within one mounted filesystem, it is reused after a file is
deleted, and it changes when a vault is copied, restored from a backup or opened
from a different machine. Anything that persists an id (a cache, an open
document, a git index) would have been keyed on something that can silently come
to mean a different file.

**The separator is fixed, so ids survive a platform change.** `Path::join` uses
the host separator, so the same vault would produce `notes/b.md` on Linux and
`notes\b.md` on Windows. The walk builds ids by string concatenation with an
explicit `/` instead, which the `ids_do_not_contain_backslashes` test pins down.

The git layer depends on both properties, since a relative, separator-stable path
is exactly what a repository-relative path needs to be.

## Git

`git` wraps libgit2 through the `git2` crate, built with the `https` and `ssh`
features. It is deliberately the mechanical half of version control: open,
inspect, stage, commit, fetch, pull, push. Note syncing and plugin installation
are policies to build on top of this, not to fold into it.

```rust
use chinuu_core::git::{Auth, GitRepo, PullOutcome};
use chinuu_core::Result;

fn sync(vault: &str) -> Result<()> {
    let repo = GitRepo::open(vault)?;

    repo.stage_all()?;
    if !repo.status()?.is_clean() {
        repo.commit("sync: daily note")?;
    }

    match repo.pull("origin", "main", &Auth::Default)? {
        PullOutcome::Conflicts => {
            for conflict in repo.conflicts()? {
                println!("clash in {}", conflict.id);
            }
        }
        other => println!("{other:?}"),
    }
    Ok(())
}
```

### Constructors

| Call | What it does |
| ---- | ------------ |
| `GitRepo::open(path)` | Open the repository rooted exactly at `path`. |
| `GitRepo::discover(path)` | Find the repository containing `path`, searching upwards. |
| `GitRepo::init(path)` | Create a repository, worktree included. |
| `GitRepo::init_bare(path)` | Create a bare repository, with no worktree. |
| `GitRepo::clone(url, into)` | Clone, leaving `origin` configured. |
| `GitRepo::is_repo(path)` | Whether `path` is a repository root. Never errors. |

### Inspection

| Call | Returns |
| ---- | ------- |
| `root()` | Worktree root, or the repository directory when bare. |
| `is_bare()` | Whether there is no worktree. |
| `repository()` | The underlying `git2::Repository`, for anything not wrapped here. |
| `has_commits()` | False right after `init`. |
| `head_oid()`, `head_branch()`, `is_detached()` | HEAD state. |
| `signature()` | Identity from config, or `MissingIdentity` when unset. |
| `set_identity(name, email)` | Write identity into the repository's local config. |
| `remote_url(name)` | Fetch URL of a remote. |
| `status()` | `RepoStatus`: branch, entries, ahead, behind. |
| `ahead_behind()` | `(ahead, behind)` against the current upstream. |
| `log(limit)` | `CommitInfo` list, newest first. |
| `branches()` | Local branches, with upstream and target. |
| `remotes()` | Configured remotes. |
| `conflicts()` | Unresolved conflicts, with the blob id on each side. |

### Operations

| Call | What it does |
| ---- | ------------ |
| `stage(paths)`, `stage_all()` | Stage given paths, or everything. A path that no longer exists stages a deletion. |
| `unstage(paths)`, `unstage_all()` | Reset the index, leaving the worktree alone. |
| `commit(message)` | Commit the index. Refuses an empty commit, and refuses while conflicted. |
| `create_branch(name, at)` | Create a branch, at a revision or at HEAD. |
| `checkout_branch(name)` | Switch branches. Safe: refuses rather than discard local edits. |
| `set_upstream(branch, remote, remote_branch)` | Configure tracking. |
| `add_remote`, `remove_remote` | Remote management. |
| `fetch(remote, auth)` | Fetch the configured refspecs into remote-tracking refs. |
| `pull(remote, branch, auth)` | Fetch, then fast-forward or merge. |
| `push(remote, branch, auth)` | Push, then update the remote-tracking ref. |
| `abort_merge()` | Throw away an in-flight operation. Destructive: it hard resets. |

### Types

| Type | Shape |
| ---- | ----- |
| `RepoStatus` | `branch`, `detached`, `entries`, `ahead`, `behind`. Helpers `is_clean`, `staged`, `unstaged`, `conflicted`. |
| `FileStatus` | `id`, `staged`, `unstaged`. One path can appear on both sides. |
| `ChangeKind` | `Added`, `Modified`, `Deleted`, `Renamed`, `TypeChange`, `Untracked`, `Conflicted`. |
| `CommitInfo` | `id`, `summary`, `message`, author name and email, time, `parents`. |
| `BranchInfo` | `name`, `is_head`, `upstream`, `target`. |
| `RemoteInfo` | `name`, `url`, `push_url`. |
| `Conflict` | `id`, plus `ancestor`, `ours`, `theirs` blob ids. |
| `PullOutcome` | `UpToDate`, `FastForward`, `Merged`, `Conflicts`. |
| `PushOutcome` | `UpToDate`, `Pushed`. |
| `Auth` | `Default`, `Token`, `SshAgent`, `SshKey`. |

### Things worth knowing

**Blocking, and not shareable.** `git2::Repository` is `Send` but not `Sync`, so
`GitRepo` cannot be shared across threads. Open one per task and call it from a
worker thread, for example `tauri::async_runtime::spawn_blocking`, rather than on
the UI thread.

**`Auth::Default` uses the user's own git setup**, trying ssh-agent and then the
git credential helper, which is where a stored personal access token usually
lives. Prefer it to duplicating credentials in app settings.

**A pull that conflicts leaves the merge in place** and returns
`PullOutcome::Conflicts` rather than backing out, so the caller can show what
clashed. `abort_merge` discards it.

**Push keeps the remote-tracking ref current**, the way `git push` does, so
ahead and behind are right without an immediate fetch.

**`log` is topological, newest first.** A pure time sort is not topological, so
with equal timestamps libgit2 can emit a parent before its child.

## Watch

`files::VaultWatcher` watches a vault and delivers debounced batches of changes.
That is what keeps a tree UI live without polling.

```rust
use chinuu_core::files::{VaultWatcher, WatchUpdate};
use chinuu_core::Result;

fn watch(vault: &str) -> Result<VaultWatcher> {
    VaultWatcher::start(vault, move |update| match update {
        WatchUpdate::Changed(events) => {
            for event in events {
                // Ids are the same shape as FsNode ids, so the UI can patch
                // its tree directly instead of reindexing the vault.
                println!("{} {}", event.kind.as_str(), event.id);
            }
        }
        WatchUpdate::Failed(failure) => eprintln!("watch failed: {failure}"),
    })
}
```

### Types

| Type | Shape |
| ---- | ----- |
| `VaultWatcher` | A running watch. Dropping it stops watching. |
| `WatchOptions` | `debounce`, `recursive`, `rules`. Builders: `with_debounce`, `ignoring`, `with_rules`, `shallow`. |
| `WatchEvent` | `id`, `kind`, `from_id` (set only for a rename). |
| `WatchEventKind` | `Created`, `Modified`, `Removed`, `Renamed`. `as_str()` gives a lowercase name. |
| `WatchUpdate` | `Changed(Vec<WatchEvent>)` or `Failed(WatchFailure)`. Helpers `events()`, `failure()`. |
| `WatchFailure` | `messages`, and a `Display` impl that joins them. |
| `DEFAULT_DEBOUNCE` | 200 ms. |

### Things worth knowing

**The callback runs on the watcher's own thread.** It must not block, and it is
not the UI thread. In a Tauri app, call `AppHandle::emit` from inside it.

**The debounce is a quiet period.** Delivery waits for the filesystem to go
quiet, so a long sequence of writes arrives as one batch rather than a flicker of
partial states. Continuous writes keep pushing delivery back, which is the point.

**A batch holds one entry per id, sorted by id.** Saving a new file emits a
create and then a data change; that folds into a single `Created`, because a new
path is what the UI has to act on.

**`.git` and `.chinuuignore` are ignored by default.** Our own git operations
churn `.git`, so watching it would feed our changes straight back to us.
`WatchOptions::ignoring` adds patterns and `with_rules` replaces the set
outright; see [Ignore rules](#ignore-rules).

**A newly created directory is scanned once.** A recursive watch only starts
covering a directory after the kernel reports it, so a directory created and then
filled immediately can lose the events for its contents. That is the normal shape
of unpacking a folder of notes, or of a checkout landing a tree, so the watcher
walks the new directory and reports what it finds. Without this, a pulled folder
arrives half populated.

**A folder rename changes every id underneath it.** Ids are paths, so the whole
subtree moves. The rename event carries `from_id`, and that is what a UI should
rebase on rather than waiting for events for every child.

## Ignore rules

A vault hides paths by listing them, one per line, in `.chinuuignore` at its
root. Indexing and watching share the rules, so a path absent from the tree will
not arrive as an event either. Both convenience entry points load the file for
you: `index_directory` and `VaultWatcher::start`.

```text
# a comment
*.tmp            # any .tmp file, at any depth
/scratch.md      # only at the vault root
private/         # a directory, at any depth
journal/2024/**  # everything under that folder
!private/keep.md # re-included; the last matching line wins
```

Syntax is gitignore's, taken from the `ignore` crate rather than reimplemented,
so globs, `**`, directory-only patterns and negation behave the way they do in
every other tool that reads this format.

| Item | Purpose |
| ---- | ------- |
| `IGNORE_FILE_NAME` | `.chinuuignore`, the file read from the vault root. |
| `DEFAULT_PATTERNS` | `.git` and `.chinuuignore`, applied to every vault. |
| `IgnoreRules` | The compiled rule set. `defaults`, `for_vault`, `parse`, `add_line`, `add_patterns`, `is_ignored`. |

`IgnoreRules::for_vault(root)` is the built-ins plus the vault's file, which is
what `index_directory` and `VaultWatcher::start` use. `index_directory_with` and
`WatchOptions::with_rules` take the rules as given, so a caller wanting something
else builds it: `IgnoreRules::parse("*.tmp")` hides only that, and
`IgnoreRules::empty()` hides nothing at all.

`is_ignored(id, is_dir)` matches ancestors as well as the path itself, so a
caller never has to prune a walk by hand: with `private/` ignored,
`private/keep.md` is ignored too. The `is_dir` argument matters because a
trailing `/` matches directories only, which is why the watcher reads it from the
event itself when a removal has already taken the path away.

An unparsable pattern is an error, `InvalidIgnorePattern`, rather than a
silently dropped line, so a typo cannot quietly un-hide something. `for_vault`
surfaces the same error for a bad line in the vault's file.

Ignored paths are hidden from indexing and watching, and nothing else. Reading,
writing and deleting still work on them, so a caller can edit or remove a path it
cannot see.

## Errors

Every public function returns `Result<_, CoreError>`. Nothing in the crate
panics on bad input or on a failed syscall.

| Variant | Message | Raised when |
| ------- | ------- | ----------- |
| `Io { path, source }` | `io error on \`{path}\`: {source}` | A syscall failed and the path is known. |
| `IoPlain(io::Error)` | transparent | A syscall failed and there is no path to attach. Also the `#[from]` target, so `?` works on a raw io error. |
| `NotAFile(PathBuf)` | `path is not a file: {path}` | `read_file` or `delete_file` on a directory, or `write_file` targeting one. |
| `NotADirectory(PathBuf)` | `path is not a directory: {path}` | `index_directory` on a file, or `delete_folder` on a file or a symbolic link. |
| `NotUtf8(PathBuf)` | `` `{path}` is not valid UTF-8 `` | `read_file` on a file that is not text. Use `read_bytes`. |
| `AlreadyExists(PathBuf)` | `` `{path}` already exists `` | `move_path` onto a path that is already there. |
| `Git(git2::Error)` | `git: {source}` | Any libgit2 failure, with libgit2's own message. |
| `MissingIdentity` | `no git identity configured: set user.name and user.email` | Committing with no identity configured. |
| `UnknownRemote(String)` | `no remote named \`{0}\`` | A remote that does not exist. |
| `UnknownBranch(String)` | `no branch named \`{0}\`` | A branch that does not exist. |
| `Conflicts` | `unresolved merge conflicts` | Committing while the index holds conflicts. |
| `NothingStaged` | `nothing staged to commit` | Committing with an index identical to HEAD. |
| `NotARepository(PathBuf)` | `` `{0}` is not inside a git repository `` | `open` or `discover` on a path with no repository. |
| `BareRepository(PathBuf)` | `` `{0}` is a bare repository and has no worktree `` | A worktree operation on a bare repository. |

`CoreError::io(path, source)` attaches a path to a raw io error, which is how the
walk reports a failure on a specific entry instead of losing that context.

## Known issues

Three things are wrong or expensive in ways that are not obvious from the API.

**`flat_index` costs more memory than it looks like.** A `Folder` value embeds
its whole subtree, so inserting every node into the map duplicates each
descendant once per ancestor folder. Total memory is roughly O(nodes x depth)
rather than O(nodes). That is fine for a shallow vault and wasteful for a deep
one. Fixing it means storing handles or `Rc` rather than owned nodes, which
changes the type, so it was left alone on purpose.

**A symlink cycle recurses forever.** The walk uses `fs::metadata`, which follows
symlinks, and there is no depth limit. A vault containing a symlink that points at
one of its own ancestors will recurse until the stack overflows. The fix is either
a depth cap or `symlink_metadata` with an explicit policy for links.

**Folder size is not a recursive total.** `size()` reports what the filesystem
reports for the directory inode, which is a few thousand bytes regardless of what
is inside. Summing children is the caller's job until a `total_size` helper
exists.

**Git has no network timeout knob.** `fetch`, `pull` and `push` block until
libgit2 finishes, and there is no cancellation. On a slow or unreachable remote
that means a hung worker thread. Calls from the UI should become cancellable at a
higher level once that matters.

**Git paths are assumed UTF-8.** Git stores paths as bytes, and the conversion
uses `String::from_utf8_lossy`, so a repository with a non-UTF-8 filename yields
a lossy id instead of an error. Vault ids being strings is the constraint that
follows from `FsNode`.

**Renaming a folder does not report its children.** By design: the rename event
carries `from_id`, and rebasing the subtree from that is cheaper and less racy
than emitting one event per descendant. A UI that ignores `from_id` will keep a
stale subtree.

**An atomic write can leave an orphan temporary file.** If the process is killed
between creating the temporary file and renaming it, a file named with
`TEMP_PREFIX` stays in the vault. It is ignored by the default rules, so it never
shows up in the tree, but nothing collects it either.

**Ignore rules are one file at the vault root.** There is no `.gitignore`
support and no nested `.chinuuignore`, so a vault cannot vary its rules per
directory. `IgnoreRules::for_vault` reads the one file and nothing deeper.

**Editing `.chinuuignore` does not reload it.** Rules are read when indexing or
when a watch is started, and the file is itself ignored, so a change to it takes
effect on the next index or restart rather than immediately.

**Watching is per-directory under the hood.** On Linux this is one inotify watch
descriptor per directory, and a very large vault can exhaust the limit; when that
happens the failure arrives at runtime as `WatchUpdate::Failed`, not from
`start`, because it happens long after the watch was established.

## Testing

```sh
cargo test -p chinuu-core --features serde   # 163 tests: 49 unit, 19 editing, 29 files, 31 git, 11 serde, 24 watcher
cargo clippy -p chinuu-core --all-targets -- -D warnings
cargo fmt -p chinuu-core --check
```

That or just use the `Makefile` commands
