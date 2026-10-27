---
title: Safety model
description: How DevTidy decides what may be removed, and how it proves it.
---

DevTidy deletes files on your machine, so its first job is to never delete
the wrong ones.

## Rules are data

Everything DevTidy can remove is described by a **rule**: an id, the paths
where something lives, a safety level, and how it may be removed. Rules are
plain TOML that ship inside the binary. They are never downloaded at runtime:
a rule decides what gets deleted, so changing one has to go through review,
CI and a release.

When rules load, any rule that could reach too far is rejected:

- paths must start with `~/` and point at least two levels below your home
  folder (`~/Library` alone, or `~/*/cache`, are refused);
- no `.` or `..` path components;
- every rule must explain what it is and what happens after removal.

## Safety levels

| Level | Meaning | Removal |
| --- | --- | --- |
| **safe** | A pure cache the tool rebuilds by itself. | Deleted permanently. |
| **caution** | Rebuildable, but slow or with side effects. | Moved to the Trash. |
| **dangerous** | Contains data that cannot be regenerated. | Trash, only on its own, after typing its id. |

`--safe` and **Select all safe** never pick `caution` or `dangerous` items.

## The tool's own cleanup first

Tools keep metadata about their caches. When a tool ships a cleanup command,
DevTidy runs it instead of deleting files behind its back:
`npm cache clean --force`, `pod cache clean --all`, `dart pub cache clean`,
`xcrun simctl delete all`.

## Running apps

A rule can name processes that must not be running, such as Xcode, the
Simulator or an Android emulator. If one is running, that item is skipped
and you are told which app to quit.

## The guard

Every removal passes one check, immediately before it happens. A target must
be either:

- one of the paths the rule matched, or
- a direct child of such a path, **if that path is a real folder** and not a
  symlink.

Paths must be absolute and normalized. As a result:

- a rule whose folder is a symlink to somewhere else never removes the
  contents of that somewhere else; at most the link itself is removed;
- folders are removed with a function that does not follow symlinks inside
  the tree;
- a plan that went stale between scan and cleanup cannot reach outside the
  rule.

## How we know

- **Property-based tests** generate random trees of files, folders, symlinks
  pointing out of the tree and hard links from outside, run cleanups on
  them, and assert that nothing outside the rule changed. Deliberately
  breaking the symlink check makes these tests fail.
- Every built-in rule has a fixture test that checks its paths resolve.
- Rule validation is tested against paths like `/etc`, `~`, `~/Library` and
  `~/../x`.
- The desktop app never sends paths to the engine, only rule ids, and every
  cleanup is planned again in Rust.
