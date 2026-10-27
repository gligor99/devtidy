//! Scan snapshots and cleanup history in SQLite, shared by the CLI and the app.
//!
//! The database lives in the platform data directory
//! (`~/Library/Application Support/devtidy/devtidy.db` on macOS) and can be
//! moved with `DEVTIDY_DATA_DIR`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use devtidy_core::clean::{Method, Outcome, Report};
use devtidy_core::disk::DiskSpace;
use devtidy_core::{Finding, Status};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("creating {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not determine the data directory")]
    NoDataDir,
    #[error("database schema version {0} is newer than this devtidy supports")]
    TooNew(i64),
}

/// Ordered migrations; the index + 1 is stored in `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[r"
    CREATE TABLE scans (
        id              INTEGER PRIMARY KEY,
        at              INTEGER NOT NULL,
        disk_total      INTEGER,
        disk_available  INTEGER
    );
    CREATE TABLE scan_items (
        scan_id      INTEGER NOT NULL REFERENCES scans(id) ON DELETE CASCADE,
        rule_id      TEXT    NOT NULL,
        status       TEXT    NOT NULL,
        allocated    INTEGER NOT NULL,
        reclaimable  INTEGER NOT NULL,
        PRIMARY KEY (scan_id, rule_id)
    );
    CREATE INDEX scan_items_rule ON scan_items (rule_id, scan_id);
    CREATE TABLE cleanups (
        id       INTEGER PRIMARY KEY,
        at       INTEGER NOT NULL,
        freed    INTEGER,
        trashed  INTEGER NOT NULL
    );
    CREATE TABLE cleanup_actions (
        cleanup_id  INTEGER NOT NULL REFERENCES cleanups(id) ON DELETE CASCADE,
        rule_id     TEXT    NOT NULL,
        method      TEXT    NOT NULL,
        estimated   INTEGER NOT NULL,
        outcome     TEXT    NOT NULL,
        detail      TEXT
    );
"];

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CleanupRecord {
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub id: i64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub at: i64,
    #[cfg_attr(feature = "specta", specta(type = Option<specta_typescript::Number>))]
    pub freed: Option<u64>,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub trashed: u64,
    pub actions: Vec<CleanupActionRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CleanupActionRecord {
    pub rule_id: String,
    /// `delete`, `trash` or `command`.
    pub method: String,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub estimated: u64,
    /// `done`, `blocked` or `failed`.
    pub outcome: String,
    pub detail: Option<String>,
}

/// Size of one rule in an earlier scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PreviousSize {
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub at: i64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub allocated: u64,
}

#[derive(Debug)]
pub struct Store {
    conn: Connection,
}

/// `DEVTIDY_DATA_DIR`, or `<platform data dir>/devtidy`.
pub fn default_dir() -> Result<PathBuf, StoreError> {
    match std::env::var_os("DEVTIDY_DATA_DIR") {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => Ok(dirs::data_dir()
            .ok_or(StoreError::NoDataDir)?
            .join("devtidy")),
    }
}

// SQLite stores INTEGER as i64. Sizes and counts never come close to i64::MAX.
#[allow(clippy::cast_possible_wrap)]
fn int(n: u64) -> i64 {
    n as i64
}

#[allow(clippy::cast_sign_loss)]
fn uint(n: i64) -> u64 {
    n.max(0) as u64
}

impl Store {
    pub fn open_default() -> Result<Self, StoreError> {
        let dir = default_dir()?;
        std::fs::create_dir_all(&dir).map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
        Self::open(&dir.join("devtidy.db"))
    }

