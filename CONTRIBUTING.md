# Contributing

Thanks for helping! The most valuable contribution is a new rule for a tool
you use.

## Adding a rule

1. Add a `[[rule]]` entry to the matching file in `crates/rules/rules/`.
   Use the most specific path you can; validation rejects anything shallower
   than two levels below home.
2. Pick `safety` honestly:
   - `safe`: a pure cache the tool rebuilds without user action;
   - `caution`: rebuildable, but slow or with side effects;
   - `dangerous`: contains data that cannot be regenerated.
3. Prefer `cleanup = { kind = "command", ... }` when the tool ships its own
   cleanup command.
4. Run `INSTA_UPDATE=always cargo test -p devtidy-rules` and commit the
   updated snapshot. The fixture test creates every rule's paths in a fake
   home and checks they resolve.

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

Use [Conventional Commits](https://www.conventionalcommits.org/)
(`feat:`, `fix:`, `docs:`, ...); release notes are generated from them.

## Releasing

1. Bump the version in `Cargo.toml`, `app/package.json` and
   `app/src-tauri/tauri.conf.json`.
2. Add a section for it to `CHANGELOG.md`.
3. Commit, then tag and push: `git tag v0.3.0 && git push origin v0.3.0`.

The release workflow checks the versions and notes, builds the universal DMG
and CLI archives, uploads them with `SHA256SUMS` and publishes the release.

## Licensing

Unless you explicitly state otherwise, any contribution you submit is
dual-licensed as MIT OR Apache-2.0, without additional terms.
