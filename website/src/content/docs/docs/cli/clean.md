---
title: devtidy clean
description: Reclaim space with a plan you see before anything changes.
---

```sh
devtidy clean [OPTIONS] [IDS]...
```

Pass rule ids, or `--safe` for every item marked safe. `clean` always prints
the plan first.

| Option | Meaning |
| --- | --- |
| `--safe` | Clean every present item marked `safe`. Never selects `caution` or `dangerous` items. |
| `-n, --dry-run` | Show the plan and exit without changing anything. |
| `-y, --yes` | Do not ask for confirmation. Required when stdin is not a terminal. |

## What happens

1. The selected rules are scanned again.
2. A plan is printed: each item, its size, its safety level and the action.
   Items whose app is running (for example Xcode) are marked as skipped.
3. You confirm with `y`. A **dangerous** item instead asks you to type its id.
4. Each target is re-checked by the [guard](../../safety/#the-guard) right
   before removal.
5. The result lists each item and the space freed, **measured on the volume**.

```text
$ devtidy clean --safe
┌───────────────────┬─────────┬────────┬────────┬──────┐
│ Item              ┆ Size    ┆ Safety ┆ Action ┆ Note │
╞═══════════════════╪═════════╪════════╪════════╪══════╡
│ Xcode DerivedData ┆ 18.2 GB ┆ safe   ┆ delete ┆      │
│ Gradle caches     ┆ 12.1 GB ┆ safe   ┆ delete ┆      │
└───────────────────┴─────────┴────────┴────────┴──────┘
Estimated to reclaim: 30.3 GB.
Proceed? [y/N] y
  done     Xcode DerivedData
  done     Gradle caches

Freed 30.1 GB (measured on disk).
```

## Removal methods

| Safety | Method |
| --- | --- |
| `safe` | Deleted permanently. |
| `caution` | Moved to the Trash. Space is freed when you empty it. |
| `dangerous` | Moved to the Trash, only when cleaned on its own. |

When a tool has its own cleanup command (`npm cache clean --force`,
`pod cache clean --all`, `xcrun simctl delete all`), DevTidy runs that
instead. Some rules fall back to removal if the tool is not installed.

## In scripts

```sh
devtidy clean --safe --yes --json > cleanup.json
```

Exits with `1` if any action failed. Every cleanup is recorded in
[history](../history/).
