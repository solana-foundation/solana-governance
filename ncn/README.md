# Solana Governance Voter Snapshot

This repo contains:

- `cli/`: A command-line tool for Operators to generate stake snapshots and vote on-chain.
- `programs/ncn-snapshot/`: The Anchor-based on-chain program used to coordinate Operator voting and finalize snapshot consensus.

[→ Governance Voter Snapshot Program Design](programs/ncn-snapshot/README.md)
[→ Verifier Service README](verifier-service/README.md)
[→ Verifier Service Deployment](verifier-service/DEPLOYMENT.md)

---

## Table of Contents

- [Project Structure](#project-structure)
- [Stake Pool Handling](#stake-pool-handling)
- [Vote Account](#vote-account)
  - [Stake Calculation](#stake-calculation)
  - [Missing Vote Account](#missing-vote-account)
- [Testing](#testing)
- [CLI Usage](#cli-usage-via-cargo-run)
  - [Program Setup](#program-setup-after-deployment)
  - [Snapshot Handling](#snapshot-handling)
  - [Log On-Chain State](#log-on-chain-state)
  - [Voting Flow](#voting-flow)
  - [Finalization & Tie-Breaking](#finalization--tie-breaking)
- [Troubleshooting](#troubleshooting)
- [Additional Testing Commands](#additional-testing-commands)

---

## Project Structure

```
.
├── cli/                  # CLI tool for snapshot ops & voting
├── programs/
    └── ncn-snapshot/           # On-chain governance snapshot program
└── tests/                # Anchor program integration tests
```

---

## Stake Pool Handling

The governance snapshot system handles stake accounts delegated by stake pools by changing the voting wallet from withdraw authorities (typically PDAs) to appropriate voting wallets to enable stake pool operators to participate in governance on behalf of their delegated stake.

### SPL Stake Pool Program

For stake accounts delegated through the **SPL Stake Pool program**, the system changes the voter from the withdraw authority (which is a PDA) to the **manager authority**.

### Marinade Liquid Staking Program

For stake accounts delegated through the **Marinade Liquid Staking Program**, the system changes the voter from the withdraw authority (which is a PDA) to the **operations wallet authority** (`opLSF7LdfyWNBby5o6FT8UFsr2A4UGKteECgtLSYrSm`).

### Sanctum Pools

For **Sanctum Pools**, since stake is either delegated to a single validator per LST or distributed with majority stake to a few validators, the LST operators are already able to vote through the validator itself. Therefore, the system keeps the voter for stake accounts as the withdraw authority (which will not be able to vote since it's a PDA). This approach recognizes that Sanctum's model already provides governance participation through validator-level voting.

### Individual Stake Accounts

For individual stake accounts not managed by any stake pool program, the system uses the withdraw authority directly as the voting wallet, allowing individual stakers to participate in governance.

---

## Vote Account

### Stake Calculation

Vote account effective stake is calculated by summing the individual active stake accounts delegated to the vote account that reads from the Bank's StakesCache. This bottom up approach differs from using the value record in Bank's `epoch_stakes` computed at epoch boundary.

### Missing Vote Account

If a vote account delegated to is missing (closed by the manager), the system will set the voting wallet to the default address `11111111111111111111111111111111`. This implies that the delegators can continue to vote, but the vote account will not be able to vote.

---

## Dependencies

1. Clone `jito-tip-router` from the **exo-tech-xyz** fork to parent directory and switch to the `ncn-snapshot` branch:

   ```bash
   git clone https://github.com/exo-tech-xyz/jito-tip-router.git ../jito-tip-router
   cd ../jito-tip-router
   git checkout ncn-snapshot
   cd ../ncn-snapshot
   ```

2. Ensure system is using Rust Version `1.89.0`, otherwise install with:

```bash
rustup toolchain install 1.89.0 // install
rustup default 1.89.0 // set as default
rustc --version // verify version
```

3. (Optional - when using Anchor CLI) Install Solana CLI version 3.0 or higher. The bundled rustc in older Solana CLI versions may not be compatible with some dependencies.

4. Build repo with `cargo build`

---

## Testing

Anchor tests can be executed directly from the root directory with:

```bash
anchor test -- --features skip-pda-check
```

Note that setup of environment variables is required (see [Dependencies](#dependencies)). For details about building the program with the `skip-pda-check` feature for local testing, see the [Program README](programs/ncn-snapshot/README.md#7-cross-program-invocation-cpi-and-testing).

---

## CLI Usage (via cargo run)

All commands assume:

- You're running from project root using `RUST_LOG=info cargo run --bin cli -- ...`
  - `--payer-path` signs transactions
  - `--authority-path` signs Operator votes
- Replace `~/.config/solana/id.json` with path to keypair file
- Replace `key1,key2,key3...` with actual base58-encode pubkeys

Use `RUST_LOG=info` to enable logs.

**Note:** Environment variables (`RESTAKING_PROGRAM_ID`, `VAULT_PROGRAM_ID`, `TIP_ROUTER_PROGRAM_ID`) are configured in `.cargo/config.toml` and will be automatically loaded by Cargo during builds. No manual export is required.

---

### Program Setup and Configuration

```bash
# Initialize ProgramConfig global singleton on-chain
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com \
  init-program-config

# Add or remove operators from whitelist
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com \
  update-operator-whitelist -a key1,key2,key3 -r key4,key5

# Update config (all arguments are optional):
# threshold, vote duration, tie-breaker-admin, proposed authority (two-step)
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com \
  update-program-config \
  --min-consensus-threshold-bps 6000 \
  --vote-duration 180 \
  --tie-breaker-admin <NEW_TIE_BREAKER_ADMIN_PUBKEY> \
  --proposed-authority <NEW_ADMIN_PUBKEY>

# Finalize proposed authority (run as the proposed authority)
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path <PATH_TO_PROPOSED_AUTHORITY_KEYPAIR> \
  --rpc-url https://api.devnet.solana.com \
  finalize-proposed-authority
```

---

### Snapshot Generation and Handling

Environment variables affecting snapshot IO:

- `NCN_SNAPSHOT_MAX_MB` (optional): maximum allowed decompressed snapshot size (in MiB) enforced by the CLI bounded decompressor when reading gzip files or raw files. Default is 256. Increase if your snapshots legitimately exceed this size.

```bash
# Generates a Solana ledger snapshot for a specific slot (from validator bank state)
# and stores at `backup-snapshots-dir`.
# Increase file descriptor limit to with `ulimit -n 1000000` if needed,
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=$(nproc) ZSTD_NBTHREADS=$(nproc) \
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn \
cargo run --release --bin cli -- \
  --ledger-path /mnt/ledger \
  --full-snapshots-path /mnt/ledger/snapshots \
  --backup-snapshots-dir /mnt/ledger/snapshots \
  snapshot-slot --slot 368478463

# (RELEASE MODE - Linux)
# Generates MetaMerkleSnapshot from the Solana ledger snapshot using release mode and tmp storage config (linux)
# Create a tmp directory for `TMPDIR` and `account-paths` for storing intermediary files.
# Output snapshot is stored in current directory by default.
TMPDIR=/mnt/ledger/gov-tmp \
RUSTFLAGS="-C target-cpu=native" \
RAYON_NUM_THREADS=$(nproc) ZSTD_NBTHREADS=$(nproc) \
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn \
cargo run --release --bin cli -- \
  --ledger-path /mnt/ledger \
  --account-paths /mnt/ledger/gov-tmp/accounts \
  --backup-snapshots-dir /mnt/ledger/backup \
  generate-meta-merkle --slot 361319354

# (RELEASE MODE - MacOS)
TMPDIR=/tmp \
RUSTFLAGS="-C target-cpu=native" \
RAYON_NUM_THREADS=$(sysctl -n hw.ncpu) ZSTD_NBTHREADS=$(sysctl -n hw.ncpu) \
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn \
cargo run --release --bin cli -- \
  --ledger-path test-ledger \
  --backup-snapshots-dir test-ledger/backup-snapshots \
  generate-meta-merkle --slot 340850340

# Log Merkle root, hash,' and operator signature from snapshot file
RUST_LOG=info cargo run --bin cli -- --authority-path ~/.config/solana/id.json log-meta-merkle-hash  --read-path ./meta_merkle-367628001.zip --is-compressed
```

#### Await Snapshot (RECOMMENDED)

Waits until the target slot is passed (by observing on-disk snapshots on specified interval), backs up the full snapshot, incremental snapshot and ledger into specified directories, and initiates snapshot creation for that slot. Optionally, it can also generate a MetaMerkle snapshot once the full snapshot is created.

This is recommended as 1) no slot watching and manual invocation is required when target slot has passed, 2) ledger replay is kept to a minimum, and 3) allows manual snapshot generation from backup files on failure.

Example:

```bash
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=$(nproc) ZSTD_NBTHREADS=$(nproc) \
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn \
cargo run --release --bin cli -- \
  await-snapshot \
  --scan-interval 1 \
  --slot 368478463 \
  --snapshots-dir /mnt/ledger/snapshots \
  --backup-snapshots-dir /mnt/ledger/gov-backup-snapshots \
  --backup-ledger-dir /mnt/ledger/gov-ledger-backup \
  --agave-ledger-tool-path /home/jito/agave/target/release/agave-ledger-tool \
  --ledger-path /mnt/ledger \
  --generate-meta-merkle   # optional
```

### Log On-Chain State

````bash
# Log ProgramConfig
RUST_LOG=info cargo run --bin cli -- \
  --rpc-url https://api.devnet.solana.com log \
  --ty program-config

# Log BallotBox (by snapshot_slot)
RUST_LOG=info cargo run --bin cli -- \
  --rpc-url https://api.devnet.solana.com log \
  --ty ballot-box --snapshot-slot <SLOT>

# Log ConsensusResult (by snapshot_slot)
RUST_LOG=info cargo run --bin cli -- \
  --rpc-url https://api.devnet.solana.com log \
  --ty consensus-result --snapshot-slot <SLOT>

---

### Voting Flow

```bash
# Vote with root + hash
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com \
  cast-vote --snapshot-slot <SLOT> \
  --root ByVtRpEnLyD1eVS8Bq21VvDnMffsqPAypaMT9KMZCZcJ \
  --hash 4seYTnZyZNby5ZQTy8ajAapDiMgUYrvYx4hzYRXVn4zH

# Vote using a snapshot file
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com \
  cast-vote-from-snapshot --snapshot-slot <SLOT> \
  --read-path ./meta_merkle-340850340.zip

# Remove vote (before consensus and voting expiry)
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com \
  remove-vote --snapshot-slot <SLOT>
````

---

### Finalization & Tie-Breaking

```bash
# Finalize winning ballot (after consensus)
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com \
  finalize-ballot --snapshot-slot <SLOT>

# Set tie-breaking result if consensus was not reached
# Note: Can set any ballot value, not limited to existing ballots
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com \
  set-tie-breaker --snapshot-slot <SLOT> \
  --root <MERKLE_ROOT> --hash <SNAPSHOT_HASH>

# Reset ballot box if bricked (before expiry, consensus not reached, tallies at max)
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com \
  reset-ballot-box --snapshot-slot <SLOT>
```

---

## Troubleshooting

### Dependency Version Conflicts When Upgrading jito-tip-router

When upgrading the `jito-tip-router` dependency to a newer version, you may encounter dependency version conflicts. If you see compilation errors related to version mismatches, run these commands:

- **Update Cargo.lock to force `solana-sysvar` version 3.0.0**: Some dependencies may pull in `solana-sysvar` version 3.1.1, which can cause compilation errors due to missing serde trait implementations. To force the use of version 3.0.0, run:

  ```bash
  cargo update -p solana-sysvar:3.1.1 --precise 3.0.0
  ```

- **Update Cargo.lock to force `solana-epoch-rewards-hasher` version 3.0.0**: Some dependencies may pull in `solana-epoch-rewards-hasher` version 3.1.0, which requires `solana-hash` version 4.0.1. However, jito-solana dependencies use `solana-hash` version 3.0.0, causing type mismatch errors. To force the use of version 3.0.0, run:

  ```bash
  cargo update -p solana-epoch-rewards-hasher:3.1.0 --precise 3.0.0
  ```

Note: Do not manually edit `Cargo.lock` - always use `cargo update` to modify dependency versions.

### Missing Incrementatal Snapshot

If you encounter an error similar to:

```
Failed to open snapshot archive '/mnt/ledger/snapshots/incremental-snapshot-368528476-368534392-AociwZMrWXr48RYipTcnZ3tZKE6ypzd1Wocms1PgWn5M.tar.zst': No such file or directory (os error 2)
```

**Solution:** Increase retention period of incremental snapshots or use an empty `backup-snapshots-dir` so full snapshot replay is enforced, or copy incremental snapshots to a new directory.

### Snapshot Bank Verification Error

If you encounter an error similar to:

```
Snapshot bank for slot 340850340 failed to verify
```

**Solution:** Comment out the line causing the `panic` invocation in the `jito-solana` dependency crate. Snapshot verification failure does not impede generation of a merkle tree snapshot from the source file.

### Genesis Creation Time Mismatch

If you encounter an error such as:

```
Bank snapshot genesis creation time does not match genesis.bin creation time
```

**Solution:** Comment out the `assert_eq` statement in the `jito-solana` dependency crate. Genesis mismatch could occur when the snapshot is retrieved from a different RPC, but does not impede merkle generation.

---

## Additional Testing Commands

### To get genesis config:

1. Create test keypairs:

```
solana-keygen new -o stake-keypair.json
solana-keygen new -o identity-keypair.json
solana-keygen new -o vote-keypair.json
```

2. Extract

```
IDENTITY=$(solana-keygen pubkey identity-keypair.json)
VOTE=$(solana-keygen pubkey vote-keypair.json)
STAKE=$(solana-keygen pubkey stake-keypair.json)
```

3. Get genesis config.

```
solana-genesis   --bootstrap-validator "$IDENTITY" "$VOTE" "$STAKE"   --ledger tmp/testnet-ledger/ --
faucet-lamports 100000000000 -u testnet --cluster-type testnet
```

### To test snapshotting with localnet:

1. Start validator with

```
solana-test-validator
```

2. Run CLI for generating ledger snapshot for a slot (e.g. 100)

```
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn cargo run --bin cli -- --ledger-path test-ledger --full-snapshots-path test-ledger/backup-snapshots --backup-snapshots-dir test-ledger/backup-snapshots snapshot-slot --slot 100
```

3. Run CLI for generating the MeteMerkleSnapshot from the ledger snapshot

```
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn cargo run --bin cli -- --ledger-path test-ledger --full-snapshots-path test-ledger/backup-snapshots --backup-snapshots-dir test-ledger/backup-snapshots generate-meta-merkle --slot 340850340
```

### To generate MetaMerkleSnapshot from testnet snapshots.

1. Find a testnet node with

```
solana gossip -u testnet
```

2. Download snapshot and genesis config from the testnet

```
wget --trust-server-names http://38.147.105.98:8899/snapshot.tar.bz2

wget http://160.202.131.117:8899/genesis.tar.bz2
```

3. Extract snapshot and genesis

```
tar -xf snapshot.tar.bz2 -C test-ledger/
tar -xf genesis.tar.bz2 -C test-ledger/
```

4. Move snapshot to `test-ledger/backup-snapshots/`.
5. Clear temp files from `test-ledger` directory after generating.

```
find test-ledger -mindepth 1 -maxdepth 1 \
  ! -name 'backup-snapshots' \
  ! -name 'rocksdb' \
  ! -name 'genesis.bin' \
  -exec rm -rf {} +
```

### Testing Snapshot Generation

```bash
# (DEV MODE - Use Release Mode for production snapshots)
# Generates MetaMerkleSnapshot from the Solana ledger snapshot and stores at save path.
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn cargo run --bin cli -- --ledger-path test-ledger --full-snapshots-path test-ledger/backup-snapshots --backup-snapshots-dir test-ledger/backup-snapshots generate-meta-merkle --slot 340850340 --save-path ./
```
