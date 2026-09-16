#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

required=(
  docs/logo.svg
  docs/screenshots/home-quick-transcribe.png
  docs/screenshots/quick-transcribe.png
  docs/screenshots/workflow-folders.png
  docs/screenshots/mobile-quick-transcribe.png
)
for file in "${required[@]}"; do
  test -s "$file" || { echo "missing public asset: $file" >&2; exit 1; }
done

for readme in README.md README.fr.md; do
  grep -q 'docs/logo.svg' "$readme"
  grep -q 'docs/screenshots/' "$readme"
done

patterns=(
  '/home/[A-Za-z0-9._-]+'
  '192\.168\.[0-9]{1,3}\.[0-9]{1,3}'
  '10\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}'
  '172\.(1[6-9]|2[0-9]|3[01])\.[0-9]{1,3}\.[0-9]{1,3}'
  'CT[0-9]{2,5}'
)
for pattern in "${patterns[@]}"; do
  if git grep -n -I -E "$pattern" -- . ':!scripts/check-public-repo.sh'; then
    echo "private marker found: $pattern" >&2
    exit 1
  fi
done

echo 'SCRIBEWATCH_PUBLIC_REPO_CHECK=PASS'
