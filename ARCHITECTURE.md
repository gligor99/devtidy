# Architecture

devtidy is a Rust workspace with one core and thin frontends.

```
crates/core    rule model, path resolution, scanner, size measurement, cleaner
crates/rules   built-in TOML rules, user rule loading, validation
crates/store   scan snapshots and cleanup history in SQLite
crates/cli     `devtidy` binary: clap, tables, JSON contract
app/           Tauri 2 desktop app: src-tauri (Rust) + React/TypeScript
website/       landing page and docs (Astro + Starlight), deployed to GitHub Pages
```

Frontends never touch the filesystem directly. Everything that reads or
deletes goes through `devtidy-core`, so the CLI and the desktop app share
one implementation of every safety rule.

## Rules are data, detectors are code

A **rule** is a declarative description of one location:

| Field | Meaning |
| --- | --- |
| `id` | Stable dotted id, e.g. `gradle.caches` |
| `paths` | `~/`-relative patterns, globs allowed |
| `safety` | `safe`, `caution` or `dangerous`; selects the removal strategy |
| `regen_cost` | `low`, `medium` or `high`; what getting it back costs |
| `description`, `regen_note` | What it is and what happens after removal |
| `cleanup` | `remove` (optionally `contents_only`) or the tool's own `command` |
| `processes` | Programs that must not run while cleaning |
| `platforms` | Optional restriction; a missing path simply matches nothing |

Rules are embedded in the binary with `include_str!`. Users can add rules in
`~/.config/devtidy/rules/*.toml` but cannot redefine built-in ids. Rules are
never downloaded at runtime: a rule decides what gets deleted, so a remote
rule feed would be remote code execution with extra steps.

Things that need logic (which Android images no AVD uses, unavailable
simulators, Docker via `docker system df`, stale projects) will be
implemented as a `Detector` trait in Rust, producing the same `Finding` type.

### Load-time invariants

`devtidy-rules::validate` rejects any rule that could reach outside its
intended scope:

- patterns must start with `~/` and have at least two literal components
  before the first glob (`~/Library` and `~/*/x` are rejected);
- no `.` or `..` components;
- ids are dotted lowercase; descriptions and regeneration notes are required;
- command cleanups must name a program.

## Measuring size honestly

`core::size::measure` walks each root without following symlinks or
crossing mount points and:

1. sums `st_blocks * 512` instead of `st_size`, so sparse files are counted
   at their real footprint;
2. tracks `(dev, ino)` for files with `nlink > 1` and counts each inode once;
3. marks an inode reclaimable only if all of its links were seen inside the
   rule's roots. A pnpm `node_modules` reports "8 GB, frees ~20 MB".

APFS clones and local snapshots cannot be detected cheaply. The cleaner
will therefore record the real freed space by comparing `statfs` before and
after, and explain any gap.

## Scan results

A `Finding` carries the matched paths, a `Measurement`
(`allocated`, `reclaimable`, `entries`, `denied`) and a `Status`:

- `missing`: nothing matched;
- `present`: fully measured;
- `partial`: some entries were unreadable, sizes are a lower bound;
- `denied`: only the roots were visible (typically needs Full Disk Access).

Rules are scanned in parallel with rayon.

## Cleaning

Cleaning is two steps, so every frontend can show the plan first:

1. `clean::plan` resolves a `Selection` (explicit ids, or every `safe` rule)
   into `Action`s with concrete targets, a method and the processes blocking
   them. A `dangerous` rule can only be selected on its own; `--safe` never
   selects `caution` or `dangerous` rules.
2. `clean::execute` re-checks every target with `clean::guard` immediately
   before removing it and records the outcome of each action.

| Safety | Method |
| --- | --- |
| `safe` | delete permanently, space is freed immediately |
| `caution` | move to Trash (NSFileManager on macOS, no extra permission) |
| `dangerous` | Trash, only when selected alone; the CLI asks you to type the id |

A rule with a `command` cleanup runs the tool's own command when the program
is on `PATH`, and otherwise falls back to removal if `fallback_remove` is set.

### The guard

Every target must be either one of the rule's matched roots, or a direct
child of a root that is a real directory (not a symlink). Paths must be
absolute with no `.` or `..` components. Consequences:

- a `contents_only` rule whose folder is a symlink removes nothing;
- a rule whose folder is a symlink and that removes the whole folder only
  removes the link, never its target;
- directory trees are removed with `std::fs::remove_dir_all`, which does not
  follow symlinks inside the tree.

All OS side effects (process checks, commands, Trash, free space) go through
the `clean::System` trait, so the cleaner is tested against a fake system and
real temporary directories.

### Measuring what was freed

`execute` reads the volume's available space before and after. Moved-to-Trash
bytes are reported separately, because they are only freed when the Trash is
emptied. If far less was freed than estimated, the CLI says why that can
happen (local snapshots, APFS clones, concurrent writes) instead of hiding it.

Each cleanup is recorded in the store (see below).

## History store

`devtidy-store` keeps four tables in SQLite: `scans`, `scan_items`,
`cleanups` and `cleanup_actions`. Schema changes are ordered migrations
tracked in `PRAGMA user_version`; a database from a newer devtidy is refused
rather than corrupted. WAL mode and a busy timeout let the CLI and the app
write at the same time.

Every scan stores one row per present rule, so growth is "size now minus the
size in the latest earlier scan that included this rule". The v1.x
background agent will simply run `devtidy scan` on a schedule.

## Desktop app

```
React (TanStack Query) ──typed commands──▶ src-tauri ──▶ devtidy-core / rules / store
        ▲                                     │
        └──────── Channel<ScanEvent> ◀────────┘  (findings stream in while scanning)
```

- **One source of truth for types.** Rust types derive `specta::Type` behind
  a `specta` feature, and `tauri-specta` generates `app/src/bindings.ts`.
  A Rust test regenerates the file and CI fails on any diff, so the UI cannot
  drift from the backend. Byte counts are exported as `number`: they stay far
  below 2^53.
- **The UI chooses ids, never paths.** `plan_clean` and `clean` take rule ids
  and re-plan from a fresh scan. The dialog shows the plan returned by Rust,
  and `clean` plans again rather than trusting it.
- **Same rules as the CLI.** A dangerous item can only be selected alone,
  "Select all safe" mirrors `--safe`, and a dangerous cleanup requires typing
  its id.
- **Blocking work off the main thread.** Scans and cleanups run in
  `spawn_blocking`; the scan is parallel and sends each finding over a
  `Channel` as soon as it is measured.
- **Shared data.** The app opens the same database as the CLI, so history
  and growth are the same in both.
- **Minimal permissions.** Capabilities allow window dragging and revealing
  a path in Finder, nothing else. The CSP only allows the app's own assets
  and the IPC channel.

## CLI contract

- Human output is a table on stdout; progress goes to stderr.
- `--json` output carries `schema_version`; any breaking change bumps it.
- Exit codes: `0` ok, `1` error, `2` usage, `3` above `--fail-above`.

## Privacy

devtidy makes no network requests. The desktop app will check GitHub
Releases for updates, and that check can be turned off.
