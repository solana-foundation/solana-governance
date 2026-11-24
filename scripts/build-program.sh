#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Regenerates the Codama IDL and the TypeScript client the frontend imports
# from a freshly built Anchor IDL, so a program change cannot leave them stale.
generate_client() {
  local package="$1"
  if ! command -v pnpm >/dev/null 2>&1; then
    echo "ERROR: pnpm is required to regenerate the $package client"
    exit 1
  fi
  echo "Regenerating the $package Codama IDL and TypeScript client..."
  cd "$REPO_ROOT"
  pnpm --filter "$package" run generate:codama-idl
  pnpm --filter "$package" run generate:clients
}

build_svmgov() {
  echo "Building svmgov program..."
  cd "$REPO_ROOT/svmgov/program"
  anchor build

  echo "Syncing IDL files..."
  IDL_SRC="$REPO_ROOT/svmgov/program/target/idl/svmgov_program.json"

  if [ ! -f "$IDL_SRC" ]; then
    echo "ERROR: IDL not found at $IDL_SRC"
    exit 1
  fi

  cp "$IDL_SRC" "$REPO_ROOT/svmgov/cli/idls/svmgov_program.json"
  echo "IDL synced to svmgov/cli/idls/"

  generate_client @solana/svmgov
}

build_ncn() {
  echo "Building ncn program..."
  cd "$REPO_ROOT/ncn"
  anchor build

  generate_client @solana/ncn-snapshot
  echo "ncn build complete"
}

case "${1:-all}" in
  svmgov) build_svmgov ;;
  ncn)    build_ncn ;;
  all)    build_svmgov; build_ncn ;;
  *)      echo "Usage: $0 [svmgov|ncn|all]"; exit 1 ;;
esac
