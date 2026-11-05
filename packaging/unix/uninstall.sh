#!/usr/bin/env sh
set -eu

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
PURGE="${PURGE:-0}"

rm -f "$INSTALL_DIR/aglink"

if [ "$PURGE" = "1" ]; then
  DATA_DIR="${AGLINK_DATA_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/agent-linker}"
  case "$DATA_DIR" in
    ""|"/"|"$HOME"|"$HOME/"|"$HOME/.local"|"$HOME/.local/share")
      echo "refusing to purge dangerous data directory: $DATA_DIR" >&2
      exit 1
      ;;
  esac
  rm -f "$DATA_DIR/agent-linker.db" \
        "$DATA_DIR/agent-linker.db-shm" \
        "$DATA_DIR/agent-linker.db-wal" \
        "$DATA_DIR/.aglink-cli-configured" \
        "$DATA_DIR/config.toml"
  rmdir "$DATA_DIR" 2>/dev/null || true
fi

echo "Agent Linker uninstalled."
