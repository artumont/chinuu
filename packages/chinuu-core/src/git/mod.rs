//! Git operations for a vault, built on libgit2 through the `git2` crate.
//!
//! This module is deliberately the mechanical half of version control: open,
//! inspect, stage, commit, fetch, pull, push. It holds no opinion about what a
//! vault does with those. Note syncing and plugin installation are different
//! policies that should be built on top, not folded in here.
//!
//! Every function is blocking. `git2::Repository` is `Send` but not `Sync`, so
//! a [`GitRepo`] must not be shared between threads; open one per task and let
//! the caller choose the thread, for example `tauri::async_runtime::spawn_blocking`.

// Submodules are private so each type has exactly one public path, `git::Type`.
// The escape hatch for anything not wrapped here is `GitRepo::repository`.
mod auth;
mod commit;
mod merge;
mod remote;
mod repo;
mod status;

pub use auth::Auth;
pub use commit::CommitInfo;
pub use merge::Conflict;
pub use remote::{BranchInfo, PullOutcome, PushOutcome, RemoteInfo};
pub use repo::GitRepo;
pub use status::{ChangeKind, FileStatus, RepoStatus};
