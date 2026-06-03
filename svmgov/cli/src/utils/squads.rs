//! CLI-specific routing for transactions that may optionally pass through a Squads
//! multisig vault.
//!
//! This module owns svmgov's transaction-orchestration policy:
//!
//! * the routing core ([`route_or_send`]) that branches between direct-mode signing and
//!   Squads-mode `vault_transaction_create` + `proposal_create` wrapping, including:
//!   - handling preflight instructions (e.g. `InitMetaMerkleProof`) as a separate
//!     locally-signed transaction when running in Squads mode,
//!   - retrying on transaction-index collisions when a concurrent proposal claims the
//!     same index,
//! * the structured output ([`RoutedOutcome::format_structured`]) printed by every
//!   subcommand handler.
//!
//! The Squads-compatibility decision ("can a vault PDA satisfy this command's on-chain
//! signer-identity check?") is **not** modeled here — it lives next to the clap
//! [`Commands`](crate::Commands) enum in `main.rs` (`squads_refusal_for`) and is enforced
//! up front before any handler runs. The router therefore trusts its caller; the routing
//! call is unconditional once the gate has passed.

use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_sdk::{
    clock::Slot,
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use anyhow::Result;
use async_trait::async_trait;
use squads_client::{Multisig, SquadsClient, SquadsError};

/// Maximum number of attempts the Squads path will make to claim a free transaction
/// index before surfacing [`SquadsError::TransactionIndexRace`].
const MAX_INDEX_ATTEMPTS: u8 = 3;

// ============================================================================
// CLI flag parsing + effective-signer helper
// ============================================================================

/// Raw `--squads*` flag values as parsed from the CLI, before the signer (proposer) is
/// known. Converted into a [`SquadsRoutingConfig`] once the keypair has been loaded.
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
    /// Builds the [`SquadsRoutingConfig`] now that the proposer pubkey is known.
    pub fn to_config(&self, proposer: Pubkey) -> SquadsRoutingConfig {
        SquadsRoutingConfig {
            multisig: self.multisig,
            vault_index: self.vault_index,
            proposer,
            program_id: self.program_id,
            memo: self.memo.clone(),
        }
    }

    /// The vault PDA that signs the wrapped instructions at execution time. When routing
    /// through Squads, this is the pubkey that must occupy any signer/authority slot inside
    /// the wrapped instruction (the vault provides its signature via CPI).
    pub fn vault_pubkey(&self) -> Pubkey {
        squads_client::vault_pda(&self.multisig, self.vault_index, self.program_id.as_ref()).0
    }
}

/// Resolves the pubkey that should occupy the signer/authority slot of an instruction.
///
/// In direct mode (`squads` is `None`) this is the local keypair's pubkey. When routing
/// through a Squads vault, the wrapped instruction is signed by the vault PDA at execution
/// time, so the authority slot must hold the vault PDA instead.
pub fn effective_signer(squads: Option<&SquadsCliOpts>, local: Pubkey) -> Pubkey {
    squads.map(|opts| opts.vault_pubkey()).unwrap_or(local)
}

// ============================================================================
// Routing config + outcome
// ============================================================================

/// Configuration describing which Squads multisig (and proposer) to route through.
///
/// Construct this only when the user has opted into Squads mode (e.g. via a `--squads`
/// flag). When `None` is passed to [`route_or_send`], the router stays in direct mode.
#[derive(Clone, Debug)]
pub struct SquadsRoutingConfig {
    /// The multisig account the vault transaction is created against.
    pub multisig: Pubkey,
    /// Index of the vault (within the multisig) that will execute the wrapped
    /// instructions at approval time. Defaults to `0` in most deployments.
    pub vault_index: u8,
    /// The member proposing (and paying rent/fees for) the vault transaction. Must be a
    /// multisig member holding the `Initiate` permission. Also used as the local
    /// signer/fee-payer for any preflight transaction.
    pub proposer: Pubkey,
    /// Optional non-canonical Squads program ID override.
    pub program_id: Option<Pubkey>,
    /// Optional indexer memo attached to the vault transaction.
    pub memo: Option<String>,
}

