use std::path::{Path, PathBuf};

use git2::{BranchType, Oid, Repository, Signature};

use crate::error::{CoreError, Result};

/// A handle to a git repository backing a vault.
///
/// Cheap to construct and cheap to drop, but not shareable: `git2::Repository`
/// is `Send` and not `Sync`, so this type cannot be used across threads. Open
/// one per task.
pub struct GitRepo {
    repo: Repository,
    root: PathBuf,
}

// `git2::Repository` has no `Debug`, so this is written by hand. It shows what
// is useful in a panic message or a log line and nothing more.
impl std::fmt::Debug for GitRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitRepo")
            .field("root", &self.root)
            .field("bare", &self.repo.is_bare())
            .finish_non_exhaustive()
    }
}

impl GitRepo {
    /// Open the repository rooted exactly at `path`, without searching upwards.
    ///
    /// Fails with [`CoreError::NotARepository`] when `path` is not one.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let repo = Repository::open(path).map_err(|e| Self::open_error(path, e))?;
        Ok(Self::new(repo))
    }

    /// Find the repository containing `path`, searching parent directories.
    ///
    /// Use this when the vault is a subdirectory of a larger repository.
    pub fn discover(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let repo = Repository::discover(path).map_err(|e| Self::open_error(path, e))?;
        Ok(Self::new(repo))
    }

    /// Create a repository at `path`, creating the directory if needed.
    pub fn init(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::new(Repository::init(path)?))
    }

    /// Create a bare repository at `path`.
    ///
    /// A bare repository has no worktree, so worktree operations fail with
    /// [`CoreError::BareRepository`]. Useful as a local push target.
    pub fn init_bare(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::new(Repository::init_bare(path)?))
    }

    /// Clone `url` into `into`, leaving `origin` configured.
    pub fn clone(url: &str, into: impl AsRef<Path>) -> Result<Self> {
        let repo = git2::build::RepoBuilder::new().clone(url, into.as_ref())?;
        Ok(Self::new(repo))
    }

    /// Whether `path` is itself a repository root. Never errors.
    pub fn is_repo(path: impl AsRef<Path>) -> bool {
        Repository::open(path).is_ok()
    }

    fn new(repo: Repository) -> Self {
        let raw = repo
            .workdir()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| repo.path().to_path_buf());
        // libgit2 returns the worktree path with a trailing separator, which
        // makes joins and equality checks awkward. Normalise it away.
        let root = raw.components().collect::<PathBuf>();
        Self { repo, root }
    }

    /// Report a missing repository as a domain error, and anything else as a
    /// plain git error rather than flattening both into one opaque failure.
    fn open_error(path: &Path, err: git2::Error) -> CoreError {
        if err.code() == git2::ErrorCode::NotFound {
            CoreError::NotARepository(path.to_path_buf())
        } else {
            CoreError::Git(err)
        }
    }

    /// The worktree root, or the repository directory when bare.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn is_bare(&self) -> bool {
        self.repo.is_bare()
    }

    /// Escape hatch for anything this crate does not wrap.
    pub fn repository(&self) -> &Repository {
        &self.repo
    }

    /// Whether HEAD points at a commit yet. False right after `init`.
    pub fn has_commits(&self) -> Result<bool> {
        Ok(!self.repo.is_empty()?)
    }

    /// The commit HEAD resolves to.
    pub fn head_oid(&self) -> Result<Oid> {
        Ok(self.repo.head()?.peel_to_commit()?.id())
    }

    /// The current branch name, or `None` when detached or unborn.
    pub fn head_branch(&self) -> Result<Option<String>> {
        match self.repo.head() {
            Ok(head) if head.is_branch() => Ok(Some(head.shorthand()?.to_owned())),
            Ok(_) => Ok(None),
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn is_detached(&self) -> Result<bool> {
        Ok(self.repo.head_detached()?)
    }

    /// The commit identity from the repository config chain.
    ///
    /// Fails with [`CoreError::MissingIdentity`] rather than inventing one,
    /// because committing under a made-up identity is worse than refusing.
    pub fn signature(&self) -> Result<Signature<'static>> {
        let config = self.repo.config()?;
        let name = config
            .get_string("user.name")
            .map_err(|_| CoreError::MissingIdentity)?;
        let email = config
            .get_string("user.email")
            .map_err(|_| CoreError::MissingIdentity)?;
        Ok(Signature::now(&name, &email)?)
    }

    /// Write `user.name` and `user.email` into the repository's local config.
    pub fn set_identity(&self, name: &str, email: &str) -> Result<()> {
        let mut config = self.repo.config()?;
        config.set_str("user.name", name)?;
        config.set_str("user.email", email)?;
        Ok(())
    }

    /// The fetch URL of a remote.
    pub fn remote_url(&self, name: &str) -> Result<String> {
        let remote = self
            .repo
            .find_remote(name)
            .map_err(|_| CoreError::UnknownRemote(name.to_owned()))?;
        Ok(remote.url()?.to_owned())
    }

    /// Look up a local branch, or fail with [`CoreError::UnknownBranch`].
    pub(crate) fn local_branch(&self, name: &str) -> Result<git2::Branch<'_>> {
        self.repo
            .find_branch(name, BranchType::Local)
            .map_err(|_| CoreError::UnknownBranch(name.to_owned()))
    }

    /// Require a worktree, for operations that touch the working directory.
    pub(crate) fn require_worktree(&self) -> Result<()> {
        if self.repo.is_bare() {
            return Err(CoreError::BareRepository(self.root.clone()));
        }
        Ok(())
    }
}

/// Convert a git-supplied path into a vault id.
///
/// Vault ids always use `/`. On Windows libgit2 can hand back backslashes; on
/// Unix a backslash is a legal filename character, so the substitution is
/// confined to the platform that needs it.
pub(crate) fn id_from_str(raw: &str) -> String {
    normalize_separators(raw.to_owned())
}

/// The same conversion for the byte paths the index hands back.
pub(crate) fn id_from_bytes(path: &[u8]) -> String {
    normalize_separators(String::from_utf8_lossy(path).into_owned())
}

fn normalize_separators(raw: String) -> String {
    if cfg!(windows) {
        raw.replace('\\', "/")
    } else {
        raw
    }
}
