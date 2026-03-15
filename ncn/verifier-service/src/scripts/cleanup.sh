#!/usr/bin/env bash
# Verifier DB cleanup script for operators 
# Deletes rows older than DAYS worth of slots per network.
#
# Usage:
#   - Make env config changes and paste script into shell.
#   - Install cron at a specific time (daily at 03:10):
#       CRON_MINUTE=10 CRON_HOUR=3 bash cleanup.sh install-cron
#   - Install cron hourly at a specific minute (e.g., minute 28 every hour):
#       CRON_MINUTE=28 CRON_HOUR="*" bash cleanup.sh install-cron

set -euo pipefail
# Disable history expansion to avoid '!'-related issues when copy-pasting interactively
set +H 2>/dev/null || set +o histexpand 2>/dev/null || true

# --- configuration (override via env) ---
DB="${DB:-/srv/verifier/data/governance.db}"
DAYS="${DAYS:-60}"
SLOTS_PER_DAY="${SLOTS_PER_DAY:-216000}"
CRON_MINUTE="${CRON_MINUTE:-0}"
CRON_HOUR="${CRON_HOUR:-8}"

maybe_exit() {
  local code="${1:-0}"
  case $- in
    *i*)
      return "$code" 2>/dev/null || true
      ;;
    *)
      exit "$code"
      ;;
  esac
}

