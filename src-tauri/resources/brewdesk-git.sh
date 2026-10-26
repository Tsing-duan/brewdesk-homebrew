#!/bin/sh
set -eu

# Homebrew filters GIT_CONFIG_* before dispatch, but preserves HOMEBREW_*
# and selects this helper through its supported HOMEBREW_GIT_PATH setting.
# Recreate the URL rules here without writing any Git configuration file.
case "${HOMEBREW_BREWDESK_GIT_ROUTE:-}" in
  official)
    route_from=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/
    route_to=https://github.com/Homebrew/
    ;;
  mirror)
    route_from=https://github.com/Homebrew/
    route_to=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/
    ;;
  *) exec /usr/bin/git "$@" ;;
esac

export GIT_CONFIG_COUNT=2
export GIT_CONFIG_KEY_0="url.${route_to}.insteadOf"
export GIT_CONFIG_VALUE_0="${route_from}"
export GIT_CONFIG_KEY_1="url.${route_to}brew.git.insteadOf"
export GIT_CONFIG_VALUE_1="${route_from}brew.git"
exec /usr/bin/git "$@"
