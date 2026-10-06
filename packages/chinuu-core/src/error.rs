use std::{io, path::PathBuf};

/// Errors produced by `chinuu-core`.
///
/// Variants carry the offending path where one is known so callers (and the
/// Tauri layer) can surface something actionable instead of a bare io error.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// A filesystem operation failed against a specific path.
    #[error("io error on `{}`: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// A filesystem operation failed and no meaningful path is associated.
    #[error(transparent)]
    IoPlain(#[from] io::Error),

    /// The path exists but is not a file.
    #[error("path is not a file: {}", .0.display())]
    NotAFile(PathBuf),

    /// A directory was expected but the path is something else.
    #[error("path is not a directory: {}", .0.display())]
    NotADirectory(PathBuf),

    /// A git operation failed.
    #[error("git: {0}")]
    Git(#[from] git2::Error),

    /// No `user.name` and `user.email` are configured, so a commit has no
    /// identity to be signed with.
    #[error("no git identity configured: set user.name and user.email")]
    MissingIdentity,

    /// The requested remote does not exist.
    #[error("no remote named `{0}`")]
    UnknownRemote(String),

    /// The requested branch does not exist.
    #[error("no branch named `{0}`")]
    UnknownBranch(String),

    /// The index still holds unresolved merge conflicts.
    #[error("unresolved merge conflicts")]
    Conflicts,

    /// Nothing was staged, so the commit was refused.
    #[error("nothing staged to commit")]
    NothingStaged,

    /// The path is not inside a git repository.
    #[error("`{}` is not inside a git repository", .0.display())]
    NotARepository(PathBuf),

    /// The operation needs a worktree, but the repository is bare.
    #[error("`{}` is a bare repository and has no worktree", .0.display())]
    BareRepository(PathBuf),
}

impl CoreError {
    /// Attach a path to a raw io error.
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        CoreError::Io {
            path: path.into(),
            source,
        }
    }
}

/// Convenience result alias used across the crate.
pub type Result<T> = std::result::Result<T, CoreError>;
