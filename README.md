<p align="center">
  <img src="assets/icon-256.png" width="128" height="128" alt="DevTidy icon">
</p>

<h1 align="center">devtidy</h1>

<p align="center"><strong>See what your developer tooling is doing to your disk, and reclaim it safely.</strong></p>

<p align="center">
  <a href="https://gligor99.github.io/devtidy/"><strong>Website</strong></a> ·
  <a href="https://gligor99.github.io/devtidy/docs/"><strong>Docs</strong></a> ·
  <a href="https://github.com/gligor99/devtidy/releases/latest"><strong>Download</strong></a> ·
  <a href="https://ko-fi.com/V7Q129KSZC"><strong>Support</strong></a>
</p>

<p align="center">
  <a href="https://github.com/gligor99/devtidy/actions/workflows/ci.yml"><img src="https://github.com/gligor99/devtidy/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" alt="License: MIT OR Apache-2.0">
  <img src="https://img.shields.io/badge/platform-macOS-lightgrey" alt="Platform: macOS">
</p>

Xcode DerivedData, simulators, Android system images, Gradle, npm, Yarn, pnpm,
CocoaPods, pub, Cargo… A few years of mobile and web development quietly eats
100+ GB. Generic disk tools show you *where* the bytes are. devtidy tells you
**what each item is, whether it is safe to remove, and what removing it will
cost you**.

```
$ devtidy scan
┌────────────────────────────────┬─────────────┬──────────┬───────────┬───────────────────────┐
│ Item                           ┆ Ecosystem   ┆ Size     ┆ Safety    ┆ Id                    │
╞════════════════════════════════╪═════════════╪══════════╪═══════════╪═══════════════════════╡
│ Android emulator system images ┆ Android     ┆  31.5 GB ┆ caution   ┆ android.system-images │
│ Simulator devices              ┆ iOS / Xcode ┆  18.3 GB ┆ dangerous ┆ simulator.devices     │
│ Xcode DerivedData              ┆ iOS / Xcode ┆  18.2 GB ┆ safe      ┆ xcode.derived-data    │
│ Gradle caches                  ┆ Android     ┆  12.1 GB ┆ safe      ┆ gradle.caches         │
│ pnpm store                     ┆ JavaScript  ┆   1.3 GB ┆ caution   ┆ pnpm.store            │
└────────────────────────────────┴─────────────┴──────────┴───────────┴───────────────────────┘
Total 81.4 GB, reclaimable ~81.4 GB (30.3 GB marked safe).
```

