# ncn-route

`ncn-route` is a small Rust utility that monitors NCN verifier metadata off-chain and cross-checks it against the Solana NCN program on-chain. It runs an in-process cron job that regularly fetches `/meta?network=mainnet|testnet` from configured verifiers and produces JSON whitelist snapshots for use by frontends or other services.

### Binaries

- **`ncn-meta-cron`**: Periodically fetches verifier metadata and compares it to on-chain `BallotBox` state, writing logs and whitelist JSON snapshots.
- **`ncn-router`**: HTTP router binary (see `src/router.rs`) that can serve or forward NCN-related data.

### Configuration

- **`config.toml`**: List of verifier names and `verification_domain` URLs.
  Each `verification_domain` should be the verifier base URL; the cron worker appends
  `/meta?network=mainnet|testnet` when polling.
  The public production router is `https://ncn-governance.solana.com`.

### Default verifier list

| Name | Verification domain |
|---|---|
| Ha1iad3 | `https://ncn.ha1iad3.com/` |
| lantern | `https://gov.lantern.one/` |
| Titan Analytics | `https://verifier.titananalytics.io` |
| Adra finance | `https://solgov.com` |
| Blocksize | `https://verifier.nops.blocksize.dev/` |
| Digital Energy | `https://ncn-verifier.digital-energy.io` |
| stakeware.xyz | `https://ncn.stakeware.xyz:3000/` |
| Prompt Logic | `https://verifier.promptlogic.systems` |
| Exo Tech | `http://ncn-verifier.exotechnologies.xyz:3000` |
| Chainflow | `https://ncn-verifier.chainflow.io` |
| Brewlabs | `https://ncn.brewlabs.so` |
- **Env vars**:
  - `NCN_CONFIG` (optional): Path to config file (default: `config.toml`)
  - `NCN_LOG` (optional): Path to log file (default: `ncn_verifier_meta.log`)
  - `SOLANA_RPC_URL` (optional): Solana RPC endpoint (defaults to mainnet/testnet public RPCs)
  - `NCN_PROGRAM_ID` (required): NCN program ID on Solana
  - `NCN_WHITELIST_MAINNET_PATH` / `NCN_WHITELIST_TESTNET_PATH` (optional): Output paths for whitelist JSON (defaults to `ncn_whitelist.mainnet.json` / `ncn_whitelist.testnet.json`)
  - `NCN_MAX_VERIFIER_SLOT_LAG` (optional): How many slots a verifier's snapshot may trail the freshest whitelisted verifier before it is marked `stale` and dropped from routing (default: `432000`, one epoch / ~2 days). A verifier that stopped uploading still matches the on-chain ballot for its own old slot, so it stays `ok` without this bound and 404s every current proof request.
- **Statuses** written to the whitelist file:
  - `ok`: routable.
  - `mismatch`: the root or the hash disagrees with the ballot the chain agreed on.
  - `stale`: too far behind the freshest verifier, see above.
  - `error`: the verifier or the RPC could not be reached.
  - `pending`: not routable, explained below.

  A verifier goes `pending` when its newest snapshot sits in a ballot box that
  operators have not voted through yet. That is normal for a few hours, because
  operators upload a snapshot before they vote on it. When that happens the
  cron asks the verifier for the newest snapshot that *was* voted through and
  checks it against that one instead, so a verifier that still serves it stays
  `ok` and routing keeps working for the proposal being voted on. The status is
  `pending` only when that check is not possible: the verifier has pruned the
  older snapshot, or it runs a verifier service older than 0.6, which ignores
  `/meta?slot=` and always answers with its newest snapshot.

  The voted-through slot normally comes from the verifiers' own reports. Once
  every verifier has moved on to the newer, unvoted slot, it comes from the
  `slot` field the previous run wrote to the whitelist file instead, so keep
  that file between runs.

### Usage

Run the cron worker (this runs forever and schedules the job every 2 hours):

```bash
cargo run --bin ncn-meta-cron -- --network mainnet
# or
cargo run --bin ncn-meta-cron -- --network testnet
# or (both networks)
cargo run --bin ncn-meta-cron

cargo run -r --bin ncn-router # runs HTTP router for NCN data
