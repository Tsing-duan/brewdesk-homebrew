#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
public_build_root=${PUBLIC_BUILD_ROOT:-}

if [ -z "$public_build_root" ]; then
  printf 'PUBLIC_BUILD_ROOT is required for the Rust output-boundary test.\n' >&2
  exit 2
fi
case "$public_build_root" in
  /*) ;;
  *)
    printf 'PUBLIC_BUILD_ROOT must be absolute.\n' >&2
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

if [ -e "$project_real/src-tauri/gen" ]; then
  printf 'Tauri generated output already exists inside the repository: src-tauri/gen\n' >&2
  exit 1
fi

target_dir="$build_real/rust-output-boundary-target"
/bin/mkdir -p "$target_dir"
CARGO_TARGET_DIR="$target_dir" cargo check --locked --manifest-path "$project_real/src-tauri/Cargo.toml"

if [ -e "$project_real/src-tauri/gen" ]; then
  printf 'Rust verification wrote Tauri generated output inside the repository.\n' >&2
  exit 1
fi

printf 'Rust and Tauri generated-output boundary passed.\n'
