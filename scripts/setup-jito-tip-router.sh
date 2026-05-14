#!/usr/bin/env bash
# Ensures the local jito-tip-router checkout matches the commit pinned in
# /networks.toml. Sourced by other scripts; can also be invoked directly.
#
# Resolves $REPO_ROOT/jito-tip-router to:
#   - cloned from jito-foundation/jito-tip-router if missing
#   - checked out at jito_tip_router_commit (read from [networks.mainnet] in
#     networks.toml, since all networks pin the same commit per release).
#
# Refuses to touch a dirty working tree so it never destroys local work.
#
# Usage (as helper):
#   source scripts/setup-jito-tip-router.sh
#   ensure_jito_tip_router
#
# Usage (standalone):
#   bash scripts/setup-jito-tip-router.sh

set -euo pipefail

JITO_TIP_ROUTER_REPO="${JITO_TIP_ROUTER_REPO:-https://github.com/jito-foundation/jito-tip-router.git}"

_setup_repo_root() {
  local script_dir
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  echo "$(cd "$script_dir/.." && pwd)"
}

_setup_networks_commit() {
  local repo_root="$1"
  awk '
    /^\[networks\.mainnet\]/ { in_section = 1; next }
    /^\[/                     { in_section = 0 }
    in_section && index($0, "jito_tip_router_commit") == 1 {
      match($0, /"[^"]*"/)
      if (RSTART > 0) {
        print substr($0, RSTART + 1, RLENGTH - 2)
        exit
      }
    }
  ' "$repo_root/networks.toml"
}

ensure_jito_tip_router() {
  local repo_root jito_dir commit
  repo_root="$(_setup_repo_root)"
  jito_dir="$repo_root/jito-tip-router"

  if [ ! -f "$repo_root/networks.toml" ]; then
    echo "error: $repo_root/networks.toml not found" >&2
    return 1
  fi

  commit="$(_setup_networks_commit "$repo_root")"
  if [ -z "$commit" ]; then
    echo "error: could not extract jito_tip_router_commit from networks.toml" >&2
    return 1
  fi

  if [ ! -d "$jito_dir/.git" ]; then
    echo "Cloning $JITO_TIP_ROUTER_REPO into $jito_dir..."
    git clone "$JITO_TIP_ROUTER_REPO" "$jito_dir"
  fi

  # Ensure the upstream remote is configured (origin may point at a fork that
  # lacks the pinned commit, e.g. legacy exo-tech-xyz clones).
  if ! git -C "$jito_dir" remote get-url upstream >/dev/null 2>&1; then
    git -C "$jito_dir" remote add upstream "$JITO_TIP_ROUTER_REPO"
  fi

  if [ -n "$(git -C "$jito_dir" status --porcelain 2>/dev/null)" ]; then
    echo "error: $jito_dir has local changes; refusing to checkout $commit." >&2
    echo "       commit or stash them, then re-run." >&2
    return 1
  fi

  local head
  head="$(git -C "$jito_dir" rev-parse HEAD 2>/dev/null || echo '')"
  if [ "$head" != "$commit" ]; then
    echo "Updating jito-tip-router to pinned commit $commit..."
    if ! git -C "$jito_dir" cat-file -e "$commit^{commit}" 2>/dev/null; then
      git -C "$jito_dir" fetch --tags upstream
    fi
    git -C "$jito_dir" checkout --quiet --detach "$commit"
  fi

  if [ ! -f "$jito_dir/meta_merkle_tree/Cargo.toml" ] || [ ! -d "$jito_dir/tip-router-operator-cli" ]; then
    echo "error: expected workspace members missing in $jito_dir" >&2
    return 1
  fi

  echo "jito-tip-router ready at $(git -C "$jito_dir" rev-parse --short HEAD) ($jito_dir)"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  ensure_jito_tip_router
fi
