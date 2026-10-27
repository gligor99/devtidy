---
title: devtidy explain
description: Explain what an item is and what removing it costs.
---

```sh
devtidy explain <ID>
```

Prints everything DevTidy knows about one rule, plus its current size and
paths on this machine.

```text
$ devtidy explain gradle.caches
Gradle caches (gradle.caches)

Downloaded dependencies, transformed artifacts and build caches shared by all Gradle builds.

Ecosystem:   Android
Safety:      Safe
Regen cost:  Medium
After removal: The next build downloads and transforms dependencies again, which can take several minutes.
Cleanup:     delete permanently
Docs:        https://docs.gradle.org/current/userguide/directory_layout.html

On this machine: 12.1 GB
  /Users/you/.gradle/caches
```

Find ids with `devtidy scan` or `devtidy rules`, or in the
[rules reference](../../rules/).
