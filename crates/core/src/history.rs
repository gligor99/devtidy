//! Append-only log of every cleanup, one JSON object per line.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::clean::{ActionResult, Report};

#[derive(Debug, Serialize)]
struct Entry<'a> {
    timestamp: u64,
    #[serde(flatten)]
    result: &'a ActionResult,
}

#[derive(Debug, Serialize)]
struct Summary {
    timestamp: u64,
    freed: Option<u64>,
    trashed: u64,
}

/// Appends one line per action and one summary line for the run.
pub fn append(log: &Path, report: &Report) -> io::Result<()> {
    if let Some(dir) = log.parent() {
        fs::create_dir_all(dir)?;
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let mut file = OpenOptions::new().create(true).append(true).open(log)?;
    let mut lines = String::new();
    for result in &report.results {
        lines.push_str(&serde_json::to_string(&Entry { timestamp, result })?);
        lines.push('\n');
    }
    lines.push_str(&serde_json::to_string(&Summary {
        timestamp,
        freed: report.freed,
        trashed: report.trashed,
    })?);
    lines.push('\n');
    file.write_all(lines.as_bytes())
}
