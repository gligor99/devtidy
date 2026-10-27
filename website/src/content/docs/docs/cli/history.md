---
title: history & rules
description: Past cleanups, and the list of known locations.
---

## devtidy history

```sh
devtidy history [--limit <N>]
```

Lists past cleanups, newest first, with the space freed, what went to the
Trash, and how many items were cleaned. Shows the last 20 by default. The app
and the CLI share this history.

```text
$ devtidy history
┌────────────┬─────────┬─────────┬──────────┐
│ When       ┆ Freed   ┆ Trashed ┆ Items    │
╞════════════╪═════════╪═════════╪══════════╡
│ 2 h ago    ┆ 30.1 GB ┆ 0 B     ┆ 2/2 done │
│ 6 days ago ┆ 4.6 GB  ┆ 1.3 GB  ┆ 7/8 done │
└────────────┴─────────┴─────────┴──────────┘
```

## devtidy rules

```sh
devtidy rules
```

Lists every rule DevTidy knows, built-in and [custom](../../rules/custom/),
with id, name, ecosystem and safety level. The
[rules reference](../../rules/) documents each one.
