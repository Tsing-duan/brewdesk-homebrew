#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
test_root=$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/brewdesk-ci-contract.XXXXXX")
trap '/bin/rm -rf "$test_root"' EXIT HUP INT TERM

/bin/mkdir -p "$test_root/good/.github/workflows" "$test_root/bad/.github/workflows"
/bin/cp "$project_dir/.github/workflows/ci.yml" "$test_root/good/.github/workflows/ci.yml"
/bin/cp "$project_dir/.github/workflows/ci.yml" "$test_root/bad/.github/workflows/ci.yml"

node "$project_dir/scripts/verify-ci-contract.mjs" "$test_root/good"

/usr/bin/sed -i '' \
  's#actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1#actions/checkout@v7#' \
  "$test_root/bad/.github/workflows/ci.yml"
set +e
node "$project_dir/scripts/verify-ci-contract.mjs" "$test_root/bad" >/dev/null 2>&1
bad_status=$?
set -e
if [ "$bad_status" -eq 0 ]; then
  printf 'Expected a mutable Action tag to fail the CI contract.\n' >&2
  exit 1
fi

printf 'CI contract checks passed.\n'
