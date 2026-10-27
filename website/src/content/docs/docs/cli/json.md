---
title: JSON output
description: The machine-readable output of devtidy.
---

Every command accepts `--json`. The output is a single JSON document on
stdout with a top-level `schema_version`. Any breaking change to the shape
increments it, so scripts can check it.

## scan

```json
{
  "schema_version": 1,
  "home": "/Users/you",
  "items": [
    {
      "id": "pub.cache",
      "name": "Dart / Flutter pub cache",
      "ecosystem": "flutter",
      "safety": "caution",
      "regen_cost": "medium",
      "status": "present",
      "paths": ["/Users/you/.pub-cache/hosted"],
      "allocated": 323223552,
      "reclaimable": 323223552,
      "entries": 16232,
      "denied": 0
    }
  ],
  "totals": {
    "allocated": 323223552,
    "reclaimable": 323223552,
    "reclaimable_safe": 0
  }
}
```

| Field | Meaning |
| --- | --- |
| `status` | `present`, `partial` (some entries unreadable), `denied` (no access) or `missing` (only with `--all`). |
| `allocated` | Bytes allocated on disk, each inode counted once. |
| `reclaimable` | Bytes removal is expected to free. Lower than `allocated` when files are hard-linked from elsewhere. |
| `entries` | Files, folders and links visited. |
| `denied` | Entries that could not be read. |

Sizes are in bytes.

## explain

The same fields as one `scan` item, plus `schema_version`.

## clean

```json
{
  "schema_version": 1,
  "plan": {
    "actions": [
      {
        "rule_id": "gradle.caches",
        "name": "Gradle caches",
        "safety": "safe",
        "method": { "kind": "delete" },
        "roots": ["/Users/you/.gradle/caches"],
        "targets": ["/Users/you/.gradle/caches"],
        "estimated": 12100000000,
        "blocked_by": []
      }
    ],
    "skipped": [{ "rule_id": "npm.cache", "reason": "not present" }]
  },
  "report": {
    "results": [
      {
        "rule_id": "gradle.caches",
        "method": { "kind": "delete" },
        "estimated": 12100000000,
        "outcome": { "result": "done" }
      }
    ],
    "freed": 12050000000,
    "trashed": 0
  }
}
```

`report` is omitted with `--dry-run`. `method.kind` is `delete`, `trash` or
`command` (with `argv`). `outcome.result` is `done`, `blocked` (with
`processes`) or `failed` (with `error`). `freed` is measured on the volume and
can be `null` if it could not be read.

## history

```json
{
  "schema_version": 1,
  "cleanups": [
    {
      "id": 3,
      "at": 1793113941,
      "freed": 12050000000,
      "trashed": 0,
      "actions": [
        { "rule_id": "gradle.caches", "method": "delete", "estimated": 12100000000, "outcome": "done", "detail": null }
      ]
    }
  ]
}
```

`at` is a Unix timestamp in seconds.
