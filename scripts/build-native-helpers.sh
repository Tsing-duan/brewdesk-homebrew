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

target_triple=$(rustc -vV | awk '/^host:/ { print $2 }')
binary_dir="$build_real/native"
module_cache="$build_real/swift-module-cache"
mkdir -p "$binary_dir" "$module_cache"

xcrun swiftc \
  -parse-as-library \
  -O \
  -whole-module-optimization \
  -module-cache-path "$module_cache" \
  -target "${target_triple%apple-darwin}apple-macosx26.0" \
  "$project_dir/src-tauri/swift/BrewDeskTranslate.swift" \
  -o "$binary_dir/brewdesk-translate-$target_triple"

xcrun swiftc \
  -parse-as-library \
  -O \
  -whole-module-optimization \
  -module-cache-path "$module_cache" \
  -target "${target_triple%apple-darwin}apple-macosx26.0" \
  "$project_dir/src-tauri/swift/BrewDeskTranslationSetup.swift" \
  -o "$binary_dir/brewdesk-translation-setup-$target_triple"

/usr/bin/strip -S -x "$binary_dir/brewdesk-translate-$target_triple"
/usr/bin/strip -S -x "$binary_dir/brewdesk-translation-setup-$target_triple"
