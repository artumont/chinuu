//! Integration tests for the git module.
//!
//! Every test builds real repositories in temp directories: normal, bare, and
//! clones of a bare one over libgit2's local transport. Nothing is mocked, so
//! these also check that the underlying libgit2 calls are wired correctly.

use std::{fs, path::Path};

use chinuu_core::{
    git::{Auth, ChangeKind, GitRepo, PullOutcome, PushOutcome},
    CoreError,
};

/// Init a repository with an identity configured, so commits work.
fn init_repo(root: &Path) -> GitRepo {
    let repo = GitRepo::init(root).unwrap();
    repo.set_identity("Chinuu Test", "test@chinuu.invalid")
        .unwrap();
    repo
}

/// Write `name`, stage it and commit it. Returns the new commit id.
fn commit_file(repo: &GitRepo, name: &str, content: &str, message: &str) -> String {
    fs::write(repo.root().join(name), content).unwrap();
    repo.stage(&[name]).unwrap();
    repo.commit(message).unwrap()
}

/// Set up a repo with one commit and return it with its branch name.
///
/// The branch name is read back rather than assumed, so the tests do not depend
/// on whatever `init.defaultBranch` happens to be on this machine.
fn repo_with_one_commit(root: &Path) -> (GitRepo, String) {
    let repo = init_repo(root);
    commit_file(&repo, "a.md", "# a\n", "first");
    let branch = repo
        .head_branch()
        .unwrap()
        .expect("branch after first commit");
    (repo, branch)
}

fn file_status<'a>(
    status: &'a chinuu_core::git::RepoStatus,
    id: &str,
) -> Option<&'a chinuu_core::git::FileStatus> {
    status.entries.iter().find(|e| e.id == id)
}

// ---------------------------------------------------------------- open and init

#[test]
fn init_creates_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    let repo = init_repo(dir.path());

    assert!(GitRepo::is_repo(dir.path()));
    assert!(repo.root().ends_with(dir.path().file_name().unwrap()));
    assert!(!repo.is_bare());
    assert!(!repo.has_commits().unwrap());
    assert!(repo.head_branch().unwrap().is_none(), "unborn HEAD");
}

#[test]
fn open_rejects_a_path_that_is_not_a_repository() {
    let dir = tempfile::tempdir().unwrap();

    let err = GitRepo::open(dir.path()).unwrap_err();
    assert!(matches!(err, CoreError::NotARepository(_)), "got {err:?}");
}

#[test]
fn discover_walks_up_to_the_repository_root() {
    let dir = tempfile::tempdir().unwrap();
    let repo = init_repo(dir.path());

    let nested = dir.path().join("notes/deep");
    fs::create_dir_all(&nested).unwrap();

    let found = GitRepo::discover(&nested).unwrap();
    assert_eq!(found.root(), repo.root());
}

#[test]
fn root_has_no_trailing_separator() {
    let dir = tempfile::tempdir().unwrap();
    let repo = init_repo(dir.path());

    // libgit2 returns a trailing slash; the handle normalises it away so that
    // joining and comparing paths behaves.
    assert_eq!(repo.root(), dir.path());
}

#[test]
fn a_bare_repository_has_no_worktree() {
    let dir = tempfile::tempdir().unwrap();
    let bare = GitRepo::init_bare(dir.path().join("remote.git")).unwrap();

    assert!(bare.is_bare());

    let err = bare.stage_all().unwrap_err();
    assert!(matches!(err, CoreError::BareRepository(_)), "got {err:?}");
}

// -------------------------------------------------------------------- identity

#[test]
fn identity_round_trips_through_the_repository_config() {
    let dir = tempfile::tempdir().unwrap();
    let repo = GitRepo::init(dir.path()).unwrap();

    repo.set_identity("Ada", "ada@example.com").unwrap();
    let signature = repo.signature().unwrap();

    assert_eq!(signature.name().unwrap(), "Ada");
    assert_eq!(signature.email().unwrap(), "ada@example.com");
}

// ---------------------------------------------------------------------- commit

