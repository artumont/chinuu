use std::path::Path;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::error::{CoreError, Result};

/// The ignore file a vault may carry at its root.
pub const IGNORE_FILE_NAME: &str = ".chinuuignore";

/// Patterns applied to every vault, before the vault's own rules.
///
/// `.git` is here because our own git operations churn it, and the ignore file
/// itself because it is configuration rather than a note. The pattern covering
/// an atomic write's temporary file is added by [`IgnoreRules::defaults`]
/// instead of being written out here, so it cannot drift from the constant that
/// names those files.
pub const DEFAULT_PATTERNS: &[&str] = &[".git", IGNORE_FILE_NAME];

/// Rules deciding which vault paths are hidden.
///
/// Cheap to clone. Ids are matched, never filesystem paths, so the same rules
/// give the same answers for a vault wherever it is rooted.
#[derive(Debug, Clone)]
pub struct IgnoreRules {
    /// The patterns as written, one entry per line. Kept because the compiled
    /// matcher cannot be extended after it is built, and a caller adding one
    /// pattern should not have to rebuild the whole set by hand.
    patterns: Vec<String>,
    matcher: Gitignore,
}

impl Default for IgnoreRules {
    /// The built-in rules only; see [`IgnoreRules::defaults`].
    fn default() -> Self {
        Self::defaults()
    }
}

impl IgnoreRules {
    /// Rules that ignore nothing.
    pub fn empty() -> Self {
        Self {
            patterns: Vec::new(),
            matcher: Gitignore::empty(),
        }
    }

    /// The built-in patterns, [`DEFAULT_PATTERNS`] plus the temporary file glob.
    pub fn defaults() -> Self {
        let mut rules = Self::empty();
        for pattern in DEFAULT_PATTERNS {
            // Constants, so this cannot fail on user input. The
            // `default_patterns_apply` test pins that it does not.
            rules
                .add_line(pattern)
                .expect("built-in ignore patterns are valid");
        }

        // Derived from the writer's own prefix so the two cannot disagree. An
        // atomic write goes through a file matching this, and it must never
        // reach the tree or a watch event.
        rules
            .add_line(&format!("{}*", crate::files::write::TEMP_PREFIX))
            .expect("the temporary file pattern is valid");

        rules
    }

    /// Rules from `text` alone, without the built-in patterns.
    ///
    /// Comments and blank lines are skipped, as in gitignore. Use
    /// [`IgnoreRules::for_vault`] for the built-ins plus a vault's own file.
    pub fn parse(text: &str) -> Result<Self> {
        let mut rules = Self::empty();
        rules.add_text(text)?;
        Ok(rules)
    }

    /// The built-in patterns plus the vault's own `.chinuuignore`.
    ///
    /// A vault with no ignore file is entirely normal and yields just the
    /// built-ins. A file that exists but cannot be read, or that holds a pattern
    /// which will not compile, is an error rather than a silent omission.
    pub fn for_vault(root: impl AsRef<Path>) -> Result<Self> {
        let mut rules = Self::defaults();
        let path = root.as_ref().join(IGNORE_FILE_NAME);

        match std::fs::read_to_string(&path) {
            Ok(text) => rules.add_text(&text)?,
            // Absent is the common case, not a failure.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(CoreError::io(&path, e)),
        }

        Ok(rules)
    }

    /// Add one pattern, in gitignore syntax.
    pub fn add_line(&mut self, line: &str) -> Result<()> {
        self.add_text(line)
    }

    /// Add several patterns.
    pub fn add_patterns<I, S>(&mut self, patterns: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for pattern in patterns {
            self.add_line(pattern.as_ref())?;
        }
        Ok(())
    }

    /// Whether a vault-relative id is hidden.
    ///
    /// `is_dir` matters because a pattern ending in `/` matches directories
    /// only, and a caller reporting a path that no longer exists has to say what
    /// it used to be.
    ///
    /// Ignores ancestors too, so a caller need not have pruned the walk: if
    /// `private/` is ignored then `private/keep.md` is as well.
    pub fn is_ignored(&self, id: &str, is_dir: bool) -> bool {
        // The root is not a path that can be hidden, and an empty id would match
        // nothing anyway.
        if id.is_empty() || self.matcher.is_empty() {
            return false;
        }

        // Ids are vault-relative by construction. An absolute path here is a
        // caller mistake, and the matcher would panic on it rather than return.
        if Path::new(id).has_root() {
            return false;
        }

        self.matcher
            .matched_path_or_any_parents(id, is_dir)
            .is_ignore()
    }

