---
title: CLI overview
description: The devtidy command line interface.
---

`devtidy` is the same engine as the app, for terminals, scripts and CI.

```sh
devtidy scan                       # what is using space
devtidy explain gradle.caches      # what an item is and what removing it costs
devtidy clean --safe --dry-run     # what would be removed
devtidy clean --safe               # remove everything marked safe (asks first)
devtidy history                    # past cleanups and what they freed
devtidy rules                      # every known location
```

## Global options

These work with every command.

| Option | Environment | Meaning |
| --- | --- | --- |
| `--json` | | Print machine-readable JSON instead of a table. See [JSON output](json/). |
| `--home <HOME>` | `DEVTIDY_HOME` | Home directory to scan. |
| `--rules-dir <DIR>` | `DEVTIDY_RULES_DIR` | Directory with extra [custom rules](../rules/custom/). Defaults to `~/.config/devtidy/rules`. |
| | `DEVTIDY_DATA_DIR` | Where the history database lives. Defaults to `~/Library/Application Support/devtidy` on macOS. |
| | `NO_COLOR` | Disable colors. |

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Success. |
| `1` | Error, or a cleanup action failed. |
| `2` | Invalid usage. |
| `3` | `scan --fail-above` threshold exceeded. |

## Output streams

Progress messages go to stderr and data to stdout, so piping is always safe:

```sh
devtidy scan --json | jq '.totals.reclaimable'
```
