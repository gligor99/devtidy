//! Human-readable output. Sizes use SI units (1 GB = 10^9 bytes), like Finder.

use std::fmt::Write;
use std::io::IsTerminal;

use comfy_table::{Cell, CellAlignment, Color, Table, presets::UTF8_FULL_CONDENSED};
use devtidy_core::{Cleanup, Rule, Safety, Status};

use crate::report::{Item, ScanReport};

const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];

#[allow(clippy::cast_precision_loss)]
pub fn bytes(n: u64) -> String {
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Parses sizes like `500M`, `20G`, `1.5TB` or a plain byte count.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let split = s
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());
    let (number, unit) = s.split_at(split);
    let number: f64 = number.parse().map_err(|_| format!("invalid size `{s}`"))?;
    let exponent = match unit.trim().to_ascii_uppercase().trim_end_matches('B') {
        "" => 0,
        "K" => 1,
        "M" => 2,
        "G" => 3,
        "T" => 4,
        _ => return Err(format!("unknown unit in `{s}`; use K, M, G or T")),
    };
    Ok((number * 1000f64.powi(exponent)).round() as u64)
}

fn colored() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

pub fn safety_cell(safety: Safety) -> Cell {
    let (text, color) = match safety {
        Safety::Safe => ("safe", Color::Green),
        Safety::Caution => ("caution", Color::Yellow),
        Safety::Dangerous => ("dangerous", Color::Red),
    };
    let cell = Cell::new(text);
    if colored() { cell.fg(color) } else { cell }
}

fn size_cell(item: &Item) -> String {
    let mut out = bytes(item.size.allocated);
    if item.size.reclaimable < item.size.allocated {
        let _ = write!(out, " (frees ~{})", bytes(item.size.reclaimable));
    }
    match item.status {
        Status::Partial => out.push_str(" +?"),
        Status::Denied => out = "no access".into(),
        Status::Present | Status::Missing => {}
    }
    out
}

pub fn table(header: &[&str]) -> Table {
    let mut table = Table::new();
    table
        .load_style(UTF8_FULL_CONDENSED)
        .set_header(header.iter().map(Cell::new));
    table
}

pub fn scan_table(report: &ScanReport, all: bool) -> String {
    let mut table = table(&["Item", "Ecosystem", "Size", "Safety", "Id"]);
    let mut hidden = 0;
    for item in &report.items {
        if item.status == Status::Missing && !all {
            hidden += 1;
            continue;
        }
        table.add_row(vec![
            Cell::new(&item.name),
            Cell::new(item.ecosystem.label()),
            Cell::new(size_cell(item)).set_alignment(CellAlignment::Right),
            safety_cell(item.safety),
            Cell::new(&item.id),
        ]);
    }

    let mut out = format!("{table}\n");
    let t = &report.totals;
    let _ = writeln!(
        out,
        "Total {}, reclaimable ~{} ({} marked safe).",
        bytes(t.allocated),
        bytes(t.reclaimable),
        bytes(t.reclaimable_safe)
    );
    if report
        .items
        .iter()
        .any(|i| matches!(i.status, Status::Partial | Status::Denied))
    {
        out.push_str("Some locations could not be fully read; run with more permissions for complete numbers.\n");
    }
    if hidden > 0 {
        let _ = writeln!(
            out,
            "{hidden} known locations not present on this machine (show with --all)."
        );
    }
    out.push_str("Run `devtidy explain <id>` to see what an item is and what removing it costs.\n");
    out
}

pub fn explain(rule: &Rule, item: &Item) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{} ({})\n", rule.name, rule.id);
    let _ = writeln!(out, "{}\n", rule.description);
    let _ = writeln!(out, "Ecosystem:   {}", rule.ecosystem.label());
    let _ = writeln!(out, "Safety:      {:?}", rule.safety);
    let _ = writeln!(out, "Regen cost:  {:?}", rule.regen_cost);
    let _ = writeln!(out, "After removal: {}", rule.regen_note);
    let _ = writeln!(out, "Cleanup:     {}", cleanup_summary(rule));
    if !rule.processes.is_empty() {
        let _ = writeln!(out, "Quit first:  {}", rule.processes.join(", "));
    }
    if let Some(docs) = &rule.docs {
        let _ = writeln!(out, "Docs:        {docs}");
    }
    let _ = writeln!(out);
    if item.status == Status::Missing {
        out.push_str("Not present on this machine.\n");
    } else {
        let _ = writeln!(out, "On this machine: {}", size_cell(item));
        for path in &item.paths {
            let _ = writeln!(out, "  {}", path.display());
        }
    }
    out
}

pub fn cleanup_summary(rule: &Rule) -> String {
    let removal = match rule.safety {
        Safety::Safe => "delete permanently",
        Safety::Caution => "move to Trash",
        Safety::Dangerous => "move to Trash, one item at a time only",
    };
    match &rule.cleanup {
        Cleanup::Command {
            argv,
            fallback_remove,
        } => {
            let mut s = format!("run `{}`", argv.join(" "));
            if *fallback_remove {
                let _ = write!(s, " (if not installed: {removal})");
            }
            s
        }
        Cleanup::Remove {
            contents_only: true,
        } => format!("{removal} (contents only, folder is kept)"),
        Cleanup::Remove {
            contents_only: false,
        } => removal.to_owned(),
    }
}

pub fn rules_table(rules: &[Rule]) -> String {
    let mut table = table(&["Id", "Name", "Ecosystem", "Safety"]);
    for rule in rules {
        table.add_row(vec![
            Cell::new(&rule.id),
            Cell::new(&rule.name),
            Cell::new(rule.ecosystem.label()),
            safety_cell(rule.safety),
        ]);
    }
    format!("{table}\n{} rules.\n", rules.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_si_sizes() {
        assert_eq!(bytes(999), "999 B");
        assert_eq!(bytes(18_200_000_000), "18.2 GB");
        assert_eq!(bytes(1_500_000), "1.5 MB");
    }

    #[test]
    fn parses_sizes() {
        assert_eq!(parse_size("20G"), Ok(20_000_000_000));
        assert_eq!(parse_size("1.5TB"), Ok(1_500_000_000_000));
        assert_eq!(parse_size("500 mb"), Ok(500_000_000));
        assert_eq!(parse_size("42"), Ok(42));
        assert!(parse_size("20X").is_err());
        assert!(parse_size("abc").is_err());
    }
}
