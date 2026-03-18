# Governance Merkle Verifier Service

A self-contained Rust web service for serving Merkle proofs and leaf nodes for Solana governance voting.

[← Back to Project README](../README.md)
[→ Deployment Guide](DEPLOYMENT.md)

## Quick Start

```bash
# Set the operator public key for signature verification (replace with your own)
export OPERATOR_PUBKEY="C5m2XDwZmjc7yHpy8N4KhQtFJLszasVpfB4c5MTuCsmg"
export METRICS_AUTH_TOKEN="change-me-please"

# Run the service
RUST_LOG=info cargo run --bin verifier-service

# Optional: Run with custom database path
DB_PATH="./data/governance.db" RUST_LOG=info cargo run --bin verifier-service

# Optional: Run with in-memory database (for testing)
DB_PATH=":memory:" RUST_LOG=info cargo run --bin verifier-service

# The service will start on http://localhost:3000
```

## API Endpoints

- `POST /upload` - Upload and index Merkle snapshots
- `GET /healthz` - Health check
- `GET /version` - Service version and build info (crate version, git hash)
- `GET /meta` - Metadata for most recent snapshot
- `GET /voter/:voting_wallet` - Get vote and stake account summaries
- `GET /proof/vote_account/:vote_account` - Get Merkle proof for vote account
- `GET /proof/stake_account/:stake_account` - Get Merkle proof for stake account
- `GET /admin/stats` - Admin metrics (requires header `X-Metrics-Token`)

## Security

### Signature Verification

The `/upload` endpoint requires Ed25519 signature verification to prevent unauthorized snapshot uploads:

- **Environment Variable**: Set `OPERATOR_PUBKEY` to the base58-encoded public key of the authorized operator
- **Message Format**: Signatures are verified over `slot.to_le_bytes() || merkle_root_bs58_string.as_bytes()`
- **Signature Format**: Base58-encoded Ed25519 signature

### Admin Endpoint Authentication

- The `/admin/stats` endpoint is protected by a static token to prevent unauthenticated access.
- Set `METRICS_AUTH_TOKEN` in the environment to enable it.
- Clients must include header `X-Metrics-Token: <token>`.
- If `METRICS_AUTH_TOKEN` is unset, the endpoint returns `503 Service Unavailable`.

Example:

```bash
curl -H "X-Metrics-Token: $METRICS_AUTH_TOKEN" http://localhost:3000/admin/stats
```

## Testing

### Running Tests

```bash
cargo test --bin verifier-service
```

## Build and Release Docker Image (using local binary on Linux)

Prepare for release:

- Ensure that **version** in Cargo.toml is updated.
- Ensure that all changes are committed to git.

```bash
# 1) Build the binary locally
cargo build --release --bin verifier-service

# 2) Build a minimal runtime image (copies the binary only)
docker build -f verifier-service/Dockerfile -t verifier-service:local .

# 3) Run the container (persists DB to ./data)
docker run --rm -p 3000:3000 \
  -e OPERATOR_PUBKEY="$OPERATOR_PUBKEY" \
  -e METRICS_AUTH_TOKEN="$METRICS_AUTH_TOKEN" \
  -e RUST_LOG=info \
  -v "$(pwd)/data:/data" \
  verifier-service:local

# 4) Health check
curl -i http://localhost:3000/healthz

# 4b) Version check
curl -s http://localhost:3000/version

# 5) Publish image to Docker Hub
docker login # login to docker hub if needed
docker tag verifier-service:local username/verifier-service:v0.1.0 # set version
docker tag verifier-service:local username/verifier-service:latest
docker push username/verifier-service:latest
docker push username/verifier-service:v0.1.0

```

Environment variables:

- OPERATOR_PUBKEY (required)
- DB_PATH (optional, defaults to /data/governance.db inside container)
- PORT (optional, defaults to 3000)
- SQLITE_MAX_CONNECTIONS (optional; default 4 for file DB, 1 for in-memory)
- UPLOAD_BODY_LIMIT (optional, bytes; default 104857600 = 100MB)
- GLOBAL_RATE_PER_SECOND, GLOBAL_RATE_BURST (optional; default 10/10)
- UPLOAD_RATE_PER_SECOND, UPLOAD_RATE_BURST (optional; default 60/2)
- NCN_SNAPSHOT_MAX_MB (optional; decompressed snapshot cap in MiB; default 256)

<!-- TODO: Add docker-compose for dev convenience -->
<!-- TODO: Add Docker HEALTHCHECK using /healthz -->

**Note**: Tests use `serial_test` to run sequentially due to shared environment variable usage.

### Upload a Snapshot

To test the upload endpoint with a snapshot (replace fields with actual values):

```bash
curl -X POST http://localhost:3000/upload \
  -F "slot=340850340" \
  -F "network=testnet" \
  -F "merkle_root=34sfrZPCyuLXsq5v1ybahTVSwQQE6A3VJyr9JcgxsW21" \
  -F "signature=3nn1EGUqZ5GSXgfAs86miP4z5HtVdKYdQeDdhm1p2M5XxfK16cxwBJYonFdN4BDT7qzpx6TyEhHUrnF2Bh7wGm71" \
  -F "file=@meta_merkle-340850340.zip" \
  -w "\nHTTP Status: %{http_code}\n" \
  -s
```