/// The result of [`route_or_send`].
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

// ============================================================================
// RouterRpc trait + blanket impl
// ============================================================================

/// Minimal RPC surface the router needs. Abstracted into a trait so the routing logic can
/// be unit-tested against an in-memory mock without a live validator.
///
/// A blanket implementation is provided for
/// `anchor_client::solana_client::nonblocking::rpc_client::RpcClient`, so production
/// callers can pass a real RPC client directly.
#[async_trait]
pub trait RouterRpc {
    /// Fetches the raw account data for `pubkey`.
    async fn fetch_account_data(&self, pubkey: &Pubkey) -> Result<Vec<u8>, SquadsError>;

    /// Fetches a recent blockhash to sign transactions against.
    async fn recent_blockhash(&self) -> Result<Hash, SquadsError>;

    /// Submits a fully-signed transaction and waits for confirmation.
    async fn submit_transaction(&self, transaction: &Transaction)
    -> Result<Signature, SquadsError>;
}

#[async_trait]
impl RouterRpc for RpcClient {
    async fn fetch_account_data(&self, pubkey: &Pubkey) -> Result<Vec<u8>, SquadsError> {
        self.get_account_data(pubkey)
            .await
            .map_err(|err| SquadsError::RpcFetch {
                pubkey: *pubkey,
                reason: err.to_string(),
            })
    }

    async fn recent_blockhash(&self) -> Result<Hash, SquadsError> {
        self.get_latest_blockhash()
            .await
            .map_err(|err| SquadsError::SendTransaction {
                reason: format!("failed to fetch latest blockhash: {err}"),
            })
    }

    async fn submit_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Signature, SquadsError> {
        self.send_and_confirm_transaction(transaction)
            .await
            .map_err(|err| SquadsError::SendTransaction {
                reason: err.to_string(),
            })
    }
}

// ============================================================================
// Public CLI entry point
// ============================================================================

