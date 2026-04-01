# NCN Operator Quickstart Guide

End-to-end walkthrough for new operators: from whitelisting to casting your first governance vote.

[← Back to Project README](README.md)

---

## Prerequisites

Before starting, ensure you have:

1. **Operator keypair** — your Solana keypair used for voting
2. **Whitelisted status** — your operator pubkey must be added to the program's operator whitelist by the admin
3. **Rust 1.89.0** — required for dependency compatibility with the jito-tip-router crate chain
   ```bash
   rustup toolchain install 1.89.0
   rustup default 1.89.0
   rustc --version  # verify: rustc 1.89.0
   ```
4. **jito-tip-router dependency** — clone the exo-tech-xyz fork (required for NCN snapshot integration):
   ```bash
   git clone https://github.com/exo-tech-xyz/jito-tip-router.git ../jito-tip-router
   cd ../jito-tip-router && git checkout ncn-snapshot && cd ../ncn-snapshot
   ```
5. **Running validator** with access to ledger data at a known path (e.g., `/mnt/ledger`)

---

## Overview: The Voting Flow

```
1. Target slot announced
        ↓
2. Wait for slot & generate snapshot (await-snapshot)
        ↓
3. Log merkle root + hash from snapshot
        ↓
4. Cast vote on-chain (cast-vote-from-snapshot)
        ↓
5. Verify on-chain state (log ballot-box)
        ↓
6. [If consensus reached] Finalize ballot
```

---

## Step 1: Wait for Target Slot Announcement

The governance admin announces a target snapshot slot. This is the slot at which validator stake will be measured for the governance vote.

**Your action:** Note the target slot number. You'll use this in all subsequent commands.

---

## Step 2: Generate Snapshot (Recommended: await-snapshot)

The `await-snapshot` command is the recommended approach — it watches for the target slot, backs up necessary files, and generates the snapshot automatically.

```bash
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=$(nproc) ZSTD_NBTHREADS=$(nproc) \
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn \
cargo run --release --bin cli -- \
  await-snapshot \
  --scan-interval 1 \
  --slot <TARGET_SLOT> \
  --snapshots-dir /mnt/ledger/snapshots \
  --backup-snapshots-dir /mnt/ledger/gov-backup-snapshots \
  --backup-ledger-dir /mnt/ledger/gov-ledger-backup \
  --agave-ledger-tool-path /home/sol/agave/target/release/agave-ledger-tool \
  --ledger-path /mnt/ledger \
  --generate-meta-merkle
```

**Key flags:**
- `--scan-interval 1` — check every second for the target slot
- `--generate-meta-merkle` — automatically generate the MetaMerkleSnapshot after the ledger snapshot is created
- `--backup-snapshots-dir` / `--backup-ledger-dir` — preserve files for manual recovery if needed

**Output:** A file named `meta_merkle-<SLOT>.zip` in your current directory.

---

## Step 3: Log Merkle Root and Hash

Extract the merkle root and snapshot hash from your generated snapshot:

```bash
RUST_LOG=info cargo run --bin cli -- \
  --authority-path ~/.config/solana/id.json \
  log-meta-merkle-hash \
  --read-path ./meta_merkle-<TARGET_SLOT>.zip \
  --is-compressed
```

**Save the output** — you'll need the `root` and `hash` values for voting.

---

## Step 4: Cast Your Vote

**Option A — Vote from snapshot file (recommended):**
```bash
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url <YOUR_RPC_URL> \
  cast-vote-from-snapshot \
  --snapshot-slot <TARGET_SLOT> \
  --read-path ./meta_merkle-<TARGET_SLOT>.zip
```

**Option B — Vote with root + hash directly:**
```bash
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url <YOUR_RPC_URL> \
  cast-vote \
  --snapshot-slot <TARGET_SLOT> \
  --root <MERKLE_ROOT> \
  --hash <SNAPSHOT_HASH>
```

**Note:** Replace `<YOUR_RPC_URL>` with your mainnet RPC endpoint.

---

## Step 5: Verify Your Vote On-Chain

Check the BallotBox to confirm your vote was recorded:

```bash
RUST_LOG=info cargo run --bin cli -- \
  --rpc-url <YOUR_RPC_URL> \
  log --ty ballot-box \
  --snapshot-slot <TARGET_SLOT>
```

---

## Step 6: Finalization

Once consensus is reached (enough operators voted for the same ballot):

```bash
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url <YOUR_RPC_URL> \
  finalize-ballot \
  --snapshot-slot <TARGET_SLOT>
```

Any signer can finalize — it just creates the `ConsensusResult` account on-chain.

---

## Removing a Vote

If you need to change your vote before consensus is reached:

```bash
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  --rpc-url <YOUR_RPC_URL> \
  remove-vote \
  --snapshot-slot <TARGET_SLOT>
```

Then re-cast with the corrected snapshot.

---

## Troubleshooting

See the [main README Troubleshooting section](README.md#troubleshooting) for common issues including:
- Missing incremental snapshots
- Snapshot bank verification errors
- Genesis creation time mismatches
- Dependency version conflicts

---

## Glossary

| Term | Description |
|------|-------------|
| **MetaMerkleSnapshot** | Top-level snapshot file containing the merkle root, slot, and all validator leaf bundles |
| **MetaMerkleLeaf** | A validator's node in the merkle tree — contains the stake sub-root and total stake |
| **StakeMerkleLeaf** | An individual stake account leaf — contains voting wallet, stake pubkey, and delegated amount |
| **BallotBox** | On-chain account storing voting state for a specific snapshot slot (1:1 mapping) |
| **ConsensusResult** | On-chain account storing the finalized merkle root and hash after consensus |
| **MetaMerkleProof** | On-chain proof data for verifying a single validator's stake in a snapshot |
| **Operator** | A whitelisted entity authorized to vote on governance snapshots |

---

## Further Reading

- [Program Design & Constraints](programs/ncn-snapshot/README.md) — account types, instruction set, design decisions
- [Verifier Service](verifier-service/README.md) — API for serving merkle proofs
- [Verifier Deployment Guide](verifier-service/DEPLOYMENT.md) — running your own verifier service
