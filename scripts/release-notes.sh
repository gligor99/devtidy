#!/usr/bin/env bash
# Prints the CHANGELOG.md section for a version, followed by install notes.
# Fails if the section is missing, so a release cannot ship without notes.
#
#   scripts/release-notes.sh 0.2.0
set -euo pipefail

version="${1:?usage: release-notes.sh <version>}"
changelog="$(dirname "$0")/../CHANGELOG.md"

section="$(awk -v v="$version" '
  $0 ~ "^## \\[" v "\\]" { found = 1; next }
  found && /^## \[/ { exit }
  found && /^\[[^]]+\]: / { exit }
  found { print }
' "$changelog")"

if [ -z "$(tr -d '[:space:]' <<< "$section")" ]; then
  echo "CHANGELOG.md has no section for $version" >&2
  exit 1
fi

printf '%s\n' "$section"
cat <<'EOF'

## Install

**Desktop app (macOS 12+, Apple Silicon and Intel):** download
`DevTidy_<version>_universal.dmg`, open it and drag DevTidy to Applications.

DevTidy is not notarized by Apple yet, so the first launch needs one extra
step: right-click DevTidy in Applications and choose **Open**, then **Open**
again. (Or run `xattr -dr com.apple.quarantine /Applications/DevTidy.app`.)

**CLI:** download the archive for your platform, extract it and move
`devtidy` somewhere on your `PATH`. Verify downloads with `SHA256SUMS`.
EOF
