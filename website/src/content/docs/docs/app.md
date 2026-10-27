---
title: Desktop app
description: A tour of the DevTidy app for macOS.
---

The app shows the same information as `devtidy scan`, in a window built for
browsing and deciding.

## The window

- **Header**: how much can be reclaimed in total and how much of that is
  marked safe, plus a disk usage bar where the reclaimable part is highlighted.
  **Rescan** runs a new scan; results stream in as each location is measured.
- **Sidebar**: Overview, every ecosystem with its total size, and History.
  **About DevTidy** sits at the bottom.
- **Item list**: every location found, largest first, with its safety level.
  Under the size you see either what removal would really free (for
  hard-linked stores such as pnpm) or how much it grew since the previous
  scan.
- **Details panel**: select an item to see what it is, what happens after
  removal, how expensive it is to regenerate, how it will be removed, which
  apps must be quit first, and where it lives, with a **Show in Finder**
  button for every path.

## Cleaning

1. Tick the items you want to remove, or use **Select all safe**.
2. Press **Clean**. DevTidy builds a plan from a fresh scan and shows exactly
   what will happen to each item: deleted, moved to the Trash, or the tool's
   own cleanup command.
3. Confirm. Items whose app is running (for example Xcode or the Simulator)
   are skipped and marked as such.
4. The result shows the space freed, measured on disk, and anything moved to
   the Trash.

A **dangerous** item can only be selected on its own. Selecting one clears
the rest of the selection, and the dialog asks you to type its id before the
**Clean** button is enabled.

## History

Every cleanup is listed with what it freed and which items it touched. The
app and the CLI share the same database, so `devtidy history` shows the same
list.

## How the app talks to the engine

The app never sends file paths to the backend. It sends rule ids, and every
cleanup is planned again from a fresh scan in Rust, so the window cannot ask
for anything a rule does not allow. See the [safety model](../safety/).
