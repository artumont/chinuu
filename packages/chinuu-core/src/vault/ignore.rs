use std::path::Path;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::vault::error::VaultError;

pub const IGNORE_FILENAME: &str = ".chinuuignore";
pub const DEFAULT_PATTERNS: &[&str] = &[".git", ".chinuu", ".chinuuignore"];

/// Index node ignore rules.
#[derive(Debug, Clone)]
pub struct IgnoreRules {
    patterns: Vec<String>,
    matcher: Gitignore,
}

impl IgnoreRules {
    /// Build ignore rules from the `.chinuuignore` file in root.
    pub fn build(root: impl AsRef<Path>) -> Result<Self, VaultError> {
        let mut rules = Self::defaults();
        let path = root.as_ref().join(IGNORE_FILENAME);

        match std::fs::read_to_string(&path) {
            Ok(text) => rules.add_text(&text)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(VaultError::io(&path, e)),
        }

        Ok(rules)
    }

    /// Check if a file is a part of the ignore rules.
    pub fn is_ignored(&self, path: impl AsRef<Path>, is_dir: bool) -> bool {
        let path = path.as_ref();
        self.matcher.matched(path, is_dir).is_ignore()
    }

    fn empty() -> IgnoreRules {
        Self {
            patterns: Vec::new(),
            matcher: Gitignore::empty(),
        }
    }

    fn defaults() -> IgnoreRules {
        let mut rules = Self::empty();

        for pattern in DEFAULT_PATTERNS {
            rules
                .add_line(pattern)
                .expect("built-in ignore patterns are valid");
        }

        rules
    }

    fn add_line(&mut self, line: &str) -> Result<(), VaultError> {
        self.add_text(line)
    }

    fn add_text(&mut self, text: &str) -> Result<(), VaultError> {
        let mut patterns = self.patterns.clone();
        patterns.extend(text.lines().map(str::to_owned));

        let matcher = compile(&patterns)?;

        self.patterns = patterns;
        self.matcher = matcher;
        Ok(())
    }
}

impl Default for IgnoreRules {
    fn default() -> Self {
        Self::defaults()
    }
}

fn compile(patterns: &[String]) -> Result<Gitignore, VaultError> {
    let mut builder = GitignoreBuilder::new("");

    for pattern in patterns {
        builder
            .add_line(None, pattern)
            .map_err(|e| VaultError::InvalidIgnorePattern(format!("`{pattern}`: {e}")))?;
    }

    builder
        .build()
        .map_err(|e| VaultError::InvalidIgnorePattern(e.to_string()))
}
