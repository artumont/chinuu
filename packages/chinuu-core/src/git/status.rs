use git2::{BranchType, Status, StatusOptions};

use crate::{
    error::Result,
    git::repo::{id_from_str, GitRepo},
};

/// How a path changed, on one side of the index.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    // Matches `ChangeKind::as_str`.
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    TypeChange,
    Untracked,
    Conflicted,
}

impl ChangeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChangeKind::Added => "added",
            ChangeKind::Modified => "modified",
            ChangeKind::Deleted => "deleted",
            ChangeKind::Renamed => "renamed",
            ChangeKind::TypeChange => "typechange",
            ChangeKind::Untracked => "untracked",
            ChangeKind::Conflicted => "conflicted",
        }
    }
}

/// One path's status, split into the staged and unstaged sides.
///
/// A path can appear on both sides at once, for example a new file that was
/// staged and then edited again.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStatus {
    /// Vault-relative path, always `/` separated.
    pub id: String,
    /// What the index has staged relative to HEAD.
    pub staged: Option<ChangeKind>,
    /// What the worktree has relative to the index.
    pub unstaged: Option<ChangeKind>,
}

impl FileStatus {
    pub fn is_staged(&self) -> bool {
        self.staged.is_some()
    }

    pub fn is_unstaged(&self) -> bool {
        self.unstaged.is_some()
    }

    pub fn is_conflicted(&self) -> bool {
        self.staged == Some(ChangeKind::Conflicted) || self.unstaged == Some(ChangeKind::Conflicted)
    }
}

/// A snapshot of the repository's working state.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepoStatus {
    /// Current branch, or `None` when detached or unborn.
    pub branch: Option<String>,
    pub detached: bool,
    /// Every changed path, sorted by id.
    pub entries: Vec<FileStatus>,
    /// Commits the local branch has that upstream does not.
    pub ahead: usize,
    /// Commits upstream has that the local branch does not.
    pub behind: usize,
}

impl RepoStatus {
    /// True when nothing is staged, unstaged or conflicted.
    pub fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Paths with something staged. Includes paths that also have unstaged edits.
    pub fn staged(&self) -> Vec<&FileStatus> {
        self.entries.iter().filter(|e| e.is_staged()).collect()
    }

    /// Paths with unstaged edits. Includes paths that also have staged edits.
    pub fn unstaged(&self) -> Vec<&FileStatus> {
        self.entries.iter().filter(|e| e.is_unstaged()).collect()
    }

    pub fn conflicted(&self) -> Vec<&FileStatus> {
        self.entries.iter().filter(|e| e.is_conflicted()).collect()
    }
}

fn staged_kind(status: Status) -> Option<ChangeKind> {
    if status.contains(Status::CONFLICTED) {
        return None;
    }
    if status.contains(Status::INDEX_NEW) {
        Some(ChangeKind::Added)
    } else if status.contains(Status::INDEX_MODIFIED) {
        Some(ChangeKind::Modified)
    } else if status.contains(Status::INDEX_DELETED) {
        Some(ChangeKind::Deleted)
    } else if status.contains(Status::INDEX_RENAMED) {
        Some(ChangeKind::Renamed)
    } else if status.contains(Status::INDEX_TYPECHANGE) {
        Some(ChangeKind::TypeChange)
    } else {
        None
    }
}

fn unstaged_kind(status: Status) -> Option<ChangeKind> {
    if status.contains(Status::CONFLICTED) {
        return Some(ChangeKind::Conflicted);
    }
    if status.contains(Status::WT_NEW) {
        Some(ChangeKind::Untracked)
    } else if status.contains(Status::WT_MODIFIED) {
        Some(ChangeKind::Modified)
    } else if status.contains(Status::WT_DELETED) {
        Some(ChangeKind::Deleted)
    } else if status.contains(Status::WT_RENAMED) {
        Some(ChangeKind::Renamed)
    } else if status.contains(Status::WT_TYPECHANGE) {
        Some(ChangeKind::TypeChange)
    } else {
        None
    }
}

impl GitRepo {
    /// Every staged, unstaged or conflicted path, plus branch and ahead/behind.
    ///
    /// Ignored paths are excluded. `update_index` is on, so the result reflects
    /// the worktree even when file mtimes alone would look unchanged.
    pub fn status(&self) -> Result<RepoStatus> {
        let mut options = StatusOptions::new();
        options
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false)
            .include_unmodified(false)
            .renames_head_to_index(true)
            .renames_index_to_workdir(true)
            .update_index(true);

        let statuses = self.repository().statuses(Some(&mut options))?;

        let mut entries = Vec::new();
        for entry in statuses.iter() {
            let status = entry.status();
            let staged = staged_kind(status);
            let unstaged = unstaged_kind(status);
            if staged.is_none() && unstaged.is_none() {
                continue;
            }
            entries.push(FileStatus {
                id: id_from_str(entry.path()?),
                staged,
                unstaged,
            });
        }
        entries.sort_by(|a, b| a.id.cmp(&b.id));

        let (ahead, behind) = self.ahead_behind()?;

        Ok(RepoStatus {
            branch: self.head_branch()?,
            detached: self.is_detached()?,
            entries,
            ahead,
            behind,
        })
    }

    /// `(ahead, behind)` against the current branch's upstream.
    ///
    /// Returns `(0, 0)` when there is no upstream, no commits yet, or HEAD is
    /// detached. There is nothing meaningful to compare in those cases, and
    /// reporting an error would just mean every caller special-cases them.
    pub fn ahead_behind(&self) -> Result<(usize, usize)> {
        let repo = self.repository();
        let Ok(head) = repo.head() else {
            return Ok((0, 0));
        };
        let Some(local) = head.target() else {
            return Ok((0, 0));
        };
        let Ok(name) = head.shorthand() else {
            return Ok((0, 0));
        };
        let Ok(branch) = repo.find_branch(name, BranchType::Local) else {
            return Ok((0, 0));
        };
        let Ok(upstream) = branch.upstream() else {
            return Ok((0, 0));
        };
        let Some(remote) = upstream.get().target() else {
            return Ok((0, 0));
        };
        Ok(repo.graph_ahead_behind(local, remote)?)
    }
}
