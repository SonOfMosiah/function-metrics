#!/usr/bin/env bash
set -euo pipefail

range=${1:?"usage: check-conventional-commits.sh <revision-range>"}
message_file=$(mktemp)
trap 'rm -f "$message_file"' EXIT

while read -r commit; do
  [[ -z $commit ]] && continue
  git show --quiet --format=%B "$commit" > "$message_file"
  if ! scripts/verify-commit-message.sh "$message_file"; then
    echo "Commit: $commit" >&2
    exit 1
  fi
done < <(git rev-list --no-merges "$range")
