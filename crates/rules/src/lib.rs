//! Built-in devtidy rules plus loading and validation of user rules.
//!
//! Rules ship inside the binary and only change with a release. They are
//! never fetched at runtime: a rule decides what gets deleted, so changing
//! one must go through review, CI and a signed release.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use devtidy_core::{Cleanup, Rule, paths};
use serde::Deserialize;
use thiserror::Error;

const BUILTIN: &[(&str, &str)] = &[
    ("ios.toml", include_str!("../rules/ios.toml")),
    ("android.toml", include_str!("../rules/android.toml")),
    ("javascript.toml", include_str!("../rules/javascript.toml")),
    ("other.toml", include_str!("../rules/other.toml")),
];

#[derive(Debug, Error)]
pub enum RuleError {
    #[error("{file}: {source}")]
    Parse {
        file: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("{file}: rule `{id}`: {reason}")]
    Invalid {
        file: String,
        id: String,
        reason: String,
    },
    #[error("{file}: rule id `{id}` is already defined")]
    Duplicate { file: String, id: String },
    #[error("reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleFile {
    rule: Vec<Rule>,
}

/// All built-in rules.
///
/// # Panics
///
/// Only if a shipped rule file is broken, which the test suite rules out.
#[must_use]
pub fn builtin() -> Vec<Rule> {
    load(
        BUILTIN
            .iter()
            .map(|(file, src)| ((*file).to_owned(), (*src).to_owned())),
    )
    .expect("built-in rules are validated by tests")
}

/// Built-in rules followed by every `*.toml` file in `dir`, if it exists.
/// User rules may add new ids but never redefine a built-in one.
pub fn builtin_and_user(dir: &Path) -> Result<Vec<Rule>, RuleError> {
    let mut sources: Vec<(String, String)> = BUILTIN
        .iter()
        .map(|(file, src)| ((*file).to_owned(), (*src).to_owned()))
        .collect();
    sources.extend(read_dir(dir)?);
    load(sources)
}

fn read_dir(dir: &Path) -> Result<Vec<(String, String)>, RuleError> {
    let io_err = |source| RuleError::Io {
        path: dir.to_owned(),
        source,
    };
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(io_err(err)),
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let src = fs::read_to_string(&path).map_err(|source| RuleError::Io {
                path: path.clone(),
                source,
            })?;
            Ok((path.display().to_string(), src))
        })
        .collect()
}

fn load(sources: impl IntoIterator<Item = (String, String)>) -> Result<Vec<Rule>, RuleError> {
    let mut seen = HashSet::new();
    let mut rules = Vec::new();
    for (file, src) in sources {
        let parsed: RuleFile = toml::from_str(&src).map_err(|source| RuleError::Parse {
            file: file.clone(),
            source,
        })?;
        for rule in parsed.rule {
            validate(&rule).map_err(|reason| RuleError::Invalid {
                file: file.clone(),
                id: rule.id.clone(),
                reason,
            })?;
            if !seen.insert(rule.id.clone()) {
                return Err(RuleError::Duplicate { file, id: rule.id });
            }
            rules.push(rule);
        }
    }
    Ok(rules)
}

/// Checks the invariants the cleaner relies on.
pub fn validate(rule: &Rule) -> Result<(), String> {
    let id_ok = rule.id.split('.').count() >= 2
        && rule.id.split('.').all(|seg| {
            !seg.is_empty()
                && seg
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        });
    if !id_ok {
        return Err("id must be dotted lowercase segments, e.g. `gradle.caches`".into());
    }
    for (field, value) in [
        ("name", &rule.name),
        ("description", &rule.description),
        ("regen_note", &rule.regen_note),
    ] {
        if value.trim().is_empty() {
            return Err(format!("`{field}` must not be empty"));
        }
    }
    if rule.paths.is_empty() {
        return Err("at least one path is required".into());
    }
    for pattern in &rule.paths {
        paths::validate_pattern(pattern).map_err(|e| e.to_string())?;
    }
    if let Cleanup::Command { argv, .. } = &rule.cleanup
        && argv.first().is_none_or(|program| program.trim().is_empty())
    {
        return Err("command cleanup needs a program in `argv`".into());
    }
    if let Some(docs) = &rule.docs
        && !docs.starts_with("https://")
    {
        return Err("`docs` must be an https URL".into());
    }
    Ok(())
}
