# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/). Release notes on GitHub are taken
from this file, so every release tag needs a section here.

## [0.2.0] - 2026-10-27

### Added

- DevTidy desktop app for macOS: items grouped by ecosystem, live scan
  progress, growth since the previous scan, a details panel explaining each
  item, and a review step before anything is cleaned.
- Dangerous items can only be cleaned on their own and require typing their id.
- Scan and cleanup history in SQLite, shared by the CLI and the app.
- `devtidy history` and `devtidy scan --no-record`.
- App icon and About window.

### Changed

- Cleanup history moved from `history.jsonl` to `devtidy.db`.

## [0.1.0] - 2026-10-27

### Added

- `devtidy scan`, `explain`, `rules` and `clean` with dry runs, JSON output
  and stable exit codes.
- 23 built-in rules for Xcode, simulators, Android, Gradle, npm, Yarn, pnpm,
  Bun, CocoaPods, pub, Cargo, pip and Homebrew.
- Honest size measurement: allocated blocks, hard links counted once,
  reclaimable estimates and measured freed space.
- Guarded cleanup that never leaves a rule's paths or follows symlinks,
  covered by property-based tests.

[0.2.0]: https://github.com/gligor99/devtidy/releases/tag/v0.2.0
[0.1.0]: https://github.com/gligor99/devtidy/commit/e8b6e16
