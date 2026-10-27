//! Resolving rule path patterns against a home directory.

use std::path::{Component, Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PatternError {
    #[error("pattern `{0}` must start with `~/`")]
    NotHomeRelative(String),
    #[error("pattern `{0}` must point at least two levels below home")]
    TooShallow(String),
    #[error("pattern `{0}` must not contain `.` or `..` components")]
    Traversal(String),
    #[error("pattern `{0}` is not a valid glob: {1}")]
    InvalidGlob(String, String),
}

/// Checks that a pattern can never resolve to home itself, to anything
/// above it, or to a top-level folder such as `~/Library`.
pub fn validate_pattern(pattern: &str) -> Result<(), PatternError> {
    let rest = pattern
        .strip_prefix("~/")
        .ok_or_else(|| PatternError::NotHomeRelative(pattern.to_owned()))?;
    let rel = Path::new(rest);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(PatternError::Traversal(pattern.to_owned()));
    }
    let literal_depth = rel
        .components()
        .take_while(|c| !is_glob(&c.as_os_str().to_string_lossy()))
        .count();
    if literal_depth < 2 {
        return Err(PatternError::TooShallow(pattern.to_owned()));
    }
    glob::Pattern::new(rest)
        .map_err(|e| PatternError::InvalidGlob(pattern.to_owned(), e.msg.to_owned()))?;
    Ok(())
}

/// Expands a validated pattern into the existing paths it matches.
/// Symlinks are returned as-is and never resolved.
pub fn expand(pattern: &str, home: &Path) -> Result<Vec<PathBuf>, PatternError> {
    validate_pattern(pattern)?;
    let rest = &pattern[2..];
    if !is_glob(rest) {
        let path = home.join(rest);
        return Ok(if path.symlink_metadata().is_ok() {
            vec![path]
        } else {
            Vec::new()
        });
    }
    let escaped_home = glob::Pattern::escape(&home.to_string_lossy());
    let full = format!("{escaped_home}/{rest}");
    let paths = glob::glob(&full)
        .map_err(|e| PatternError::InvalidGlob(pattern.to_owned(), e.msg.to_owned()))?
        .filter_map(Result::ok)
        .collect();
    Ok(paths)
}

fn is_glob(s: &str) -> bool {
    s.contains(['*', '?', '['])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_patterns_that_could_escape_or_hit_home() {
        assert!(matches!(
            validate_pattern("/etc/hosts"),
            Err(PatternError::NotHomeRelative(_))
        ));
        assert!(matches!(
            validate_pattern("~"),
            Err(PatternError::NotHomeRelative(_))
        ));
        assert!(matches!(
            validate_pattern("~/Library"),
            Err(PatternError::TooShallow(_))
        ));
        assert!(matches!(
            validate_pattern("~/*/caches"),
            Err(PatternError::TooShallow(_))
        ));
        assert!(matches!(
            validate_pattern("~/.gradle/../x"),
            Err(PatternError::Traversal(_))
        ));
        assert!(matches!(
            validate_pattern("~/./.gradle/x"),
            Err(PatternError::Traversal(_))
        ));
        assert!(validate_pattern("~/.gradle/caches").is_ok());
        assert!(validate_pattern("~/Library/Caches/*").is_ok());
    }

    #[test]
    fn expands_literal_and_glob_patterns() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".gradle/caches")).unwrap();
        std::fs::create_dir_all(home.path().join("a/b/one")).unwrap();
        std::fs::create_dir_all(home.path().join("a/b/two")).unwrap();

        assert_eq!(expand("~/.gradle/caches", home.path()).unwrap().len(), 1);
        assert_eq!(
            expand("~/.gradle/missing", home.path()).unwrap(),
            Vec::<PathBuf>::new()
        );
        assert_eq!(expand("~/a/b/*", home.path()).unwrap().len(), 2);
    }
}
