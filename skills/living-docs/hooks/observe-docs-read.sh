#!/usr/bin/env bash
#
# observe-docs-read.sh — PostToolUse capture for consumption metrics (issue #58).
#
# Appends one JSONL line per Read/Grep/Glob of a record under the docs bundle:
#   {"ts":…,"path":…,"record":…,"status":…,"tokens":…,"task":…}
# `living-docs scorecard` reads this log to report docs tokens read, stale
# reads, and (with a findings log) the doc-trail finding share.
#
# OFF BY DEFAULT: this hook is not wired into the shipped registry — a project
# opts in by adding it as a PostToolUse hook in its own settings. It is
# fail-open and NEVER blocks: every path exits 0, so a broken capture can
# never break a tool call.
#
# Env:  LIVING_DOCS_CONSUMPTION_LOG=<file>  (default <bundle>/../.living-docs/consumption.jsonl)
#       LIVING_DOCS_BUNDLE=<dir>            (default docs)

set -u
BUNDLE="${LIVING_DOCS_BUNDLE:-docs}"
LOG="${LIVING_DOCS_CONSUMPTION_LOG:-.living-docs/consumption.jsonl}"

command -v jq >/dev/null 2>&1 || exit 0
INPUT="$(cat 2>/dev/null)" || exit 0

FILE="$(jq -r '.tool_input.file_path // .tool_input.path // empty' <<<"$INPUT" 2>/dev/null)"
[ -n "$FILE" ] || exit 0

case "$FILE" in
  *"$BUNDLE"/*.md) ;;
  *) exit 0 ;;
esac
[ -f "$FILE" ] || exit 0

RECORD="${FILE##*"$BUNDLE"/}"
RECORD="${RECORD%.md}"
STATUS="$(awk 'NR==1 && $0!="---"{exit} /^status:/{sub(/^status:[[:space:]]*/,"");gsub(/["'\'']/,"");print;exit} $0=="---" && NR>1{exit}' "$FILE" 2>/dev/null)"
CHARS="$(wc -m <"$FILE" 2>/dev/null | tr -d ' ')"
CHARS="${CHARS:-0}"
TOKENS=$(( CHARS / 4 ))
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null)"
TASK="${CLAUDE_SESSION_ID:-}"

mkdir -p "$(dirname "$LOG")" 2>/dev/null || exit 0
LINE="$(jq -cn \
  --arg ts "$TS" --arg path "$FILE" --arg record "$RECORD" \
  --arg status "$STATUS" --argjson tokens "$TOKENS" --arg task "$TASK" \
  '{ts:$ts,path:$path,record:$record,status:$status,tokens:$tokens,task:$task}' 2>/dev/null)" || exit 0
printf '%s\n' "$LINE" >>"$LOG" 2>/dev/null || exit 0
exit 0