/// Routes `vault_ixs` either directly (local sign + send) or through a Squads multisig
/// vault, depending on `squads`.
///
/// This function does **not** check whether a vault PDA can satisfy the wrapped
/// instructions' on-chain signer-identity requirements. That check is performed up front
/// by `squads_refusal_for` in `main.rs` before any handler is invoked; by the time we
/// reach the router the caller is committed to routing.
pub async fn route(
    rpc: &RpcClient,
    vault_ixs: Vec<Instruction>,
    preflight_ixs: Vec<Instruction>,
    signers: &[&Keypair],
    squads: Option<&SquadsRoutingConfig>,
) -> Result<RoutedOutcome> {
    route_or_send(rpc, vault_ixs, preflight_ixs, signers, squads)
        .await
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

// ============================================================================
// Routing core
// ============================================================================

/// Routes `vault_ixs` either directly (sign + send locally) or through a Squads multisig
/// vault, depending on whether `squads` is `Some`.
///
/// * `vault_ixs` — the instruction(s) the user wants performed. In Squads mode these are
///   wrapped inside `vault_transaction_create`.
/// * `preflight_ixs` — setup instructions that do not need vault authority. In Squads mode
///   they are sent first as a separate, locally-signed transaction; in direct mode they
///   are prepended to `vault_ixs` and sent atomically.
/// * `signers` — keypairs that sign the locally-submitted transaction(s). `signers[0]` is
///   treated as the fee payer (and, in Squads mode, must correspond to
///   `SquadsRoutingConfig::proposer`).
/// * `squads` — `Some(..)` to route through a multisig vault, `None` for direct mode.
///
pub async fn route_or_send<R: RouterRpc + ?Sized>(
    rpc: &R,
    vault_ixs: Vec<Instruction>,
    preflight_ixs: Vec<Instruction>,
    signers: &[&Keypair],
    squads: Option<&SquadsRoutingConfig>,
) -> Result<RoutedOutcome, SquadsError> {
    match squads {
        None => {
            // Direct mode: preflight + vault in a single atomic transaction.
            let mut instructions = preflight_ixs;
            instructions.extend(vault_ixs);
            let signature = send_instructions(rpc, &instructions, signers).await?;
            Ok(RoutedOutcome::Direct {
                signature,
                slot: None,
            })
        }
        Some(config) => route_via_squads(rpc, vault_ixs, preflight_ixs, signers, config).await,
    }
}

/// Builds the Squads `vault_transaction_create` + `proposal_create` pair and submits it,
/// retrying on transaction-index collisions.
async fn route_via_squads<R: RouterRpc + ?Sized>(
    rpc: &R,
    vault_ixs: Vec<Instruction>,
    preflight_ixs: Vec<Instruction>,
    signers: &[&Keypair],
    config: &SquadsRoutingConfig,
) -> Result<RoutedOutcome, SquadsError> {
    // 1. Run any preflight instructions as their own direct-mode transaction first.
    if !preflight_ixs.is_empty() {
        send_instructions(rpc, &preflight_ixs, signers).await?;
    }

    let squads = match config.program_id {
        Some(program_id) => SquadsClient::with_program_id(program_id),
        None => SquadsClient::new(),
    };

    // 2. Build + submit, retrying with a freshly-fetched index on "already in use"
    //    collisions.
    let mut attempt: u8 = 0;
    loop {
        attempt += 1;

        let multisig_data = rpc.fetch_account_data(&config.multisig).await?;
        let multisig = Multisig::try_deserialize(&multisig_data)?;

        squads.verify_proposer(&config.multisig, &multisig, &config.proposer)?;

        let built = squads.build_vault_tx_with_proposal(
            &config.multisig,
            multisig.transaction_index,
            config.vault_index,
            &config.proposer,
            &config.proposer,
            &vault_ixs,
            &[],
            config.memo.clone(),
        )?;

        match send_instructions(rpc, &built.instructions, signers).await {
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
                    web_url: squads_transaction_url(&config.multisig, built.transaction_index),
                });
            }
            Err(SquadsError::SendTransaction { reason }) if is_account_collision(&reason) => {
                if attempt >= MAX_INDEX_ATTEMPTS {
                    return Err(SquadsError::TransactionIndexRace {
                        multisig: config.multisig,
                        attempts: MAX_INDEX_ATTEMPTS,
                    });
                }
                // Loop: re-fetch the multisig (its transaction_index will have advanced)
                // and retry with the next free index.
            }
            Err(other) => return Err(other),
        }
    }
}

/// Signs `instructions` with `signers` (treating `signers[0]` as the fee payer) against a
/// freshly-fetched blockhash and submits the transaction.
async fn send_instructions<R: RouterRpc + ?Sized>(
    rpc: &R,
    instructions: &[Instruction],
    signers: &[&Keypair],
) -> Result<Signature, SquadsError> {
    let blockhash = rpc.recent_blockhash().await?;
    let payer = signers.first().map(|keypair| keypair.pubkey());
    let transaction =
        Transaction::new_signed_with_payer(instructions, payer.as_ref(), signers, blockhash);
    rpc.submit_transaction(&transaction).await
}

/// Returns `true` if a send error's reason indicates a PDA-account collision (i.e. the
/// vault transaction or proposal index was claimed by a concurrent proposal).
fn is_account_collision(reason: &str) -> bool {
    reason.contains("already in use")
}