#[test]
fn stage_and_commit_creates_one_log_entry() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    let log = repo.log(10).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].summary, "first");
    assert_eq!(log[0].author_name, "Chinuu Test");
    assert_eq!(log[0].author_email, "test@chinuu.invalid");
    assert!(log[0].parents.is_empty(), "first commit has no parent");
    assert_eq!(log[0].id.len(), 40, "full hex oid");
    assert!(repo.has_commits().unwrap());
}

#[test]
fn log_is_newest_first_and_honours_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());
    commit_file(&repo, "b.md", "# b\n", "second");
    commit_file(&repo, "c.md", "# c\n", "third");

    let all = repo.log(10).unwrap();
    assert_eq!(
        all.iter().map(|c| c.summary.as_str()).collect::<Vec<_>>(),
        vec!["third", "second", "first"]
    );
    assert_eq!(repo.log(2).unwrap().len(), 2);

    assert_eq!(all[0].parents.len(), 1);
    assert_eq!(all[0].parents[0], all[1].id);
}

#[test]
fn log_on_an_unborn_branch_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let repo = init_repo(dir.path());

    assert!(repo.log(10).unwrap().is_empty());
}

#[test]
fn committing_nothing_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    let err = repo.commit("empty").unwrap_err();
    assert!(matches!(err, CoreError::NothingStaged), "got {err:?}");
}

#[test]
fn commit_uses_the_staged_tree_not_the_worktree() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    // Stage one content, then change the file again without staging.
    fs::write(repo.root().join("a.md"), "# staged\n").unwrap();
    repo.stage(&["a.md"]).unwrap();
    fs::write(repo.root().join("a.md"), "# unstaged\n").unwrap();
    repo.commit("staged version").unwrap();

    // The commit holds the staged content, and the later edit is still pending.
    let status = repo.status().unwrap();
    let entry = file_status(&status, "a.md").expect("a.md is modified");
    assert_eq!(entry.staged, None, "already committed");
    assert_eq!(entry.unstaged, Some(ChangeKind::Modified));
}

// ---------------------------------------------------------------------- status

#[test]
fn status_is_clean_right_after_a_commit() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, branch) = repo_with_one_commit(dir.path());

    let status = repo.status().unwrap();
    assert!(status.is_clean(), "got {:?}", status.entries);
    assert!(status.is_empty());
    assert_eq!(status.branch.as_deref(), Some(branch.as_str()));
    assert!(!status.detached);
    assert_eq!((status.ahead, status.behind), (0, 0));
}

#[test]
fn status_separates_untracked_staged_and_modified() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    fs::write(repo.root().join("untracked.md"), "u\n").unwrap();
    fs::write(repo.root().join("a.md"), "# changed\n").unwrap();
    fs::write(repo.root().join("new.md"), "n\n").unwrap();
    repo.stage(&["new.md"]).unwrap();

    let status = repo.status().unwrap();

    let untracked = file_status(&status, "untracked.md").unwrap();
    assert_eq!(untracked.unstaged, Some(ChangeKind::Untracked));
    assert_eq!(untracked.staged, None);

    let modified = file_status(&status, "a.md").unwrap();
    assert_eq!(modified.unstaged, Some(ChangeKind::Modified));

    let added = file_status(&status, "new.md").unwrap();
    assert_eq!(added.staged, Some(ChangeKind::Added));

    assert_eq!(status.len(), 3);
    assert!(
        status.entries.windows(2).all(|w| w[0].id < w[1].id),
        "sorted by id"
    );
}

#[test]
fn status_reports_a_modified_path_on_both_sides() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    // Staged edit, then a further unstaged edit to the same path.
    fs::write(repo.root().join("a.md"), "# one\n").unwrap();
    repo.stage(&["a.md"]).unwrap();
    fs::write(repo.root().join("a.md"), "# two\n").unwrap();

    let status = repo.status().unwrap();
    let entry = file_status(&status, "a.md").unwrap();

    assert_eq!(entry.staged, Some(ChangeKind::Modified));
    assert_eq!(entry.unstaged, Some(ChangeKind::Modified));
    assert!(entry.is_staged() && entry.is_unstaged());
    assert_eq!(status.staged().len(), 1);
    assert_eq!(status.unstaged().len(), 1);
}

