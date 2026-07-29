#!/usr/bin/env bash
#
# run.sh — fixture tests for scripts/verify-release-assets.sh (ADR 0025 verification
# criteria), driven entirely by a stub `gh` placed first on PATH. No network access and no
# real GitHub release are involved.
#
# The stub logs every invocation to $GH_LOG, serves `release view` from a per-case fixture
# list of asset names (or fails when the case declares the release absent), serves
# `release download` by writing a payload file plus a matching or deliberately mismatching
# .sha256, and serves `release edit` by logging the exact flag it was invoked with
# (`--draft` or `--draft=false`), which is how promote/demote is distinguished below.
#
# Exit: 0 = all cases pass, 1 = at least one failed.

set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/../../verify-release-assets.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

TAG="v9.9.9"
GH_LOG="$TMP/gh.log"
STUB_BIN="$TMP/bin"
mkdir -p "$STUB_BIN"

cat >"$STUB_BIN/gh" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail

log_call() { printf '%s\n' "$*" >>"$GH_LOG"; }

sha256_of_stdin() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  else
    shasum -a 256 | awk '{print $1}'
  fi
}

release_view() {
  local tag="$1"
  log_call release view "$tag" --json assets --jq .assets[].name
  [[ "${GH_RELEASE_ABSENT:-0}" == "1" ]] && exit 1
  cat "$GH_ASSETS_FILE"
}

release_download() {
  local tag="$1" pattern="" dir=""
  shift
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --pattern) pattern="$2"; shift 2 ;;
      --dir) dir="$2"; shift 2 ;;
      --clobber) shift ;;
      *) shift ;;
    esac
  done
  log_call release download "$tag" --pattern "$pattern" --dir "$dir" --clobber
  if [[ "$pattern" == *.sha256 ]]; then
    local base="${pattern%.sha256}" hash mismatch_list=",${GH_MISMATCH_ASSETS:-},"
    hash="$(printf 'payload:%s' "$base" | sha256_of_stdin)"
    [[ "$mismatch_list" == *",$base,"* ]] && hash="$(printf '%064d' 0)"
    printf '%s  %s\n' "$hash" "$base" >"$dir/$pattern"
  else
    printf 'payload:%s' "$pattern" >"$dir/$pattern"
  fi
}

release_edit() {
  local tag="$1"
  shift
  log_call release edit "$tag" "$@"
}

case "$1 $2" in
  "release view") shift 2; release_view "$@" ;;
  "release download") shift 2; release_download "$@" ;;
  "release edit") shift 2; release_edit "$@" ;;
  *) exit 1 ;;
esac
STUB
chmod +x "$STUB_BIN/gh"

ALL_ASSETS=(
  "living-docs-skill-${TAG}.zip"
  "living-docs-skill-${TAG}.zip.sha256"
  "living-docs-aarch64-apple-darwin"
  "living-docs-aarch64-apple-darwin.sha256"
  "living-docs-x86_64-apple-darwin"
  "living-docs-x86_64-apple-darwin.sha256"
  "living-docs-x86_64-unknown-linux-gnu"
  "living-docs-x86_64-unknown-linux-gnu.sha256"
  "living-docs-aarch64-unknown-linux-gnu"
  "living-docs-aarch64-unknown-linux-gnu.sha256"
)

write_assets_file() { # write_assets_file <file> <names...>
  local file="$1"
  shift
  printf '%s\n' "$@" >"$file"
}

ASSETS_ALL="$TMP/assets-all.txt"
write_assets_file "$ASSETS_ALL" "${ALL_ASSETS[@]}"

ASSETS_MISSING_ONE="$TMP/assets-missing-one.txt"
write_assets_file "$ASSETS_MISSING_ONE" \
  "living-docs-skill-${TAG}.zip" \
  "living-docs-skill-${TAG}.zip.sha256" \
  "living-docs-x86_64-apple-darwin" \
  "living-docs-x86_64-apple-darwin.sha256" \
  "living-docs-x86_64-unknown-linux-gnu" \
  "living-docs-x86_64-unknown-linux-gnu.sha256" \
  "living-docs-aarch64-unknown-linux-gnu" \
  "living-docs-aarch64-unknown-linux-gnu.sha256"

ASSETS_ZIP_ONLY="$TMP/assets-zip-only.txt"
write_assets_file "$ASSETS_ZIP_ONLY" "${ALL_ASSETS[@]:0:2}"

fail=0

reset_gh_state() { : >"$GH_LOG"; }

invoke() { # invoke <ENV=val>... -- <script args...>
  local envs=()
  while [[ "$1" != "--" ]]; do
    envs+=("$1")
    shift
  done
  shift
  reset_gh_state
  OUT="$(env "${envs[@]}" PATH="$STUB_BIN:$PATH" GH_LOG="$GH_LOG" bash "$SCRIPT" "$@" 2>&1)"
  RC=$?
  LOG=""
  [[ -f "$GH_LOG" ]] && LOG="$(cat "$GH_LOG")"
}

check() { # check <name> <ok:0|1>
  if [[ "$2" == 1 ]]; then
    printf '  ok    %s\n' "$1"
  else
    printf '  FAIL  %s\n' "$1"
    printf '        exit=%s\n%s\n' "$RC" "$OUT" | sed 's/^/        out | /'
    printf '%s\n' "$LOG" | sed 's/^/        log | /'
    fail=1
  fi
}

