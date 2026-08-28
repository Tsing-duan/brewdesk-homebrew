#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
public_build_root=${PUBLIC_BUILD_ROOT:-}

if [ -z "$public_build_root" ]; then
  printf 'PUBLIC_BUILD_ROOT must name a private build directory outside the repository.\n' >&2
  exit 2
fi
case "$public_build_root" in
  /*) ;;
  *)
    printf 'PUBLIC_BUILD_ROOT must be an absolute path.\n' >&2
    exit 2
    ;;
esac
case "$public_build_root" in
  *'/../'*|*/..|*'/./'*|*/.)
    printf 'PUBLIC_BUILD_ROOT must be normalized.\n' >&2
    exit 2
    ;;
esac
if [ ! -d "$public_build_root" ] || [ -L "$public_build_root" ]; then
  printf 'PUBLIC_BUILD_ROOT must be an existing non-symbolic-link directory.\n' >&2
  exit 2
fi
project_real=$(/bin/realpath "$project_dir")
build_real=$(/bin/realpath "$public_build_root")
case "$build_real/" in
  "$project_real/"*)
    printf 'PUBLIC_BUILD_ROOT must resolve outside the repository.\n' >&2
    exit 2
    ;;
esac
if [ "$(/usr/bin/stat -f '%u' "$build_real")" != "$(/usr/bin/id -u)" ]; then
  printf 'PUBLIC_BUILD_ROOT must be owned by the current user.\n' >&2
  exit 2
fi
if [ "$(/usr/bin/stat -f '%Lp' "$build_real")" != "700" ]; then
  printf 'PUBLIC_BUILD_ROOT must have mode 0700.\n' >&2
  exit 2
fi

node_version=$(node --version)
case "$node_version" in
  v24.*) ;;
  *)
    printf 'Node.js 24 is required; found %s.\n' "$node_version" >&2
    exit 2
    ;;
esac

for tool in "$project_dir/node_modules/.bin/tsc" "$project_dir/node_modules/.bin/vite"; do
  if [ ! -x "$tool" ]; then
    printf 'Missing project-local build tool: %s\n' "$tool" >&2
    exit 2
  fi
done

web_output="$build_real/web"
/bin/mkdir -p "$web_output"

(
  cd "$project_dir"
  "$project_dir/node_modules/.bin/tsc"
  "$project_dir/node_modules/.bin/vite" build --outDir "$web_output" --emptyOutDir
)
