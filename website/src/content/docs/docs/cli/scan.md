---
title: devtidy scan
description: Show what developer tooling is using disk space.
---

```sh
devtidy scan [OPTIONS]
```

Measures every known location and prints them largest first, with their
safety level. Locations that do not exist on your machine are hidden.

| Option | Meaning |
| --- | --- |
| `-e, --ecosystem <NAME>` | Only these ecosystems. Repeatable. One of `ios`, `android`, `javascript`, `flutter`, `rust`, `python`, `docker`, `system`. |
| `--all` | Also list known locations that are not present. |
| `--no-record` | Do not save this scan to the history database. |
| `--fail-above <SIZE>` | Exit with code `3` if the reclaimable total exceeds `SIZE`, e.g. `500M`, `20G`, `1.5T`. |

## Sizes

Sizes use SI units (1 GB = 1,000,000,000 bytes), like Finder. When an item's
files are also hard-linked from elsewhere, the table shows both numbers:
`8.4 GB (frees ~1.3 GB)`. A trailing `+?` means some files could not be read
and the size is a lower bound; `no access` means the folder needs more
permissions. See [how sizes are measured](../../sizes/).

## History

Every scan is recorded so the app can show growth since the previous scan.
Use `--no-record` for one-off or scripted scans you do not want in history.

## Warn before the disk fills up

```sh
# cron, launchd or CI: non-zero exit when there is more than 50 GB to reclaim
devtidy scan --no-record --fail-above 50G > /dev/null || echo "Time to tidy up"
```
