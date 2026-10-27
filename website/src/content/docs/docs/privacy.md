---
title: Privacy
description: DevTidy never sends anything anywhere.
---

DevTidy makes **no network requests**. There is no telemetry, no analytics,
no crash reporting and no account.

- **Rules** ship inside the app and change only with a new release. They are
  never fetched from the internet.
- **History** of scans and cleanups is stored locally in
  `~/Library/Application Support/devtidy/devtidy.db` (or `$DEVTIDY_DATA_DIR`).
  Delete it at any time.
- **Permissions**: DevTidy reads the locations its rules describe. It does not
  need Full Disk Access, and the app can only reveal paths in Finder and open
  links in your browser.

This website is a static site on GitHub Pages and uses no analytics or
third-party scripts. The Ko-fi link is a plain link; nothing is loaded from
Ko-fi until you click it.

DevTidy is open source, so all of this can be verified in the
[source code](https://github.com/gligor99/devtidy).