    /// The patterns as written, in order.
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }

    /// How many patterns are in force. Comments and blank lines do not count.
    pub fn len(&self) -> usize {
        self.matcher.len()
    }

    pub fn is_empty(&self) -> bool {
        self.matcher.is_empty()
    }

    /// Append text and rebuild the matcher once, so a whole file costs one
    /// compile rather than one per line.
    fn add_text(&mut self, text: &str) -> Result<()> {
        let mut patterns = self.patterns.clone();
        patterns.extend(text.lines().map(str::to_owned));

        let matcher = compile(&patterns)?;

        self.patterns = patterns;
        self.matcher = matcher;
        Ok(())
    }
}

/// Compile a pattern set. An empty root is correct here: rules are matched
/// against vault-relative ids, so `/` in a pattern anchors to the vault root.
fn compile(patterns: &[String]) -> Result<Gitignore> {
    let mut builder = GitignoreBuilder::new("");

    for pattern in patterns {
        builder
            .add_line(None, pattern)
            .map_err(|e| CoreError::InvalidIgnorePattern(format!("`{pattern}`: {e}")))?;
    }

    builder
        .build()
        .map_err(|e| CoreError::InvalidIgnorePattern(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(text: &str) -> IgnoreRules {
        IgnoreRules::parse(text).unwrap()
    }

    #[test]
    fn default_patterns_apply() {
        let rules = IgnoreRules::defaults();

        assert!(rules.is_ignored(".git", true));
        assert!(rules.is_ignored(".git/config", false));
        assert!(rules.is_ignored(".chinuuignore", false));
        assert!(!rules.is_ignored("a.md", false));
        assert!(!rules.is_ignored("notes/a.md", false));
    }

    #[test]
    fn an_empty_set_ignores_nothing() {
        let rules = IgnoreRules::empty();

        assert!(rules.is_empty());
        assert_eq!(rules.len(), 0);
        assert!(!rules.is_ignored(".git", true));
        assert!(!rules.is_ignored("anything", false));
    }

    #[test]
    fn a_bare_name_matches_at_any_depth() {
        let rules = rules("*.tmp");

        assert!(rules.is_ignored("a.tmp", false));
        assert!(rules.is_ignored("notes/a.tmp", false));
        assert!(rules.is_ignored("notes/deep/a.tmp", false));
        assert!(!rules.is_ignored("a.md", false));
    }

    #[test]
    fn a_leading_slash_anchors_to_the_vault_root() {
        let rules = rules("/scratch.md");

        assert!(rules.is_ignored("scratch.md", false));
        assert!(!rules.is_ignored("notes/scratch.md", false));
    }

    #[test]
    fn a_pattern_with_a_slash_is_anchored_without_a_leading_slash() {
        let rules = rules("notes/draft.md");

        assert!(rules.is_ignored("notes/draft.md", false));
        assert!(!rules.is_ignored("draft.md", false));
        assert!(!rules.is_ignored("deep/notes/draft.md", false));
    }

    #[test]
    fn a_trailing_slash_matches_directories_only() {
        let rules = rules("private/");

        assert!(rules.is_ignored("private", true));
        assert!(rules.is_ignored("notes/private", true));
        assert!(!rules.is_ignored("private", false), "a file of that name");
        assert!(!rules.is_ignored("notes/private", false));
    }

    #[test]
    fn a_match_covers_the_children_of_an_ignored_directory() {
        let rules = rules("private/");

        // No need for the caller to have pruned anything.
        assert!(rules.is_ignored("private/keep.md", false));
        assert!(rules.is_ignored("notes/private/deep/x.md", false));
    }

    #[test]
    fn a_negation_re_includes_a_path() {
        let rules = rules("private/\n!private/keep.md");

        assert!(rules.is_ignored("private/other.md", false));
        assert!(!rules.is_ignored("private/keep.md", false));
    }

    #[test]
    fn the_last_matching_line_wins() {
        let rules = rules("*.md\n!notes/keep.md\nnotes/keep.md\n!notes/keep.md");

        // The final line is a re-include, so it wins.
        assert!(!rules.is_ignored("notes/keep.md", false));
        assert!(rules.is_ignored("notes/other.md", false));
    }

    #[test]
    fn a_wildcard_parent_matches_directories_at_any_depth() {
        let rules = rules("node_modules/");

        assert!(rules.is_ignored("node_modules", true));
        assert!(rules.is_ignored("apps/desktop/node_modules", true));
        assert!(rules.is_ignored("apps/desktop/node_modules/left-pad/index.js", false));
    }

    #[test]
    fn a_double_star_scopes_a_subtree() {
        let rules = rules("journal/2024/**");

        assert!(rules.is_ignored("journal/2024/a.md", false));
        assert!(rules.is_ignored("journal/2024/deep/b.md", false));
        assert!(!rules.is_ignored("journal/2023/a.md", false));
    }

    #[test]
    fn comments_and_blank_lines_are_skipped_and_not_counted() {
        let rules = rules("# a comment\n\n*.tmp\n\n# another\n");

        assert_eq!(rules.len(), 1);
        assert!(rules.is_ignored("a.tmp", false));
        assert!(!rules.is_ignored("# a comment", false));
    }

    #[test]
    fn the_root_id_is_never_ignored() {
        let rules = rules("**\n/");

        assert!(!rules.is_ignored("", false));
        assert!(!rules.is_ignored("", true));
    }

    #[test]
    fn an_absolute_path_is_refused_rather_than_panicking() {
        let rules = rules("*.tmp");

        // The matcher panics on a path with a root, so this must not reach it.
        assert!(!rules.is_ignored("/home/me/vault/a.tmp", false));
    }

    #[test]
    fn an_unparsable_pattern_is_reported_with_the_pattern() {
        // An unclosed character class such as `a[.md` is accepted by the engine
        // as a literal, so use a pattern it genuinely refuses.
        let err = IgnoreRules::parse("[z-a].md").unwrap_err();

        assert!(
            matches!(err, CoreError::InvalidIgnorePattern(_)),
            "got {err:?}"
        );
        assert!(err.to_string().contains("[z-a].md"), "got {err}");
    }

    #[test]
    fn adding_patterns_keeps_the_earlier_ones() {
        let mut rules = IgnoreRules::parse("*.tmp").unwrap();
        rules.add_patterns(["private/", "*.bak"]).unwrap();

        assert!(rules.is_ignored("a.tmp", false));
        assert!(rules.is_ignored("private", true));
        assert!(rules.is_ignored("a.bak", false));
        assert_eq!(rules.len(), 3);
    }

    #[test]
    fn a_failed_addition_leaves_the_rules_untouched() {
        let mut rules = IgnoreRules::parse("*.tmp").unwrap();

        assert!(rules.add_line("[z-a].md").is_err());

        // The bad pattern was not kept, and the good one still works.
        assert_eq!(rules.len(), 1);
        assert!(rules.is_ignored("a.tmp", false));
    }

    #[test]
    fn for_vault_reads_the_ignore_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(IGNORE_FILE_NAME), "*.tmp\nprivate/\n").unwrap();

        let rules = IgnoreRules::for_vault(dir.path()).unwrap();

        // The file's patterns and the built-ins both apply.
        assert!(rules.is_ignored("a.tmp", false));
        assert!(rules.is_ignored("private", true));
        assert!(rules.is_ignored(".git", true));
    }

    #[test]
    fn for_vault_without_a_file_yields_the_built_ins() {
        let dir = tempfile::tempdir().unwrap();

        let rules = IgnoreRules::for_vault(dir.path()).unwrap();

        assert!(rules.is_ignored(".git", true));
        assert!(!rules.is_ignored("a.tmp", false));
    }

    #[test]
    fn for_vault_reports_a_bad_pattern_in_the_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(IGNORE_FILE_NAME), "*.tmp\n[z-a].md\n").unwrap();

        let err = IgnoreRules::for_vault(dir.path()).unwrap_err();

        assert!(
            matches!(err, CoreError::InvalidIgnorePattern(_)),
            "got {err:?}"
        );
    }
}
