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

check "VERSION" "$file_ver"

skill_mds=("$root"/skills/*/SKILL.md)
if [[ ! -e "${skill_mds[0]}" ]]; then
	echo "ERROR: no skills/*/SKILL.md files found" >&2
	exit 1
fi
for skill_md in "${skill_mds[@]}"; do
	v="$(grep -E '^version:' "$skill_md" | head -1 | sed -E 's/^version:[[:space:]]*"?([^"]+)"?.*/\1/')"
	check "${skill_md#"$root"/}" "$v"
done

plugin_json="$root/.claude-plugin/plugin.json"
plugin_v="$(grep -E '"version":' "$plugin_json" | head -1 | sed -E 's/.*"version":[[:space:]]*"([^"]+)".*/\1/')"
check ".claude-plugin/plugin.json" "$plugin_v"

# Files under .github/instructions/ may legitimately carry no `version:` line
# (e.g. an applyTo-only frontmatter block) — those are skipped, not gated.
instruction_mds=("$root"/.github/instructions/*.md)
if [[ ! -e "${instruction_mds[0]}" ]]; then
	echo "ERROR: no .github/instructions/*.md files found" >&2
	exit 1
fi
for instruction_md in "${instruction_mds[@]}"; do
	grep -qE '^version:' "$instruction_md" || continue
	v="$(grep -E '^version:' "$instruction_md" | head -1 | sed -E 's/^version:[[:space:]]*"?([^"]+)"?.*/\1/')"
	check "${instruction_md#"$root"/}" "$v"
done

# Files under .cursor/rules/ may legitimately carry no `version:` line
# (e.g. an applyTo-only frontmatter block) — those are skipped, not gated.
cursor_rule_mdcs=("$root"/.cursor/rules/*.mdc)
if [[ ! -e "${cursor_rule_mdcs[0]}" ]]; then
	echo "ERROR: no .cursor/rules/*.mdc files found" >&2
	exit 1
fi
for cursor_rule_mdc in "${cursor_rule_mdcs[@]}"; do
	grep -qE '^version:' "$cursor_rule_mdc" || continue
	v="$(grep -E '^version:' "$cursor_rule_mdc" | head -1 | sed -E 's/^version:[[:space:]]*"?([^"]+)"?.*/\1/')"
	check "${cursor_rule_mdc#"$root"/}" "$v"
done

if [[ "$fail" -ne 0 ]]; then
	echo "Version check FAILED."
	exit 1
fi
echo "Version OK: $expected"
