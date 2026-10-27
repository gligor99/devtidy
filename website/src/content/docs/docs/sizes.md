---
title: How sizes are measured
description: Why DevTidy's numbers can differ from other disk tools, and why they are right.
---

Most disk tools add up file lengths. On modern file systems that overstates
what you would get back. DevTidy measures three things differently.

## Allocated blocks, not length

A **sparse file**, such as the virtual disk of Docker Desktop, can report a
length of 64 GB while using 8 GB. DevTidy counts the blocks actually
allocated (`st_blocks`), so sizes match `du`, not Finder's "Size".

## Hard links counted once

pnpm and Bun keep one copy of each package in a global store and
**hard-link** it into every project. The same bytes appear in many places.
DevTidy counts each file once per item.

## What removal would really free

If a file inside an item is also linked from somewhere else, removing the
item does not free it. DevTidy tracks how many links each file has and how
many it saw, and reports two numbers when they differ:

```text
pnpm store   8.4 GB (frees ~1.3 GB)
```

## Measured after cleaning

APFS clones and local Time Machine snapshots can also keep blocks in use, and
they cannot be detected cheaply. So after a cleanup DevTidy reads the free
space of the volume before and after, and reports what was **really** freed.
If that is much less than estimated, it says why instead of hiding the gap.

## Units

Sizes use SI units, like Finder: 1 GB is 1,000,000,000 bytes. `du -h` and
`df -h` use 1024-based units, so their numbers look about 7% smaller.

## Permissions

DevTidy does not need Full Disk Access. Locations it cannot read are shown as
`no access`, and partially readable ones are marked so you know the size is a
lower bound.
