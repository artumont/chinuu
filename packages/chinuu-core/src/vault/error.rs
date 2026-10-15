use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    /// A filesystem operation failed against a specific path.
    #[error("io error on `{}`: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// A directory was expected but the path is something else.
    #[error("path is not a directory: {}", .0.display())]
    NotADirectory(PathBuf),

    /// A file was expected but the path is something else.
    #[error("path is not a file: {}", .0.display())]
    NotAFile(PathBuf),

    /// An ignore rule could not be compiled.
    #[error("invalid ignore pattern: {0}")]
    InvalidIgnorePattern(String),

    /// A Path failed to be converted to a relative path.
    #[error("path conversion failed: {0}")]
    FailedPathConversion(String),
}

impl VaultError {
    /// Convert a VaultError to a string.
    pub fn to_str(&self) -> &'static str {
        match self {
            VaultError::Io { .. } => "io",
            VaultError::NotADirectory(_) => "not-a-directory",
            VaultError::NotAFile(_) => "not-a-file",
            VaultError::InvalidIgnorePattern(_) => "invalid-ignore-pattern",
            VaultError::FailedPathConversion(_) => "failed-path-conversion",
        }
    }

    /// Attach a path to a raw io error.
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        VaultError::Io {
            path: path.into(),
            source,
        }
    }
}
