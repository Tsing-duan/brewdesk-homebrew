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

if [ ! -x "$project_real/node_modules/.bin/tauri" ]; then
  printf 'The exact project-local Tauri CLI is required.\n' >&2
  exit 2
fi
tauri_version=$("$project_real/node_modules/.bin/tauri" --version)
if [ "$tauri_version" != "tauri-cli 2.11.4" ]; then
  printf 'Expected project-local tauri-cli 2.11.4; found %s.\n' "$tauri_version" >&2
  exit 2
fi

bundle_identifier=$(node -e 'const fs=require("node:fs"); const path=require("node:path"); const root=process.argv[1]; const value=JSON.parse(fs.readFileSync(path.join(root,"src-tauri","tauri.conf.json"),"utf8")); process.stdout.write(value.identifier)' "$project_real")
case "$bundle_identifier" in
  ''|PENDING_*|com.example.*|io.github.placeholder.*)
    printf 'The final user-approved Bundle Identifier is required before application build.\n' >&2
    exit 3
    ;;
esac

PUBLIC_BUILD_ROOT="$build_real" /bin/sh "$project_real/scripts/build-web.sh"
PUBLIC_BUILD_ROOT="$build_real" /bin/sh "$project_real/scripts/build-native-helpers.sh"
override_path=$(node "$project_real/scripts/write-tauri-stage-a-config.mjs" "$build_real" "$project_real" "$bundle_identifier")

CARGO_TARGET_DIR="$build_real/cargo-target" "$project_real/node_modules/.bin/tauri" build \
  --ci \
  --bundles app \
  --target aarch64-apple-darwin \
  --config "$override_path" \
  --no-sign

app_path="$build_real/cargo-target/aarch64-apple-darwin/release/bundle/macos/BrewDesk.app"
if [ ! -d "$app_path" ]; then
  app_path="$build_real/cargo-target/release/bundle/macos/BrewDesk.app"
fi
if [ ! -d "$app_path" ]; then
  printf 'Tauri did not produce the expected BrewDesk.app under PUBLIC_BUILD_ROOT.\n' >&2
  exit 1
fi

/usr/bin/codesign --force --deep --options runtime --sign - "$app_path"
/bin/sh "$project_real/scripts/verify-app.sh" "$app_path" "$bundle_identifier"
printf 'SOURCE_BUILD_VERIFIED_APP=%s\n' "$app_path"
