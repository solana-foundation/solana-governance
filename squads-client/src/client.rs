//! `SquadsClient` — high-level façade combining PDA derivation, member-permission
//! verification, and bundled instruction emission for the canonical "create a vault
//! transaction and immediately activate a proposal" flow.
//!
//! This module deliberately does NOT depend on any RPC client. Callers are responsible
//! for fetching the [`Multisig`] account state and supplying the current
//! `transaction_index` to the builder; this keeps `squads-client` portable across
//! Solana RPC stacks (e.g., `solana-client`, `solana-rpc-client`, or any test harness).

use solana_program::instruction::Instruction;
use solana_program::message::AddressLookupTableAccount;
use solana_program::pubkey::Pubkey;

use crate::error::{Result, SquadsError};
use crate::id::PROGRAM_ID;
use crate::instructions::{
    proposal_create_ix, vault_transaction_create_from_instructions, ProposalCreateAccounts,
    ProposalCreateArgs,
};
use crate::pda::{multisig_pda, proposal_pda, transaction_pda, vault_pda};
use crate::state::{Multisig, Permission};

/// High-level façade for assembling Squads V4 instructions.
#[derive(Clone, Copy, Debug)]
pub struct SquadsClient {
    /// Program ID this client targets.
    pub program_id: Pubkey,
}

impl Default for SquadsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SquadsClient {
    /// Constructs a client targeting the canonical [`PROGRAM_ID`].
    pub fn new() -> Self {
        Self {
            program_id: PROGRAM_ID,
        }
    }

    /// Constructs a client targeting a non-canonical deployment.
    pub fn with_program_id(program_id: Pubkey) -> Self {
        Self { program_id }
    }

    // ----- PDA helpers -----

    /// Derives the multisig PDA from its `create_key`.
    pub fn pda_multisig(&self, create_key: &Pubkey) -> (Pubkey, u8) {
        multisig_pda(create_key, Some(&self.program_id))
    }

    /// Derives a vault PDA for the given multisig and vault index.
    pub fn pda_vault(&self, multisig: &Pubkey, vault_index: u8) -> (Pubkey, u8) {
        vault_pda(multisig, vault_index, Some(&self.program_id))
    }

    /// Derives a `VaultTransaction` PDA for the given multisig and transaction index.
    pub fn pda_transaction(&self, multisig: &Pubkey, transaction_index: u64) -> (Pubkey, u8) {
        transaction_pda(multisig, transaction_index, Some(&self.program_id))
    }

    /// Derives a `Proposal` PDA for the given multisig and transaction index.
    pub fn pda_proposal(&self, multisig: &Pubkey, transaction_index: u64) -> (Pubkey, u8) {
        proposal_pda(multisig, transaction_index, Some(&self.program_id))
    }

    // ----- Member verification -----

    /// Validates that `proposer` is a multisig member with the `Initiate` permission.
    /// Returns `Ok(())` on success; otherwise [`SquadsError::ProposerNotMember`] or
    /// [`SquadsError::ProposerNotAuthorized`].
    pub fn verify_proposer(
        &self,
        multisig: &Pubkey,
        multisig_account: &Multisig,
        proposer: &Pubkey,
    ) -> Result<()> {
        if multisig_account.is_member(proposer).is_none() {
            return Err(SquadsError::ProposerNotMember {
                proposer: *proposer,
                multisig: *multisig,
            });
        }
        if !multisig_account.member_has_permission(proposer, Permission::Initiate) {
            return Err(SquadsError::ProposerNotAuthorized {
                proposer: *proposer,
                multisig: *multisig,
            });
        }
        Ok(())
    }

    // ----- Top-level builder -----

    /// Builds the full `(vault_transaction_create, proposal_create)` instruction pair to
    /// wrap `inner_instructions` and immediately activate the proposal for voting.
    ///
    /// `current_transaction_index` is the value of `Multisig::transaction_index` at the
    /// moment of submission; the new transaction is assigned `current_transaction_index + 1`.
    /// Callers MUST re-fetch the multisig immediately before calling this to minimize
    /// the chance of a race where another in-flight proposal grabs the same index.
    ///
    /// Returns a [`BuiltVaultTransaction`] containing the two emitted instructions plus
    /// the derived `transaction_pda`, `proposal_pda`, and the new transaction index.
    #[allow(clippy::too_many_arguments)]
    pub fn build_vault_tx_with_proposal(
        &self,
        multisig: &Pubkey,
        current_transaction_index: u64,
        vault_index: u8,
        creator: &Pubkey,
        rent_payer: &Pubkey,
        inner_instructions: &[Instruction],
        address_lookup_table_accounts: &[AddressLookupTableAccount],
        memo: Option<String>,
    ) -> Result<BuiltVaultTransaction> {
        let new_transaction_index = current_transaction_index.checked_add(1).ok_or_else(|| {
            SquadsError::BorshEncode(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "transaction_index overflow",
            ))
        })?;

