//! `devtidy clean`: show the plan, confirm, execute, record.

use std::fmt::Write as _;
use std::io::{BufRead, IsTerminal, Write as _};
use std::process::ExitCode;

use anyhow::{Result, bail};
use comfy_table::{Cell, CellAlignment};
use devtidy_core::clean::{self, Method, Outcome, Plan};
use devtidy_core::{RealSystem, Rule, Safety, ScanContext, Selection, scan};
use serde::Serialize;

use crate::format::{bytes, safety_cell, table};
use crate::report::Versioned;

// Each bool mirrors one CLI flag.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct Options {
    pub ids: Vec<String>,
    pub safe: bool,
    pub dry_run: bool,
    pub yes: bool,
    pub json: bool,
}

#[derive(Serialize)]
struct JsonOutput<'a> {
    plan: &'a Plan,
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<&'a clean::Report>,
}

pub fn run(rules: &[Rule], ctx: &ScanContext, opts: &Options) -> Result<ExitCode> {
    let selection = match (opts.safe, opts.ids.is_empty()) {
        (true, true) => Selection::AllSafe,
        (false, false) => Selection::Ids(opts.ids.clone()),
        _ => bail!("pass rule ids to clean, or --safe for every item marked safe"),
    };
    let candidates: Vec<Rule> = rules
        .iter()
        .filter(|r| match &selection {
            Selection::AllSafe => r.safety == Safety::Safe,
            Selection::Ids(ids) => ids.contains(&r.id),
        })
        .cloned()
        .collect();

    let sys = RealSystem;
    let findings = scan(&candidates, ctx);
    let plan = clean::plan(rules, &findings, &selection, &sys)?;
    let runnable = plan.actions.iter().any(|a| a.blocked_by.is_empty());

    if opts.json && (opts.dry_run || !runnable) {
        let out = Versioned::new(JsonOutput {
            plan: &plan,
            report: None,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(ExitCode::SUCCESS);
    }
    if !opts.json {
        print!("{}", plan_table(&plan, opts.safe));
    }
    if !runnable {
        if !opts.json {
            println!("Nothing to clean.");
        }
        return Ok(ExitCode::SUCCESS);
    }
    if opts.dry_run {
        println!("Dry run: nothing was changed.");
        return Ok(ExitCode::SUCCESS);
    }
    if !opts.yes {
        if opts.json || !std::io::stdin().is_terminal() {
            bail!("refusing to clean without confirmation; pass --yes in non-interactive use");
        }
        if !confirm(&plan)? {
            println!("Cancelled.");
            return Ok(ExitCode::SUCCESS);
        }
    }

    let report = clean::execute(&plan, &ctx.home, &sys);
    if let Err(err) = crate::record_cleanup(&report) {
        eprintln!("warning: could not record history: {err:#}");
    }
    if opts.json {
        let out = Versioned::new(JsonOutput {
            plan: &plan,
            report: Some(&report),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        print!("{}", summary(&plan, &report));
    }
    Ok(if report.failed() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn method_label(method: &Method) -> String {
    match method {
        Method::Delete => "delete".to_owned(),
        Method::Trash => "move to Trash".to_owned(),
        Method::Command { argv } => format!("run `{}`", argv.join(" ")),
    }
}

fn plan_table(plan: &Plan, all_safe: bool) -> String {
    let mut t = table(&["Item", "Size", "Safety", "Action", "Note"]);
    for action in &plan.actions {
        let note = if action.blocked_by.is_empty() {
            String::new()
        } else {
            format!("skipped: quit {} first", action.blocked_by.join(", "))
        };
        t.add_row(vec![
            Cell::new(&action.name),
            Cell::new(bytes(action.estimated)).set_alignment(CellAlignment::Right),
            safety_cell(action.safety),
            Cell::new(method_label(&action.method)),
            Cell::new(note),
        ]);
    }
    let mut out = format!("{t}\n");
    for skipped in &plan.skipped {
        // With --safe, absent and empty locations are expected noise.
        if all_safe && matches!(skipped.reason.as_str(), "not present" | "already empty") {
            continue;
        }
        let _ = writeln!(out, "Skipping {}: {}.", skipped.rule_id, skipped.reason);
    }
    let _ = writeln!(out, "Estimated to reclaim: {}.", bytes(plan.estimated()));
    out
}

fn confirm(plan: &Plan) -> Result<bool> {
    let mut stdout = std::io::stdout();
    let mut answer = String::new();
    if let Some(action) = plan.actions.iter().find(|a| a.safety == Safety::Dangerous) {
        print!(
            "{} is marked dangerous and cannot be fully regenerated.\nType `{}` to confirm: ",
            action.name, action.rule_id
        );
        stdout.flush()?;
        std::io::stdin().lock().read_line(&mut answer)?;
        return Ok(answer.trim() == action.rule_id);
    }
    print!("Proceed? [y/N] ");
    stdout.flush()?;
    std::io::stdin().lock().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

fn summary(plan: &Plan, report: &clean::Report) -> String {
    let mut out = String::new();
    for (action, result) in plan.actions.iter().zip(&report.results) {
        let _ = match &result.outcome {
            Outcome::Done => writeln!(out, "  done     {}", action.name),
            Outcome::Blocked { processes } => {
                writeln!(
                    out,
                    "  skipped  {} (quit {} first)",
                    action.name,
                    processes.join(", ")
                )
            }
            Outcome::Failed { error } => writeln!(out, "  failed   {}: {error}", action.name),
        };
    }

    let deleted_estimate: u64 = plan
        .actions
        .iter()
        .zip(&report.results)
        .filter(|(a, r)| r.outcome == Outcome::Done && a.method != Method::Trash)
        .map(|(a, _)| a.estimated)
        .sum();
    match report.freed {
        Some(freed) => {
            let _ = writeln!(out, "\nFreed {} (measured on disk).", bytes(freed));
            if deleted_estimate > 100_000_000 && freed < deleted_estimate / 2 {
                out.push_str(
                    "That is less than estimated: local Time Machine snapshots or APFS clones can keep \
                     blocks in use, and other apps may have written to disk meanwhile.\n",
                );
            }
        }
        None => {
            let _ = writeln!(
                out,
                "\nFreed about {} (could not measure the volume).",
                bytes(deleted_estimate)
            );
        }
    }
    if report.trashed > 0 {
        let _ = writeln!(
            out,
            "Moved {} to the Trash. Empty it to free that space.",
            bytes(report.trashed)
        );
    }
    out
}
