//! The desktop app: a thin Tauri layer over `devtidy-core`.
//!
//! The frontend never sends paths or plans to execute. It sends rule ids,
//! and every cleanup is re-planned here from a fresh scan, so the UI cannot
//! ask the backend to remove anything a rule does not allow.

// Tauri commands must take `State` by value.
#![allow(clippy::needless_pass_by_value)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use devtidy_core::clean::{self, Plan, Report, Selection};
use devtidy_core::disk::{self, DiskSpace};
use devtidy_core::scan::scan_rule;
use devtidy_core::{Finding, Platform, RealSystem, Rule, ScanContext};
use devtidy_store::{CleanupRecord, PreviousSize, Store};
use rayon::prelude::*;
use serde::Serialize;
use tauri::State;
use tauri::ipc::Channel;

struct AppState {
    rules: Vec<Rule>,
    ctx: ScanContext,
    store: Mutex<Store>,
}

type Shared<'a> = State<'a, Arc<AppState>>;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ScanEvent {
    Started { total: u32 },
    Finding { finding: Finding },
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ScanResult {
    #[specta(type = specta_typescript::Number)]
    pub scanned_at: i64,
    pub disk: Option<DiskSpace>,
    /// In rule order.
    pub findings: Vec<Finding>,
    /// Size of each rule in the previous scan that included it.
    pub previous: HashMap<String, PreviousSize>,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Runs blocking work off the async runtime.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(err)?
}

#[tauri::command]
#[specta::specta]
fn get_rules(state: Shared<'_>) -> Vec<Rule> {
    state.rules.clone()
}

#[tauri::command]
#[specta::specta]
fn get_disk(state: Shared<'_>) -> Option<DiskSpace> {
    disk::space(&state.ctx.home)
}

#[tauri::command]
#[specta::specta]
async fn scan(state: Shared<'_>, on_event: Channel<ScanEvent>) -> Result<ScanResult, String> {
    let state = Arc::clone(&state);
    blocking(move || {
        let rules: Vec<&Rule> = state
            .rules
            .iter()
            .filter(|r| r.applies_to(state.ctx.platform))
            .collect();
        let _ = on_event.send(ScanEvent::Started {
            total: u32::try_from(rules.len()).unwrap_or(u32::MAX),
        });
        let findings: Vec<Finding> = rules
            .par_iter()
            .map(|rule| {
                let finding = scan_rule(rule, &state.ctx);
                let _ = on_event.send(ScanEvent::Finding {
                    finding: finding.clone(),
                });
                finding
            })
            .collect();

        let scanned_at = now();
        let disk = disk::space(&state.ctx.home);
        let mut store = state.store.lock().map_err(err)?;
        let scan_id = store
            .record_scan(scanned_at, disk, &findings)
            .map_err(err)?;
        let previous = store.previous_sizes(scan_id).map_err(err)?;
        Ok(ScanResult {
            scanned_at,
            disk,
            findings,
            previous,
        })
    })
    .await
}

fn fresh_plan(state: &AppState, ids: Vec<String>) -> Result<Plan, String> {
    let selected: Vec<Rule> = state
        .rules
        .iter()
        .filter(|r| ids.contains(&r.id))
        .cloned()
        .collect();
    let findings = devtidy_core::scan(&selected, &state.ctx);
    clean::plan(&state.rules, &findings, &Selection::Ids(ids), &RealSystem).map_err(err)
}

/// What cleaning these rules would do right now. Changes nothing.
#[tauri::command]
#[specta::specta]
async fn plan_clean(state: Shared<'_>, ids: Vec<String>) -> Result<Plan, String> {
    let state = Arc::clone(&state);
    blocking(move || fresh_plan(&state, ids)).await
}

/// Re-plans from a fresh scan and executes. The frontend only chooses ids.
#[tauri::command]
#[specta::specta]
async fn clean(state: Shared<'_>, ids: Vec<String>) -> Result<Report, String> {
    let state = Arc::clone(&state);
    blocking(move || {
        let plan = fresh_plan(&state, ids)?;
        let report = clean::execute(&plan, &state.ctx.home, &RealSystem);
        state
            .store
            .lock()
            .map_err(err)?
            .record_cleanup(now(), &report)
            .map_err(err)?;
        Ok(report)
    })
    .await
}

#[tauri::command]
#[specta::specta]
fn get_history(state: Shared<'_>, limit: u32) -> Result<Vec<CleanupRecord>, String> {
    state
        .store
        .lock()
        .map_err(err)?
        .cleanups(limit)
        .map_err(err)
}

fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        get_rules,
        get_disk,
        scan,
        plan_clean,
        clean,
        get_history
    ])
}

fn export_bindings(builder: &tauri_specta::Builder<tauri::Wry>) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts");
    builder
        .export(
            specta_typescript::Typescript::default().header("// @ts-nocheck"),
            path,
        )
        .expect("failed to export TypeScript bindings");
}

fn app_state() -> Result<AppState, Box<dyn std::error::Error>> {
    let home = std::env::var_os("DEVTIDY_HOME")
        .map(Into::into)
        .or_else(dirs_home)
        .ok_or("could not determine the home directory")?;
    let rules = devtidy_rules::builtin_and_user(
        &std::path::Path::new(&home).join(".config/devtidy/rules"),
    )?;
    Ok(AppState {
        rules,
        ctx: ScanContext {
            home,
            platform: Platform::current(),
        },
        store: Mutex::new(Store::open_default()?),
    })
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(Into::into)
}

/// Starts the app.
///
/// # Panics
///
/// If the window cannot be created or the history database cannot be opened.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();
    #[cfg(debug_assertions)]
    export_bindings(&builder);

    let state = app_state().expect("failed to initialize DevTidy");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::new(state))
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running DevTidy");
}

#[cfg(test)]
mod tests {
    /// Keeps `src/bindings.ts` in sync; CI fails if this produces a diff.
    #[test]
    fn export_typescript_bindings() {
        super::export_bindings(&super::specta_builder());
    }
}
