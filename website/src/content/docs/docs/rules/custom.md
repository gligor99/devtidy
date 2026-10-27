---
title: Custom rules
description: Teach DevTidy about tools it does not know yet.
---

Add your own rules as TOML files in `~/.config/devtidy/rules/` (or the folder
given by `--rules-dir` / `DEVTIDY_RULES_DIR`). They work in the CLI and the
app.

```toml
# ~/.config/devtidy/rules/acme.toml
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
processes = ["acme-daemon"]
docs = "https://acme.dev/docs/cache"
```

## Fields

| Field | Required | Meaning |
| --- | --- | --- |
| `id` | yes | Dotted lowercase id, e.g. `acme.cache`. Must not clash with a built-in id. |
| `name` | yes | Shown in the app and the CLI. |
| `ecosystem` | yes | `ios`, `android`, `javascript`, `flutter`, `rust`, `python`, `docker` or `system`. |
| `paths` | yes | Home-relative patterns starting with `~/`, at least two levels deep. Globs (`*`, `?`, `[ ]`) allowed. |
| `safety` | yes | `safe`, `caution` or `dangerous`. See the [safety model](../../safety/). |
| `regen_cost` | yes | `low`, `medium` or `high`. |
| `description` | yes | What it is. |
| `regen_note` | yes | What happens after removal. |
| `cleanup` | yes | `{ kind = "remove" }`, optionally with `contents_only = true` to keep the folder, or `{ kind = "command", argv = ["tool", "clean"] }`, optionally with `fallback_remove = true`. |
| `platforms` | no | `macos`, `linux`, `windows`. Empty means all. |
| `processes` | no | Process names that must not be running while cleaning. |
| `docs` | no | An `https://` link. |

## Choosing a safety level

- **safe**: a pure cache the tool rebuilds on its own with no user action.
- **caution**: rebuildable, but slow or with side effects (for example, a
  re-download of several GB).
- **dangerous**: contains something that cannot be regenerated.

When unsure, pick the more careful level.

## Validation

Invalid rules stop DevTidy with an error that names the file and the rule.
Rules are rejected if a path could reach your home folder itself, a
top-level folder like `~/Library`, or anything above home.

## Contributing a rule

Built-in rules live in
[`crates/rules/rules/`](https://github.com/gligor99/devtidy/tree/main/crates/rules/rules).
Open a pull request; see
[CONTRIBUTING.md](https://github.com/gligor99/devtidy/blob/main/CONTRIBUTING.md).
