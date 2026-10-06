use git2::build::CheckoutBuilder;
use git2::{RepositoryState, ResetType};

use crate::{
    error::Result,
    git::repo::{id_from_bytes, GitRepo},
};

/// One unresolved conflict in the index.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// Vault-relative path, always `/` separated.
    pub id: String,
    /// Blob id from the common ancestor, when there is one.
    pub ancestor: Option<String>,
    /// Blob id from the local side.
    pub ours: Option<String>,
    /// Blob id from the incoming side.
    pub theirs: Option<String>,
}

impl GitRepo {
    /// Whether any operation is mid-flight, including merges, reverts,
    /// cherry-picks and rebases.
    pub fn is_operation_in_progress(&self) -> bool {
        self.repository().state() != RepositoryState::Clean
    }

    pub fn is_merging(&self) -> bool {
        self.repository().state() == RepositoryState::Merge
    }

    pub fn has_conflicts(&self) -> Result<bool> {
        Ok(self.repository().index()?.has_conflicts())
    }

    /// Every unresolved conflict, sorted by path, with the blob id on each side.
    pub fn conflicts(&self) -> Result<Vec<Conflict>> {
        let index = self.repository().index()?;
        let mut out = Vec::new();
        for conflict in index.conflicts()? {
            let conflict = conflict?;
            let id = conflict
                .our
                .as_ref()
                .or(conflict.their.as_ref())
                .or(conflict.ancestor.as_ref())
                .map(|entry| id_from_bytes(&entry.path))
                .unwrap_or_default();
            out.push(Conflict {
                id,
                ancestor: conflict.ancestor.as_ref().map(|e| e.id.to_string()),
                ours: conflict.our.as_ref().map(|e| e.id.to_string()),
                theirs: conflict.their.as_ref().map(|e| e.id.to_string()),
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// Throw away an in-flight operation and hard reset to HEAD.
    ///
    /// This discards uncommitted work, which is what `git merge --abort` does,
    /// so callers should confirm with the user first. A no-op when the
    /// repository is already clean.
    pub fn abort_merge(&self) -> Result<()> {
        self.require_worktree()?;
        let repo = self.repository();
        if repo.state() == RepositoryState::Clean {
            return Ok(());
        }

        let head = repo.head()?.peel_to_commit()?.into_object();
        let mut checkout = CheckoutBuilder::new();
        checkout.force();
        repo.reset(&head, ResetType::Hard, Some(&mut checkout))?;
        repo.cleanup_state()?;
        Ok(())
    }
}
