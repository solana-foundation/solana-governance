# 0) Modify environment variables for your server
# These are prompted from the user at runtime (mandatory; do not press Enter).
# The Docker image tag is auto-selected from the network.
OPERATOR_PUBKEY_DEFAULT="C5m2XDwZmjc7yHpy8N4KhQtFJLszasVpfB4c5MTuCsmg"
METRICS_AUTH_TOKEN_DEFAULT="1234567890"

# Network -> Docker image tag
# You can set VERIFIER_NETWORK=mainnet|testnet when calling this script.
VERIFIER_NETWORK="${VERIFIER_NETWORK:-mainnet}"
case "${VERIFIER_NETWORK}" in
  testnet)
    IMAGE="verifier-service:latest-testnet"
    ;;
  *)
    IMAGE="verifier-service:latest-mainnet"
    ;;
esac

# Prompt only when running interactively.
if [ -t 0 ]; then
  # Read a required value; reprompt until non-empty.
  read_required() {
    # $1 = prompt text (should not include trailing colon)
    local prompt="$1"
    local value=""
    while true; do
      read -r -p "${prompt}: " value
      if [ -n "${value}" ]; then
        echo "${value}"
        return 0
      fi
      echo "This value is required."
    done
  }

  echo "Setup verifier-service configuration:"
  OPERATOR_PUBKEY="$(read_required "OPERATOR_PUBKEY (example: ${OPERATOR_PUBKEY_DEFAULT})")"
  METRICS_AUTH_TOKEN="$(read_required "METRICS_AUTH_TOKEN (example: ${METRICS_AUTH_TOKEN_DEFAULT})")"
  PORT_HOST="$(read_required "PORT_HOST (host port to expose, example: 9090)")"
else
  echo "This script requires interactive input for OPERATOR_PUBKEY, METRICS_AUTH_TOKEN, and PORT_HOST." >&2
  exit 1
fi

# Validate host port format/range early.
if ! [[ "${PORT_HOST}" =~ ^[0-9]+$ ]] || [ "${PORT_HOST}" -lt 1 ] || [ "${PORT_HOST}" -gt 65535 ]; then
  echo "Invalid PORT_HOST '${PORT_HOST}'. Expected an integer in range 1-65535." >&2
  exit 1
fi

# Network and Directories
PORT_CONTAINER=3000
DATA_DIR=/srv/verifier/data
DB_PATH=/data/governance.db

# Rate and File Upload Limits
GLOBAL_REFILL_INTERVAL=10
GLOBAL_RATE_BURST=10
UPLOAD_REFILL_INTERVAL=60
UPLOAD_RATE_BURST=2
# Upload body size limit (bytes)
UPLOAD_BODY_LIMIT=$((100 * 1024 * 1024)) # 100MB
# Max decompressed snapshot size (MiB) enforced by CLI bounded decompressor
NCN_SNAPSHOT_MAX_MB=256

# SQLite pool size
SQLITE_MAX_CONNECTIONS=4

# Docker log rotation
DOCKER_LOG_DRIVER="json-file"
DOCKER_LOG_MAX_SIZE="2g"
DOCKER_LOG_MAX_FILE="5"

# 1) Ensure Docker is available (installs if missing)
# If Docker is already installed, skip package installation.
if command -v docker >/dev/null 2>&1; then
  echo "Docker already installed; skipping Docker package install."
else
  echo "Docker is not installed; installing Docker package..."
  sudo apt-get update
  sudo apt-get install -y docker.io ca-certificates
fi

# Ensure Docker daemon is enabled/running.
sudo systemctl enable --now docker

# Helpful note: docker CLI access without sudo requires docker group membership.
if ! groups "${USER:-$(id -un)}" 2>/dev/null | grep -q '\bdocker\b'; then
  echo "Note: your user is not in the 'docker' group; run docker commands with sudo."
  echo "To enable non-sudo docker CLI access later:"
  echo "  sudo usermod -aG docker ${USER:-$(id -un)} && newgrp docker"
fi

# 2) Prepare persistent state dir (UID 10001 matches your Dockerfile USER)
sudo mkdir -p "$(dirname "$DATA_DIR")"
sudo mkdir -p "$DATA_DIR"
sudo chown -R 10001:10001 /srv/verifier

# 3) Pull image only if it's not already present locally.
# This lets you run `cargo build` + `docker build` first and then deploy without relying on network access.
if sudo docker image inspect "$IMAGE" >/dev/null 2>&1; then
  echo "Docker image '$IMAGE' already exists locally; skipping 'docker pull'."
else
  sudo docker pull "$IMAGE"
fi

# 4) Re-create container idempotently, then run (daemonized, restarts on reboot/crash)
# Stop and remove existing container if it exists
sudo docker rm -f verifier >/dev/null 2>&1 || true

# Ensure requested host port is free before attempting docker run.
if sudo ss -lnt "( sport = :${PORT_HOST} )" | awk 'NR>1 {exit 0} END {exit 1}'; then
  echo "Host port ${PORT_HOST} is already in use." >&2
  echo "Choose another PORT_HOST or stop the process using it (example: sudo lsof -iTCP:${PORT_HOST} -sTCP:LISTEN)." >&2
  exit 1
fi

sudo docker run -d --name verifier --restart unless-stopped \
  --log-driver ${DOCKER_LOG_DRIVER} \
  --log-opt max-size=${DOCKER_LOG_MAX_SIZE} \
  --log-opt max-file=${DOCKER_LOG_MAX_FILE} \
  -p ${PORT_HOST}:${PORT_CONTAINER} \
  -e OPERATOR_PUBKEY="${OPERATOR_PUBKEY}" \
  -e DB_PATH="${DB_PATH}" \
  -e PORT="${PORT_CONTAINER}" \
  -e RUST_LOG=info \
  -e METRICS_AUTH_TOKEN="${METRICS_AUTH_TOKEN}" \
  -e GLOBAL_REFILL_INTERVAL="${GLOBAL_REFILL_INTERVAL}" \
  -e GLOBAL_RATE_BURST="${GLOBAL_RATE_BURST}" \
  -e UPLOAD_REFILL_INTERVAL="${UPLOAD_REFILL_INTERVAL}" \
  -e UPLOAD_RATE_BURST="${UPLOAD_RATE_BURST}" \
  -e UPLOAD_BODY_LIMIT="${UPLOAD_BODY_LIMIT}" \
  -e SQLITE_MAX_CONNECTIONS="${SQLITE_MAX_CONNECTIONS}" \
  -e NCN_SNAPSHOT_MAX_MB="${NCN_SNAPSHOT_MAX_MB}" \
  -v "${DATA_DIR}:/data" \
  "${IMAGE}"

# 5) Verify
sudo docker ps
curl -fsS "http://127.0.0.1:${PORT_HOST}/healthz" || sudo docker logs --tail=200 verifier
echo
