#!/usr/bin/env sh
# Offline installer fixture; pass the built Linux aglink executable as $1.
set -eu
AGLINK_EXE="$(realpath "$1")"
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
FIXTURE="$(mktemp -d -t aglink-install-test.XXXXXXXX)"
trap 'rm -rf "$FIXTURE"' EXIT
mkdir -p "$FIXTURE/assets" "$FIXTURE/mock-bin" "$FIXTURE/package"
mkdir -p "$FIXTURE/home"
export HOME="$FIXTURE/home"
cd "$FIXTURE"
export AGLINK_TEST_ASSETS="$FIXTURE/assets"
cat > "$FIXTURE/mock-bin/curl" <<'SH'
#!/usr/bin/env sh
set -eu
while [ "$#" -gt 0 ]; do
  case "$1" in
    -fsSL) shift ;;
    -o) output="$2"; shift 2 ;;
    *) url="$1"; shift ;;
  esac
done
cp "$AGLINK_TEST_ASSETS/${url##*/}" "$output"
SH
chmod +x "$FIXTURE/mock-bin/curl"
export PATH="$FIXTURE/mock-bin:$PATH"
export VERSION=0.0.0-smoke
export INSTALL_DIR="$FIXTURE/installed"
export XDG_DATA_HOME="$FIXTURE/xdg"
unset AGLINK_DATA_DIR
ASSET=aglink-v0.0.0-smoke-x86_64-unknown-linux-gnu.tar.gz
mkdir -p "$FIXTURE/package/skills/aglink-cli/references"
printf '# Fixture bundled skill\n\nInstaller acceptance fixture.\n' > "$FIXTURE/package/skills/aglink-cli/SKILL.md"
printf 'Fixture command reference\n' > "$FIXTURE/package/skills/aglink-cli/references/commands.md"
make_archive() {
  tar -czf "$FIXTURE/assets/$ASSET" -C "$FIXTURE/package" aglink skills
  (cd "$FIXTURE/assets" && sha256sum "$ASSET" > "$ASSET.sha256")
}
cp "$AGLINK_EXE" "$FIXTURE/package/aglink"
make_archive
sh "$SCRIPT_DIR/install.sh"
"$INSTALL_DIR/aglink" harness list | grep -q claude
test -s "$XDG_DATA_HOME/agent-linker/agent-linker.db"
CHECK_OUTPUT="$("$INSTALL_DIR/aglink" db check)"
printf '%s\n' "$CHECK_OUTPUT" | grep -Eq '^Schema: 4 / latest 4$'
printf '%s\n' "$CHECK_OUTPUT" | grep -Eq '^Status: healthy_writable$'
# Source/manual extraction follows the documented explicit initialization step.
AGLINK_DATA_DIR="$FIXTURE/manual" "$AGLINK_EXE" db migrate
AGLINK_DATA_DIR="$FIXTURE/manual" "$AGLINK_EXE" group list
# Each initialization command must independently propagate its native failure.
# The wrapper forwards preceding commands to the real executable, allowing the
# check branch to prove that migrate succeeded before check itself failed.
export AGLINK_TEST_REAL_EXE="$AGLINK_EXE"
cat > "$FIXTURE/package/aglink" <<'SH'
#!/usr/bin/env sh
set -eu
if [ "${1-}" = db ] && [ "${2-}" = "${AGLINK_TEST_FAIL_DB_COMMAND-}" ]; then
  printf '%s' "$AGLINK_TEST_FAIL_DB_COMMAND" > "$AGLINK_TEST_FAIL_MARKER"
  echo "fixture injected db $AGLINK_TEST_FAIL_DB_COMMAND failure" >&2
  exit 17
fi
exec "$AGLINK_TEST_REAL_EXE" "$@"
SH
chmod +x "$FIXTURE/package/aglink"
make_archive
for phase in migrate check; do
  export AGLINK_TEST_FAIL_DB_COMMAND="$phase"
  export AGLINK_TEST_FAIL_MARKER="$FIXTURE/injected-$phase.txt"
  export INSTALL_DIR="$FIXTURE/failed-$phase-installed"
  export XDG_DATA_HOME="$FIXTURE/failed-$phase-xdg"
  if sh "$SCRIPT_DIR/install.sh"; then
    echo "installer ignored db $phase failure" >&2
    exit 1
  else
    status=$?
    test "$status" -eq 17
  fi
  test "$(cat "$AGLINK_TEST_FAIL_MARKER")" = "$phase"
done
echo 'PASS: Unix installer schema v4 and ordinary first query, source/manual initialization, separate migrate/check failures'
