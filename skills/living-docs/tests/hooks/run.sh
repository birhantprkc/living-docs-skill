#!/usr/bin/env bash
#
# run.sh — fixture tests for the block-docs-handwrite PreToolUse hook
# (ADR 0021 verification criteria; ADR 0019 block rules under ADR 0020 scope).
#
# Each case feeds a synthetic PreToolUse JSON payload to the hook against a
# throwaway docs bundle and asserts the exit code AND a message substring
# (present on stderr for blocks, absent for allows).
#
# Exit: 0 = all cases pass, 1 = at least one failed.

set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
HOOK="$HERE/../../hooks/block-docs-handwrite.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
unset LIVING_DOCS_ENFORCE LIVING_DOCS_BUNDLE

RECORD="$TMP/docs/adr/0001-test-decision.md"
mkdir -p "$TMP/docs/adr" "$TMP/docs/research" "$TMP/src"
cat >"$RECORD" <<'EOF'
---
type: ADR
title: Test decision
description: A fixture record.
status: Proposed
tags: [fixture]
timestamp: 2026-01-01T00:00:00Z
---

# 0001. Test decision

## Context

Body prose mentioning frontmatter in passing.

```yaml
status: Rejected
type: Example
```

## Decision

We will keep this fixture minimal.
EOF
printf '# ADR index\n' >"$TMP/docs/adr/index.md"
printf '# Docs\n' >"$TMP/docs/index.md"

fail=0

payload_write() { # payload_write <file> <content>
	jq -n --arg f "$1" --arg c "$2" \
		'{tool_name: "Write", tool_input: {file_path: $f, content: $c}}'
}

payload_edit() { # payload_edit <file> <old> <new>
	jq -n --arg f "$1" --arg o "$2" --arg n "$3" \
		'{tool_name: "Edit", tool_input: {file_path: $f, old_string: $o, new_string: $n}}'
}

run_case() { # run_case <name> <expected_exit> <present|absent> <substring> <env...> -- <payload>
	local name="$1" exp="$2" mode="$3" sub="$4"
	shift 4
	local envs=()
	while [[ "$1" != "--" ]]; do
		envs+=("$1")
		shift
	done
	shift
	local out rc ok=1
	out="$(printf '%s' "$1" | env "${envs[@]}" bash "$HOOK" 2>&1)"
	rc=$?
	[[ "$rc" == "$exp" ]] || ok=0
	if [[ "$mode" == "present" ]]; then
		grep -qF -- "$sub" <<<"$out" || ok=0
	else
		grep -qF -- "$sub" <<<"$out" && ok=0
	fi
	if ((ok == 1)); then
		printf '  ok    %s\n' "$name"
	else
		printf '  FAIL  %s — exit %s (expected %s), expected %s: "%s"\n' \
			"$name" "$rc" "$exp" "$mode" "$sub"
		printf '%s\n' "$out" | sed 's/^/          | /'
		fail=1
	fi
}

echo "block-docs-handwrite hook fixtures"
echo

run_case new-record-write-blocked 2 present "living-docs new" _=1 -- \
	"$(payload_write "$TMP/docs/adr/0002-new-thing.md" $'---\ntype: ADR\n---\nbody')"
run_case type-index-write-blocked 2 present "living-docs index" _=1 -- \
	"$(payload_write "$TMP/docs/adr/index.md" '# ADR index')"
run_case owned-key-edit-blocked 2 present "CLI-owned" _=1 -- \
	"$(payload_edit "$RECORD" 'status: Proposed' 'status: Accepted')"
run_case owned-value-edit-blocked 2 present "CLI-owned" _=1 -- \
	"$(payload_edit "$RECORD" 'Proposed' 'Accepted')"
run_case timestamp-rewrite-blocked 2 present "CLI-owned" _=1 -- \
	"$(payload_write "$RECORD" "$(sed 's/^timestamp:.*/timestamp: 2027-01-01T00:00:00Z/' "$RECORD")")"

run_case body-edit-allowed 0 absent "living-docs" _=1 -- \
	"$(payload_edit "$RECORD" 'keep this fixture minimal' 'keep this fixture tiny')"
run_case description-edit-allowed 0 absent "living-docs" _=1 -- \
	"$(payload_edit "$RECORD" 'description: A fixture record.' 'description: A better fixture record.')"
run_case tags-edit-allowed 0 absent "living-docs" _=1 -- \
	"$(payload_edit "$RECORD" 'tags: [fixture]' 'tags: [fixture, hooks]')"
run_case fenced-status-edit-allowed 0 absent "living-docs" _=1 -- \
	"$(payload_edit "$RECORD" 'status: Rejected' 'status: Retired')"
run_case same-frontmatter-rewrite-allowed 0 absent "living-docs" _=1 -- \
	"$(payload_write "$RECORD" "$(cat "$RECORD")")"

run_case research-record-allowed 0 absent "living-docs" _=1 -- \
	"$(payload_write "$TMP/docs/research/0003-notes.md" $'---\ntype: Research\n---\nbody')"
run_case bundle-root-index-allowed 0 absent "living-docs" _=1 -- \
	"$(payload_write "$TMP/docs/index.md" '# Docs')"
run_case non-docs-path-allowed 0 absent "living-docs" _=1 -- \
	"$(payload_write "$TMP/src/main.rs" 'fn main() {}')"

run_case warn-mode-allows-with-notice 0 present "CLI-owned" LIVING_DOCS_ENFORCE=warn -- \
	"$(payload_edit "$RECORD" 'status: Proposed' 'status: Accepted')"
run_case custom-bundle-scoped 2 present "living-docs new" LIVING_DOCS_BUNDLE=documentation -- \
	"$(payload_write "$TMP/documentation/adr/0002-x.md" 'body')"
run_case garbage-payload-fails-open 0 absent "living-docs" _=1 -- \
	'not json at all'

multi_bad="$(jq -n --arg f "$RECORD" '{tool_name: "MultiEdit", tool_input: {file_path: $f, edits: [
	{old_string: "fixture minimal", new_string: "fixture small"},
	{old_string: "status: Proposed", new_string: "status: Accepted"}]}}')"
run_case multiedit-owned-key-blocked 2 present "CLI-owned" _=1 -- "$multi_bad"

multi_ok="$(jq -n --arg f "$RECORD" '{tool_name: "MultiEdit", tool_input: {file_path: $f, edits: [
	{old_string: "fixture minimal", new_string: "fixture small"},
	{old_string: "Body prose", new_string: "Prose"}]}}')"
run_case multiedit-body-allowed 0 absent "living-docs" _=1 -- "$multi_ok"

echo
if ((fail == 0)); then
	echo "All hook fixtures passed."
	exit 0
else
	echo "Hook fixture failures."
	exit 1
fi