        let (transaction, _) = self.pda_transaction(multisig, new_transaction_index);
        let (proposal, _) = self.pda_proposal(multisig, new_transaction_index);
        let (vault, _) = self.pda_vault(multisig, vault_index);

        let create_ix = vault_transaction_create_from_instructions(
            &self.program_id,
            multisig,
            &transaction,
            creator,
            rent_payer,
            vault_index,
            &vault,
            inner_instructions,
            address_lookup_table_accounts,
            memo,
        )?;

        let proposal_ix = proposal_create_ix(
            &self.program_id,
            ProposalCreateAccounts {
                multisig: *multisig,
                proposal,
                creator: *creator,
                rent_payer: *rent_payer,
            },
            &ProposalCreateArgs {
                transaction_index: new_transaction_index,
                draft: false,
            },
        )
        .map_err(SquadsError::BorshEncode)?;

        Ok(BuiltVaultTransaction {
            instructions: vec![create_ix, proposal_ix],
            transaction,
            proposal,
            vault,
            transaction_index: new_transaction_index,
        })
    }

    /// Returns the canonical Squads web UI URL for the given multisig.
    pub fn web_url(&self, multisig: &Pubkey) -> String {
        format!("https://app.squads.so/squads/{}/home", multisig)
    }
}

/// Result of [`SquadsClient::build_vault_tx_with_proposal`].
#[derive(Clone, Debug)]
pub struct BuiltVaultTransaction {
    /// The pair of instructions to bundle into a single Solana transaction:
    /// `[vault_transaction_create, proposal_create]`.
    pub instructions: Vec<Instruction>,
    /// PDA of the newly-created `VaultTransaction` account.
    pub transaction: Pubkey,
    /// PDA of the newly-created `Proposal` account.
    pub proposal: Pubkey,
    /// PDA of the vault that will execute the transaction at approval time.
    pub vault: Pubkey,
    /// Transaction index assigned to this vault TX (= `Multisig::transaction_index + 1`).
    pub transaction_index: u64,
}

// ============================================================================
// SquadsMultisigClient (RPC-backed wrapper)
// ============================================================================

/// Async wrapper around [`SquadsClient`] that owns a Solana RPC connection and a
/// specific multisig + default vault, hydrating on-chain `Multisig` state automatically
/// for the builder calls.
///
/// Construct from a multisig pubkey directly via [`SquadsMultisigClient::new`], or from
/// a `create_key` via [`SquadsMultisigClient::from_create_key`] (the multisig PDA is
/// derived on construction). Vault index defaults to `0`; override with
/// [`SquadsMultisigClient::with_vault_index`].
///
/// Only available with the `rpc` feature flag (on by default).
#[cfg(feature = "rpc")]
#[derive(Clone)]
pub struct SquadsMultisigClient {
    /// Lower-level instruction builder.
    pub squads: SquadsClient,
    /// The multisig this client is bound to.
    pub multisig: Pubkey,
    /// The default vault index used by [`SquadsMultisigClient::build_vault_tx_with_proposal`].
    pub vault_index: u8,
    /// Shared RPC handle used to hydrate `Multisig` state on demand.
    pub rpc: std::sync::Arc<solana_rpc_client::nonblocking::rpc_client::RpcClient>,
}

#[cfg(feature = "rpc")]
impl SquadsMultisigClient {
    /// Constructs a client bound to `multisig`, targeting the canonical Squads program ID
    /// and vault index `0`. Use [`Self::with_vault_index`] or [`Self::with_program_id`] to override.
    pub fn new(
        rpc: std::sync::Arc<solana_rpc_client::nonblocking::rpc_client::RpcClient>,
        multisig: Pubkey,
    ) -> Self {
        Self {
            squads: SquadsClient::new(),
            multisig,
            vault_index: 0,
            rpc,
        }
    }

    /// Constructs a client by deriving the multisig PDA from `create_key`.
    pub fn from_create_key(
        rpc: std::sync::Arc<solana_rpc_client::nonblocking::rpc_client::RpcClient>,
        create_key: &Pubkey,
    ) -> Self {
        let squads = SquadsClient::new();
        let (multisig, _) = squads.pda_multisig(create_key);
        Self {
            squads,
            multisig,
            vault_index: 0,
            rpc,
        }
    }

    /// Returns a new client targeting a non-canonical program ID.
    #[must_use]
    pub fn with_program_id(mut self, program_id: Pubkey) -> Self {
        self.squads = SquadsClient::with_program_id(program_id);
        self
    }