#[test]
fn status_ignores_ignored_paths() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    fs::write(repo.root().join(".gitignore"), "ignored.md\n").unwrap();
    repo.stage(&[".gitignore"]).unwrap();
    repo.commit("ignore").unwrap();

    fs::write(repo.root().join("ignored.md"), "x\n").unwrap();
    fs::write(repo.root().join("visible.md"), "x\n").unwrap();

    let status = repo.status().unwrap();
    assert!(file_status(&status, "ignored.md").is_none());
    assert!(file_status(&status, "visible.md").is_some());
}

// ------------------------------------------------------------- stage / unstage

#[test]
fn stage_all_picks_up_new_modified_and_deleted_paths() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());
    commit_file(&repo, "gone.md", "bye\n", "add gone.md");

    fs::write(repo.root().join("a.md"), "# edited\n").unwrap();
    fs::write(repo.root().join("fresh.md"), "new\n").unwrap();
    fs::remove_file(repo.root().join("gone.md")).unwrap();

    repo.stage_all().unwrap();
    let status = repo.status().unwrap();

    assert_eq!(
        file_status(&status, "fresh.md").unwrap().staged,
        Some(ChangeKind::Added)
    );
    assert_eq!(
        file_status(&status, "a.md").unwrap().staged,
        Some(ChangeKind::Modified)
    );
    assert_eq!(
        file_status(&status, "gone.md").unwrap().staged,
        Some(ChangeKind::Deleted)
    );
    assert!(status.staged().len() == 3);
}

#[test]
fn staging_a_deleted_path_records_a_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    fs::remove_file(repo.root().join("a.md")).unwrap();
    repo.stage(&["a.md"]).unwrap();

    let status = repo.status().unwrap();
    assert_eq!(
        file_status(&status, "a.md").unwrap().staged,
        Some(ChangeKind::Deleted)
    );
}

#[test]
fn unstage_returns_a_path_to_the_worktree() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    fs::write(repo.root().join("a.md"), "# changed\n").unwrap();
    repo.stage(&["a.md"]).unwrap();
    assert_eq!(
        file_status(&repo.status().unwrap(), "a.md").unwrap().staged,
        Some(ChangeKind::Modified)
    );

    repo.unstage(&["a.md"]).unwrap();
    let status = repo.status().unwrap();
    let entry = file_status(&status, "a.md").unwrap();
    assert_eq!(entry.staged, None);
    assert_eq!(entry.unstaged, Some(ChangeKind::Modified));
    // The worktree content survives unstaging.
    assert_eq!(
        fs::read_to_string(repo.root().join("a.md")).unwrap(),
        "# changed\n"
    );
}

#[test]
fn unstage_all_clears_the_index() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    fs::write(repo.root().join("a.md"), "# one\n").unwrap();
    fs::write(repo.root().join("b.md"), "b\n").unwrap();
    repo.stage_all().unwrap();
    assert_eq!(repo.status().unwrap().staged().len(), 2);

    repo.unstage_all().unwrap();
    let status = repo.status().unwrap();
    assert!(status.staged().is_empty());
    assert_eq!(status.unstaged().len(), 2);
}

// -------------------------------------------------------------------- branches

#[test]
fn branches_lists_the_created_branches() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, branch) = repo_with_one_commit(dir.path());
    repo.create_branch("feature", None).unwrap();

    let branches = repo.branches().unwrap();
    let names: Vec<_> = branches.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&branch.as_str()));
    assert!(names.contains(&"feature"));

    let head = branches.iter().find(|b| b.is_head).unwrap();
    assert_eq!(head.name, branch);
    assert!(head.target.is_some());
}

