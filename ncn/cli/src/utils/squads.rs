//! CLI-specific glue for routing transactions through a Squads multisig vault.
//!
//! This module owns ncn's transaction-orchestration policy:
//!
//! * the [`RoutedOutcome`] type returned by every Squads-aware `send_*` helper,
//! * [`route_via_squads`], the synchronous routing core that wraps user instructions in a
//!   Squads `vault_transaction_create` + `proposal_create` pair, retrying on
//!   transaction-index collisions,
//! * the [`effective_signer`] helper that swaps the local signer for the vault PDA when
//!   running in Squads mode.
//!
//! The ncn CLI uses the blocking Anchor client and owns a tokio runtime for snapshot
//! processing, so it deliberately avoids building on the shared async router (which would
//! require nesting runtimes) and instead drives the Squads flow against the blocking RPC
//! client directly.
//!
//! The Squads-compatibility decision ("can a vault PDA satisfy this command's on-chain
//! signer-identity check?") is **not** modeled here — it lives next to the clap
//! [`Commands`](crate::Commands) enum in `main.rs` (`squads_refusal_for`) and is enforced
//! up front before any handler runs. The router therefore trusts its caller; the routing
//! call is unconditional once the gate has passed.
//!

use anchor_client::{
    solana_sdk::{
        clock::Slot,
        instruction::Instruction,
        pubkey::Pubkey,
        signature::{Keypair, Signature, Signer},
        transaction::Transaction,
    },
    Program,
};
use anyhow::{anyhow, Result};
use squads_client::{vault_pda, Multisig, SquadsClient};

/// Configuration that pins a Squads vault transaction to a specific multisig + proposer.
///
/// Constructed once per CLI invocation (from `--squads*` flags + the loaded proposer
/// keypair) and threaded through to the routing function.
#[derive(Clone, Debug)]
pub struct SquadsRoutingConfig {
    /// Target multisig account.
    pub multisig: Pubkey,
    /// Vault index within the multisig (defaults to 0).
    pub vault_index: u8,
    /// Pubkey of the proposer initiating the vault transaction. Must be a member of the
    /// multisig with the `Initiate` permission.
    pub proposer: Pubkey,
    /// Optional non-canonical Squads program ID. `None` means the canonical address.
    pub program_id: Option<Pubkey>,
    /// Optional memo to attach to the vault transaction.
    pub memo: Option<String>,
}

/// Result of routing a logical command through either the direct or Squads path.
#[derive(Clone, Debug)]
pub enum RoutedOutcome {
    /// A direct, locally-signed transaction was submitted and confirmed.
    Direct {
        /// Signature of the confirmed transaction.
        signature: Signature,
        /// Slot the transaction landed in, if known. The router does not currently
        /// resolve this and leaves it `None`; the field exists for forward-compatibility.
        slot: Option<Slot>,
    },
    /// A Squads vault transaction + proposal pair was created against the multisig.
    Squads {
        /// The multisig the vault transaction was created against.
        multisig: Pubkey,
        /// The vault PDA that will execute the wrapped instructions at approval time.
        vault: Pubkey,
        /// Transaction index assigned to the new vault transaction.
        transaction_index: u64,
        /// PDA of the newly-created `VaultTransaction` account.
        vault_transaction_pda: Pubkey,
        /// PDA of the newly-created `Proposal` account.
        proposal_pda: Pubkey,
        /// Signature of the transaction that created the vault TX + proposal.
        creation_signature: Signature,
        /// Approval threshold of the multisig (the `m` in "m of n").
        threshold: u16,
        /// Total number of multisig members (the `n` in "m of n").
        total_members: usize,
        /// Canonical Squads web UI URL for the created transaction.
        web_url: String,
    },
}

impl RoutedOutcome {
    /// Renders the outcome as a human-readable, structured block for CLI output.
    pub fn format_structured(&self) -> String {
        match self {
            RoutedOutcome::Direct { signature, slot } => match slot {
                Some(slot) => format!(
                    "[Direct] Transaction confirmed.\n  {:<23}{}\n  {:<23}{}",
                    "signature:", signature, "slot:", slot
                ),
                None => format!(
                    "[Direct] Transaction confirmed.\n  {:<23}{}",
                    "signature:", signature
                ),
            },
            RoutedOutcome::Squads {
                multisig,
                vault,
                transaction_index,
                vault_transaction_pda,
                proposal_pda,
                creation_signature,
                threshold,
                total_members,
                web_url,
            } => {
                let mut out = String::from("[Squads] Vault transaction created.\n");
                out.push_str(&format!("  {:<23}{}\n", "multisig:", multisig));
                out.push_str(&format!("  {:<23}{}\n", "vault:", vault));
                out.push_str(&format!(
                    "  {:<23}{}\n",
                    "transaction_index:", transaction_index
                ));
                out.push_str(&format!(
                    "  {:<23}{}\n",
                    "vault_transaction_pda:", vault_transaction_pda
                ));
                out.push_str(&format!("  {:<23}{}\n", "proposal_pda:", proposal_pda));
                out.push_str(&format!(
                    "  {:<23}{}\n",
                    "creation_signature:", creation_signature
                ));
                out.push_str(&format!(
                    "  {:<23}{} of {}\n",
                    "threshold:", threshold, total_members
                ));
                out.push_str(&format!("  {:<23}{}", "url:", web_url));
                out
            }
        }
    }
}

/// Maximum number of attempts to claim a free transaction index before giving up on a
/// transaction-index collision.
const MAX_INDEX_ATTEMPTS: u8 = 3;

