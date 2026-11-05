#!/usr/bin/env sh
# Black-box URL fixture: intercept curl and stop at an intentionally invalid tar.
set -eu
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
FIXTURE="$(mktemp -d -t aglink-url-blind.XXXXXXXX)"
trap 'rm -rf "$FIXTURE"' EXIT HUP INT TERM
mkdir -p "$FIXTURE/mock-bin" "$FIXTURE/home"
export HOME="$FIXTURE/home"
export XDG_DATA_HOME="$FIXTURE/xdg"
unset AGLINK_DATA_DIR VERSION REPO
export BLIND_PAYLOAD="$FIXTURE/invalid.tar.gz"
printf 'URL acceptance fixture; deliberately not an archive' > "$BLIND_PAYLOAD"
export BLIND_HASH="$(sha256sum "$BLIND_PAYLOAD" | cut -d ' ' -f 1)"
cat > "$FIXTURE/mock-bin/curl" <<'SH'
#!/usr/bin/env sh
set -eu
output=''
url=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o|--output) output="$2"; shift 2 ;;
    -H|--header|-A|--user-agent) shift 2 ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done
printf '%s\n' "$url" >> "$BLIND_REQUEST_LOG"
case "$url" in
  */releases/latest) printf '{"tag_name":"v9.8.7"}\n' ;;
  *.tar.gz) cp "$BLIND_PAYLOAD" "$output" ;;
  *.tar.gz.sha256) printf '%s  %s\n' "$BLIND_HASH" "${url##*/}" | sed 's/\.sha256$//' > "$output" ;;
  *) echo "Unexpected fixture URL: $url" >&2; exit 90 ;;
esac
SH
chmod +x "$FIXTURE/mock-bin/curl"
export PATH="$FIXTURE/mock-bin:$PATH"
cd "$FIXTURE"
for fixture_case in default explicit; do
  export INSTALL_DIR="$FIXTURE/$fixture_case-install"
  export BLIND_REQUEST_LOG="$FIXTURE/$fixture_case-requests"
  if [ "$fixture_case" = default ]; then
    unset REPO
    expected_repo='SinVC/agent-linker'
  else
    export REPO='ExampleOwner/explicit-repo'
    expected_repo="$REPO"
  fi
  if sh "$SCRIPT_DIR/install.sh" > "$FIXTURE/$fixture_case-output" 2>&1; then
    echo 'Invalid archive unexpectedly completed installation' >&2; exit 1
  fi
  asset="https://github.com/$expected_repo/releases/download/v9.8.7/aglink-v9.8.7-x86_64-unknown-linux-gnu.tar.gz"
  printf '%s\n' "https://api.github.com/repos/$expected_repo/releases/latest" "$asset" "$asset.sha256" > "$FIXTURE/expected"
  if ! cmp "$FIXTURE/expected" "$BLIND_REQUEST_LOG"; then
    cat "$BLIND_REQUEST_LOG" "$FIXTURE/$fixture_case-output" >&2; exit 1
  fi
  test ! -e "$INSTALL_DIR/aglink"
  printf '%s URL requests:\n' "$fixture_case"
  cat "$BLIND_REQUEST_LOG"
done
echo 'PASS: Unix default and explicit-REPO latest API/archive/checksum URL contracts; no network or installation'
