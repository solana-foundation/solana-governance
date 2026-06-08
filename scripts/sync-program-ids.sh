#!/usr/bin/env bash
# Reads /networks.toml and rewrites every source file that hard-codes the
# svmgov / ncn-snapshot program IDs OR per-network RPC URLs so the whole
# repo (programs, both CLIs, frontend, ncn-router) stays in sync.
#
# Two kinds of substitutions are made:
#
#   1. Program IDs — `declare_id!`, `pubkey!`, `Anchor.toml`, IDL JSONs, TS
#      type files. Rewritten from whatever the canonical `lib.rs` files
#      currently say to the chosen network's IDs.
#
#   2. RPC URLs — named maps in Rust constants (`DEFAULT_MAINNET_RPC_URL`,
#      `DEFAULT_TESTNET_RPC_URL`), the TS map in frontend/getRpcUrls.ts, and
#      the match arms in ncn-router/cron_job.rs. These are always pinned to
#      networks.toml's mainnet/testnet entries (the active network only
#      drives the single-value `DEFAULT_RPC_URL` / `DEFAULT_WSS_URL` fallback).
#
# Idempotent: re-running with the same network is a no-op. Switching networks
# rewrites every reference in one pass.
#
# Usage:
#   ./scripts/sync-program-ids.sh <network> [--dry-run]
#
# Networks: mainnet | mainnet-staging | testnet | testnet-staging | localnet.

set -euo pipefail

usage() {
  cat <<EOF >&2
usage: $0 <network> [--dry-run]
  network: mainnet | mainnet-staging | testnet | testnet-staging | localnet
EOF
  exit 2
}

NETWORK=""
DRY_RUN=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    -h|--help) usage ;;
    *)
      if [ -z "$NETWORK" ]; then
        NETWORK="$arg"
      else
        echo "unexpected arg: $arg" >&2
        usage
      fi
      ;;
  esac
done
[ -n "$NETWORK" ] || usage

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
[ -f networks.toml ] || { echo "error: networks.toml not found at $REPO_ROOT" >&2; exit 1; }

# Extract a key from a [networks.<name>] block in networks.toml.
extract_field() {
  local net="$1" key="$2"
  awk -v section="[networks.$net]" -v key="$key" '
    $0 == section { in_section = 1; next }
    /^\[/         { in_section = 0 }
    in_section && index($0, key) == 1 {
      match($0, /"[^"]*"/)
      if (RSTART > 0) {
        print substr($0, RSTART + 1, RLENGTH - 2)
        exit
      }
    }
  ' networks.toml
}

NEW_SVMGOV=$(extract_field "$NETWORK" svmgov_program_id)
NEW_NCN=$(extract_field "$NETWORK" ncn_snapshot_program_id)
NEW_RPC=$(extract_field "$NETWORK" rpc_url)
MAINNET_RPC=$(extract_field mainnet rpc_url)
TESTNET_RPC=$(extract_field testnet rpc_url)
case "$NEW_RPC" in
  https://*) NEW_WSS="${NEW_RPC/https:\/\//wss://}" ;;
  http://*)  NEW_WSS="${NEW_RPC/http:\/\//ws://}" ;;
  *)         NEW_WSS="$NEW_RPC" ;;
esac

if [ -z "$NEW_SVMGOV" ] || [ -z "$NEW_NCN" ] || [ -z "$NEW_RPC" ] || [ -z "$MAINNET_RPC" ] || [ -z "$TESTNET_RPC" ]; then
  echo "error: networks.toml is missing one of the required entries for $NETWORK / mainnet / testnet" >&2
  exit 1
fi

# Whatever declare_id! currently says is the "before" state we're rewriting from.
canonical_id() {
  grep -oE 'declare_id!\("[1-9A-HJ-NP-Za-km-z]+"\)' "$1" \
    | head -1 \
    | sed -E 's/^declare_id!\("([^"]+)"\)$/\1/'
}

OLD_SVMGOV=$(canonical_id svmgov/program/programs/svmgov_program/src/lib.rs)
OLD_NCN=$(canonical_id ncn/programs/ncn-snapshot/src/lib.rs)

if [ -z "$OLD_SVMGOV" ] || [ -z "$OLD_NCN" ]; then
  echo "error: could not read current declare_id! from canonical lib.rs files" >&2
  exit 1
fi

echo "Network: $NETWORK"
echo "  svmgov:   $OLD_SVMGOV -> $NEW_SVMGOV"
echo "  ncn:      $OLD_NCN -> $NEW_NCN"
echo "  rpc:      $NEW_RPC (active)"
echo "  rpc/wss:  $NEW_WSS (active)"
echo "  mainnet:  $MAINNET_RPC"
echo "  testnet:  $TESTNET_RPC"