/// Builds the canonical Squads web UI URL for a created vault transaction.
fn squads_transaction_url(multisig: &Pubkey, transaction_index: u64) -> String {
    format!("https://app.squads.so/squads/{multisig}/transactions/{transaction_index}")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshDeserialize;
    use squads_client::discriminator::instruction_discriminator;
    use squads_client::{
        Member, Multisig, PROGRAM_ID, Permission, Permissions, TransactionMessage,
        VaultTransactionCreateArgs,
    };
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[test]
    fn effective_signer_substitutes_vault_pda_only_in_squads_mode() {
        let local = Pubkey::new_unique();
        let opts = SquadsCliOpts {
            multisig: Pubkey::new_unique(),
            vault_index: 0,
            program_id: None,
            memo: None,
        };

        // Direct mode keeps the local signer.
        assert_eq!(effective_signer(None, local), local);

        // Squads mode swaps in the vault PDA (and never the local key).
        let vault = effective_signer(Some(&opts), local);
        assert_eq!(vault, opts.vault_pubkey());
        assert_ne!(vault, local);
    }

    // ----- Router orchestration tests (mock RPC) -----

    /// Behavior the mock applies to a single `submit_transaction` call.
    enum SendBehavior {
        /// Succeed and return a default signature.
        Succeed,
        /// Fail with an "already in use" account-collision error and advance the stored
        /// multisig's transaction index by one (simulating a competing proposal).
        FailAlreadyInUse,
    }

    struct MockRpc {
        multisig: Mutex<Multisig>,
        sent: Mutex<Vec<Transaction>>,
        behaviors: Mutex<VecDeque<SendBehavior>>,
    }

    impl MockRpc {
        fn new(multisig: Multisig) -> Self {
            Self {
                multisig: Mutex::new(multisig),
                sent: Mutex::new(Vec::new()),
                behaviors: Mutex::new(VecDeque::new()),
            }
        }

        fn with_behaviors(multisig: Multisig, behaviors: Vec<SendBehavior>) -> Self {
            let mock = Self::new(multisig);
            *mock.behaviors.lock().unwrap() = behaviors.into();
            mock
        }

        fn sent_transactions(&self) -> Vec<Transaction> {
            self.sent.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl RouterRpc for MockRpc {
        async fn fetch_account_data(&self, _pubkey: &Pubkey) -> Result<Vec<u8>, SquadsError> {
            Ok(serialize_multisig(&self.multisig.lock().unwrap()))
        }

        async fn recent_blockhash(&self) -> Result<Hash, SquadsError> {
            Ok(Hash::default())
        }

        async fn submit_transaction(
            &self,
            transaction: &Transaction,
        ) -> Result<Signature, SquadsError> {
            self.sent.lock().unwrap().push(transaction.clone());
            let behavior = self.behaviors.lock().unwrap().pop_front();
            match behavior {
                Some(SendBehavior::FailAlreadyInUse) => {
                    self.multisig.lock().unwrap().transaction_index += 1;
                    Err(SquadsError::SendTransaction {
                        reason: "Allocate: account Address { .. } already in use".to_string(),
                    })
                }
                _ => Ok(Signature::default()),
            }
        }
    }

    /// Serializes a `Multisig` into its on-chain byte layout (mirrors the canonical
    /// shape; the production code only ever reads `Multisig` accounts).
    fn serialize_multisig(m: &Multisig) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&Multisig::discriminator());
        bytes.extend_from_slice(&m.create_key.to_bytes());
        bytes.extend_from_slice(&m.config_authority.to_bytes());
        bytes.extend_from_slice(&m.threshold.to_le_bytes());
        bytes.extend_from_slice(&m.time_lock.to_le_bytes());
        bytes.extend_from_slice(&m.transaction_index.to_le_bytes());
        bytes.extend_from_slice(&m.stale_transaction_index.to_le_bytes());
        match &m.rent_collector {
            Some(rc) => {
                bytes.push(1);
                bytes.extend_from_slice(&rc.to_bytes());
            }
            None => bytes.push(0),
        }
        bytes.push(m.bump);
        bytes.extend_from_slice(&(m.members.len() as u32).to_le_bytes());
        for member in &m.members {
            bytes.extend_from_slice(&member.key.to_bytes());
            bytes.push(member.permissions.mask);
        }
        bytes
    }

    /// Builds a multisig where `proposer` is an `Initiate`-capable member, plus two other
    /// members, with the supplied starting transaction index.
    fn multisig_with_proposer(proposer: &Pubkey, transaction_index: u64) -> Multisig {
        Multisig {
            create_key: Pubkey::new_unique(),
            config_authority: Pubkey::default(),
            threshold: 2,
            time_lock: 0,
            transaction_index,
            stale_transaction_index: 0,
            rent_collector: None,
            bump: 254,
            members: vec![
                Member {
                    key: *proposer,
                    permissions: Permissions::from_vec(&[Permission::Initiate, Permission::Vote]),
                },
                Member {
                    key: Pubkey::new_unique(),
                    permissions: Permissions::from_vec(&[Permission::Vote]),
                },
                Member {
                    key: Pubkey::new_unique(),
                    permissions: Permissions::from_vec(&[Permission::Vote, Permission::Execute]),
                },
            ],
        }
    }

    /// Locates the `vault_transaction_create` instruction in `tx` and decodes the wrapped
    /// [`TransactionMessage`] it carries.
    fn decode_wrapped_message(tx: &Transaction, program_id: &Pubkey) -> TransactionMessage {
        let disc = instruction_discriminator("vault_transaction_create");
        for cix in &tx.message.instructions {
            let program = tx.message.account_keys[cix.program_id_index as usize];
            if &program == program_id && cix.data.len() >= 8 && cix.data[..8] == disc[..] {
                let args = VaultTransactionCreateArgs::try_from_slice(&cix.data[8..]).unwrap();
                return TransactionMessage::try_from_slice(&args.transaction_message).unwrap();
            }
        }
        panic!("vault_transaction_create instruction not found in transaction");
    }

    /// Returns the program ID each top-level instruction of `tx` targets, in order.
    fn instruction_program_ids(tx: &Transaction) -> Vec<Pubkey> {
        tx.message
            .instructions
            .iter()
            .map(|cix| tx.message.account_keys[cix.program_id_index as usize])
            .collect()
    }

    fn user_instruction(data: Vec<u8>) -> Instruction {
        use anchor_client::solana_sdk::instruction::AccountMeta;
        Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![AccountMeta::new(Pubkey::new_unique(), false)],
            data,
        }
    }

    #[tokio::test]
    async fn squads_mode_sends_preflight_then_vault_tx() {
        let proposer = Keypair::new();
        let mock = MockRpc::new(multisig_with_proposer(&proposer.pubkey(), 0));
        let config = SquadsRoutingConfig {
            multisig: Pubkey::new_unique(),
            vault_index: 0,
            proposer: proposer.pubkey(),
            program_id: None,
            memo: None,
        };
        let init_ix = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![0xab],
        };
        let vault_ix = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![0xcd],
        };

        route_or_send(
            &mock,
            vec![vault_ix.clone()],
            vec![init_ix.clone()],
            &[&proposer],
            Some(&config),
        )
        .await
        .unwrap();

        let sent = mock.sent_transactions();
        assert_eq!(sent.len(), 2, "preflight TX followed by the vault TX");

        // First send: the preflight, carrying only init_ix (no vault_transaction_create).
        let preflight_programs = instruction_program_ids(&sent[0]);
        assert_eq!(preflight_programs, vec![init_ix.program_id]);

        // Second send: the wrapped vault_ix
        let wrapped = decode_wrapped_message(&sent[1], &PROGRAM_ID);
        let inner = wrapped.instructions.as_slice();
        let keys = wrapped.account_keys.as_slice();
        assert_eq!(inner.len(), 1);
        assert_eq!(
            keys[inner[0].program_id_index as usize],
            vault_ix.program_id
        );
    }

    #[tokio::test]
    async fn direct_mode_merges_preflight_and_vault_into_single_tx() {
        let payer = Keypair::new();
        let mock = MockRpc::new(multisig_with_proposer(&payer.pubkey(), 0));
        let init_ix = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![0x01],
        };
        let vault_ix = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![0x02],
        };

        route_or_send(
            &mock,
            vec![vault_ix.clone()],
            vec![init_ix.clone()],
            &[&payer],
            None,
        )
        .await
        .unwrap();

        let sent = mock.sent_transactions();
        assert_eq!(sent.len(), 1, "preflight + vault merged into one TX");

        let program_ids = instruction_program_ids(&sent[0]);
        assert_eq!(
            program_ids,
            vec![init_ix.program_id, vault_ix.program_id],
            "preflight first, then vault, in order"
        );
    }

    #[tokio::test]
    async fn squads_mode_retries_on_index_collision_then_succeeds() {
        let proposer = Keypair::new();
        // First submit fails ("already in use") and bumps the stored index; the second
        // submit succeeds.
        let mock = MockRpc::with_behaviors(
            multisig_with_proposer(&proposer.pubkey(), 5),
            vec![SendBehavior::FailAlreadyInUse, SendBehavior::Succeed],
        );
        let config = SquadsRoutingConfig {
            multisig: Pubkey::new_unique(),
            vault_index: 0,
            proposer: proposer.pubkey(),
            program_id: None,
            memo: None,
        };

        let outcome = route_or_send(
            &mock,
            vec![user_instruction(vec![7])],
            vec![],
            &[&proposer],
            Some(&config),
        )
        .await
        .unwrap();

        // Two submit attempts were made.
        assert_eq!(mock.sent_transactions().len(), 2);

        // The successful attempt used the advanced index: stored 5 -> first attempt 6
        // (collision, stored bumped to 6) -> second attempt 7.
        match outcome {
            RoutedOutcome::Squads {
                transaction_index, ..
            } => assert_eq!(transaction_index, 7),
            other => panic!("expected Squads outcome, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn squads_mode_gives_up_after_max_index_collisions() {
        let proposer = Keypair::new();
        let mock = MockRpc::with_behaviors(
            multisig_with_proposer(&proposer.pubkey(), 0),
            vec![
                SendBehavior::FailAlreadyInUse,
                SendBehavior::FailAlreadyInUse,
                SendBehavior::FailAlreadyInUse,
            ],
        );
        let config = SquadsRoutingConfig {
            multisig: Pubkey::new_unique(),
            vault_index: 0,
            proposer: proposer.pubkey(),
            program_id: None,
            memo: None,
        };

        let err = route_or_send(
            &mock,
            vec![user_instruction(vec![7])],
            vec![],
            &[&proposer],
            Some(&config),
        )
        .await
        .unwrap_err();

        match err {
            SquadsError::TransactionIndexRace { attempts, .. } => {
                assert_eq!(attempts, MAX_INDEX_ATTEMPTS)
            }
            other => panic!("expected TransactionIndexRace, got {other:?}"),
        }
        assert_eq!(mock.sent_transactions().len(), MAX_INDEX_ATTEMPTS as usize);
    }

    #[tokio::test]
    async fn squads_mode_rejects_proposer_without_initiate_permission() {
        let proposer = Keypair::new();
        // Multisig where the proposer is NOT a member.
        let mock = MockRpc::new(multisig_with_proposer(&Pubkey::new_unique(), 0));
        let config = SquadsRoutingConfig {
            multisig: Pubkey::new_unique(),
            vault_index: 0,
            proposer: proposer.pubkey(),
            program_id: None,
            memo: None,
        };

        let err = route_or_send(
            &mock,
            vec![user_instruction(vec![7])],
            vec![],
            &[&proposer],
            Some(&config),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, SquadsError::ProposerNotMember { .. }));
        // No transaction should have been submitted.
        assert!(mock.sent_transactions().is_empty());
    }

    #[test]
    fn format_structured_squads_matches_golden_block() {
        let multisig = Pubkey::new_unique();
        let vault = Pubkey::new_unique();
        let vault_transaction_pda = Pubkey::new_unique();
        let proposal_pda = Pubkey::new_unique();
        let creation_signature = Signature::default();
        let transaction_index = 42u64;
        let web_url = squads_transaction_url(&multisig, transaction_index);

        let outcome = RoutedOutcome::Squads {
            multisig,
            vault,
            transaction_index,
            vault_transaction_pda,
            proposal_pda,
            creation_signature,
            threshold: 2,
            total_members: 3,
            web_url: web_url.clone(),
        };

        // Golden block: 2-space indent, labels padded so values begin at column 25.
        let expected = format!(
            "[Squads] Vault transaction created.\n  \
multisig:              {multisig}\n  \
vault:                 {vault}\n  \
transaction_index:     {transaction_index}\n  \
vault_transaction_pda: {vault_transaction_pda}\n  \
proposal_pda:          {proposal_pda}\n  \
creation_signature:    {creation_signature}\n  \
threshold:             2 of 3\n  \
url:                   {web_url}",
        );

        assert_eq!(outcome.format_structured(), expected);
    }
}