/// Raw `--squads*` flag values as parsed from the CLI, before the proposer keypair is
/// loaded. Converted into a [`SquadsRoutingConfig`] once the proposer pubkey is known.
#[derive(Clone, Debug)]
pub struct SquadsCliOpts {
    /// Target multisig account.
    pub multisig: Pubkey,
    /// Vault index within the multisig (defaults to 0).
    pub vault_index: u8,
    /// Optional non-canonical Squads program ID.
    pub program_id: Option<Pubkey>,
    /// Optional memo attached to the vault transaction.
    pub memo: Option<String>,
}

impl SquadsCliOpts {
    /// Builds the shared [`SquadsRoutingConfig`] now that the proposer pubkey is known.
    pub fn to_config(&self, proposer: Pubkey) -> SquadsRoutingConfig {
        SquadsRoutingConfig {
            multisig: self.multisig,
            vault_index: self.vault_index,
            proposer,
            program_id: self.program_id,
            memo: self.memo.clone(),
        }
    }
}

/// The vault PDA that signs the wrapped instructions at execution time.
pub fn vault_pubkey(config: &SquadsRoutingConfig) -> Pubkey {
    vault_pda(
        &config.multisig,
        config.vault_index,
        config.program_id.as_ref(),
    )
    .0
}

/// Resolves the pubkey that should occupy the signer/authority slot of an instruction.
///
/// In direct mode (`squads` is `None`) this is the local keypair's pubkey. When routing
/// through a Squads vault, the wrapped instruction is signed by the vault PDA at execution
/// time, so the authority slot must hold the vault PDA instead.
pub fn effective_signer(squads: Option<&SquadsRoutingConfig>, local: Pubkey) -> Pubkey {
    squads.map(vault_pubkey).unwrap_or(local)
}

/// Builds the Squads `vault_transaction_create` + `proposal_create` pair wrapping
/// `vault_ixs` and submits it on behalf of the proposer, retrying on transaction-index
/// collisions. Runs synchronously against the blocking Anchor RPC client.
///
/// No compute-budget instruction is injected into the wrapped vault message: the compute
/// budget that governs execution is set on the outer `vault_transaction_execute`
/// transaction (typically by the Squads UI at approval time), so a
/// `set_compute_unit_limit` packaged inside the wrapped message would have no effect.
pub fn route_via_squads(
    program: &Program<&Keypair>,
    vault_ixs: Vec<Instruction>,
    proposer: &Keypair,
    config: &SquadsRoutingConfig,
) -> Result<RoutedOutcome> {
    let squads = match config.program_id {
        Some(program_id) => SquadsClient::with_program_id(program_id),
        None => SquadsClient::new(),
    };

    let rpc = program.rpc();
    let mut attempt: u8 = 0;
    loop {
        attempt += 1;

        let multisig_data = rpc
            .get_account_data(&config.multisig)
            .map_err(|err| anyhow!("failed to fetch multisig {}: {}", config.multisig, err))?;
        let multisig =
            Multisig::try_deserialize(&multisig_data).map_err(|err| anyhow!(err.to_string()))?;

        squads
            .verify_proposer(&config.multisig, &multisig, &config.proposer)
            .map_err(|err| anyhow!(err.to_string()))?;

        let built = squads
            .build_vault_tx_with_proposal(
                &config.multisig,
                multisig.transaction_index,
                config.vault_index,
                &config.proposer,
                &config.proposer,
                &vault_ixs,
                &[],
                config.memo.clone(),
            )
            .map_err(|err| anyhow!(err.to_string()))?;

        let blockhash = rpc.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &built.instructions,
            Some(&proposer.pubkey()),
            &[proposer],
            blockhash,
        );

        match rpc.send_and_confirm_transaction(&transaction) {
            Ok(creation_signature) => {
                let (vault, _) = squads.pda_vault(&config.multisig, config.vault_index);
                return Ok(RoutedOutcome::Squads {
                    multisig: config.multisig,
                    vault,
                    transaction_index: built.transaction_index,
                    vault_transaction_pda: built.transaction,
                    proposal_pda: built.proposal,
                    creation_signature,
                    threshold: multisig.threshold,
                    total_members: multisig.members.len(),
                    web_url: format!(
                        "https://app.squads.so/squads/{}/transactions/{}",
                        config.multisig, built.transaction_index
                    ),
                });
            }
            Err(err) => {
                let reason = err.to_string();
                if reason.contains("already in use") && attempt < MAX_INDEX_ATTEMPTS {
                    // Re-fetch the multisig (its transaction_index will have advanced) and
                    // retry with the next free index.
                    continue;
                }
                return Err(anyhow!(
                    "failed to submit Squads vault transaction: {}",
                    reason
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config(multisig: Pubkey) -> SquadsRoutingConfig {
        SquadsRoutingConfig {
            multisig,
            vault_index: 0,
            proposer: Pubkey::new_unique(),
            program_id: None,
            memo: None,
        }
    }

    #[test]
    fn effective_signer_substitutes_vault_pda_only_in_squads_mode() {
        let local = Pubkey::new_unique();
        let config = sample_config(Pubkey::new_unique());

        // Direct mode keeps the local signer.
        assert_eq!(effective_signer(None, local), local);

        // Squads mode swaps in the vault PDA (and never the local key).
        let vault = effective_signer(Some(&config), local);
        assert_eq!(vault, vault_pubkey(&config));
        assert_ne!(vault, local);
    }
}