    /// Returns a new client with the default `vault_index` overridden.
    #[must_use]
    pub fn with_vault_index(mut self, vault_index: u8) -> Self {
        self.vault_index = vault_index;
        self
    }

    /// Returns the vault PDA derived from the bound `multisig` and `vault_index`.
    pub fn vault_pda(&self) -> Pubkey {
        self.squads.pda_vault(&self.multisig, self.vault_index).0
    }

    /// Returns the canonical Squads web UI URL for the bound multisig.
    pub fn web_url(&self) -> String {
        self.squads.web_url(&self.multisig)
    }

    /// Fetches the `Multisig` account from RPC and decodes it.
    ///
    /// Returns [`SquadsError::RpcFetch`] on RPC failure (network error, account not
    /// found, etc.) and [`SquadsError::AccountDecode`] or the lower-level discriminator
    /// / borsh errors on a parse failure.
    pub async fn fetch_multisig(&self) -> Result<Multisig> {
        let data = self
            .rpc
            .get_account_data(&self.multisig)
            .await
            .map_err(|err| SquadsError::RpcFetch {
                pubkey: self.multisig,
                reason: err.to_string(),
            })?;

        Multisig::try_deserialize(&data)
    }

    /// Fetches the bound multisig and validates that `proposer` is a member with
    /// the [`Permission::Initiate`](crate::Permission::Initiate) permission.
    pub async fn verify_proposer(&self, proposer: &Pubkey) -> Result<()> {
        let multisig_account = self.fetch_multisig().await?;
        self.squads
            .verify_proposer(&self.multisig, &multisig_account, proposer)
    }

    /// Hydrates the multisig state from RPC, validates that `creator` is allowed to
    /// initiate transactions, and emits the
    /// `[vault_transaction_create, proposal_create]` instruction pair wrapping
    /// `inner_instructions`.
    ///
    /// The new transaction index is `multisig.transaction_index + 1` as read at the
    /// moment of this call. Callers SHOULD submit the resulting transaction promptly to
    /// minimize the chance of another in-flight proposal claiming the same index.
    #[allow(clippy::too_many_arguments)]
    pub async fn build_vault_tx_with_proposal(
        &self,
        creator: &Pubkey,
        rent_payer: &Pubkey,
        inner_instructions: &[Instruction],
        address_lookup_table_accounts: &[AddressLookupTableAccount],
        memo: Option<String>,
    ) -> Result<BuiltVaultTransaction> {
        let multisig_account = self.fetch_multisig().await?;
        self.squads
            .verify_proposer(&self.multisig, &multisig_account, creator)?;

        self.squads.build_vault_tx_with_proposal(
            &self.multisig,
            multisig_account.transaction_index,
            self.vault_index,
            creator,
            rent_payer,
            inner_instructions,
            address_lookup_table_accounts,
            memo,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Member, Permissions};

    fn make_multisig(members: Vec<Member>) -> Multisig {
        Multisig {
            create_key: Pubkey::new_unique(),
            config_authority: Pubkey::default(),
            threshold: 2,
            time_lock: 0,
            transaction_index: 0,
            stale_transaction_index: 0,
            rent_collector: None,
            bump: 254,
            members,
        }
    }

    #[test]
    fn verify_proposer_rejects_non_member() {
        let outsider = Pubkey::new_unique();
        let ms_key = Pubkey::new_unique();
        let ms = make_multisig(vec![Member {
            key: Pubkey::new_unique(),
            permissions: Permissions::from_vec(&[Permission::Initiate]),
        }]);
        let client = SquadsClient::new();
        match client.verify_proposer(&ms_key, &ms, &outsider) {
            Err(SquadsError::ProposerNotMember { .. }) => {}
            other => panic!("expected ProposerNotMember, got {:?}", other),
        }
    }

    #[test]
    fn verify_proposer_rejects_missing_initiate_permission() {
        let voter = Pubkey::new_unique();
        let ms_key = Pubkey::new_unique();
        let ms = make_multisig(vec![Member {
            key: voter,
            permissions: Permissions::from_vec(&[Permission::Vote]),
        }]);
        let client = SquadsClient::new();
        match client.verify_proposer(&ms_key, &ms, &voter) {
            Err(SquadsError::ProposerNotAuthorized { .. }) => {}
            other => panic!("expected ProposerNotAuthorized, got {:?}", other),
        }
    }

    #[test]
    fn verify_proposer_accepts_initiate_member() {
        let proposer = Pubkey::new_unique();
        let ms_key = Pubkey::new_unique();
        let ms = make_multisig(vec![Member {
            key: proposer,
            permissions: Permissions::from_vec(&[Permission::Initiate, Permission::Vote]),
        }]);
        let client = SquadsClient::new();
        assert!(client.verify_proposer(&ms_key, &ms, &proposer).is_ok());
    }

    #[test]
    fn build_vault_tx_returns_two_instructions_with_correct_index() {
        let creator = Pubkey::new_unique();
        let rent_payer = creator;
        let multisig = Pubkey::new_unique();

        let client = SquadsClient::new();
        let inner = vec![Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![1, 2, 3],
        }];

        let built = client
            .build_vault_tx_with_proposal(
                &multisig,
                /* current_transaction_index */ 7,
                /* vault_index */ 0,
                &creator,
                &rent_payer,
                &inner,
                &[],
                Some("hello".into()),
            )
            .unwrap();

        assert_eq!(built.instructions.len(), 2);
        assert_eq!(built.transaction_index, 8);
        // First instruction is vault_transaction_create.
        assert_eq!(
            &built.instructions[0].data[..8],
            &crate::discriminator::instruction_discriminator("vault_transaction_create"),
        );
        // Second instruction is proposal_create.
        assert_eq!(
            &built.instructions[1].data[..8],
            &crate::discriminator::instruction_discriminator("proposal_create"),
        );
        // Derived transaction PDA matches the same derivation produced from scratch.
        let (expected_tx, _) = client.pda_transaction(&multisig, 8);
        assert_eq!(built.transaction, expected_tx);
        let (expected_prop, _) = client.pda_proposal(&multisig, 8);
        assert_eq!(built.proposal, expected_prop);
    }

