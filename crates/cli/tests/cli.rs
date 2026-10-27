use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;

fn devtidy(home: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("devtidy").unwrap();
    cmd.env("DEVTIDY_HOME", home).env("NO_COLOR", "1");
    cmd
}

fn fake_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let caches = home.path().join(".gradle/caches/modules-2");
    fs::create_dir_all(&caches).unwrap();
    fs::write(caches.join("dep.jar"), vec![1u8; 2 * 1024 * 1024]).unwrap();
    home
}

#[test]
fn scan_json_follows_the_schema() {
    let home = fake_home();
    let out = devtidy(home.path())
        .args(["scan", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&out).unwrap();

    assert_eq!(json["schema_version"], 1);
    let gradle = json["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == "gradle.caches")
        .unwrap();
    assert_eq!(gradle["status"], "present");
    assert_eq!(gradle["safety"], "safe");
    assert!(gradle["allocated"].as_u64().unwrap() >= 2 * 1024 * 1024);
    assert!(json["totals"]["reclaimable_safe"].as_u64().unwrap() >= 2 * 1024 * 1024);
}

#[test]
fn scan_table_hides_missing_items_by_default() {
    let home = fake_home();
    devtidy(home.path())
        .arg("scan")
        .assert()
        .success()
        .stdout(predicate::str::contains("gradle.caches"))
        .stdout(predicate::str::contains("npm.cache").not());
}

#[test]
fn fail_above_exits_with_code_3() {
    let home = fake_home();
    devtidy(home.path())
        .args(["scan", "--fail-above", "1M"])
        .assert()
        .code(3);
    devtidy(home.path())
        .args(["scan", "--fail-above", "1G"])
        .assert()
        .success();
}

#[test]
fn explain_shows_cost_and_cleanup() {
    let home = fake_home();
    devtidy(home.path())
        .args(["explain", "npm.cache"])
        .assert()
        .success()
        .stdout(predicate::str::contains("npm cache clean --force"))
        .stdout(predicate::str::contains("Not present on this machine"));
}

#[test]
fn unknown_rule_is_an_error() {
    devtidy(fake_home().path())
        .args(["explain", "nope.nope"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("unknown rule"));
}

#[test]
fn bad_usage_exits_with_code_2() {
    devtidy(fake_home().path())
        .args(["scan", "--fail-above", "lots"])
        .assert()
        .code(2);
}

fn data_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn clean_dry_run_changes_nothing() {
    let home = fake_home();
    let data = data_dir();
    devtidy(home.path())
        .env("DEVTIDY_DATA_DIR", data.path())
        .args(["clean", "--safe", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Gradle caches"))
        .stdout(predicate::str::contains("Dry run"));
    assert!(
        home.path()
            .join(".gradle/caches/modules-2/dep.jar")
            .exists()
    );
    assert!(!data.path().join("history.jsonl").exists());
}

#[test]
fn clean_refuses_without_confirmation_when_not_interactive() {
    let home = fake_home();
    devtidy(home.path())
        .env("DEVTIDY_DATA_DIR", data_dir().path())
        .args(["clean", "gradle.caches"])
        .write_stdin("y\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("--yes"));
    assert!(
        home.path()
            .join(".gradle/caches/modules-2/dep.jar")
            .exists()
    );
}

#[test]
fn clean_with_yes_deletes_and_records_history() {
    let home = fake_home();
    let data = data_dir();
    devtidy(home.path())
        .env("DEVTIDY_DATA_DIR", data.path())
        .args(["clean", "--safe", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("done     Gradle caches"));
    assert!(!home.path().join(".gradle/caches").exists());

    let history = fs::read_to_string(data.path().join("history.jsonl")).unwrap();
    let first: serde_json::Value = serde_json::from_str(history.lines().next().unwrap()).unwrap();
    assert_eq!(first["rule_id"], "gradle.caches");
    assert_eq!(first["outcome"]["result"], "done");
}

#[test]
fn clean_needs_ids_or_safe() {
    devtidy(fake_home().path()).arg("clean").assert().code(2);
    devtidy(fake_home().path())
        .args(["clean", "--safe", "gradle.caches"])
        .assert()
        .code(2);
}

#[test]
fn dangerous_rule_cannot_be_combined() {
    devtidy(fake_home().path())
        .args(["clean", "gradle.caches", "xcode.archives", "--dry-run"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("dangerous"));
}

#[test]
fn clean_json_dry_run_contains_the_plan() {
    let home = fake_home();
    let out = devtidy(home.path())
        .args(["clean", "gradle.caches", "--dry-run", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["plan"]["actions"][0]["rule_id"], "gradle.caches");
    assert_eq!(json["plan"]["actions"][0]["method"]["kind"], "delete");
}
