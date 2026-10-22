use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum ChinuuFileIoError {
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

    /// A node id was not found.
    #[error("node id does not exist: {0}")]
    NodeIdDoesNotExist(String),

    /// A prefix stripping error.
    #[error("unable to strip prefix: {prefix} from {source_path}")]
    StripPrefixError {
        prefix: PathBuf,
        source_path: PathBuf,
    },
}

impl ChinuuFileIoError {
    /// Convert a VaultError to a string.
    pub fn to_str(&self) -> &'static str {
        match self {
            ChinuuFileIoError::Io { .. } => "io",
            ChinuuFileIoError::NotADirectory(_) => "not-a-directory",
            ChinuuFileIoError::NotAFile(_) => "not-a-file",
            ChinuuFileIoError::InvalidIgnorePattern(_) => "invalid-ignore-pattern",
            ChinuuFileIoError::FailedPathConversion(_) => "failed-path-conversion",
            ChinuuFileIoError::NodeIdDoesNotExist(_) => "node-id-does-not-exist",
            ChinuuFileIoError::StripPrefixError { .. } => "strip-prefix-error",
        }
    }

    /// Attach a path to a raw io error.
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        ChinuuFileIoError::Io {
            path: path.into(),
            source,
        }
    }

    /// Create a strip-prefix-error.
    pub fn strip_prefix(prefix: impl AsRef<Path>, source: impl AsRef<Path>) -> Self {
        ChinuuFileIoError::StripPrefixError {
            prefix: prefix.as_ref().to_path_buf(),
            source_path: source.as_ref().to_path_buf(),
        }
    }
}
