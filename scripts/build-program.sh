#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# shellcheck source=scripts/setup-jito-tip-router.sh
source "$REPO_ROOT/scripts/setup-jito-tip-router.sh"

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
  cp "$IDL_SRC" "$REPO_ROOT/frontend/src/chain/idl/svmgov_program.json"
  echo "IDL synced to svmgov/cli/idls/ and frontend/src/chain/idl/"
}

build_ncn() {
  ensure_jito_tip_router
  echo "Building ncn program..."
  cd "$REPO_ROOT/ncn"
  anchor build
  echo "ncn build complete"
}

case "${1:-all}" in
  svmgov) build_svmgov ;;
  ncn)    build_ncn ;;
  all)    build_svmgov; build_ncn ;;
  *)      echo "Usage: $0 [svmgov|ncn|all]"; exit 1 ;;
esac