#[test]
fn checkout_branch_switches_the_worktree_content() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, main) = repo_with_one_commit(dir.path());

    repo.create_branch("feature", None).unwrap();
    repo.checkout_branch("feature").unwrap();
    assert_eq!(repo.head_branch().unwrap().as_deref(), Some("feature"));

    commit_file(&repo, "a.md", "# feature\n", "work on feature");

    repo.checkout_branch(&main).unwrap();
    assert_eq!(repo.head_branch().unwrap().as_deref(), Some(main.as_str()));
    assert_eq!(
        fs::read_to_string(repo.root().join("a.md")).unwrap(),
        "# a\n",
        "worktree follows the branch"
    );
}

#[test]
fn checkout_branch_refuses_to_clobber_local_edits() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, main) = repo_with_one_commit(dir.path());

    repo.create_branch("feature", None).unwrap();
    repo.checkout_branch("feature").unwrap();
    commit_file(&repo, "a.md", "# feature\n", "feature edit");
    repo.checkout_branch(&main).unwrap();

    // Dirty the file so switching branches would have to overwrite it.
    fs::write(repo.root().join("a.md"), "# local uncommitted\n").unwrap();

    let err = repo.checkout_branch("feature").unwrap_err();
    assert!(
        matches!(err, CoreError::Git(_)),
        "a safe checkout should refuse, got {err:?}"
    );
    assert_eq!(
        fs::read_to_string(repo.root().join("a.md")).unwrap(),
        "# local uncommitted\n",
        "the local edit is untouched"
    );
}

#[test]
fn checkout_branch_rejects_an_unknown_branch() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    let err = repo.checkout_branch("nope").unwrap_err();
    assert!(matches!(err, CoreError::UnknownBranch(_)), "got {err:?}");
}

// --------------------------------------------------------------------- remotes

#[test]
fn remotes_can_be_added_listed_and_removed() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    assert!(repo.remotes().unwrap().is_empty());

    repo.add_remote("origin", "https://example.invalid/repo.git")
        .unwrap();
    let remotes = repo.remotes().unwrap();
    assert_eq!(remotes.len(), 1);
    assert_eq!(remotes[0].name, "origin");
    assert_eq!(remotes[0].url, "https://example.invalid/repo.git");

    repo.remove_remote("origin").unwrap();
    assert!(repo.remotes().unwrap().is_empty());
}

#[test]
fn unknown_remote_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    let err = repo.remote_url("nope").unwrap_err();
    assert!(
        matches!(&err, CoreError::UnknownRemote(name) if name == "nope"),
        "got {err:?}"
    );

    let err = repo.remove_remote("nope").unwrap_err();
    assert!(
        matches!(&err, CoreError::UnknownRemote(name) if name == "nope"),
        "got {err:?}"
    );
}

// ---------------------------------------------------------------- push and pull

/// A bare repo plus a working clone wired to it.
struct Fixture {
    _dir: tempfile::TempDir,
    remote: GitRepo,
    work: GitRepo,
    branch: String,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let remote = GitRepo::init_bare(dir.path().join("remote.git")).unwrap();
    let remote_url = remote.root().to_str().unwrap().to_owned();

    let work = init_repo(&dir.path().join("work"));
    commit_file(&work, "shared.md", "base\n", "base commit");
    let branch = work.head_branch().unwrap().unwrap();
    work.add_remote("origin", &remote_url).unwrap();

    Fixture {
        _dir: dir,
        remote,
        work,
        branch,
    }
}

#[test]
fn push_updates_the_remote_then_reports_up_to_date() {
    let f = fixture();

    assert_eq!(
        f.work.push("origin", &f.branch, &Auth::Default).unwrap(),
        PushOutcome::Pushed
    );
    // The remote now has the branch.
    assert!(f.remote.has_commits().unwrap());

    assert_eq!(
        f.work.push("origin", &f.branch, &Auth::Default).unwrap(),
        PushOutcome::UpToDate
    );
}

#[test]
fn clone_sees_what_was_pushed() {
    let f = fixture();
    f.work.push("origin", &f.branch, &Auth::Default).unwrap();

    let clone = GitRepo::clone(
        f.remote.root().to_str().unwrap(),
        f._dir.path().join("clone"),
    )
    .unwrap();

    assert_eq!(
        clone.head_branch().unwrap().as_deref(),
        Some(f.branch.as_str())
    );
    assert_eq!(
        fs::read_to_string(clone.root().join("shared.md")).unwrap(),
        "base\n"
    );
    assert_eq!(clone.log(10).unwrap().len(), 1);
}