# Choose sudo when available and necessary
SUDO_BIN=""
if command -v sudo >/dev/null 2>&1 && [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
  SUDO_BIN="sudo"
fi

ensure_cron_installed_and_running() {
  # Install a cron daemon if missing, then enable and start it
  if ! command -v cron >/dev/null 2>&1 && ! command -v crond >/dev/null 2>&1; then
    echo "[cleanup] cron not found. Attempting automatic installation..." >&2
    if command -v apt-get >/dev/null 2>&1; then
      $SUDO_BIN apt-get update -y >/dev/null 2>&1 || true
      $SUDO_BIN apt-get install -y cron || true
    elif command -v yum >/dev/null 2>&1; then
      $SUDO_BIN yum install -y cronie || true
    elif command -v dnf >/dev/null 2>&1; then
      $SUDO_BIN dnf install -y cronie || true
    elif command -v apk >/dev/null 2>&1; then
      $SUDO_BIN apk add --no-cache dcron || $SUDO_BIN apk add --no-cache cronie || true
    elif command -v zypper >/dev/null 2>&1; then
      $SUDO_BIN zypper --non-interactive install cron || true
    elif command -v pacman >/dev/null 2>&1; then
      $SUDO_BIN pacman -Sy --noconfirm cronie || true
    fi
  fi

  # Start/enable the cron service
  if command -v systemctl >/dev/null 2>&1; then
    $SUDO_BIN systemctl enable --now cron 2>/dev/null || \
    $SUDO_BIN systemctl enable --now crond 2>/dev/null || true
  fi
  $SUDO_BIN service cron start 2>/dev/null || \
  $SUDO_BIN service crond start 2>/dev/null || \
  ($SUDO_BIN crond -s -b 2>/dev/null & ) || true
}

materialize_sql_runner() {
  # Writes a self-contained SQL cleanup runner to a fixed path so cron can call it
  local target="/usr/local/bin/verifier-cleanup-sql.sh"
  echo "[cleanup] Ensuring SQL runner exists at ${target}"
  local tmp
  tmp="$(mktemp)"
  cat >"${tmp}" <<'RUNNER'
#!/usr/bin/env bash
set -euo pipefail

DB="${DB:-/srv/verifier/data/governance.db}"
DAYS="${DAYS:-60}"
SLOTS_PER_DAY="${SLOTS_PER_DAY:-216000}"

SUDO_BIN=""
if command -v sudo >/dev/null 2>&1 && [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
  SUDO_BIN="sudo"
fi

DB_DIR="$(dirname -- "$DB")"
SQLITE_RUNNER="sqlite3"
if [[ ! -w "$DB" || ! -w "$DB_DIR" ]]; then
  if [[ -n "$SUDO_BIN" ]]; then
    SQLITE_RUNNER="$SUDO_BIN sqlite3"
  fi
fi

$SQLITE_RUNNER "$DB" <<SQL

SELECT 'Busy timeout:' AS log;
PRAGMA busy_timeout = 5000;
BEGIN;

CREATE TEMP TABLE _cutoff AS
  SELECT
    network,
    MAX(slot) - (${DAYS} * ${SLOTS_PER_DAY}) AS cutoff_slot
  FROM snapshot_meta
  GROUP BY network;

SELECT 'Cutoff slots per network:' AS log;
SELECT network, cutoff_slot FROM _cutoff;

DELETE FROM vote_accounts
WHERE rowid IN (
  SELECT va.rowid
  FROM vote_accounts va
  JOIN _cutoff c
    ON c.network = va.network
   AND va.snapshot_slot < c.cutoff_slot
);

DELETE FROM stake_accounts
WHERE rowid IN (
  SELECT sa.rowid
  FROM stake_accounts sa
  JOIN _cutoff c
    ON c.network = sa.network
   AND sa.snapshot_slot < c.cutoff_slot
);

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
RUNNER

  $SUDO_BIN install -m 0755 "${tmp}" "${target}"
  rm -f -- "${tmp}"
}

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "[cleanup] sqlite3 not found. Attempting automatic installation..." >&2
  OS_NAME="$(uname -s 2>/dev/null || echo unknown)"

  install_ok=false

  if [[ "$OS_NAME" == "Darwin" ]]; then
    if command -v brew >/dev/null 2>&1; then
      echo "[cleanup] Using Homebrew to install sqlite..."
      (brew update -q || true) && brew install sqlite || true
      command -v sqlite3 >/dev/null 2>&1 && install_ok=true
    else
      echo "[cleanup] Homebrew not found on macOS. Please install Homebrew from https://brew.sh then re-run this script, or install sqlite manually (brew install sqlite)." >&2
    fi
  else
    if command -v apt-get >/dev/null 2>&1; then
      echo "[cleanup] Using apt-get to install sqlite3..."
      sudo apt-get update -y && sudo apt-get install -y sqlite3 && install_ok=true || true
    elif command -v yum >/dev/null 2>&1; then
      echo "[cleanup] Using yum to install sqlite..."
      sudo yum install -y sqlite && install_ok=true || true
    elif command -v dnf >/dev/null 2>&1; then
      echo "[cleanup] Using dnf to install sqlite..."
      sudo dnf install -y sqlite && install_ok=true || true
    elif command -v apk >/dev/null 2>&1; then
      echo "[cleanup] Using apk to install sqlite..."
      sudo apk add --no-cache sqlite && install_ok=true || true
    elif command -v zypper >/dev/null 2>&1; then
      echo "[cleanup] Using zypper to install sqlite3..."
      sudo zypper --non-interactive install sqlite3 && install_ok=true || true
    elif command -v pacman >/dev/null 2>&1; then
      echo "[cleanup] Using pacman to install sqlite..."
      sudo pacman -Sy --noconfirm sqlite && install_ok=true || true
    fi
  fi

  if ! $install_ok; then
    if ! command -v sqlite3 >/dev/null 2>&1; then
      echo "Error: Failed to install sqlite3 automatically. Please install sqlite3 using your system's package manager and re-run. Examples: 'sudo apt-get install -y sqlite3' (Debian/Ubuntu), 'sudo yum install -y sqlite' (RHEL), 'brew install sqlite' (macOS)." >&2
      maybe_exit 1
    fi
  fi
fi

if [[ "${1:-install-cron}" == "install-cron" ]]; then
  ensure_cron_installed_and_running
  materialize_sql_runner
  echo "[cleanup] Installing cron entry at ${CRON_HOUR}:${CRON_MINUTE} daily using /usr/local/bin/verifier-cleanup-sql.sh"
  # Write via temp file so we can expand env vars, escape % for cron, and keep awk's $0 intact
  tmp_cron_file="$(mktemp)"
  cat >"${tmp_cron_file}" <<EOF
SHELL=/bin/bash
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
${CRON_MINUTE} ${CRON_HOUR} * * * root DB=${DB} DAYS=${DAYS} SLOTS_PER_DAY=${SLOTS_PER_DAY} /usr/bin/bash /usr/local/bin/verifier-cleanup-sql.sh 2>&1 | awk '{ print strftime("[\%Y-\%m-\%d \%H:\%M:\%S]"), \$0; fflush(); }' >> /var/log/verifier-cleanup.log
EOF
  $SUDO_BIN install -m 0644 "${tmp_cron_file}" /etc/cron.d/verifier-cleanup
  rm -f -- "${tmp_cron_file}"
  $SUDO_BIN chmod 644 /etc/cron.d/verifier-cleanup
  $SUDO_BIN service cron reload 2>/dev/null || $SUDO_BIN service crond reload 2>/dev/null || true
  echo "[cleanup] Cron installed."
  maybe_exit 0
fi
