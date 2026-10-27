use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use devtidy_core::clean::{self, Method, Outcome, PlanError, Selection, System, guard};
use devtidy_core::{Cleanup, Ecosystem, Platform, RegenCost, Rule, Safety, ScanContext, scan};
use proptest::prelude::*;

/// Fake OS: trash moves files into `trash_dir`, commands are recorded.
#[derive(Default)]
struct FakeSystem {
    running: Vec<String>,
    programs: Vec<String>,
    trash_dir: Option<PathBuf>,
    ran: RefCell<Vec<Vec<String>>>,
}

impl System for FakeSystem {
    fn is_running(&self, process: &str) -> bool {
        self.running.iter().any(|p| p == process)
    }
    fn has_program(&self, program: &str) -> bool {
        self.programs.iter().any(|p| p == program)
    }
    fn run(&self, argv: &[String]) -> Result<(), String> {
        self.ran.borrow_mut().push(argv.to_vec());
        Ok(())
    }
    fn trash(&self, paths: &[PathBuf]) -> Result<(), String> {
        let dir = self.trash_dir.as_ref().expect("trash dir");
        for (i, path) in paths.iter().enumerate() {
            fs::rename(
                path,
                dir.join(format!(
                    "{i}-{}",
                    path.file_name().unwrap().to_string_lossy()
                )),
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn available_space(&self, _path: &Path) -> Option<u64> {
        None
    }
}

fn rule(id: &str, path: &str, safety: Safety, cleanup: Cleanup) -> Rule {
    Rule {
        id: id.into(),
        name: id.into(),
        ecosystem: Ecosystem::System,
        platforms: Vec::new(),
        paths: vec![path.into()],
        safety,
        regen_cost: RegenCost::Low,
        description: "test".into(),
        regen_note: "test".into(),
        cleanup,
        processes: vec!["ToolApp".into()],
        docs: None,
    }
}

fn remove(contents_only: bool) -> Cleanup {
    Cleanup::Remove { contents_only }
}

fn ctx(home: &Path) -> ScanContext {
    ScanContext {
        home: home.to_owned(),
        platform: Platform::current(),
    }
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn run(
    rules: &[Rule],
    home: &Path,
    selection: &Selection,
    sys: &FakeSystem,
) -> Result<clean::Report, PlanError> {
    let findings = scan(rules, &ctx(home));
    let plan = clean::plan(rules, &findings, selection, sys)?;
    Ok(clean::execute(&plan, home, sys))
}

#[test]
fn safe_contents_only_deletes_children_and_keeps_the_folder() {
    let home = tempfile::tempdir().unwrap();
    write(&home.path().join("tool/cache/a/b.bin"), "x");
    write(&home.path().join("tool/cache/c.bin"), "x");
    let rules = [rule(
        "tool.cache",
        "~/tool/cache",
        Safety::Safe,
        remove(true),
    )];

    let report = run(
        &rules,
        home.path(),
        &Selection::AllSafe,
        &FakeSystem::default(),
    )
    .unwrap();

    assert_eq!(report.results[0].outcome, Outcome::Done);
    let cache = home.path().join("tool/cache");
    assert!(cache.is_dir());
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
}

#[test]
fn caution_goes_to_trash() {
    let home = tempfile::tempdir().unwrap();
    let trash = tempfile::tempdir().unwrap();
    write(&home.path().join("tool/store/pkg"), "x");
    let rules = [rule(
        "tool.store",
        "~/tool/store",
        Safety::Caution,
        remove(false),
    )];
    let sys = FakeSystem {
        trash_dir: Some(trash.path().to_owned()),
        ..FakeSystem::default()
    };

    let report = run(
        &rules,
        home.path(),
        &Selection::Ids(vec!["tool.store".into()]),
        &sys,
    )
    .unwrap();

    assert_eq!(report.results[0].method, Method::Trash);
    assert!(!home.path().join("tool/store").exists());
    assert!(trash.path().join("0-store/pkg").exists());
    assert!(report.trashed > 0);
}

#[test]
fn all_safe_never_selects_caution_or_dangerous() {
    let home = tempfile::tempdir().unwrap();
    write(&home.path().join("tool/a/f"), "x");
    write(&home.path().join("tool/b/f"), "x");
    let rules = [
        rule("tool.a", "~/tool/a", Safety::Caution, remove(false)),
        rule("tool.b", "~/tool/b", Safety::Dangerous, remove(false)),
    ];
    let err = run(
        &rules,
        home.path(),
        &Selection::AllSafe,
        &FakeSystem::default(),
    )
    .unwrap_err();
    assert_eq!(err, PlanError::Empty);
    assert!(home.path().join("tool/a/f").exists() && home.path().join("tool/b/f").exists());
}

#[test]
fn dangerous_rules_must_be_cleaned_alone() {
    let home = tempfile::tempdir().unwrap();
    let rules = [
        rule("tool.a", "~/tool/a", Safety::Safe, remove(false)),
        rule("tool.b", "~/tool/b", Safety::Dangerous, remove(false)),
    ];
    let err = run(
        &rules,
        home.path(),
        &Selection::Ids(vec!["tool.a".into(), "tool.b".into()]),
        &FakeSystem::default(),
    )
    .unwrap_err();
    assert_eq!(err, PlanError::DangerousNotAlone("tool.b".into()));
}

#[test]
fn running_processes_block_the_action() {
    let home = tempfile::tempdir().unwrap();
    write(&home.path().join("tool/cache/f"), "x");
    let rules = [rule(
        "tool.cache",
        "~/tool/cache",
        Safety::Safe,
        remove(false),
    )];
    let sys = FakeSystem {
        running: vec!["ToolApp".into()],
        ..FakeSystem::default()
    };

    let report = run(&rules, home.path(), &Selection::AllSafe, &sys).unwrap();

    assert_eq!(
        report.results[0].outcome,
        Outcome::Blocked {
            processes: vec!["ToolApp".into()]
        }
    );
    assert!(home.path().join("tool/cache/f").exists());
}

#[test]
fn commands_are_preferred_and_fall_back_when_missing() {
    let home = tempfile::tempdir().unwrap();
    write(&home.path().join("tool/cache/f"), "x");
    let cleanup = Cleanup::Command {
        argv: vec!["tool".into(), "clean".into()],
        fallback_remove: true,
    };
    let rules = [rule("tool.cache", "~/tool/cache", Safety::Safe, cleanup)];

    let with_tool = FakeSystem {
        programs: vec!["tool".into()],
        ..FakeSystem::default()
    };
    run(&rules, home.path(), &Selection::AllSafe, &with_tool).unwrap();
    assert_eq!(
        *with_tool.ran.borrow(),
        vec![vec!["tool".to_owned(), "clean".to_owned()]]
    );
    assert!(
        home.path().join("tool/cache/f").exists(),
        "command path must not delete files itself"
    );

    run(
        &rules,
        home.path(),
        &Selection::AllSafe,
        &FakeSystem::default(),
    )
    .unwrap();
    assert!(!home.path().join("tool/cache").exists());
}

#[test]
fn missing_command_without_fallback_is_skipped() {
    let home = tempfile::tempdir().unwrap();
    write(&home.path().join("tool/cache/f"), "x");
    let cleanup = Cleanup::Command {
        argv: vec!["tool".into()],
        fallback_remove: false,
    };
    let rules = [rule("tool.cache", "~/tool/cache", Safety::Safe, cleanup)];
    let findings = scan(&rules, &ctx(home.path()));

    let plan = clean::plan(
        &rules,
        &findings,
        &Selection::AllSafe,
        &FakeSystem::default(),
    )
    .unwrap();

    assert!(plan.actions.is_empty());
    assert_eq!(plan.skipped[0].reason, "`tool` is not installed");
}

#[test]
fn guard_rejects_everything_but_roots_and_their_direct_children() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("tool/cache");
    fs::create_dir_all(&root).unwrap();
    let roots = vec![root.clone()];

    assert!(guard(&root, &roots).is_ok());
    assert!(guard(&root.join("child"), &roots).is_ok());
    assert!(guard(&root.join("child/grandchild"), &roots).is_err());
    assert!(guard(&root.join("../other"), &roots).is_err());
    assert!(guard(home.path(), &roots).is_err());
    assert!(guard(Path::new("relative/path"), &roots).is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_root_only_removes_the_link() {
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(&outside.path().join("precious"), "keep");
    fs::create_dir_all(home.path().join("tool")).unwrap();
    std::os::unix::fs::symlink(outside.path(), home.path().join("tool/cache")).unwrap();

    for contents_only in [true, false] {
        let rules = [rule(
            "tool.cache",
            "~/tool/cache",
            Safety::Safe,
            remove(contents_only),
        )];
        let _ = run(
            &rules,
            home.path(),
            &Selection::AllSafe,
            &FakeSystem::default(),
        );
        assert_eq!(
            fs::read_to_string(outside.path().join("precious")).unwrap(),
            "keep"
        );
    }
    assert!(
        home.path().join("tool/cache").symlink_metadata().is_err(),
        "link itself is removed"
    );
}

// ---------------------------------------------------------------------------
// Property test: whatever the tree looks like, nothing outside the rule's
// paths changes.

#[derive(Debug, Clone)]
enum Node {
    File,
    Dir,
    SymlinkOutside,
    SymlinkToSibling,
    HardlinkFromOutside,
}

fn node() -> impl Strategy<Value = Node> {
    prop_oneof![
        3 => Just(Node::File),
        2 => Just(Node::Dir),
        1 => Just(Node::SymlinkOutside),
        1 => Just(Node::SymlinkToSibling),
        1 => Just(Node::HardlinkFromOutside),
    ]
}

/// Relative paths like `cache/b/a` under one of a few top-level folders.
fn entry() -> impl Strategy<Value = (Vec<&'static str>, Node)> {
    let top = prop::sample::select(vec!["tool/cache", "tool/other", "keep"]);
    let rest = prop::collection::vec(prop::sample::select(vec!["a", "b", "c"]), 0..3);
    (top, rest, node()).prop_map(|(top, rest, node)| {
        let mut segments: Vec<&'static str> = top.split('/').collect();
        segments.extend(rest);
        segments.push("n");
        (segments, node)
    })
}

#[cfg(unix)]
fn build(
    home: &Path,
    outside: &Path,
    entries: &[(Vec<&'static str>, Node)],
    root_is_symlink: bool,
) {
    use std::os::unix::fs::symlink;
    write(&outside.join("target/file"), "outside");
    write(&outside.join("linked"), "shared");
    if root_is_symlink {
        fs::create_dir_all(home.join("tool")).unwrap();
        symlink(outside.join("target"), home.join("tool/cache")).unwrap();
    }
    for (i, (segments, node)) in entries.iter().enumerate() {
        let mut path = home.to_owned();
        for s in segments {
            path.push(s);
        }
        let path = path.with_file_name(format!("n{i}"));
        // Skip entries whose parent chain is blocked by a file or symlink.
        let parent = path.parent().unwrap();
        if fs::create_dir_all(parent).is_err()
            || !parent.symlink_metadata().is_ok_and(|m| m.is_dir())
        {
            continue;
        }
        let _ = match node {
            Node::File => fs::write(&path, format!("content {i}")),
            Node::Dir => fs::create_dir_all(&path),
            Node::SymlinkOutside => symlink(outside.join("target"), &path),
            Node::SymlinkToSibling => symlink(home.join("keep"), &path),
            Node::HardlinkFromOutside => fs::hard_link(outside.join("linked"), &path),
        };
    }
}

/// Every entry under `dir`, without following symlinks, except those under `excluded`.
fn snapshot(dir: &Path, excluded: &Path) -> BTreeMap<PathBuf, String> {
    walkdir_no_follow(dir)
        .into_iter()
        .filter(|p| !p.starts_with(excluded))
        .map(|p| {
            let meta = p.symlink_metadata().unwrap();
            let desc = if meta.is_symlink() {
                format!("link -> {}", fs::read_link(&p).unwrap().display())
            } else if meta.is_dir() {
                "dir".to_owned()
            } else {
                format!("file {}", fs::read_to_string(&p).unwrap())
            };
            (p, desc)
        })
        .collect()
}

fn walkdir_no_follow(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_owned()];
    while let Some(path) = stack.pop() {
        let Ok(meta) = path.symlink_metadata() else {
            continue;
        };
        if meta.is_dir() {
            for entry in fs::read_dir(&path).unwrap() {
                stack.push(entry.unwrap().path());
            }
        }
        out.push(path);
    }
    out
}

#[cfg(unix)]
proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn cleaning_never_touches_anything_outside_the_rule(
        entries in prop::collection::vec(entry(), 0..40),
        contents_only in any::<bool>(),
        safety in prop::sample::select(vec![Safety::Safe, Safety::Caution]),
        root_is_symlink in any::<bool>(),
    ) {
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let trash = tempfile::tempdir().unwrap();
        build(home.path(), outside.path(), &entries, root_is_symlink);

        let root = home.path().join("tool/cache");
        let root_existed = root.symlink_metadata().is_ok();
        let home_before = snapshot(home.path(), &root);
        let outside_before = snapshot(outside.path(), Path::new("/nonexistent"));

        let rules = [rule("tool.cache", "~/tool/cache", safety, remove(contents_only))];
        let sys = FakeSystem { trash_dir: Some(trash.path().to_owned()), ..FakeSystem::default() };
        let _ = run(&rules, home.path(), &Selection::Ids(vec!["tool.cache".into()]), &sys);

        prop_assert_eq!(snapshot(home.path(), &root), home_before);
        prop_assert_eq!(snapshot(outside.path(), Path::new("/nonexistent")), outside_before);

        let root_meta = root.symlink_metadata();
        if !root_existed {
            prop_assert!(root_meta.is_err());
        } else if root_is_symlink {
            // Contents are never listed through a symlink; only the link may go.
            prop_assert!(contents_only == root_meta.is_ok());
        } else if contents_only {
            prop_assert!(root_meta.is_ok_and(|m| m.is_dir()));
            prop_assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        } else {
            prop_assert!(root_meta.is_err());
        }
    }
}

#[test]
fn empty_contents_only_folders_are_skipped() {
    let home = tempfile::tempdir().unwrap();
    fs::create_dir_all(home.path().join("tool/cache")).unwrap();
    let rules = [rule(
        "tool.cache",
        "~/tool/cache",
        Safety::Safe,
        remove(true),
    )];
    let findings = scan(&rules, &ctx(home.path()));

    let plan = clean::plan(
        &rules,
        &findings,
        &Selection::AllSafe,
        &FakeSystem::default(),
    )
    .unwrap();

    assert!(plan.actions.is_empty());
    assert_eq!(plan.skipped[0].reason, "already empty");
}
