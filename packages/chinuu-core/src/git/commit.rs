use std::path::Path;

use git2::{IndexAddOption, Sort};

use crate::{
    error::{CoreError, Result},
    git::repo::GitRepo,
};

/// One entry from the commit log.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    /// Full hex object id.
    pub id: String,
    /// First line of the message.
    pub summary: String,
    /// Full message, including the summary line.
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    /// Author time, seconds since the Unix epoch.
    pub time: i64,
    /// Author timezone offset, minutes from UTC.
    pub offset_minutes: i32,
    pub parents: Vec<String>,
}

impl GitRepo {
    /// Stage the given vault-relative paths.
    ///
    /// A path that no longer exists in the worktree is staged as a deletion,
    /// which is what `git add` does for a removed file.
    pub fn stage(&self, paths: &[&str]) -> Result<()> {
        self.require_worktree()?;
        let mut index = self.repository().index()?;
        for id in paths {
            let relative = Path::new(id);
            if self.root().join(relative).exists() {
                index.add_path(relative)?;
            } else {
                index.remove_path(relative)?;
            }
        }
        index.write()?;
        Ok(())
    }

    /// Stage every change, deletions included, honouring the ignore rules.
    ///
    /// `add_all` covers new and modified paths, `update_all` covers tracked
    /// paths that were deleted. Neither picks up ignored files.
    pub fn stage_all(&self) -> Result<()> {
        self.require_worktree()?;
        let mut index = self.repository().index()?;
        index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
        index.update_all(["*"], None)?;
        index.write()?;
        Ok(())
    }

    /// Unstage the given paths, leaving the worktree untouched.
    pub fn unstage(&self, paths: &[&str]) -> Result<()> {
        let head = self.head_object()?;
        self.repository().reset_default(head.as_ref(), paths)?;
        Ok(())
    }

    /// Unstage everything, leaving the worktree untouched.
    pub fn unstage_all(&self) -> Result<()> {
        let head = self.head_object()?;
        self.repository().reset_default(head.as_ref(), ["*"])?;
        Ok(())
    }

    /// Commit whatever is staged, returning the new commit id as hex.
    ///
    /// Refuses an empty commit and refuses to commit while conflicts are
    /// unresolved. An accidental empty commit is noise nobody asked for.
    pub fn commit(&self, message: &str) -> Result<String> {
        let repo = self.repository();
        let mut index = repo.index()?;
        if index.has_conflicts() {
            return Err(CoreError::Conflicts);
        }

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let signature = self.signature()?;

        let parent = self.head_commit()?;
        if let Some(parent) = &parent {
            if parent.tree_id() == tree_id {
                return Err(CoreError::NothingStaged);
            }
        }

        let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
        let oid = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?;
        Ok(oid.to_string())
    }

    /// The most recent commits, newest first.
    pub fn log(&self, limit: usize) -> Result<Vec<CommitInfo>> {
        let repo = self.repository();
        let mut walk = repo.revwalk()?;
        // Topological order, newest first. Time alone is not topological: with
        // equal timestamps libgit2 can emit a parent before its child, which is
        // exactly what a log must not do.
        walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
        if walk.push_head().is_err() {
            // Unborn HEAD: a repository with no commits has no log.
            return Ok(Vec::new());
        }

        let mut out = Vec::new();
        for oid in walk.take(limit) {
            let commit = repo.find_commit(oid?)?;
            let author = commit.author();
            let when = author.when();
            out.push(CommitInfo {
                id: commit.id().to_string(),
                summary: commit.summary()?.unwrap_or_default().to_owned(),
                message: commit.message()?.to_owned(),
                author_name: author.name().unwrap_or_default().to_owned(),
                author_email: author.email().unwrap_or_default().to_owned(),
                time: when.seconds(),
                offset_minutes: when.offset_minutes(),
                parents: commit.parent_ids().map(|p| p.to_string()).collect(),
            });
        }
        Ok(out)
    }

    /// The commit HEAD resolves to, or `None` before the first commit.
    pub(crate) fn head_commit(&self) -> Result<Option<git2::Commit<'_>>> {
        match self.repository().head() {
            Ok(head) => Ok(Some(head.peel_to_commit()?)),
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// HEAD as an `Object`, which is what `reset_default` wants.
    fn head_object(&self) -> Result<Option<git2::Object<'_>>> {
        Ok(self.head_commit()?.map(|commit| commit.into_object()))
    }
}
