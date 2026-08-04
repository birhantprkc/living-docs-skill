#!/usr/bin/env bash
#
# check-version.sh — assert the release version is consistent everywhere it is declared.
#
# The version is necessarily declared in more than one place (the VERSION file and each
# SKILL.md's frontmatter). Duplication drifts — so it is gated: this script is the
# instrument that keeps the copies in agreement ("a constraint without an instrument is a
# vibe"). CI runs it with no argument (internal consistency); the release workflow runs it
# with the git tag (so a forgotten bump fails the release).
#
# Usage:  check-version.sh [EXPECTED]   (default: contents of VERSION)
# Exit:   0 = all agree, 1 = mismatch.

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
file_ver="$(tr -d '[:space:]' < "$root/VERSION")"
expected="${1:-$file_ver}"
expected="${expected#v}" # tolerate a leading 'v' from a git tag

fail=0
check() { # check <label> <actual>
	if [[ "$2" != "$expected" ]]; then
		printf 'MISMATCH: %-40s = %-10s (expected %s)\n' "$1" "'$2'" "'$expected'"
		fail=1
	fi
}

# Detection is deliberately looser than extraction: a file must be caught as "declares a
# version" even when its key is malformed (e.g. a stray space before the colon), so that
# case fails loudly below instead of extraction's strict anchor silently missing it and the
# file being read as if it declared no version at all.
check_versioned_class() { # check_versioned_class <required|optional> <file>...
	local requirement="$1"
	shift
	local f rel v
	for f in "$@"; do
		rel="${f#"$root"/}"
		if ! grep -qE '^[[:space:]]*version[[:space:]]*:' "$f"; then
			[[ "$requirement" == required ]] && check "$rel" ""
			continue
		fi
		v="$(sed -nE 's/^version:[[:space:]]*"?([^"]+)"?.*/\1/p' "$f" | head -1)"
		if [[ -z "$v" ]]; then
			printf 'MALFORMED: %-40s declares a version key not in canonical form (expected: version: X)\n' "$rel"
			fail=1
			continue
		fi
		check "$rel" "$v"
	done
}

check "VERSION" "$file_ver"

# cli/Cargo.toml declares `version = ` in both [package] and (potentially) [dependencies]
# entries (e.g. a dotted-key dependency table), so extraction is scoped to the [package]
# section only — never a bare grep across the whole file.
check_cargo_package_version() { # check_cargo_package_version <file>
	local f="$1" rel scoped v
	rel="${f#"$root"/}"
	if [[ ! -e "$f" ]]; then
		check "$rel" ""
		return
	fi
	scoped="$(sed -nE '/^\[package\]/,/^\[/p' "$f")"
	if ! grep -qE '^version[[:space:]]*=' <<<"$scoped"; then
		check "$rel" ""
		return
	fi
	v="$(sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' <<<"$scoped" | head -1)"
	if [[ -z "$v" ]]; then
		printf 'MALFORMED: %-40s declares a version key not in canonical form (expected: version = "X")\n' "$rel"
		fail=1
		return
	fi
	check "$rel" "$v"
}

check_cargo_package_version "$root/cli/Cargo.toml"

skill_mds=("$root"/skills/*/SKILL.md)
if [[ ! -e "${skill_mds[0]}" ]]; then
	echo "ERROR: no skills/*/SKILL.md files found" >&2
	exit 1
fi
check_versioned_class required "${skill_mds[@]}"

plugin_json="$root/.claude-plugin/plugin.json"
plugin_v="$(grep -E '"version":' "$plugin_json" | head -1 | sed -E 's/.*"version":[[:space:]]*"([^"]+)".*/\1/')"
check ".claude-plugin/plugin.json" "$plugin_v"

instruction_mds=("$root"/.github/instructions/*.md)
if [[ ! -e "${instruction_mds[0]}" ]]; then
	echo "ERROR: no .github/instructions/*.md files found" >&2
	exit 1
fi
check_versioned_class optional "${instruction_mds[@]}"

cursor_rule_mdcs=("$root"/.cursor/rules/*.mdc)
if [[ ! -e "${cursor_rule_mdcs[0]}" ]]; then
	echo "ERROR: no .cursor/rules/*.mdc files found" >&2
	exit 1
fi
check_versioned_class optional "${cursor_rule_mdcs[@]}"

if [[ "$fail" -ne 0 ]]; then
	echo "Version check FAILED."
	exit 1
fi
echo "Version OK: $expected"
