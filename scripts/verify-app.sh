#!/bin/sh
set -eu

app_path=${1:-}
expected_identifier=${2:-}

if [ -z "$app_path" ] || [ ! -d "$app_path" ] || [ -z "$expected_identifier" ]; then
  printf 'Usage: %s /path/to/BrewDesk.app approved.bundle.identifier\n' "$0" >&2
  exit 2
fi
case "$expected_identifier" in
  PENDING_*|com.example.*|io.github.placeholder.*)
    printf 'The Bundle Identifier must be the exact user-approved value.\n' >&2
    exit 2
    ;;
esac

plist="$app_path/Contents/Info.plist"
main_binary="$app_path/Contents/MacOS/brewdesk"
translate_binary="$app_path/Contents/MacOS/brewdesk-translate"
setup_binary="$app_path/Contents/MacOS/brewdesk-translation-setup"

for required_path in "$plist" "$main_binary" "$translate_binary" "$setup_binary"; do
  if [ ! -e "$required_path" ]; then
    printf 'Missing required app item: %s\n' "$required_path" >&2
    exit 1
  fi
done

actual_identifier=$(/usr/bin/plutil -extract CFBundleIdentifier raw "$plist")
short_version=$(/usr/bin/plutil -extract CFBundleShortVersionString raw "$plist")
bundle_version=$(/usr/bin/plutil -extract CFBundleVersion raw "$plist")
minimum_system=$(/usr/bin/plutil -extract LSMinimumSystemVersion raw "$plist")

if [ "$actual_identifier" != "$expected_identifier" ]; then
  printf 'Unexpected CFBundleIdentifier: %s\n' "$actual_identifier" >&2
  exit 1
fi
if [ "$short_version" != "0.1.0" ]; then
  printf 'Unexpected CFBundleShortVersionString: %s\n' "$short_version" >&2
  exit 1
fi
if [ "$bundle_version" != "1" ]; then
  printf 'Unexpected CFBundleVersion: %s\n' "$bundle_version" >&2
  exit 1
fi
if [ "$minimum_system" != "26.0" ]; then
  printf 'Unexpected LSMinimumSystemVersion: %s\n' "$minimum_system" >&2
  exit 1
fi

for executable in "$main_binary" "$translate_binary" "$setup_binary"; do
  if [ "$(/usr/bin/lipo -archs "$executable")" != "arm64" ]; then
    printf 'Expected arm64-only executable: %s\n' "$executable" >&2
    exit 1
  fi
done

/usr/bin/codesign --verify --deep --strict --verbose=2 "$app_path"

if /usr/bin/find "$app_path" -type f \( \
  -name '*.map' -o -name '*.ts' -o -name '*.tsx' -o -name '*.rs' \
  -o -name '.env' -o -name '.env.*' -o -name '*.pem' -o -name '*.p12' \
\) | /usr/bin/grep -q .; then
  printf 'Source, environment, or credential material found in app bundle.\n' >&2
  exit 1
fi

printf 'Verified app structure: %s\n' "$app_path"
printf 'CFBundleIdentifier: %s\n' "$actual_identifier"
printf 'CFBundleShortVersionString: %s\n' "$short_version"
printf 'CFBundleVersion: %s\n' "$bundle_version"
printf 'LSMinimumSystemVersion: %s\n' "$minimum_system"
printf 'Architecture: arm64\n'
