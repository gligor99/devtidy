# Architecture

devtidy is a Rust workspace with one core and thin frontends.

```
crates/core    rule model, path resolution, scanner, size measurement, (cleaner)
crates/rules   built-in TOML rules, user rule loading, validation
crates/store   scan history in SQLite                      (v0.2)
crates/cli     `devtidy` binary: clap, tables, JSON contract
app/           Tauri 2 desktop app, React + TypeScript     (v0.2)
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

## Cleaning (in progress)

Removal strategy by safety:

| Safety | Strategy |
| --- | --- |
| `safe` | delete permanently, space is freed immediately |
| `caution` | move to Trash |
| `dangerous` | Trash, only when selected individually |

Before cleaning, devtidy checks the rule's `processes`. Every action is
appended to `history.jsonl` in the data directory. `clean` without `--yes`
always prints the plan first.

## CLI contract

- Human output is a table on stdout; progress goes to stderr.
- `--json` output carries `schema_version`; any breaking change bumps it.
- Exit codes: `0` ok, `1` error, `2` usage, `3` above `--fail-above`.

## Privacy

devtidy makes no network requests. The desktop app will check GitHub
Releases for updates, and that check can be turned off.
