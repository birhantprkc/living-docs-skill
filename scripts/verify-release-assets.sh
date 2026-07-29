#!/usr/bin/env bash
#
# verify-release-assets.sh — assert a GitHub Release carries every expected asset with a
# valid SHA-256 checksum (ADR 0025). The release is born a draft; this gate promotes it with
# `gh release edit --draft=false` only once every expected asset is present and
# checksum-valid. On failure it keeps the release draft (`gh release edit --draft`, an
# idempotent safety net) and still exits 1, so install.sh never sees a half-published tag.
# Checksum comparison mirrors install.sh's cli_verify_sha256 sha256sum-or-shasum portability
# handling.
#
# Usage:  verify-release-assets.sh [--verify-only] <tag>
# Exit:   0 = every expected asset is present and checksum-valid (release promoted unless
#             --verify-only)
#         1 = an asset is missing or checksum-mismatched (release kept/ensured draft unless
#             --verify-only), or the release itself could not be read (no mutation attempted)
#         2 = usage error (missing tag argument)

set -euo pipefail

readonly TARGET_TRIPLES=(
  aarch64-apple-darwin
  x86_64-apple-darwin
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-gnu
)

usage() {
  printf 'Usage: %s [--verify-only] <tag>\n' "$(basename "$0")" >&2
}

parse_args() {
  MUTATE=1
  TAG=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --verify-only) MUTATE=0 ;;
      -h|--help) usage; exit 0 ;;
      *) TAG="$1" ;;
    esac
    shift
  done
  [[ -n "$TAG" ]] || { usage; exit 2; }
}

expected_assets() {
  local tag="$1" triple
  printf '%s\n' "living-docs-skill-${tag}.zip"
  printf '%s\n' "living-docs-skill-${tag}.zip.sha256"
  for triple in "${TARGET_TRIPLES[@]}"; do
    printf '%s\n' "living-docs-${triple}"
    printf '%s\n' "living-docs-${triple}.sha256"
  done
}

published_assets() {
  local tag="$1"
  gh release view "$tag" --json assets --jq '.assets[].name'
}

missing_assets() {
  local published="$1" name
  while IFS= read -r name; do
    grep -qxF -- "$name" <<<"$published" || printf '%s\n' "$name"
  done < <(expected_assets "$TAG")
}

sha256_of() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  else
    shasum -a 256 "$file" | awk '{print $1}'
  fi
}

checksum_valid() {
  local file="$1" sumfile="$2" expected actual
  expected="$(awk '{print $1}' "$sumfile")"
  actual="$(sha256_of "$file")"
  [[ -n "$expected" && "$expected" == "$actual" ]]
}

download_binary_assets() {
  local dir="$1" triple asset
  for triple in "${TARGET_TRIPLES[@]}"; do
    asset="living-docs-${triple}"
    gh release download "$TAG" --pattern "$asset" --dir "$dir" --clobber
    gh release download "$TAG" --pattern "${asset}.sha256" --dir "$dir" --clobber
  done
}

mismatched_assets() {
  local dir="$1" triple asset
  download_binary_assets "$dir"
  for triple in "${TARGET_TRIPLES[@]}"; do
    asset="living-docs-${triple}"
    checksum_valid "$dir/$asset" "$dir/${asset}.sha256" || printf '%s\n' "$asset"
  done
}

ensure_draft() {
  [[ $MUTATE -eq 1 ]] || return 0
  gh release edit "$TAG" --draft >/dev/null
}

promote_release() {
  [[ $MUTATE -eq 1 ]] || return 0
  gh release edit "$TAG" --draft=false >/dev/null
}

fail_with() {
  printf '%s\n' "$@"
  ensure_draft || true
  exit 1
}

main() {
  parse_args "$@"

  local published missing mismatched count
  WORKDIR="$(mktemp -d)"
  trap 'rm -rf "$WORKDIR"' EXIT

  if ! published="$(published_assets "$TAG" 2>/dev/null)"; then
    printf 'release %s not found or unreadable\n' "$TAG" >&2
    exit 1
  fi

  missing="$(missing_assets "$published")"
  [[ -z "$missing" ]] || fail_with "$missing"

  mismatched="$(mismatched_assets "$WORKDIR")"
  [[ -z "$mismatched" ]] || fail_with "$mismatched"

  count="$(expected_assets "$TAG" | wc -l | tr -d ' ')"
  printf 'verified %s release assets for %s\n' "$count" "$TAG"
  promote_release
}

main "$@"
