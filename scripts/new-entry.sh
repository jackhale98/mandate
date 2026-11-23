#!/usr/bin/env bash
# Create a dated log entry from a template: scripts/new-entry.sh progress|balance "<title>"
set -euo pipefail
kind="${1:-}"; title="${2:-}"
if [[ ( "$kind" != progress && "$kind" != balance ) || -z "$title" ]]; then
  echo 'usage: scripts/new-entry.sh progress|balance "<title>"' >&2
  exit 2
fi
root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$root/docs/development/$kind"
mkdir -p "$dir"
date="$(date -u +%Y-%m-%d)"
slug="$(printf '%s' "$title" | sed 's/([^)]*)//g' | tr 'A-Z' 'a-z' | sed 's/[^a-z0-9]\+/-/g; s/^-//; s/-$//' | cut -d- -f1-6)"
file="$dir/$date-$slug.md"
if [[ -e "$file" ]]; then echo "exists: $file" >&2; exit 1; fi
if [[ "$kind" == progress ]]; then
  printf '# %s\n\n**Date:** %s\n**Status:**\n\n(What was done, what is left.)\n' "$title" "$date" > "$file"
else
  printf '# %s\n\n**Date:** %s\n**Area:**\n\n## Parameter and change\n\n## Evidence\n\n## Effect\n' "$title" "$date" > "$file"
fi
echo "$file"
