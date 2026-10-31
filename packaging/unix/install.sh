#!/usr/bin/env sh
set -eu

REPO="${REPO:-winddxr/agent-linker}"
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

if [ "$VERSION" = "latest" ]; then
  need_cmd sed
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

echo "aglink installed to $INSTALL_DIR/aglink"
echo "Make sure $INSTALL_DIR is on PATH."