> **Status:** early development (v0.2.0). The CLI and a first version of the
> desktop app work; signed builds come with v1.0. See the [roadmap](#roadmap).

## Principles

- **Never lies about sizes.** devtidy counts allocated blocks, not file length,
  so sparse files (Docker's VM disk) are not inflated. Hard-linked files are
  counted once, and items whose files are also linked from elsewhere (pnpm,
  bun) show how much would *actually* be freed.
- **Never deletes outside a rule's paths.** Rules can only point at least two
  levels below your home directory, symlinks are never followed, and mounts
  are never crossed. These invariants are enforced at load time and tested.
- **Prefers the tool's own cleanup.** When a tool has a cleanup command
  (`npm cache clean`, `pod cache clean`, `xcrun simctl`), devtidy runs it
  instead of deleting files behind the tool's back.
- **Removal follows risk.** `safe` items are deleted, `caution` items go to
  the Trash, `dangerous` items (archives with dSYMs, simulators with app data)
  can only be removed one at a time.
- **Never phones home.** No telemetry, no analytics, no remotely fetched
  rules. Rules ship inside the signed binary and change only through a
  reviewed release.
- **Works without Full Disk Access.** Locations it cannot read are reported
  as such instead of silently skipped.

## Desktop app

DevTidy shows the same information as `devtidy scan` in a native window:
items grouped by ecosystem, a details panel that explains each one, growth
since the previous scan, and a review step before anything is cleaned. Scans
stream in live, and the app and the CLI share one history database.

The app never sends paths to the backend. It selects rule ids, and every
cleanup is re-planned from a fresh scan in Rust, so the UI cannot ask for
anything a rule does not allow.

## Download

Get the latest release from
**[GitHub Releases](https://github.com/gligor99/devtidy/releases/latest)**:

- **DevTidy for macOS** (12+, Apple Silicon and Intel): `DevTidy_<version>_universal.dmg`
- **CLI**: `devtidy-<version>-macos-universal.tar.gz` or `devtidy-<version>-linux-x86_64.tar.gz`

DevTidy is not notarized by Apple yet. On first launch, right-click the app
in Applications and choose **Open**, then **Open** again.

## Install from source

CLI (requires Rust 1.88+):

```sh
cargo install --git https://github.com/gligor99/devtidy devtidy
```

Homebrew and a notarized macOS app will follow with v1.0. Desktop app:

```sh
cd app
pnpm install
pnpm tauri dev      # or `pnpm tauri build` for a local .app
```

## Usage

```sh
devtidy scan                       # what is using space
devtidy scan --all                 # include known locations that are absent
devtidy scan -e ios -e android     # only some ecosystems
devtidy scan --json | jq           # machine-readable, versioned schema
devtidy scan --fail-above 50G      # exit code 3 when over a threshold (CI, cron)
devtidy explain xcode.archives     # what it is, what removal costs, how it is removed
devtidy rules                      # every known location

devtidy clean --safe --dry-run     # show what would be removed
devtidy clean --safe               # remove everything marked safe (asks first)
devtidy clean gradle.caches yarn.cache
devtidy clean xcode.archives       # dangerous: alone, confirmed by typing its id
devtidy history                    # past cleanups and what they freed
```

`clean` always prints the plan before touching anything. It refuses to run
non-interactively without `--yes`, skips items whose app is running (quit
Xcode or the Simulator first), and reports the space actually freed, measured
on the volume.

Every scan and cleanup is recorded in a SQLite database in your data
directory (`~/Library/Application Support/devtidy/devtidy.db` on macOS, or
`$DEVTIDY_DATA_DIR`), which is how devtidy shows growth since the last scan.
Use `scan --no-record` to skip it.

Exit codes: `0` success, `1` error, `2` invalid usage, `3` reclaimable total
above `--fail-above`.

Progress goes to stderr and data to stdout, so piping is always safe.
`NO_COLOR` is respected.

## Custom rules

Drop a TOML file into `~/.config/devtidy/rules/`:

```toml
[[rule]]
id = "acme.cache"
name = "Acme build cache"
ecosystem = "system"
paths = ["~/.acme/cache"]
safety = "safe"
regen_cost = "low"
description = "Build outputs cached by the Acme toolchain."
regen_note = "Rebuilt on the next Acme build."
cleanup = { kind = "remove", contents_only = true }
```

User rules can add new ids but never redefine built-in ones. See
[CONTRIBUTING.md](CONTRIBUTING.md) to upstream a rule.

## Roadmap

| Version | Scope |
| --- | --- |
| 0.1 | CLI: scanner, honest sizes, ~20 rules, `scan` / `explain` / `rules` / `clean` |
| 0.2 | Desktop app (Tauri), scan and cleanup history in SQLite |
| 0.3 | Project scanning: stale `node_modules`, `Pods`, build folders |
| 1.0 | Signed and notarized app, Homebrew |
| 1.x | Menu bar, scheduled background scans and alerts, Linux |

## How we know it is safe

- Rule validation rejects any path pattern that could reach home itself, a
  top-level folder, or anything above home.
- Every built-in rule has a fixture test, and sizes are checked against `du`,
  including sparse files, hard links and symlinks.
- Property-based tests generate random trees with files, directories,
  symlinks out of the tree and hard links from outside, then clean them and
  assert nothing outside the rule's paths changed, including when the rule's
  folder is itself a symlink. Mutation checks confirm these tests fail when
  the symlink protection is removed.
- The cleanup dialog is tested with a mocked backend: nothing runs before
  confirmation, dangerous items need their id typed, blocked items cannot run.
- CI runs on macOS and Linux with `clippy -D warnings`, `cargo deny`, the
  TypeScript type check, lint and Vitest, and fails if the generated
  TypeScript bindings are out of date.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the design.

## Support

DevTidy is free and always will be. If it saved you a few gigabytes and you
want to say thanks, you can [buy me a coffee on Ko-fi](https://ko-fi.com/V7Q129KSZC).
Starring the repo and reporting issues help just as much.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
