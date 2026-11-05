#!/usr/bin/env sh
set -eu

REPO="${REPO:-SinVC/agent-linker}"
VERSION="${VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "required command not found: $1" >&2
    exit 1
  }
}

need_cmd curl
need_cmd tar
need_cmd uname
need_cmd sha256sum
need_cmd sed
need_cmd awk

if [ "$VERSION" = "latest" ]; then
  TAG="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
else
  case "$VERSION" in
    v*) TAG="$VERSION" ;;
    *) TAG="v$VERSION" ;;
  esac
fi

PLAIN_VERSION="${TAG#v}"
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS:$ARCH" in
  Linux:x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
  *) echo "unsupported platform: $OS $ARCH" >&2; exit 1 ;;
esac

ASSET="aglink-v$PLAIN_VERSION-$TARGET.tar.gz"
BASE_URL="https://github.com/$REPO/releases/download/$TAG"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

curl -fsSL "$BASE_URL/$ASSET" -o "$TMP_DIR/$ASSET"
curl -fsSL "$BASE_URL/$ASSET.sha256" -o "$TMP_DIR/$ASSET.sha256"

cd "$TMP_DIR"
sha256sum -c "$ASSET.sha256"
mkdir -p "$INSTALL_DIR"
tar -xzf "$ASSET"
install -m 0755 aglink "$INSTALL_DIR/aglink"
"$INSTALL_DIR/aglink" db migrate
"$INSTALL_DIR/aglink" db check

# Resolve the CLI's actual database directory, including AGLINK_DATA_DIR overrides.
DB_PATH="$("$INSTALL_DIR/aglink" db path | sed -n 's/^Path: //p')"
DATA_DIR="$(dirname -- "$DB_PATH")"
DATA_DIR="$(CDPATH= cd -- "$DATA_DIR" && pwd -P)"
SKILL_PATH="$DATA_DIR/skills/aglink-cli"
MARKER="$DATA_DIR/.aglink-cli-configured"
ENTRIES="$("$INSTALL_DIR/aglink" skill list)"
REGISTERED_SOURCE="$(printf '%s\n' "$ENTRIES" | awk -F '\t' 'tolower($1)=="aglink-cli" {print $4}')"
if [ -n "$REGISTERED_SOURCE" ] && [ "$REGISTERED_SOURCE" != "$SKILL_PATH" ]; then
  echo "skill aglink-cli is already registered from another source: $REGISTERED_SOURCE" >&2
  exit 1
fi
if [ -L "$DATA_DIR/skills" ] || { [ -d "$DATA_DIR/skills" ] && [ -n "$(find "$DATA_DIR/skills" -type l -print -quit)" ]; }; then
  echo 'refusing skill installation through symlinks' >&2
  exit 1
fi
mkdir -p "$DATA_DIR/skills"
cp -R skills/. "$DATA_DIR/skills/"
if [ -z "$REGISTERED_SOURCE" ]; then
  "$INSTALL_DIR/aglink" skill add "$SKILL_PATH" --name aglink-cli
else
  "$INSTALL_DIR/aglink" skill refresh aglink-cli
fi
if [ ! -f "$MARKER" ]; then
  "$INSTALL_DIR/aglink" init defaults add aglink-cli
  printf '%s\n' 'Configured by the aglink installer.' > "$MARKER"
fi

echo "aglink installed to $INSTALL_DIR/aglink"
echo "Make sure $INSTALL_DIR is on PATH."
