#!/usr/bin/env bash
# Standalone SQLite cleanup script for local testing
# Keeps rows newer than DAYS by slot cutoff: cutoff = latest_slot - DAYS * SLOTS_PER_DAY (per network)

set -euo pipefail

# --- configuration (override via env) ---
DB="${DB:-./data/governance.db}"
DAYS="${DAYS:-60}"
SLOTS_PER_DAY="${SLOTS_PER_DAY:-216000}"

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "Error: sqlite3 not found. Install with: 'brew install sqlite' (macOS) or 'sudo apt-get install -y sqlite3' (Debian/Ubuntu)." >&2
  exit 1
fi

echo "Running verifier cleanup: DB=${DB}, cutoff from latest = $((DAYS * SLOTS_PER_DAY)) slots"

sqlite3 "$DB" <<SQL

SELECT 'Busy timeout:' AS log;
PRAGMA busy_timeout = 5000;
BEGIN;


-- Build cutoff table directly from snapshot_meta
CREATE TEMP TABLE _cutoff AS
  SELECT
    network,
    MAX(slot) - (${DAYS} * ${SLOTS_PER_DAY}) AS cutoff_slot
  FROM snapshot_meta
  GROUP BY network;

-- Log the computed cutoffs
SELECT 'Cutoff slots per network:' AS log;
SELECT network, cutoff_slot FROM _cutoff;

-- Delete old vote_accounts
DELETE FROM vote_accounts
WHERE rowid IN (
  SELECT va.rowid
  FROM vote_accounts va
  JOIN _cutoff c
    ON c.network = va.network
   AND va.snapshot_slot < c.cutoff_slot
);

-- Delete old stake_accounts
DELETE FROM stake_accounts
WHERE rowid IN (
  SELECT sa.rowid
  FROM stake_accounts sa
  JOIN _cutoff c
    ON c.network = sa.network
   AND sa.snapshot_slot < c.cutoff_slot
);

-- Delete old snapshot_meta entries
DELETE FROM snapshot_meta
WHERE rowid IN (
  SELECT sm.rowid
  FROM snapshot_meta sm
  JOIN _cutoff c
    ON c.network = sm.network
   AND sm.slot < c.cutoff_slot
);

DROP TABLE _cutoff;

COMMIT;
SELECT 'WAL checkpoint results:' AS log;
PRAGMA wal_checkpoint(TRUNCATE);
SQL

echo "Cleanup complete."

