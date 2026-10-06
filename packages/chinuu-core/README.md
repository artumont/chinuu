# Chinuu-Core

The Rust side of chinuu: the vault on disk, addressed by stable ids, plus the
git operations that sync it.

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
| `files::index_directory` | `(impl AsRef<Path>) -> Result<FsIndex>` | Index any directory as a vault root. |
| `files::read_file` | `(impl AsRef<Path>) -> Result<String>` | Read a file as UTF-8 text. |
| `files::write_file` | `(impl AsRef<Path>, &str) -> Result<()>` | Write text, creating missing parent directories. |
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

## Errors

Every public function returns `Result<_, CoreError>`. Nothing in the crate
panics on bad input or on a failed syscall.

| Variant | Message | Raised when |
| ------- | ------- | ----------- |
| `Io { path, source }` | `io error on \`{path}\`: {source}` | A syscall failed and the path is known. |
| `IoPlain(io::Error)` | transparent | A syscall failed and there is no path to attach. Also the `#[from]` target, so `?` works on a raw io error. |
| `NotAFile(PathBuf)` | `path is not a file: {path}` | `read_file` on a directory, or `write_file` targeting one. |
| `NotADirectory(PathBuf)` | `path is not a directory: {path}` | `index_directory` on a file or a missing path. |
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

## Testing

```sh
cargo test -p chinuu-core        # 44 tests: 2 unit, 11 files, 31 git
cargo clippy -p chinuu-core --all-targets -- -D warnings
cargo fmt -p chinuu-core --check
```

That or just use the `Makefile` commands
