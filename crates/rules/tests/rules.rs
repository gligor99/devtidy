use std::fs;
use std::path::Path;

use devtidy_core::{Platform, ScanContext, Status, scan};
use devtidy_rules::{RuleError, builtin, builtin_and_user};

/// Turns a rule pattern into a concrete path that the pattern matches.
fn materialize(pattern: &str, home: &Path) -> std::path::PathBuf {
    home.join(pattern.trim_start_matches("~/").replace('*', "fixture"))
}

#[test]
fn every_builtin_rule_resolves_against_a_fake_home() {
    let home = tempfile::tempdir().unwrap();
    let rules = builtin();
    for rule in &rules {
        for pattern in &rule.paths {
            let dir = materialize(pattern, home.path());
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("payload.bin"), vec![0u8; 64 * 1024]).unwrap();
        }
    }

    for platform in [Platform::Macos, Platform::Linux] {
        let ctx = ScanContext {
            home: home.path().to_owned(),
            platform,
        };
        let findings = scan(&rules, &ctx);
        let report: Vec<String> = findings
            .iter()
            .map(|f| {
                assert!(
                    f.size.allocated >= 64 * 1024,
                    "{} measured {} bytes",
                    f.rule_id,
                    f.size.allocated
                );
                let paths: Vec<String> = f
                    .paths
                    .iter()
                    .map(|p| p.strip_prefix(home.path()).unwrap().display().to_string())
                    .collect();
                format!("{} {:?} {}", f.rule_id, f.status, paths.join(", "))
            })
            .collect();
        assert!(findings.iter().all(|f| f.status == Status::Present));
        insta::assert_yaml_snapshot!(format!("fake_home_{platform:?}").to_lowercase(), report);
    }
}

#[test]
fn rules_scan_as_missing_on_an_empty_home() {
    let home = tempfile::tempdir().unwrap();
    let ctx = ScanContext {
        home: home.path().to_owned(),
        platform: Platform::Macos,
    };
    assert!(
        scan(&builtin(), &ctx)
            .iter()
            .all(|f| f.status == Status::Missing)
    );
}

const VALID: &str = r#"
[[rule]]
id = "acme.cache"
name = "Acme cache"
ecosystem = "system"
paths = ["~/.acme/cache"]
safety = "safe"
regen_cost = "low"
description = "Acme downloads."
regen_note = "Downloaded again."
cleanup = { kind = "remove" }
"#;

fn load_user(src: &str) -> Result<Vec<devtidy_core::Rule>, RuleError> {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("user.toml"), src).unwrap();
    builtin_and_user(dir.path())
}

#[test]
fn user_rules_are_appended() {
    let rules = load_user(VALID).unwrap();
    assert_eq!(rules.last().unwrap().id, "acme.cache");
    assert_eq!(rules.len(), builtin().len() + 1);
}

#[test]
fn missing_user_dir_is_fine() {
    assert_eq!(
        builtin_and_user(Path::new("/nonexistent/devtidy"))
            .unwrap()
            .len(),
        builtin().len()
    );
}

#[test]
fn user_rules_cannot_shadow_builtins() {
    let err = load_user(&VALID.replace("acme.cache", "gradle.caches")).unwrap_err();
    assert!(matches!(err, RuleError::Duplicate { .. }), "{err}");
}

#[test]
fn dangerous_paths_are_rejected() {
    for bad in ["/etc", "~", "~/Library", "~/../root/x", "~/*/x"] {
        let err = load_user(&VALID.replace("~/.acme/cache", bad)).unwrap_err();
        assert!(matches!(err, RuleError::Invalid { .. }), "{bad}: {err}");
    }
}

#[test]
fn malformed_rules_are_rejected() {
    let cases = [
        VALID.replace("acme.cache", "Acme"),
        VALID.replace(
            "cleanup = { kind = \"remove\" }",
            "cleanup = { kind = \"command\", argv = [] }",
        ),
        VALID.replace("regen_note = \"Downloaded again.\"", "regen_note = \"\""),
        VALID.replace("safety = \"safe\"", "safety = \"yolo\""),
        format!("{VALID}\nunknown_field = 1"),
    ];
    for case in cases {
        assert!(load_user(&case).is_err(), "accepted:\n{case}");
    }
}