#[test]
fn pull_fast_forwards_a_behind_clone() {
    let f = fixture();
    f.work.push("origin", &f.branch, &Auth::Default).unwrap();

    let clone = GitRepo::clone(
        f.remote.root().to_str().unwrap(),
        f._dir.path().join("clone"),
    )
    .unwrap();
    clone.set_identity("Clone", "clone@chinuu.invalid").unwrap();

    // A second commit on the original, pushed to the remote.
    commit_file(&f.work, "shared.md", "from-work\n", "second");
    f.work.push("origin", &f.branch, &Auth::Default).unwrap();

    let outcome = clone.pull("origin", &f.branch, &Auth::Default).unwrap();
    assert_eq!(outcome, PullOutcome::FastForward);
    assert_eq!(clone.log(10).unwrap().len(), 2);
    assert_eq!(
        fs::read_to_string(clone.root().join("shared.md")).unwrap(),
        "from-work\n"
    );

    // Pulling again has nothing to do.
    assert_eq!(
        clone.pull("origin", &f.branch, &Auth::Default).unwrap(),
        PullOutcome::UpToDate
    );
}

#[test]
fn pull_reports_conflicts_and_abort_restores_the_worktree() {
    let f = fixture();
    f.work.push("origin", &f.branch, &Auth::Default).unwrap();

    let clone_dir = f._dir.path().join("clone");
    let clone = GitRepo::clone(f.remote.root().to_str().unwrap(), &clone_dir).unwrap();
    clone.set_identity("Clone", "clone@chinuu.invalid").unwrap();

    // Diverge: the original and the clone both edit the same line.
    commit_file(&f.work, "shared.md", "from-work\n", "work edit");
    f.work.push("origin", &f.branch, &Auth::Default).unwrap();
    commit_file(&clone, "shared.md", "from-clone\n", "clone edit");

    let outcome = clone.pull("origin", &f.branch, &Auth::Default).unwrap();
    assert_eq!(outcome, PullOutcome::Conflicts);
    assert!(clone.is_merging());
    assert!(clone.is_operation_in_progress());
    assert!(clone.has_conflicts().unwrap());

    let conflicts = clone.conflicts().unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].id, "shared.md");
    assert!(conflicts[0].ours.is_some());
    assert!(conflicts[0].theirs.is_some());
    assert!(
        conflicts[0].ancestor.is_some(),
        "a shared base commit exists"
    );

    // Committing while conflicted must refuse rather than record markers.
    assert!(matches!(
        clone.commit("oops").unwrap_err(),
        CoreError::Conflicts
    ));

    clone.abort_merge().unwrap();
    assert!(!clone.has_conflicts().unwrap());
    assert!(!clone.is_merging());
    assert!(clone.conflicts().unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(clone.root().join("shared.md")).unwrap(),
        "from-clone\n",
        "abort restores the clone's own commit"
    );
}

#[test]
fn abort_merge_is_a_no_op_when_clean() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, _) = repo_with_one_commit(dir.path());

    assert!(!repo.is_operation_in_progress());
    repo.abort_merge().unwrap();
}

#[test]
fn ahead_and_behind_counts_unpushed_commits() {
    let f = fixture();
    f.work.push("origin", &f.branch, &Auth::Default).unwrap();
    // Ahead and behind are measured against the configured upstream, so set it
    // up the way a first `git push -u` would.
    f.work.set_upstream(&f.branch, "origin", &f.branch).unwrap();

    commit_file(&f.work, "shared.md", "one\n", "ahead one");
    commit_file(&f.work, "shared.md", "two\n", "ahead two");

    let status = f.work.status().unwrap();
    assert_eq!(status.ahead, 2, "two commits not yet pushed");
    assert_eq!(status.behind, 0);

    f.work.push("origin", &f.branch, &Auth::Default).unwrap();
    assert_eq!(f.work.status().unwrap().ahead, 0);
}
