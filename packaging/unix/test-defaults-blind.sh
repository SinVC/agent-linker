#!/usr/bin/env sh
# Independent offline acceptance: only invokes installer/CLI and inspects artifacts.
set -eu
PACKAGE="$(realpath "$1")"
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
FIXTURE="$(mktemp -d -t aglink-defaults-blind.XXXXXXXX)"
trap 'rm -rf "$FIXTURE"' EXIT HUP INT TERM
mkdir -p "$FIXTURE/assets" "$FIXTURE/mock-bin" "$FIXTURE/unpacked" "$FIXTURE/home" "$FIXTURE/project"
export HOME="$FIXTURE/home"
export XDG_DATA_HOME="$FIXTURE/xdg"
export INSTALL_DIR="$FIXTURE/installed"
export VERSION=0.0.0-blind
export AGLINK_TEST_ASSETS="$FIXTURE/assets"
unset AGLINK_DATA_DIR
ASSET=aglink-v0.0.0-blind-x86_64-unknown-linux-gnu.tar.gz
cp "$PACKAGE" "$FIXTURE/assets/$ASSET"
(cd "$FIXTURE/assets" && sha256sum "$ASSET" > "$ASSET.sha256")
tar -xzf "$PACKAGE" -C "$FIXTURE/unpacked"
test -x "$FIXTURE/unpacked/aglink"
test -s "$FIXTURE/unpacked/skills/aglink-cli/SKILL.md"
test -d "$FIXTURE/unpacked/skills/aglink-cli/references"
test -n "$(find "$FIXTURE/unpacked/skills/aglink-cli/references" -type f -size +0c -print)"
find "$FIXTURE/unpacked/skills/aglink-cli/references" -type f -size 0c -print | while IFS= read -r empty; do
  echo "Empty reference in package: $empty" >&2
  exit 1
done
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
cd "$FIXTURE/project"
sh "$SCRIPT_DIR/install.sh"
CLI="$INSTALL_DIR/aglink"
DATA="$XDG_DATA_HOME/agent-linker"
"$CLI" db check | grep -q '^Schema: 4 / latest 4$'
test -s "$DATA/skills/aglink-cli/SKILL.md"
test -n "$(find "$DATA/skills/aglink-cli/references" -type f -size +0c -print)"
test -f "$DATA/.aglink-cli-configured"
SHOW="$("$CLI" skill show aglink-cli)"
printf '%s\n' "$SHOW" | grep -Fq "$DATA/skills/aglink-cli"
ID="$(printf '%s\n' "$SHOW" | grep -Eo 'itm_[0-9a-f]+' | head -n 1)"
test -n "$ID"
"$CLI" init defaults list | grep -q aglink-cli
"$CLI" init
test -L CLAUDE.md
test "$(readlink -f CLAUDE.md)" = "$FIXTURE/project/AGENTS.md"
test -L .claude/skills
test "$(readlink -f .claude/skills)" = "$FIXTURE/project/.agents/skills"
test -L .agents/skills/aglink-cli
test "$(readlink -f .agents/skills/aglink-cli)" = "$DATA/skills/aglink-cli"
cmp .claude/skills/aglink-cli/SKILL.md "$DATA/skills/aglink-cli/SKILL.md"
"$CLI" status --json > "$FIXTURE/status.json"
python3 - "$FIXTURE/status.json" <<'PY'
import json, sys
status = json.load(open(sys.argv[1], encoding="utf-8"))
assert status["project_root"] and status["manifest_path"]
assert len(status["links"]) == 3
for link in status["links"]:
    assert {"id", "item_id", "item_name", "link_path", "source_path", "link_kind", "provider_backend", "status"} <= link.keys()
    assert "correct" in json.dumps(link["status"]).lower(), link
PY
cp .agents/links.toml "$FIXTURE/manifest-before"
sleep 1.1
"$CLI" init
cmp .agents/links.toml "$FIXTURE/manifest-before"
"$CLI" init defaults remove aglink-cli
sh "$SCRIPT_DIR/install.sh"
NEW_ID="$("$CLI" skill show aglink-cli | grep -Eo 'itm_[0-9a-f]+' | head -n 1)"
test "$ID" = "$NEW_ID"
if "$CLI" init defaults list | grep -q aglink-cli; then
  echo 'Upgrade restored a user-removed default' >&2; exit 1
fi
"$CLI" init
test -L .agents/skills/aglink-cli

# A new default is applied on a later init; previous links remain recorded.
printf 'resource acceptance payload\n' > "$FIXTURE/source-resource.txt"
"$CLI" resource add "$FIXTURE/source-resource.txt" --name unix-resource --target-dir docs --alias shared.txt
"$CLI" init defaults add unix-resource
test ! -e docs/shared.txt
"$CLI" init
test -L docs/shared.txt
test "$(readlink -f docs/shared.txt)" = "$FIXTURE/source-resource.txt"
test -L .agents/skills/aglink-cli

# Same-name different-source user registrations must remain intact on failure.
"$CLI" skill remove aglink-cli
mkdir "$FIXTURE/user-aglink-cli"
printf '# User-owned same-name skill\n' > "$FIXTURE/user-aglink-cli/SKILL.md"
"$CLI" skill add "$FIXTURE/user-aglink-cli" --name aglink-cli
CONFLICT_SHOW="$("$CLI" skill show aglink-cli)"
CONFLICT_ID="$(printf '%s\n' "$CONFLICT_SHOW" | grep -Eo 'itm_[0-9a-f]+' | head -n 1)"
if sh "$SCRIPT_DIR/install.sh" > "$FIXTURE/conflict.log" 2>&1; then
  echo 'Installer silently accepted a same-name different-source skill' >&2; exit 1
fi
cat "$FIXTURE/conflict.log"
grep -Fq "$FIXTURE/user-aglink-cli" "$FIXTURE/conflict.log"
AFTER="$("$CLI" skill show aglink-cli)"
printf '%s\n' "$AFTER" | grep -Fq "$CONFLICT_ID"
printf '%s\n' "$AFTER" | grep -Fq "$FIXTURE/user-aglink-cli"
echo 'PASS: Linux artifact contents, first install, real init links/status, idempotence, upgrade choice/ID, late defaults and conflict preservation'