    #[test]
    fn web_url_format() {
        let client = SquadsClient::new();
        let ms = Pubkey::new_unique();
        let url = client.web_url(&ms);
        assert!(url.starts_with("https://app.squads.so/squads/"));
        assert!(url.ends_with("/home"));
        assert!(url.contains(&ms.to_string()));
    }

    // ----- SquadsMultisigClient construction tests (sync portions only) -----

    #[cfg(feature = "rpc")]
    #[test]
    fn multisig_client_new_binds_multisig_and_default_vault_index() {
        use std::sync::Arc;
        let rpc = Arc::new(solana_rpc_client::nonblocking::rpc_client::RpcClient::new(
            "http://localhost:8899".into(),
        ));
        let multisig = Pubkey::new_unique();
        let client = SquadsMultisigClient::new(rpc, multisig);
        assert_eq!(client.multisig, multisig);
        assert_eq!(client.vault_index, 0);
        assert_eq!(client.squads.program_id, PROGRAM_ID);
    }

    #[cfg(feature = "rpc")]
    #[test]
    fn multisig_client_from_create_key_derives_multisig_pda() {
        use std::sync::Arc;
        let rpc = Arc::new(solana_rpc_client::nonblocking::rpc_client::RpcClient::new(
            "http://localhost:8899".into(),
        ));
        let create_key = Pubkey::new_unique();
        let client = SquadsMultisigClient::from_create_key(rpc, &create_key);
        let (expected, _) = crate::pda::multisig_pda(&create_key, Some(&PROGRAM_ID));
        assert_eq!(client.multisig, expected);
        assert_eq!(client.vault_index, 0);
    }

    #[cfg(feature = "rpc")]
    #[test]
    fn multisig_client_builder_overrides_vault_index_and_program_id() {
        use std::sync::Arc;
        let rpc = Arc::new(solana_rpc_client::nonblocking::rpc_client::RpcClient::new(
            "http://localhost:8899".into(),
        ));
        let multisig = Pubkey::new_unique();
        let alt_program = Pubkey::new_unique();
        let client = SquadsMultisigClient::new(rpc, multisig)
            .with_vault_index(3)
            .with_program_id(alt_program);
        assert_eq!(client.vault_index, 3);
        assert_eq!(client.squads.program_id, alt_program);
    }

    #[cfg(feature = "rpc")]
    #[test]
    fn multisig_client_vault_pda_uses_stored_vault_index() {
        use std::sync::Arc;
        let rpc = Arc::new(solana_rpc_client::nonblocking::rpc_client::RpcClient::new(
            "http://localhost:8899".into(),
        ));
        let multisig = Pubkey::new_unique();
        let client = SquadsMultisigClient::new(rpc, multisig).with_vault_index(7);

        let computed = client.vault_pda();
        let (expected, _) = crate::pda::vault_pda(&multisig, 7, Some(&PROGRAM_ID));
        assert_eq!(computed, expected);
    }

    #[cfg(feature = "rpc")]
    #[test]
    fn multisig_client_web_url_routes_to_bound_multisig() {
        use std::sync::Arc;
        let rpc = Arc::new(solana_rpc_client::nonblocking::rpc_client::RpcClient::new(
            "http://localhost:8899".into(),
        ));
        let multisig = Pubkey::new_unique();
        let url = SquadsMultisigClient::new(rpc, multisig).web_url();
        assert!(url.contains(&multisig.to_string()));
    }
}