    pub fn open(path: &Path) -> Result<Self, StoreError> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self, StoreError> {
        // The CLI and the app may write at the same time.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        let known = i64::try_from(MIGRATIONS.len()).unwrap_or(i64::MAX);
        if version > known {
            return Err(StoreError::TooNew(version));
        }
        let tx = conn.transaction()?;
        for (i, sql) in MIGRATIONS
            .iter()
            .enumerate()
            .skip(usize::try_from(version).unwrap_or(0))
        {
            tx.execute_batch(sql)?;
            tx.pragma_update(
                None,
                "user_version",
                i64::try_from(i + 1).unwrap_or(i64::MAX),
            )?;
        }
        tx.commit()?;
        Ok(Self { conn })
    }

    /// Records a scan and returns its id.
    pub fn record_scan(
        &mut self,
        at: i64,
        disk: Option<DiskSpace>,
        findings: &[Finding],
    ) -> Result<i64, StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO scans (at, disk_total, disk_available) VALUES (?1, ?2, ?3)",
            params![
                at,
                disk.map(|d| int(d.total)),
                disk.map(|d| int(d.available))
            ],
        )?;
        let scan_id = tx.last_insert_rowid();
        {
            let mut insert = tx.prepare(
                "INSERT INTO scan_items (scan_id, rule_id, status, allocated, reclaimable)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for f in findings.iter().filter(|f| f.status != Status::Missing) {
                insert.execute(params![
                    scan_id,
                    f.rule_id,
                    status_str(f.status),
                    int(f.size.allocated),
                    int(f.size.reclaimable)
                ])?;
            }
        }
        tx.commit()?;
        Ok(scan_id)
    }

    /// For each rule, its size in the most recent scan before `scan_id`.
    pub fn previous_sizes(
        &self,
        scan_id: i64,
    ) -> Result<HashMap<String, PreviousSize>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT i.rule_id, s.at, i.allocated
             FROM scan_items i JOIN scans s ON s.id = i.scan_id
             WHERE i.scan_id = (
                 SELECT MAX(i2.scan_id) FROM scan_items i2
                 WHERE i2.rule_id = i.rule_id AND i2.scan_id < ?1
             )",
        )?;
        let rows = stmt.query_map([scan_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                PreviousSize {
                    at: row.get(1)?,
                    allocated: uint(row.get(2)?),
                },
            ))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Time of the most recent scan, if any.
    pub fn last_scan_at(&self) -> Result<Option<i64>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT MAX(at) FROM scans", [], |row| row.get(0))
            .optional()?
            .flatten())
    }

    pub fn record_cleanup(&mut self, at: i64, report: &Report) -> Result<i64, StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO cleanups (at, freed, trashed) VALUES (?1, ?2, ?3)",
            params![at, report.freed.map(int), int(report.trashed)],
        )?;
        let cleanup_id = tx.last_insert_rowid();
        {
            let mut insert = tx.prepare(
                "INSERT INTO cleanup_actions (cleanup_id, rule_id, method, estimated, outcome, detail)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for r in &report.results {
                let (outcome, detail) = match &r.outcome {
                    Outcome::Done => ("done", None),
                    Outcome::Blocked { processes } => ("blocked", Some(processes.join(", "))),
                    Outcome::Failed { error } => ("failed", Some(error.clone())),
                };
                let method = match &r.method {
                    Method::Delete => "delete",
                    Method::Trash => "trash",
                    Method::Command { .. } => "command",
                };
                insert.execute(params![
                    cleanup_id,
                    r.rule_id,
                    method,
                    int(r.estimated),
                    outcome,
                    detail
                ])?;
            }
        }
        tx.commit()?;
        Ok(cleanup_id)
    }

    /// Most recent cleanups first.
    pub fn cleanups(&self, limit: u32) -> Result<Vec<CleanupRecord>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, at, freed, trashed FROM cleanups ORDER BY id DESC LIMIT ?1")?;
        let mut records: Vec<CleanupRecord> = stmt
            .query_map([limit], |row| {
                Ok(CleanupRecord {
                    id: row.get(0)?,
                    at: row.get(1)?,
                    freed: row.get::<_, Option<i64>>(2)?.map(uint),
                    trashed: uint(row.get(3)?),
                    actions: Vec::new(),
                })
            })?
            .collect::<Result<_, _>>()?;

        let mut actions = self.conn.prepare(
            "SELECT rule_id, method, estimated, outcome, detail FROM cleanup_actions
             WHERE cleanup_id = ?1 ORDER BY rowid",
        )?;
        for record in &mut records {
            record.actions = actions
                .query_map([record.id], |row| {
                    Ok(CleanupActionRecord {
                        rule_id: row.get(0)?,
                        method: row.get(1)?,
                        estimated: uint(row.get(2)?),
                        outcome: row.get(3)?,
                        detail: row.get(4)?,
                    })
                })?
                .collect::<Result<_, _>>()?;
        }
        Ok(records)
    }
}

