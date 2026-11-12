#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

required=(
  docs/logo.svg
)
for locale in en fr zh-CN; do
  for screen in home record documents settings mobile; do
    required+=("docs/screenshots/${locale}/${screen}.png")
  done
done
for file in "${required[@]}"; do
  test -s "$file" || { echo "missing public asset: $file" >&2; exit 1; }
done

for readme in README.md README.fr.md README.zh-CN.md; do
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
  matches="$(git grep -n -I -E "$pattern" -- . ':!scripts/check-public-repo.sh' || true)"
  # Reserved example address used in the HTTP client unit-test fixture.
  # Exempt only that exact value, not the whole file or private network range.
  matches="$(printf '%s\n' "$matches" | grep -vE '^src/api/speaches.rs:[0-9]+:.*http://10\.20\.30\.40:8005' || true)"
  matches="$(printf '%s\n' "$matches" | grep -vE '^(frontend/tests/(local-loopback-installer|speaches-discovery)\.test\.mjs|src/api/(local_llm|speaches)\.rs):[0-9]+:.*(172\.20\.20\.10|172\.31\.255\.254)' || true)"
  if [[ -n "$matches" ]]; then
    printf '%s\n' "$matches" >&2
    echo "private marker found: $pattern" >&2
    exit 1
  fi
done

echo 'SCRIBEWATCH_PUBLIC_REPO_CHECK=PASS'
