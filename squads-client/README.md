# squads-client

A minimal Rust client library for the [Squads V4](https://github.com/Squads-Protocol/v4) multisig program. Provides PDA helpers, account deserializers, instruction builders, and `TransactionMessage` compilation suitable for embedding inside a `vault_transaction_create` call — all without depending on `anchor-lang`.

## Why this crate exists

The official `squads-multisig` crate published on crates.io (v2.1.0) transitively pins `anchor-lang = "=0.29.0"` via its `squads-multisig-program` dependency. Codebases that use a different Anchor major version cannot consume that crate as-is — Cargo will refuse to unify the exact-pin `=0.29.0` with anything else. This crate is a hand-rolled minimal subset of the upstream client SDK with no Anchor dependency and a flat dependency graph.

## What it does

| Module | Responsibility |
|---|---|
| `id` | The mainnet/devnet program ID constant (`SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf`) and a helper for non-default deployments (e.g. Eclipse). |
| `pda` | Synchronous PDA derivation for `Multisig`, `vault`, `transaction`, `proposal`, `program_config`, `ephemeral_signer`, `spending_limit`. |
| `discriminator` | Computes Anchor-style 8-byte discriminators: `sha256("global:<snake_case>")[..8]` for instructions and `sha256("account:<PascalCase>")[..8]` for accounts. |
| `small_vec` | `SmallVec<L, T>` — a vector whose Borsh length prefix is `L` (`u8` or `u16`) instead of the default `u32`. Required by the on-chain `TransactionMessage` wire format. |
| `state` | Account structs (`Multisig`, `Member`, `Permission`, `Permissions`, `Proposal`, `ProposalStatus`, `VaultTransaction`) with manual Borsh ser/de + discriminator validation. |
| `message` | `TransactionMessage`, `CompiledInstruction`, `MessageAddressTableLookup`, and the `try_compile` helper that compresses a `&[Instruction]` into the wire format `vault_transaction_create` expects. |
| `instructions` | Builders for `vault_transaction_create`, `proposal_create`, `proposal_approve`. |
| `client` | Two layers: <br>• `SquadsClient` — synchronous, RPC-agnostic instruction builder. Caller supplies `Multisig::transaction_index` and the bound `multisig` pubkey on every call. <br>• `SquadsMultisigClient` (behind the default `rpc` feature) — async wrapper that owns an `Arc<RpcClient>` plus a multisig pubkey + vault index, hydrating the `Multisig` account on each builder call so callers don't have to. |

## Feature flags

| Flag | Default | Purpose |
|---|---|---|
| `rpc` | on | Pulls in `solana-rpc-client` and exposes `SquadsMultisigClient`. Disable with `default-features = false` if you'd rather use only the low-level `SquadsClient` against a custom RPC stack (e.g. the Jito fork). |

## Quick examples

### Low-level: `SquadsClient` (RPC-agnostic, sync)

```rust,no_run
use solana_program::pubkey::Pubkey;
use solana_program::instruction::Instruction;
use squads_client::{Multisig, SquadsClient};

// You fetch the `Multisig` account with your own RPC client.
let multisig_pubkey: Pubkey = "...".parse().unwrap();
let raw_account_data: Vec<u8> = todo!("rpc.get_account(&multisig_pubkey)?.data");
let multisig = Multisig::try_deserialize(&raw_account_data).unwrap();

let client = SquadsClient::new();
let proposer: Pubkey = Pubkey::default();
let inner_ixs: Vec<Instruction> = vec![/* instructions written from the vault's POV */];

let built = client
    .build_vault_tx_with_proposal(
        &multisig_pubkey,
        /* current_transaction_index */ multisig.transaction_index,
        /* vault_index */ 0,
        /* creator */ &proposer,
        /* rent_payer */ &proposer,
        &inner_ixs,
        &[],
        /* memo */ None,
    )
    .unwrap();
// `built.instructions` is [vault_transaction_create, proposal_create].
// Sign and send as a single Solana transaction signed by `proposer`.
```

### High-level: `SquadsMultisigClient` (RPC-backed, async)

```rust,no_run
use std::sync::Arc;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use squads_client::SquadsMultisigClient;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let rpc = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".into()));
let multisig: Pubkey = "...".parse()?;
let proposer: Pubkey = Pubkey::default();

// vault_index defaults to 0; override with `.with_vault_index(N)`.
let squads = SquadsMultisigClient::new(rpc, multisig);

let inner_ixs: Vec<Instruction> = vec![/* instructions from the vault's POV */];
let built = squads
    .build_vault_tx_with_proposal(
        /* creator */ &proposer,
        /* rent_payer */ &proposer,
        &inner_ixs,
        &[],
        /* memo */ None,
    )
    .await?;
// Multisig was fetched automatically; `built.transaction_index` is multisig.transaction_index + 1.
# Ok(())
# }
```

## Provenance and license

The `try_compile` routine, `CompiledKeys` helper, and `SmallVec` wire-format implementation are ports of code from [`Squads-Protocol/v4`](https://github.com/Squads-Protocol/v4) (MIT OR Apache-2.0). The upstream commit used as the reference is the version on `main` as of 2026-05-29; the on-chain program audited commit (`64af7330413d5c85cbbccfd8c27a05d45b6e666f`) defines the canonical wire formats.

This crate is also dual-licensed under MIT and Apache-2.0.
