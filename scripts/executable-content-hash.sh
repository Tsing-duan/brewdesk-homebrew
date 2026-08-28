#!/bin/sh
set -eu

binary_path=${1:-}
if [ -z "$binary_path" ] || [ ! -f "$binary_path" ]; then
  printf 'Usage: %s /path/to/signed-mach-o\n' "$0" >&2
  exit 2
fi

temporary_binary=$(/usr/bin/mktemp /tmp/brewdesk-executable.XXXXXX)
trap '/bin/rm -f "$temporary_binary"' EXIT HUP INT TERM
/usr/bin/ditto --norsrc --noextattr --noqtn --noacl "$binary_path" "$temporary_binary"
/usr/bin/codesign --remove-signature "$temporary_binary" >/dev/null 2>&1 || true
/usr/bin/shasum -a 256 "$temporary_binary" | /usr/bin/awk '{print $1}'
