//! Planning and executing cleanups.
//!
//! Cleaning is split in two so every frontend can show exactly what will
//! happen before anything is touched:
//!
//! 1. [`plan`] turns a selection of rules into [`Action`]s with concrete
//!    targets and a removal method chosen from the rule's safety level.
//! 2. [`execute`] re-checks every target against the [`guard`] right before
//!    removing it, so a plan can never touch anything outside its rule's
//!    paths, even if the filesystem changed in between.

use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use thiserror::Error;

use crate::model::{Cleanup, Rule, Safety};
use crate::scan::{Finding, Status};

/// Side effects the cleaner needs from the operating system, injectable for tests.
pub trait System {
    fn is_running(&self, process: &str) -> bool;
    fn has_program(&self, program: &str) -> bool;
    fn run(&self, argv: &[String]) -> Result<(), String>;
    fn trash(&self, paths: &[PathBuf]) -> Result<(), String>;
    fn available_space(&self, path: &Path) -> Option<u64>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// Explicit rule ids. A `dangerous` rule may only be selected alone.
    Ids(Vec<String>),
    /// Every rule marked `safe`.
    AllSafe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Method {
    /// Delete permanently; space is freed immediately.
    Delete,
    /// Move to the Trash; space is freed when the Trash is emptied.
    Trash,
    /// Run the tool's own cleanup command.
    Command { argv: Vec<String> },
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Action {
    pub rule_id: String,
    pub name: String,
    pub safety: Safety,
    pub method: Method,
    /// The paths the rule matched. Every target must stay within these.
    pub roots: Vec<PathBuf>,
    /// What will be removed. Empty for command cleanups.
    pub targets: Vec<PathBuf>,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub estimated: u64,
    /// Processes from the rule that are currently running.
    pub blocked_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Skipped {
    pub rule_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Plan {
    pub actions: Vec<Action>,
    pub skipped: Vec<Skipped>,
}

impl Plan {
    #[must_use]
    pub fn estimated(&self) -> u64 {
        self.actions
            .iter()
            .filter(|a| a.blocked_by.is_empty())
            .map(|a| a.estimated)
            .sum()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlanError {
    #[error("unknown rule `{0}`")]
    UnknownRule(String),
    #[error("`{0}` is marked dangerous and can only be cleaned on its own")]
    DangerousNotAlone(String),
    #[error("nothing selected")]
    Empty,
}

/// Builds a plan for the selected rules from fresh scan findings.
pub fn plan(
    rules: &[Rule],
    findings: &[Finding],
    selection: &Selection,
    sys: &dyn System,
) -> Result<Plan, PlanError> {
    let selected: Vec<&Rule> = match selection {
        Selection::AllSafe => rules.iter().filter(|r| r.safety == Safety::Safe).collect(),
        Selection::Ids(ids) => {
            let mut selected = Vec::new();
            for id in ids {
                let rule = rules
                    .iter()
                    .find(|r| &r.id == id)
                    .ok_or_else(|| PlanError::UnknownRule(id.clone()))?;
                if rule.safety == Safety::Dangerous && ids.len() > 1 {
                    return Err(PlanError::DangerousNotAlone(id.clone()));
                }
                selected.push(rule);
            }
            selected
        }
    };
    if selected.is_empty() {
        return Err(PlanError::Empty);
    }

    let mut plan = Plan::default();
    for rule in selected {
        let skip = |reason: &str| Skipped {
            rule_id: rule.id.clone(),
            reason: reason.to_owned(),
        };
        let Some(finding) = findings.iter().find(|f| f.rule_id == rule.id) else {
            plan.skipped.push(skip("not scanned"));
            continue;
        };
        match finding.status {
            Status::Missing => {
                plan.skipped.push(skip("not present"));
                continue;
            }
            Status::Denied => {
                plan.skipped.push(skip("no permission to read it"));
                continue;
            }
            Status::Present | Status::Partial => {}
        }

        let removal = if rule.safety == Safety::Safe {
            Method::Delete
        } else {
            Method::Trash
        };
        let (method, contents_only) = match &rule.cleanup {
            Cleanup::Remove { contents_only } => (removal, *contents_only),
            Cleanup::Command {
                argv,
                fallback_remove,
            } => {
                if sys.has_program(&argv[0]) {
                    (Method::Command { argv: argv.clone() }, false)
                } else if *fallback_remove {
                    (removal, false)
                } else {
                    plan.skipped
                        .push(skip(&format!("`{}` is not installed", argv[0])));
                    continue;
                }
            }
        };
        let targets = if matches!(method, Method::Command { .. }) {
            Vec::new()
        } else {
            removal_targets(&finding.paths, contents_only)
        };
        if targets.is_empty() && !matches!(method, Method::Command { .. }) {
            plan.skipped.push(skip("already empty"));
            continue;
        }
        let blocked_by = rule
            .processes
            .iter()
            .filter(|p| sys.is_running(p))
            .cloned()
            .collect();

        plan.actions.push(Action {
            rule_id: rule.id.clone(),
            name: rule.name.clone(),
            safety: rule.safety,
            method,
            roots: finding.paths.clone(),
            targets,
            estimated: finding.size.reclaimable,
            blocked_by,
        });
    }
    Ok(plan)
}

fn removal_targets(roots: &[PathBuf], contents_only: bool) -> Vec<PathBuf> {
    if !contents_only {
        return roots.to_vec();
    }
    let mut targets = Vec::new();
    for root in roots {
        // Never list a directory through a symlink: its contents live elsewhere.
        let is_real_dir = root.symlink_metadata().is_ok_and(|m| m.is_dir());
        if !is_real_dir {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(root) {
            targets.extend(entries.filter_map(Result::ok).map(|e| e.path()));
        }
    }
    targets.sort();
    targets
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GuardError {
    #[error("{0} is not a normalized absolute path")]
    NotNormalized(PathBuf),
    #[error("{0} is outside the rule's paths")]
    OutsideRoots(PathBuf),
    #[error("{0} is inside a symlinked directory")]
    ThroughSymlink(PathBuf),
}

/// The single check every removal passes through: a target must be one of
/// the rule's roots, or a direct child of a root that is a real directory.
pub fn guard(target: &Path, roots: &[PathBuf]) -> Result<(), GuardError> {
    let normalized = target.is_absolute()
        && target.components().all(|c| {
            matches!(
                c,
                Component::Normal(_) | Component::RootDir | Component::Prefix(_)
            )
        });
    if !normalized {
        return Err(GuardError::NotNormalized(target.to_owned()));
    }
    if roots.iter().any(|root| root == target) {
        return Ok(());
    }
    let Some(root) = target
        .parent()
        .and_then(|parent| roots.iter().find(|root| *root == parent))
    else {
        return Err(GuardError::OutsideRoots(target.to_owned()));
    };
    if root.symlink_metadata().is_ok_and(|m| m.is_dir()) {
        Ok(())
    } else {
        Err(GuardError::ThroughSymlink(target.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum Outcome {
    Done,
    Blocked { processes: Vec<String> },
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ActionResult {
    pub rule_id: String,
    pub method: Method,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub estimated: u64,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Report {
    pub results: Vec<ActionResult>,
    /// Free space gained on the volume, measured with `statfs`. `None` if
    /// it could not be measured.
    #[cfg_attr(feature = "specta", specta(type = Option<specta_typescript::Number>))]
    pub freed: Option<u64>,
    /// Estimated bytes moved to the Trash (freed only once it is emptied).
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub trashed: u64,
}

impl Report {
    #[must_use]
    pub fn failed(&self) -> bool {
        self.results
            .iter()
            .any(|r| matches!(r.outcome, Outcome::Failed { .. }))
    }
}

/// Executes a plan. `volume` is any path on the volume being cleaned and is
/// used to measure the space actually freed.
pub fn execute(plan: &Plan, volume: &Path, sys: &dyn System) -> Report {
    let before = sys.available_space(volume);
    let mut trashed = 0;
    let results = plan
        .actions
        .iter()
        .map(|action| {
            let outcome = if action.blocked_by.is_empty() {
                match run_action(action, sys) {
                    Ok(()) => Outcome::Done,
                    Err(error) => Outcome::Failed { error },
                }
            } else {
                Outcome::Blocked {
                    processes: action.blocked_by.clone(),
                }
            };
            if outcome == Outcome::Done && action.method == Method::Trash {
                trashed += action.estimated;
            }
            ActionResult {
                rule_id: action.rule_id.clone(),
                method: action.method.clone(),
                estimated: action.estimated,
                outcome,
            }
        })
        .collect();
    let freed = before
        .zip(sys.available_space(volume))
        .map(|(b, a)| a.saturating_sub(b));
    Report {
        results,
        freed,
        trashed,
    }
}

fn run_action(action: &Action, sys: &dyn System) -> Result<(), String> {
    for target in &action.targets {
        guard(target, &action.roots).map_err(|e| e.to_string())?;
    }
    match &action.method {
        Method::Command { argv } => sys.run(argv),
        Method::Trash => {
            let existing: Vec<PathBuf> = action
                .targets
                .iter()
                .filter(|t| t.symlink_metadata().is_ok())
                .cloned()
                .collect();
            if existing.is_empty() {
                Ok(())
            } else {
                sys.trash(&existing)
            }
        }
        Method::Delete => {
            let mut errors = Vec::new();
            for target in &action.targets {
                if let Err(err) = delete(target) {
                    errors.push(format!("{}: {err}", target.display()));
                }
            }
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("; "))
            }
        }
    }
}

/// Deletes a file, symlink or directory tree without following symlinks.
fn delete(target: &Path) -> io::Result<()> {
    let meta = match target.symlink_metadata() {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if meta.is_dir() {
        // std's remove_dir_all does not follow symlinks inside the tree.
        std::fs::remove_dir_all(target)
    } else {
        std::fs::remove_file(target)
    }
}

/// The real operating system.
#[derive(Debug, Default)]
pub struct RealSystem;

impl System for RealSystem {
    fn is_running(&self, process: &str) -> bool {
        Command::new("pgrep")
            .args(["-x", process])
            .output()
            .is_ok_and(|out| out.status.success())
    }

    fn has_program(&self, program: &str) -> bool {
        let Some(path) = std::env::var_os("PATH") else {
            return false;
        };
        std::env::split_paths(&path).any(|dir| dir.join(program).is_file())
    }

    fn run(&self, argv: &[String]) -> Result<(), String> {
        let (program, rest) = argv.split_first().ok_or("empty command")?;
        let out = Command::new(program)
            .args(rest)
            .output()
            .map_err(|e| format!("`{program}`: {e}"))?;
        if out.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&out.stderr);
        let detail = stderr
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("no output");
        Err(format!(
            "`{}` failed ({}): {detail}",
            argv.join(" "),
            out.status
        ))
    }

    fn trash(&self, paths: &[PathBuf]) -> Result<(), String> {
        #[allow(unused_mut)]
        let mut ctx = trash::TrashContext::default();
        #[cfg(target_os = "macos")]
        {
            use trash::macos::{DeleteMethod, TrashContextExtMacos};
            // Finder needs an Automation permission; NSFileManager needs none.
            ctx.set_delete_method(DeleteMethod::NsFileManager);
        }
        ctx.delete_all(paths).map_err(|e| e.to_string())
    }

    fn available_space(&self, path: &Path) -> Option<u64> {
        fs4::available_space(path).ok()
    }
}
