use std::{
    io,
    path::{Path, PathBuf},
};

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

    /// The path exists and is not valid UTF-8, so it cannot be read as text.
    ///
    /// Separate from a plain io error because it is not a failure so much as a
    /// statement about the file: a UI can act on it by opening the file some
    /// other way.
    #[error("`{}` is not valid UTF-8", .0.display())]
    NotUtf8(PathBuf),

    /// A directory was expected but the path is something else.
    #[error("path is not a directory: {}", .0.display())]
    NotADirectory(PathBuf),

    /// The destination exists, and the operation refuses to overwrite it.
    #[error("`{}` already exists", .0.display())]
    AlreadyExists(PathBuf),

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

    /// A filesystem watch could not be established.
    #[error("watch error: {0}")]
    Watch(#[from] notify::Error),

    /// An ignore rule could not be compiled.
    #[error("invalid ignore pattern: {0}")]
    InvalidIgnorePattern(String),
}

impl CoreError {
    /// Attach a path to a raw io error.
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        CoreError::Io {
            path: path.into(),
            source,
        }
    }

    /// A stable, machine-readable name for this error.
    ///
    /// A frontend should match on this rather than parsing the message, which is
    /// written for a human and may be reworded. Every variant has its own code,
    /// and adding one without extending this match does not compile.
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::Io { .. } | CoreError::IoPlain(_) => "io",
            CoreError::NotAFile(_) => "not-a-file",
            CoreError::NotUtf8(_) => "not-utf8",
            CoreError::NotADirectory(_) => "not-a-directory",
            CoreError::AlreadyExists(_) => "already-exists",
            CoreError::Git(_) => "git",
            CoreError::MissingIdentity => "missing-identity",
            CoreError::UnknownRemote(_) => "unknown-remote",
            CoreError::UnknownBranch(_) => "unknown-branch",
            CoreError::Conflicts => "conflicts",
            CoreError::NothingStaged => "nothing-staged",
            CoreError::NotARepository(_) => "not-a-repository",
            CoreError::BareRepository(_) => "bare-repository",
            CoreError::Watch(_) => "watch",
            CoreError::InvalidIgnorePattern(_) => "invalid-ignore-pattern",
        }
    }

    /// The path this error is about, when it is about one.
    pub fn path(&self) -> Option<&Path> {
        match self {
            CoreError::Io { path, .. }
            | CoreError::NotAFile(path)
            | CoreError::NotUtf8(path)
            | CoreError::NotADirectory(path)
            | CoreError::AlreadyExists(path)
            | CoreError::NotARepository(path)
            | CoreError::BareRepository(path) => Some(path),
            _ => None,
        }
    }

    /// The kind of io error underneath, when there is one.
    ///
    /// Lets a caller tell "not found" from "permission denied" without losing
    /// the path or the message. `None` for errors that did not come from io.
    pub fn io_kind(&self) -> Option<io::ErrorKind> {
        match self {
            CoreError::Io { source, .. } | CoreError::IoPlain(source) => Some(source.kind()),
            _ => None,
        }
    }
}

/// Serialized as `{ "code", "message", "path" }`.
///
/// The message is for a human; the code is the part a frontend should branch on.
/// `path` is `null` when the error is not about one path.
#[cfg(feature = "serde")]
impl serde::Serialize for CoreError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct as _;

        let mut state = serializer.serialize_struct("CoreError", 3)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.serialize_field("path", &self.path())?;
        state.end()
    }
}

/// Convenience result alias used across the crate.
pub type Result<T> = std::result::Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use std::{
        io,
        path::{Path, PathBuf},
    };

    use super::*;

    /// Every variant, so a new one cannot be added without a code to match.
    /// Adding a variant breaks `code()` first, which is the point.
    fn every_variant() -> Vec<CoreError> {
        vec![
            CoreError::Io {
                path: PathBuf::from("/vault/a.md"),
                source: io::Error::new(io::ErrorKind::NotFound, "missing"),
            },
            CoreError::IoPlain(io::Error::other("plain")),
            CoreError::NotAFile(PathBuf::from("/vault/a.md")),
            CoreError::NotUtf8(PathBuf::from("/vault/a.md")),
            CoreError::NotADirectory(PathBuf::from("/vault/notes")),
            CoreError::AlreadyExists(PathBuf::from("/vault/b.md")),
            CoreError::Git(git2::Error::from_str("boom")),
            CoreError::MissingIdentity,
            CoreError::UnknownRemote("origin".to_owned()),
            CoreError::UnknownBranch("main".to_owned()),
            CoreError::Conflicts,
            CoreError::NothingStaged,
            CoreError::NotARepository(PathBuf::from("/vault")),
            CoreError::BareRepository(PathBuf::from("/vault")),
            CoreError::Watch(notify::Error::generic("boom")),
            CoreError::InvalidIgnorePattern("[z-a]".to_owned()),
        ]
    }

    #[test]
    fn the_code_vocabulary_is_exactly_this() {
        let mut codes: Vec<&str> = every_variant().iter().map(CoreError::code).collect();
        codes.sort_unstable();
        codes.dedup();

        // A new variant has to add its code here, which is what makes this
        // worth pinning: the string is part of the frontend's contract.
        assert_eq!(
            codes,
            vec![
                "already-exists",
                "bare-repository",
                "conflicts",
                "git",
                "invalid-ignore-pattern",
                "io",
                "missing-identity",
                "not-a-directory",
                "not-a-file",
                "not-a-repository",
                "not-utf8",
                "nothing-staged",
                "unknown-branch",
                "unknown-remote",
                "watch",
            ]
        );
    }

    #[test]
    fn both_io_shapes_share_one_code() {
        // Deliberate: whether the path was known is an implementation detail,
        // and `io_kind` is what tells a caller what actually went wrong.
        let with_path = CoreError::Io {
            path: PathBuf::from("/vault/a.md"),
            source: io::Error::new(io::ErrorKind::NotFound, "missing"),
        };

        assert_eq!(with_path.code(), "io");
        assert_eq!(CoreError::IoPlain(io::Error::other("x")).code(), "io");
    }

    #[test]
    fn codes_are_lowercase_kebab_case() {
        for error in every_variant() {
            let code = error.code();
            assert!(
                code.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "`{code}` is not lowercase kebab-case"
            );
            assert!(
                !code.starts_with('-') && !code.ends_with('-'),
                "`{code}` has a leading or trailing dash"
            );
        }
    }

    #[test]
    fn path_is_reported_for_the_variants_that_carry_one() {
        assert_eq!(
            CoreError::NotAFile(PathBuf::from("/vault/a.md")).path(),
            Some(Path::new("/vault/a.md"))
        );
        assert_eq!(
            CoreError::AlreadyExists(PathBuf::from("/vault/b")).path(),
            Some(Path::new("/vault/b"))
        );

        assert_eq!(CoreError::Conflicts.path(), None);
        assert_eq!(CoreError::MissingIdentity.path(), None);
    }

    #[test]
    fn io_kind_surfaces_the_underlying_kind() {
        let error = CoreError::Io {
            path: PathBuf::from("/vault/a.md"),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "nope"),
        };

        assert_eq!(error.io_kind(), Some(io::ErrorKind::PermissionDenied));
        assert_eq!(CoreError::Conflicts.io_kind(), None);
        assert_eq!(
            CoreError::IoPlain(io::Error::new(io::ErrorKind::NotFound, "nope")).io_kind(),
            Some(io::ErrorKind::NotFound)
        );
    }
}
