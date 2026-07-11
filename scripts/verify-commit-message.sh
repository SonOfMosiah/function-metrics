#!/usr/bin/env bash
set -euo pipefail

message_file=${1:?"usage: verify-commit-message.sh <message-file>"}
subject=$(head -n 1 "$message_file")

# Conventional Commits: type, optional scope, optional breaking marker, then a description.
pattern='^[a-z][a-z0-9-]*(\([[:alnum:]./_-]+\))?!?: .+'
if [[ ! $subject =~ $pattern ]]; then
  cat >&2 <<EOF_MESSAGE
Invalid commit message:
  $subject

Use Conventional Commits, for example:
  feat: add counter instrumentation
  fix(labels): preserve borrowed values
  feat!: change the generated metric names
EOF_MESSAGE
  exit 1
fi
