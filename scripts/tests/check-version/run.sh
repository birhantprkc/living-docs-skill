#!/usr/bin/env bash
#
# run.sh — fixture tests for scripts/check-version.sh.
#
# check-version.sh derives its `root` from its own location (dirname "$0"/..), so each case
# builds a synthetic repo under mktemp -d and runs a COPY of the real script from
# $TMP/<case>/scripts/check-version.sh against that synthetic tree. No network access.
#
# Exit: 0 = all cases pass, 1 = at least one failed.

set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT_SRC="$HERE/../../check-version.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail=0

write_frontmatter() { # write_frontmatter <file> <version-line-or-empty>
  local file="$1" line="$2"
  mkdir -p "$(dirname "$file")"
  {
    echo "---"
    echo "title: fixture"
    [[ -n "$line" ]] && echo "$line"
    echo "---"
    echo "# fixture"
  } >"$file"
}

write_plugin() { # write_plugin <dir> <version>
  cat >"$1/.claude-plugin/plugin.json" <<JSON
{
  "name": "fixture",
  "version": "$2"
}
JSON
}

set_all_versions() { # set_all_versions <dir> <version>
  local dir="$1" version="$2"
  printf '%s' "$version" >"$dir/VERSION"
  write_frontmatter "$dir/skills/foo/SKILL.md" "version: \"$version\""
  write_plugin "$dir" "$version"
  write_frontmatter "$dir/.github/instructions/foo.md" "version: \"$version\""
  write_frontmatter "$dir/.cursor/rules/foo.mdc" "version: \"$version\""
}

new_repo() { # new_repo <name> -> prints the path to a consistent baseline repo at 0.9.0
  local dir="$TMP/$1"
  mkdir -p "$dir/scripts" "$dir/skills/foo" "$dir/.claude-plugin" \
    "$dir/.github/instructions" "$dir/.cursor/rules"
  cp "$SCRIPT_SRC" "$dir/scripts/check-version.sh"
  chmod +x "$dir/scripts/check-version.sh"
  set_all_versions "$dir" "0.9.0"
  printf '%s' "$dir"
}

invoke() { # invoke <repo-dir> [args...]
  local dir="$1"
  shift
  OUT="$("$dir/scripts/check-version.sh" "$@" 2>&1)"
  RC=$?
}

check() { # check <name> <ok:0|1>
  if [[ "$2" == 1 ]]; then
    printf '  ok    %s\n' "$1"
  else
    printf '  FAIL  %s\n' "$1"
    printf '        exit=%s\n%s\n' "$RC" "$OUT" | sed 's/^/        out | /'
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

assert_out_count() { # assert_out_count <name> <substring> <expected-count>
  local ok=0 actual
  actual="$(grep -cF -- "$2" <<<"$OUT")"
  [[ "$actual" == "$3" ]] && ok=1
  check "$1" "$ok"
}

echo "check-version fixtures"
echo

echo "case 1: fully consistent tree"
repo="$(new_repo case1)"
invoke "$repo"
assert_exit    "1-exit-0"      0
assert_out_has "1-version-ok"  "Version OK"

echo "case 2: a skills/*/SKILL.md drifted"
repo="$(new_repo case2)"
write_frontmatter "$repo/skills/foo/SKILL.md" 'version: "0.0.1"'
invoke "$repo"
assert_exit    "2-exit-1"     1
assert_out_has "2-names-file" "skills/foo/SKILL.md"

echo "case 3: .claude-plugin/plugin.json drifted"
repo="$(new_repo case3)"
write_plugin "$repo" "0.0.1"
invoke "$repo"
assert_exit    "3-exit-1"     1
assert_out_has "3-names-file" ".claude-plugin/plugin.json"

echo "case 4: a .github/instructions/*.md drifted"
repo="$(new_repo case4)"
write_frontmatter "$repo/.github/instructions/foo.md" 'version: "0.0.1"'
invoke "$repo"
assert_exit    "4-exit-1"     1
assert_out_has "4-names-file" ".github/instructions/foo.md"

echo "case 5: a .cursor/rules/*.mdc drifted"
repo="$(new_repo case5)"
write_frontmatter "$repo/.cursor/rules/foo.mdc" 'version: "0.0.1"'
invoke "$repo"
assert_exit    "5-exit-1"     1
assert_out_has "5-names-file" ".cursor/rules/foo.mdc"

echo "case 6: regression — .cursor/rules/*.mdc with a space before the colon"
repo="$(new_repo case6)"
write_frontmatter "$repo/.cursor/rules/foo.mdc" 'version : "0.0.1"'
invoke "$repo"
assert_exit    "6-exit-1"        1
assert_out_has "6-malformed"     "MALFORMED"
assert_out_has "6-names-file"    ".cursor/rules/foo.mdc"

echo "case 7: same malformed form in .github/instructions and skills/*/SKILL.md, both reported"
repo="$(new_repo case7)"
write_frontmatter "$repo/.github/instructions/foo.md" 'version : "0.0.1"'
write_frontmatter "$repo/skills/foo/SKILL.md" 'version : "0.0.1"'
invoke "$repo"
assert_exit     "7-exit-1"           1
assert_out_has  "7-names-instruction" ".github/instructions/foo.md"
assert_out_has  "7-names-skill"       "skills/foo/SKILL.md"
assert_out_count "7-both-reported"    "MALFORMED" 2

echo "case 8: .github/instructions/*.md with no version key at all — legitimately skipped"
repo="$(new_repo case8)"
write_frontmatter "$repo/.github/instructions/foo.md" ""
invoke "$repo"
assert_exit    "8-exit-0"     0
assert_out_has "8-version-ok" "Version OK"

echo "case 9: skills/*/SKILL.md with no version key at all — required, so it fails"
repo="$(new_repo case9)"
write_frontmatter "$repo/skills/foo/SKILL.md" ""
invoke "$repo"
assert_exit "9-exit-1" 1

echo "case 10: explicit tag argument, matching tree"
repo="$(new_repo case10a)"
set_all_versions "$repo" "1.2.3"
invoke "$repo" v1.2.3
assert_exit    "10a-exit-0"     0
assert_out_has "10a-version-ok" "Version OK: 1.2.3"

echo "case 10: explicit tag argument, mismatched tree"
repo="$(new_repo case10b)"
invoke "$repo" v1.2.3
assert_exit "10b-exit-1" 1

echo "case 11: empty glob for a required class"
repo="$(new_repo case11)"
rm -rf "${repo:?}/skills"
invoke "$repo"
assert_exit    "11-exit-1"    1
assert_out_has "11-error"     "ERROR: no skills/*/SKILL.md files found"

echo
if ((fail == 0)); then
  echo "All check-version fixtures passed."
  exit 0
else
  echo "check-version fixture failures."
  exit 1
fi