### Get Metadata

```bash
curl http://localhost:3000/meta?network=testnet
```

Example response:

```json
{
  "network": "testnet",
  "slot": 340850340,
  "merkle_root": "8oaP5t8E6GEMVE19NFbCNAUxQ7GZe6q8c6XVWvgBgs5p",
  "snapshot_hash": "2ejpKvga5pGMyQGhmi59U6PThwKFzLy8SAjxt5yG8raH",
  "created_at": "2025-08-05T16:17:25.855006+00:00"
}
```

### Get Voter Summary

```bash
curl -i "http://localhost:3000/voter/5KjCzFvbCkRswE9x776udwrRXADbRiboNnmFQRhEHEuR?network=testnet&slot=340850340"
```

Example response:

```json
{
  "network": "testnet",
  "snapshot_slot": 340850340,
  "stake_accounts": [],
  "vote_accounts": [
    {
      "active_stake": :33334695348563,
      "vote_account": "1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd"
    }
  ],
  "voting_wallet": "5KjCzFvbCkRswE9x776udwrRXADbRiboNnmFQRhEHEuR"
}
```

### Get Vote Proof

```bash
curl -i "http://localhost:3000/proof/vote_account/1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd?network=testnet&slot=340850340"
```

Example response:

```json
{
  "meta_merkle_leaf": {
    "active_stake": 33334695348563,
    "stake_merkle_root": "88dMM15gT735bBKNt6ejZFqVJZT7RH7jms1nwPErcr5K",
    "vote_account": "1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd",
    "voting_wallet": "5KjCzFvbCkRswE9x776udwrRXADbRiboNnmFQRhEHEuR"
  },
  "meta_merkle_proof": [
    "DekswL1ny57JTqM9dqgZydN8siHNCkpT9K4pbwMJvygU",
    "7hDL2wvLL6Gj1qU9tvZTmMGvoiGn3q6huFyTvWiPAuze",
    "EKArYLKqkg9n5BpTACHbrK5jtwoCF3xYr7HVXLBmnwyc",
    "7VvZx4gkdi9k8fwRaY2UBQeYPXTv6pfqnAAsaCRUkLVy",
    "FCoAuswGk3hrQCcd68wUWNrXYBZmNkavE76zALTRsPo1",
    "Eo38DkkZ6k5DDHosWFa7yUf2GYcWzVycYt21fzFFY7M2",
    "AxHkTpxsNgPA12b2aX4R6DPuQbAn6ni4Kg3ExpFr6Kxw",
    "68xkyfSadARiN8v2NPxSeGE5V9GsMUFoZGJpXvFScFHr",
    "Eh3owVJxCLheASwEMJUY2jsCC3VXMGk8cqQuAKMc7BZ8",
    "8LkeSkyDR38UC2PofhWqpfrTgQSPYTxegqt5wAgS86Fd",
    "J8WL9x4uedpC4ZNd2sp7nwMqBpX1ew92VtEM3pbz9Tc5",
    "8Hn8LWDnKLnsZhQSpPyV3cQWDm7apyKvVDRWhfWTo69b"
  ],
  "network": "testnet",
  "snapshot_slot": 340850340
}
```

### Get Stake Proof

```bash
curl -i "http://localhost:3000/proof/stake_account/DXmtAZdYsVZT8ir8uPkuY4cgBtsxWpZU4QKdpcAbFngo?network=testnet&slot=340850340"
```

Example response:

```json
{
  "network": "testnet",
  "snapshot_slot": 340850340,
  "stake_merkle_leaf": {
    "active_stake": 9997717120,
    "stake_account": "DXmtAZdYsVZT8ir8uPkuY4cgBtsxWpZU4QKdpcAbFngo",
    "voting_wallet": "9w7BxC28QqDqCuKSPYVwDi1GeNvrXKhMKUuFzF2T3eUr"
  },
  "stake_merkle_proof": [
    "Gu8E91fBN2XeJECWpmxCH8gnx4zmsBor1ewWWGHyA375",
    "HAsYab37zUZDdT37CCS6mNx1Z93WkA9Sobs4i4cJ8H5u"
  ],
  "vote_account": "1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd"
}
```

### Check SQL Database

```bash
sqlite3 ./data/governance.db

# List tables
.tables

# List rows in snapshot_meta
select * from snapshot_meta limit 10;

# List rows in vote_accounts
select * from vote_accounts limit 10;

# List rows in stake_accounts
select * from stake_accounts limit 10;
```

### Health Check

```bash
curl -i http://localhost:3000/healthz
```

### Version

```bash
curl -i http://localhost:3000/version
```

## Dependencies

### Key Crates

- `axum` - Web framework
- `solana-sdk` - Solana blockchain SDK for signature verification
- `rusqlite` - SQLite database interface
- `serial_test` - Sequential test execution for environment variable isolation
- `anyhow` - Error handling

## Usage Notes

- If verifier service is used with CloudFlare proxy, there may be a origin timeout limit of 100s that will result in 504 errors for long running /upload requests. If modification on CloudFlare is not possible, it is recommended to bypass the proxy and use the public IP address of the instance.