# Files that hard-code one or both program IDs. Add new entries here if you
# introduce another spot that needs to stay in sync.
TARGETS=(
  svmgov/program/programs/svmgov_program/src/lib.rs
  svmgov/program/Anchor.toml
  svmgov/cli/idls/svmgov_program.json
  ncn/programs/ncn-snapshot/src/lib.rs
  ncn/Anchor.toml
  frontend/src/chain/idl/svmgov_program.json
  frontend/src/chain/idl/gov-v1.json
  frontend/src/chain/types/svmgov_program.ts
  frontend/src/chain/types/gov-v1.ts
)

CHANGED=()
SKIPPED=()

# Applies multiple sed substitutions to a file and records whether any of
# them actually changed the file contents. Honors $DRY_RUN by writing to a
# temp copy and discarding.
apply_sed() {
  local file="$1"; shift
  if [ ! -f "$file" ]; then
    SKIPPED+=("$file (not present)")
    return
  fi
  local args=()
  for pattern in "$@"; do args+=(-e "$pattern"); done
  local tmp
  tmp=$(mktemp)
  sed "${args[@]}" "$file" >"$tmp"
  if cmp -s "$file" "$tmp"; then
    SKIPPED+=("$file (no match)")
    rm -f "$tmp"
    return
  fi
  if [ "$DRY_RUN" -eq 0 ]; then
    mv "$tmp" "$file"
    CHANGED+=("$file")
  else
    rm -f "$tmp"
    CHANGED+=("$file (dry-run)")
  fi
}

# --- 1. Program ID substitutions across canonical targets -----------------
for f in "${TARGETS[@]}"; do
  if [ ! -f "$f" ]; then
    SKIPPED+=("$f (not present)")
    continue
  fi
  modified=0
  if [ "$OLD_SVMGOV" != "$NEW_SVMGOV" ] && grep -q "$OLD_SVMGOV" "$f"; then
    [ "$DRY_RUN" -eq 0 ] && sed -i "s|$OLD_SVMGOV|$NEW_SVMGOV|g" "$f"
    modified=1
  fi
  if [ "$OLD_NCN" != "$NEW_NCN" ] && grep -q "$OLD_NCN" "$f"; then
    [ "$DRY_RUN" -eq 0 ] && sed -i "s|$OLD_NCN|$NEW_NCN|g" "$f"
    modified=1
  fi
  if [ $modified -eq 1 ]; then
    CHANGED+=("$f")
  else
    SKIPPED+=("$f (no match)")
  fi
done

# --- 2. RPC URL substitutions in named maps -------------------------------

# svmgov CLI constants — named maps (mainnet/testnet) get networks.toml's
# fixed entries; bare DEFAULT_RPC_URL/DEFAULT_WSS_URL follow the active network.
apply_sed svmgov/cli/src/constants.rs \
  "s|^pub const DEFAULT_MAINNET_RPC_URL: &str = \".*\";|pub const DEFAULT_MAINNET_RPC_URL: \&str = \"$MAINNET_RPC\";|" \
  "s|^pub const DEFAULT_TESTNET_RPC_URL: &str = \".*\";|pub const DEFAULT_TESTNET_RPC_URL: \&str = \"$TESTNET_RPC\";|" \
  "s|^pub const DEFAULT_RPC_URL: &str = \".*\";|pub const DEFAULT_RPC_URL: \&str = \"$NEW_RPC\";|" \
  "s|^pub const DEFAULT_WSS_URL: &str = \".*\";|pub const DEFAULT_WSS_URL: \&str = \"$NEW_WSS\";|"

# Frontend default RPC URL map — keep mainnet/testnet pinned to networks.toml.
apply_sed frontend/src/lib/getRpcUrls.ts \
  "s|mainnet: \"[^\"]*\"|mainnet: \"$MAINNET_RPC\"|" \
  "s|testnet: \"[^\"]*\"|testnet: \"$TESTNET_RPC\"|"

# ncn-router cron job default-rpc match — anchored on the literal
# "testnet" => "..." arm and the fallback `_ => "https://api.*.solana.com"` arm.
apply_sed ncn-router/src/cron_job.rs \
  "s|\"testnet\" => \"https://api\\.[a-z-]*\\.solana\\.com\",|\"testnet\" => \"$TESTNET_RPC\",|" \
  "s|_ => \"https://api\\.[a-z-]*\\.solana\\.com\",|_ => \"$MAINNET_RPC\",|"

echo
if [ ${#CHANGED[@]} -gt 0 ]; then
  echo "Updated:"
  printf '  %s\n' "${CHANGED[@]}"
fi
if [ ${#SKIPPED[@]} -gt 0 ]; then
  echo "Skipped:"
  printf '  %s\n' "${SKIPPED[@]}"
fi
if [ "$DRY_RUN" -eq 1 ]; then
  echo "(dry-run: no files written)"
fi