assert_exit() { # assert_exit <name> <expected>
  local ok=0
  [[ "$RC" == "$2" ]] && ok=1
  check "$1" "$ok"
}

assert_out_has() { # assert_out_has <name> <substring>
  local ok=0
  grep -qF -- "$2" <<<"$OUT" && ok=1
  check "$1" "$ok"
}

assert_out_line() { # assert_out_line <name> <exact-line>
  local ok=0
  grep -qxF -- "$2" <<<"$OUT" && ok=1
  check "$1" "$ok"
}

assert_out_line_count() { # assert_out_line_count <name> <expected-count>
  local ok=0 actual
  actual="$(wc -l <<<"$OUT" | tr -d ' ')"
  [[ "$actual" == "$2" ]] && ok=1
  check "$1" "$ok"
}

assert_log_lacks() { # assert_log_lacks <name> <substring>
  local ok=1
  grep -qF -- "$2" <<<"$LOG" && ok=0
  check "$1" "$ok"
}

assert_log_line() { # assert_log_line <name> <exact-line>
  local ok=0
  grep -qxF -- "$2" <<<"$LOG" && ok=1
  check "$1" "$ok"
}

assert_log_line_lacks() { # assert_log_line_lacks <name> <exact-line>
  local ok=1
  grep -qxF -- "$2" <<<"$LOG" && ok=0
  check "$1" "$ok"
}

echo "verify-release-assets fixtures"
echo

echo "case 1: all ten assets present, checksums valid"
invoke "GH_ASSETS_FILE=$ASSETS_ALL" -- "$TAG"
assert_exit  "1-exit-0"                 0
assert_out_has "1-names-verified-count" "10"
assert_log_line "1-promotes"            "release edit $TAG --draft=false"
assert_log_line_lacks "1-never-demotes" "release edit $TAG --draft"

echo "case 2: one binary asset missing"
invoke "GH_ASSETS_FILE=$ASSETS_MISSING_ONE" -- "$TAG"
assert_exit    "2-exit-1"          1
assert_out_has "2-names-missing"   "living-docs-aarch64-apple-darwin"
assert_log_line "2-demotes"             "release edit $TAG --draft"
assert_log_line_lacks "2-never-promotes" "release edit $TAG --draft=false"

echo "case 3: matrix skipped, only zip assets present"
invoke "GH_ASSETS_FILE=$ASSETS_ZIP_ONLY" -- "$TAG"
assert_exit "3-exit-1" 1
assert_out_line_count "3-names-missing-count" 8
for triple in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
  assert_out_line "3-names-missing-living-docs-$triple"          "living-docs-$triple"
  assert_out_line "3-names-missing-living-docs-$triple-checksum" "living-docs-$triple.sha256"
done
assert_log_line "3-demotes"              "release edit $TAG --draft"
assert_log_line_lacks "3-never-promotes" "release edit $TAG --draft=false"

echo "case 4: all present, one checksum mismatched"
invoke "GH_ASSETS_FILE=$ASSETS_ALL" "GH_MISMATCH_ASSETS=living-docs-x86_64-unknown-linux-gnu" -- "$TAG"
assert_exit    "4-exit-1"        1
assert_out_line "4-names-mismatch" "living-docs-x86_64-unknown-linux-gnu"
assert_log_line "4-demotes"              "release edit $TAG --draft"
assert_log_line_lacks "4-never-promotes" "release edit $TAG --draft=false"

echo "case 5: release not found"
invoke "GH_RELEASE_ABSENT=1" -- "$TAG"
assert_exit      "5-exit-1"       1
assert_out_has   "5-names-tag"    "$TAG"
assert_log_lacks "5-no-edit"      "release edit"

echo "case 6: release not found, --verify-only"
invoke "GH_RELEASE_ABSENT=1" -- --verify-only "$TAG"
assert_exit      "6-exit-1"       1
assert_out_has   "6-names-tag"    "$TAG"
assert_log_lacks "6-no-edit"      "release edit"

echo "case 7: --verify-only with a missing asset"
invoke "GH_ASSETS_FILE=$ASSETS_MISSING_ONE" -- --verify-only "$TAG"
assert_exit       "7-exit-1"    1
assert_log_lacks  "7-no-edit"   "release edit"

echo "case 8: --verify-only with all ten assets present"
invoke "GH_ASSETS_FILE=$ASSETS_ALL" -- --verify-only "$TAG"
assert_exit       "8-exit-0"    0
assert_log_lacks  "8-no-edit"   "release edit"

echo "case 9: no tag argument"
invoke --
assert_exit    "9-exit-2"   2
assert_out_has "9-usage"    "Usage:"

echo "case 10: release.yml declares draft: true exactly twice (ADR 0025 fitness function)"
WORKFLOW="$HERE/../../../.github/workflows/release.yml"
draft_true_count="$(grep -c 'draft: true' "$WORKFLOW")"
ok=0
[[ "$draft_true_count" == "2" ]] && ok=1
check "10-two-draft-true-declarations" "$ok"

echo
if ((fail == 0)); then
  echo "All verify-release-assets fixtures passed."
  exit 0
else
  echo "verify-release-assets fixture failures."
  exit 1
fi
