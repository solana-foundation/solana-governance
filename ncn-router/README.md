# ncn-route

`ncn-route` is a small Rust utility that monitors NCN verifier metadata off-chain and cross-checks it against the Solana NCN program on-chain. It runs an in-process cron job that regularly fetches `/meta?network=mainnet|testnet` from configured verifiers and produces JSON whitelist snapshots for use by frontends or other services.

### Binaries

- **`ncn-meta-cron`**: Periodically fetches verifier metadata and compares it to on-chain `BallotBox` state, writing logs and whitelist JSON snapshots.
- **`ncn-router`**: HTTP router binary (see `src/router.rs`) that can serve or forward NCN-related data.

### Configuration

- **`config.toml`**: List of verifier names and `verification_domain` URLs.
- **Env vars**:
  - `NCN_CONFIG` (optional): Path to config file (default: `config.toml`)
  - `NCN_LOG` (optional): Path to log file (default: `ncn_verifier_meta.log`)
  - `SOLANA_RPC_URL` (optional): Solana RPC endpoint (defaults to mainnet/testnet public RPCs)
  - `NCN_PROGRAM_ID` (required): NCN program ID on Solana
  - `NCN_WHITELIST_MAINNET_PATH` / `NCN_WHITELIST_TESTNET_PATH` (optional): Output paths for whitelist JSON (defaults to `ncn_whitelist.mainnet.json` / `ncn_whitelist.testnet.json`)

### Usage

Run the cron worker (this runs forever and schedules the job every 2 hours):

```bash
cargo run --bin ncn-meta-cron -- --network mainnet
# or
cargo run --bin ncn-meta-cron -- --network testnet
# or (both networks)
cargo run --bin ncn-meta-cron

cargo run -r --bin ncn-router # runs HTTP router for NCN data