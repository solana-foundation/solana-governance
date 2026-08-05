#!/bin/bash

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
NCN_DIR="$REPO_ROOT/ncn"

echo "Select network:"
echo "1) mainnet"
echo "2) testnet"

NETWORK=""
while true; do
  read -r -p "> " choice
  if [ "$choice" = "1" ]; then
    NETWORK="mainnet"
    break
  elif [ "$choice" = "2" ]; then
    NETWORK="testnet"
    break
  else
    echo "Please enter 1 or 2."
  fi
done

if [ "$NETWORK" = "testnet" ]; then
  IMAGE_TAG="verifier-service:latest-testnet"
else
  IMAGE_TAG="verifier-service:latest-mainnet"
fi

echo -e "${YELLOW}Building verifier-service binary...${NC}"
if ! command -v cargo >/dev/null 2>&1; then
  echo -e "${RED}Error: cargo not found in PATH.${NC}" >&2
  echo "Install Rust/Cargo and ensure it is available to your current user shell." >&2
  exit 1
fi

cd "$NCN_DIR"
cargo build --locked --release --bin verifier-service

if ! command -v docker >/dev/null 2>&1; then
  echo -e "${YELLOW}Docker is not installed; installing Docker package...${NC}"
  sudo apt-get update
  sudo apt-get install -y docker.io ca-certificates
fi
sudo systemctl enable --now docker

echo -e "${YELLOW}Building Docker image: ${IMAGE_TAG}${NC}"
sudo docker build -f verifier-service/Dockerfile -t "${IMAGE_TAG}" .

echo -e "${YELLOW}Starting verifier-service container via setup.sh...${NC}"
cd "$NCN_DIR/verifier-service/src/scripts"

# setup.sh asks for OPERATOR_PUBKEY, METRICS_AUTH_TOKEN, and PORT_HOST
export VERIFIER_NETWORK="$NETWORK"
sudo VERIFIER_NETWORK="$VERIFIER_NETWORK" bash setup.sh

echo -e "${GREEN}Done.${NC}"
