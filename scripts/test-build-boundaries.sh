#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
test_root=$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/brewdesk-build-boundary.XXXXXX")
trap '/bin/rm -rf "$test_root"' EXIT HUP INT TERM

sandbox="$test_root/project"
/bin/mkdir -p "$sandbox/scripts" "$sandbox/src-tauri/swift"
/bin/cp "$project_dir/scripts/build-native-helpers.sh" "$sandbox/scripts/build-native-helpers.sh"
/bin/cp "$project_dir/scripts/build-web.sh" "$sandbox/scripts/build-web.sh"
/bin/cp "$project_dir/scripts/build-app.sh" "$sandbox/scripts/build-app.sh"
/bin/cp "$project_dir/src-tauri/swift/BrewDeskTranslate.swift" "$sandbox/src-tauri/swift/BrewDeskTranslate.swift"
/bin/cp "$project_dir/src-tauri/swift/BrewDeskTranslationSetup.swift" "$sandbox/src-tauri/swift/BrewDeskTranslationSetup.swift"

set +e
env -u PUBLIC_BUILD_ROOT /bin/sh "$sandbox/scripts/build-native-helpers.sh" >/dev/null 2>&1
missing_status=$?
set -e
if [ "$missing_status" -ne 2 ]; then
  printf 'Expected missing PUBLIC_BUILD_ROOT to exit 2, got %s.\n' "$missing_status" >&2
  exit 1
fi

set +e
/bin/mkdir -p "$sandbox/build"
PUBLIC_BUILD_ROOT="$sandbox/build" /bin/sh "$sandbox/scripts/build-native-helpers.sh" >/dev/null 2>&1
inside_status=$?
set -e
if [ "$inside_status" -ne 2 ]; then
  printf 'Expected an in-project PUBLIC_BUILD_ROOT to exit 2, got %s.\n' "$inside_status" >&2
  exit 1
fi

/bin/mkdir -p "$sandbox/generated"
/bin/ln -s "$sandbox" "$test_root/project-link"
set +e
PUBLIC_BUILD_ROOT="$test_root/project-link/generated" /bin/sh "$sandbox/scripts/build-native-helpers.sh" >/dev/null 2>&1
linked_inside_status=$?
set -e
if [ "$linked_inside_status" -ne 2 ]; then
  printf 'Expected a symlink-resolved in-project build root to exit 2, got %s.\n' "$linked_inside_status" >&2
  exit 1
fi

set +e
env -u PUBLIC_BUILD_ROOT /bin/sh "$sandbox/scripts/build-web.sh" >/dev/null 2>&1
web_missing_status=$?
set -e
if [ "$web_missing_status" -ne 2 ]; then
  printf 'Expected web build without PUBLIC_BUILD_ROOT to exit 2, got %s.\n' "$web_missing_status" >&2
  exit 1
fi

set +e
PUBLIC_BUILD_ROOT="$sandbox/build" /bin/sh "$sandbox/scripts/build-web.sh" >/dev/null 2>&1
web_inside_status=$?
set -e
if [ "$web_inside_status" -ne 2 ]; then
  printf 'Expected web build with in-project PUBLIC_BUILD_ROOT to exit 2, got %s.\n' "$web_inside_status" >&2
  exit 1
fi

set +e
PUBLIC_BUILD_ROOT="$test_root/project-link/generated" /bin/sh "$sandbox/scripts/build-web.sh" >/dev/null 2>&1
web_linked_inside_status=$?
set -e
if [ "$web_linked_inside_status" -ne 2 ]; then
  printf 'Expected web build with symlink-resolved in-project root to exit 2, got %s.\n' "$web_linked_inside_status" >&2
  exit 1
fi

set +e
env -u PUBLIC_BUILD_ROOT /bin/sh "$sandbox/scripts/build-app.sh" >/dev/null 2>&1
app_missing_status=$?
set -e
if [ "$app_missing_status" -ne 2 ]; then
  printf 'Expected app build without PUBLIC_BUILD_ROOT to exit 2, got %s.\n' "$app_missing_status" >&2
  exit 1
fi

set +e
PUBLIC_BUILD_ROOT="$sandbox/build" /bin/sh "$sandbox/scripts/build-app.sh" >/dev/null 2>&1
app_inside_status=$?
set -e
if [ "$app_inside_status" -ne 2 ]; then
  printf 'Expected app build with in-project PUBLIC_BUILD_ROOT to exit 2, got %s.\n' "$app_inside_status" >&2
  exit 1
fi

printf 'Build boundary checks passed.\n'