fn status_str(status: Status) -> &'static str {
    match status {
        Status::Missing => "missing",
        Status::Present => "present",
        Status::Partial => "partial",
        Status::Denied => "denied",
    }
}

#[cfg(test)]
mod tests {
    use devtidy_core::Measurement;
    use devtidy_core::clean::ActionResult;

    use super::*;

    fn finding(id: &str, allocated: u64) -> Finding {
        Finding {
            rule_id: id.into(),
            status: Status::Present,
            paths: Vec::new(),
            size: Measurement {
                allocated,
                reclaimable: allocated,
                entries: 1,
                denied: 0,
            },
        }
    }

    #[test]
    fn previous_sizes_come_from_the_latest_earlier_scan_per_rule() {
        let mut store = Store::open_in_memory().unwrap();
        store
            .record_scan(100, None, &[finding("a", 10), finding("b", 20)])
            .unwrap();
        store.record_scan(200, None, &[finding("a", 15)]).unwrap();
        let current = store
            .record_scan(300, None, &[finding("a", 40), finding("b", 25)])
            .unwrap();

        let prev = store.previous_sizes(current).unwrap();
        assert_eq!(
            prev["a"],
            PreviousSize {
                at: 200,
                allocated: 15
            }
        );
        assert_eq!(
            prev["b"],
            PreviousSize {
                at: 100,
                allocated: 20
            }
        );
        assert_eq!(store.last_scan_at().unwrap(), Some(300));
    }

    #[test]
    fn missing_findings_are_not_stored() {
        let mut store = Store::open_in_memory().unwrap();
        let mut gone = finding("gone", 0);
        gone.status = Status::Missing;
        store.record_scan(1, None, &[gone]).unwrap();
        let current = store.record_scan(2, None, &[]).unwrap();
        assert!(store.previous_sizes(current).unwrap().is_empty());
    }

    #[test]
    fn cleanups_round_trip_newest_first() {
        let mut store = Store::open_in_memory().unwrap();
        let report = |freed| Report {
            results: vec![
                ActionResult {
                    rule_id: "a".into(),
                    method: Method::Delete,
                    estimated: 5,
                    outcome: Outcome::Done,
                },
                ActionResult {
                    rule_id: "b".into(),
                    method: Method::Trash,
                    estimated: 7,
                    outcome: Outcome::Blocked {
                        processes: vec!["Xcode".into()],
                    },
                },
            ],
            freed,
            trashed: 0,
        };
        store.record_cleanup(10, &report(Some(5))).unwrap();
        store.record_cleanup(20, &report(None)).unwrap();

        let records = store.cleanups(10).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].at, 20);
        assert_eq!(records[0].freed, None);
        assert_eq!(records[1].freed, Some(5));
        assert_eq!(records[1].actions[1].outcome, "blocked");
        assert_eq!(records[1].actions[1].detail.as_deref(), Some("Xcode"));
    }

    #[test]
    fn reopening_keeps_data_and_does_not_rerun_migrations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("devtidy.db");
        Store::open(&path)
            .unwrap()
            .record_scan(1, None, &[finding("a", 1)])
            .unwrap();
        let store = Store::open(&path).unwrap();
        assert_eq!(store.last_scan_at().unwrap(), Some(1));
    }
}
