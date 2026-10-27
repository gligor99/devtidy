//! Turning rules into findings.

use std::path::PathBuf;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::model::{Platform, Rule};
use crate::paths;
use crate::size::{self, Measurement};

/// Everything a scan depends on, injected so tests can use a fake home.
#[derive(Debug, Clone)]
pub struct ScanContext {
    pub home: PathBuf,
    pub platform: Platform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Nothing matched on this machine.
    Missing,
    /// Found and fully measured.
    Present,
    /// Found, but some entries could not be read; sizes are a lower bound.
    Partial,
    /// Found, but nothing inside could be read without more permissions.
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Finding {
    pub rule_id: String,
    pub status: Status,
    pub paths: Vec<PathBuf>,
    pub size: Measurement,
}

/// Scans every rule that applies to the context's platform, in parallel.
/// Findings keep the order of `rules`.
#[must_use]
pub fn scan(rules: &[Rule], ctx: &ScanContext) -> Vec<Finding> {
    rules
        .par_iter()
        .filter(|rule| rule.applies_to(ctx.platform))
        .map(|rule| scan_rule(rule, ctx))
        .collect()
}

#[must_use]
pub fn scan_rule(rule: &Rule, ctx: &ScanContext) -> Finding {
    let mut paths: Vec<PathBuf> = rule
        .paths
        .iter()
        // Patterns are validated when rules load; an invalid one matches nothing.
        .filter_map(|pattern| paths::expand(pattern, &ctx.home).ok())
        .flatten()
        .collect();
    paths.sort();
    paths.dedup();

    let size = size::measure(&paths);
    let status = match (paths.is_empty(), size.denied) {
        (true, _) => Status::Missing,
        (false, 0) => Status::Present,
        // Only the roots themselves were visible.
        (false, _) if size.entries <= paths.len() as u64 => Status::Denied,
        (false, _) => Status::Partial,
    };
    Finding {
        rule_id: rule.id.clone(),
        status,
        paths,
        size,
    }
}
