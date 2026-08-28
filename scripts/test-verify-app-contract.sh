#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

set +e
/bin/sh "$project_dir/scripts/verify-app.sh" "/definitely/missing/BrewDesk.app" "com.example.invalid" >/dev/null 2>&1
status=$?
set -e

if [ "$status" -ne 2 ]; then
  printf 'Expected invalid app input to exit 2, got %s.\n' "$status" >&2
  exit 1
fi

printf 'App verifier input contract passed.\n'
