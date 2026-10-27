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
