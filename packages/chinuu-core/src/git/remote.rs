use git2::build::CheckoutBuilder;
use git2::{BranchType, Direction, FetchOptions, PushOptions, RepositoryState};

use crate::{
    error::{CoreError, Result},
    git::auth::{remote_callbacks, Auth},
    git::repo::GitRepo,
};

/// A configured remote.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

/// A local branch.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    /// Remote-tracking branch this one follows, when one is configured.
    pub upstream: Option<String>,
    /// Commit the branch points at, as hex.
    pub target: Option<String>,
}

/// What a pull actually did.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullOutcome {
    /// Remote and local were already identical.
    UpToDate,
    /// Local moved forward to the remote commit.
    FastForward,
    /// A merge commit was created.
    Merged,
    /// The merge stopped with conflicts, which are now in the worktree.
    Conflicts,
}

/// What a push actually did.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushOutcome {
    /// The remote already had the commits.
    UpToDate,
    /// At least one ref was updated.
    Pushed,
}

impl GitRepo {
    /// Every configured remote, sorted by name.
    pub fn remotes(&self) -> Result<Vec<RemoteInfo>> {
        let names = self.repository().remotes()?;
        let mut out = Vec::new();
        for entry in names.iter() {
            // StringArray yields Result<Option<&str>>: a read failure, or an
            // entry that is absent.
            let Some(name) = entry? else { continue };
            let remote = self.repository().find_remote(name)?;
            out.push(RemoteInfo {
                name: name.to_owned(),
                url: remote.url().unwrap_or_default().to_owned(),
                push_url: remote.pushurl().ok().flatten().map(str::to_owned),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Add a remote, with the default fetch refspec for its name.
    pub fn add_remote(&self, name: &str, url: &str) -> Result<()> {
        self.repository().remote(name, url)?;
        Ok(())
    }

    pub fn remove_remote(&self, name: &str) -> Result<()> {
        self.repository()
            .remote_delete(name)
            .map_err(|_| CoreError::UnknownRemote(name.to_owned()))?;
        Ok(())
    }

    /// Fetch the remote's configured refspecs into remote-tracking refs.
    pub fn fetch(&self, name: &str, auth: &Auth) -> Result<()> {
        let config = self.repository().config()?;
        let mut remote = self
            .repository()
            .find_remote(name)
            .map_err(|_| CoreError::UnknownRemote(name.to_owned()))?;

        // The refspecs are passed explicitly on purpose. An empty refspec array
        // means "fetch nothing" to libgit2, so relying on the default here
        // would silently be a no-op.
        let configured = remote.fetch_refspecs()?;
        let mut refspecs: Vec<String> = Vec::new();
        for spec in configured.iter() {
            if let Some(spec) = spec? {
                refspecs.push(spec.to_owned());
            }
        }
        if refspecs.is_empty() {
            refspecs.push(format!("+refs/heads/*:refs/remotes/{name}/*"));
        }
        let refspecs: Vec<&str> = refspecs.iter().map(String::as_str).collect();

        let mut options = FetchOptions::new();
        options.remote_callbacks(remote_callbacks(config, auth));
        remote.fetch(&refspecs, Some(&mut options), Some("chinuu fetch"))?;
        Ok(())
    }

    /// Fetch, then integrate the remote branch into the local branch of the
    /// same name.
    ///
    /// Fast-forwards when possible and creates a merge commit otherwise. On
    /// conflicts it leaves the merge in progress and reports
    /// [`PullOutcome::Conflicts`] rather than aborting, so the caller can show
    /// the user what clashed. Use `abort_merge` to back out.
    pub fn pull(&self, remote_name: &str, branch: &str, auth: &Auth) -> Result<PullOutcome> {
        self.require_worktree()?;
        self.fetch(remote_name, auth)?;

        let repo = self.repository();
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetched = repo.reference_to_annotated_commit(&fetch_head)?;
        let (analysis, _) = repo.merge_analysis(&[&fetched])?;

        if analysis.is_up_to_date() {
            return Ok(PullOutcome::UpToDate);
        }

        if analysis.is_fast_forward() || analysis.is_unborn() {
            let refname = format!("refs/heads/{branch}");
            let target = repo.find_commit(fetched.id())?;
            // Update the worktree before moving the ref. If the checkout
            // refuses (local edits in the way), nothing has moved yet, so the
            // repository is still consistent.
            let mut checkout = CheckoutBuilder::new();
            checkout.safe();
            repo.checkout_tree(target.as_object(), Some(&mut checkout))?;
            repo.reference(&refname, fetched.id(), true, "pull: fast-forward")?;
            repo.set_head(&refname)?;
            return Ok(PullOutcome::FastForward);
        }

        repo.merge(&[&fetched], None, None)?;
        let mut index = repo.index()?;
        if index.has_conflicts() {
            return Ok(PullOutcome::Conflicts);
        }

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let signature = self.signature()?;
        let head_commit = repo.head()?.peel_to_commit()?;
        let fetched_commit = repo.find_commit(fetched.id())?;
        let message = format!("Merge {remote_name}/{branch} into {branch}");
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &[&head_commit, &fetched_commit],
        )?;
        repo.cleanup_state()?;
        Ok(PullOutcome::Merged)
    }

    /// Push the local branch to the branch of the same name on the remote.
    ///
    /// Keeps the remote-tracking ref in step, the way `git push` does, so ahead
    /// and behind stay correct without an immediate fetch.
    pub fn push(&self, remote_name: &str, branch: &str, auth: &Auth) -> Result<PushOutcome> {
        let repo = self.repository();
        let target_ref = format!("refs/heads/{branch}");
        let local_oid = self.local_branch(branch)?.get().target();

        let mut remote = repo
            .find_remote(remote_name)
            .map_err(|_| CoreError::UnknownRemote(remote_name.to_owned()))?;

        // libgit2 invokes `push_update_reference` even when a ref did not move,
        // so that callback cannot separate a real push from a no-op. Asking the
        // remote what it currently advertises can.
        let advertised = {
            let connection = remote.connect_auth(
                Direction::Push,
                Some(remote_callbacks(repo.config()?, auth)),
                None,
            )?;
            connection
                .list()?
                .iter()
                .find(|head| head.name() == target_ref.as_str())
                .map(|head| head.oid())
        };

        if local_oid.is_some() && advertised == local_oid {
            return Ok(PushOutcome::UpToDate);
        }

        let mut options = PushOptions::new();
        options.remote_callbacks(remote_callbacks(repo.config()?, auth));
        remote.push(&[target_ref.as_str()], Some(&mut options))?;

        // The remote now holds this commit, so record it locally as well.
        if let Some(oid) = local_oid {
            let tracking = format!("refs/remotes/{remote_name}/{branch}");
            repo.reference(&tracking, oid, true, "chinuu push: update tracking ref")?;
        }

        Ok(PushOutcome::Pushed)
    }

    /// Every local branch, sorted by name.
    pub fn branches(&self) -> Result<Vec<BranchInfo>> {
        let mut out = Vec::new();
        for entry in self.repository().branches(Some(BranchType::Local))? {
            let (branch, _) = entry?;
            let upstream = branch
                .upstream()
                .ok()
                .and_then(|up| up.name().ok().flatten().map(str::to_owned));
            out.push(BranchInfo {
                name: branch.name()?.unwrap_or_default().to_owned(),
                is_head: branch.is_head(),
                upstream,
                target: branch.get().target().map(|oid| oid.to_string()),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Create a branch at `at` (any revision), or at HEAD when `at` is `None`.
    pub fn create_branch(&self, name: &str, at: Option<&str>) -> Result<()> {
        let commit = match at {
            Some(rev) => self.repository().revparse_single(rev)?.peel_to_commit()?,
            None => self.repository().head()?.peel_to_commit()?,
        };
        self.repository().branch(name, &commit, false)?;
        Ok(())
    }

    /// Switch HEAD to an existing local branch and update the worktree.
    ///
    /// Refuses while another operation is in progress, and uses a safe
    /// checkout, so local edits are never silently discarded.
    pub fn checkout_branch(&self, name: &str) -> Result<()> {
        self.require_worktree()?;
        let repo = self.repository();
        if repo.state() != RepositoryState::Clean {
            return Err(CoreError::Git(git2::Error::from_str(
                "another git operation is in progress; resolve it before switching branches",
            )));
        }
        let target = self.local_branch(name)?.get().peel_to_tree()?;

        // Move the worktree first, then HEAD. If the checkout refuses because
        // local edits would be lost, HEAD has not moved and the repository is
        // still consistent. The reverse order, `set_head` followed by
        // `checkout_head`, reports success here without touching the worktree,
        // which silently leaves the wrong file contents in place.
        let mut checkout = CheckoutBuilder::new();
        checkout.safe();
        repo.checkout_tree(target.as_object(), Some(&mut checkout))?;
        repo.set_head(&format!("refs/heads/{name}"))?;
        Ok(())
    }

    /// Point a local branch at its remote-tracking counterpart.
    pub fn set_upstream(&self, branch: &str, remote_name: &str, remote_branch: &str) -> Result<()> {
        let mut local = self.local_branch(branch)?;
        local.set_upstream(Some(&format!("{remote_name}/{remote_branch}")))?;
        Ok(())
    }
}
