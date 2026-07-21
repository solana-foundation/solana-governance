pub mod consts;
pub mod merkle;
pub mod utils;

pub use ncn_ledger as ledger;
pub use ncn_merkle_tree as merkle_tree;

use crate::consts::{MARINADE_OPS_VOTING_WALLET, MARINADE_WITHDRAW_AUTHORITY};
use im::HashMap;
pub use merkle::*;
use ncn_ledger::STAKE_POOL_PROGRAM_ID;

use anchor_lang::prelude::Pubkey as AnchorPubkey;
use anyhow::Error;
use borsh_stake::BorshDeserialize;
use crate::merkle_tree::{get_proof, Delegation, MerkleTree};
use ncn_snapshot::{MetaMerkleLeaf, StakeMerkleLeaf};
use solana_accounts_db::accounts_index::IndexKey;
use solana_program::pubkey::Pubkey;
use solana_runtime::bank::Bank;
use solana_sdk::account::from_account;
use solana_sdk::account::AccountSharedData;
use solana_sdk::account::ReadableAccount;
use solana_sdk::clock::Epoch;
use solana_stake_interface::stake_history::StakeHistory;
use solana_stake_interface::state::StakeStateV2;
use solana_stake_interface::sysvar::stake_history;
use spl_stake_pool::find_withdraw_authority_program_address;
use spl_stake_pool::state::AccountType;
use spl_stake_pool::state::StakePool;
use std::sync::Arc;

pub fn upload_signature_message(
    slot: u64,
    network: &str,
    merkle_root: &str,
    snapshot_hash: &str,
) -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(&slot.to_le_bytes());
    message.extend_from_slice(network.as_bytes());
    message.extend_from_slice(merkle_root.as_bytes());
    message.extend_from_slice(snapshot_hash.as_bytes());
    message
}

fn to_anchor_pubkey(pubkey: Pubkey) -> AnchorPubkey {
    AnchorPubkey::from(pubkey.to_bytes())
}

fn get_validator_identity(
    bank: &solana_runtime::bank::Bank,
    vote_account: &Pubkey,
) -> Option<Pubkey> {
    let account = bank.get_account(vote_account)?;
    if account.owner() != &solana_vote_program::id() {
        return None;
    }
    let data = Arc::new(account.data().to_vec());
    let vote_state = solana_vote::vote_state_view::VoteStateView::try_new(data).ok()?;
    Some(*vote_state.node_pubkey())
}

/// If `account` is an active stake-program account, decode its delegation and
/// append it (grouped by the voter/validator pubkey it delegates to) to `map`.
///
/// Upstream agave 4.1.0 makes `Stakes::stake_delegations()` and the
/// `StakeAccount` accessors `pub(crate)`, so we can no longer read the stakes
/// cache from outside the runtime crate. Instead we rebuild the delegation set
/// by scanning stake-program accounts. Active stake is computed exactly as the
/// stakes cache does (`Delegation::stake` with the current epoch, stake history,
/// and warmup/cooldown rate epoch), and zero-active delegations are dropped —
/// matching the previous `group_delegations_by_voter_pubkey_active_stake`.
fn collect_stake_delegation(
    map: &mut std::collections::HashMap<Pubkey, Vec<Delegation>>,
    account: &AccountSharedData,
    stake_account_pubkey: &Pubkey,
    epoch: Epoch,
    stake_history: &StakeHistory,
    new_rate_epoch: Option<Epoch>,
) {
    if account.owner() != &solana_stake_interface::program::id() {
        return;
    }
    // `from_account` is sysvar-only upstream, so decode the on-chain stake state
    // directly via bincode (the canonical `StakeStateV2` wire format).
    let Ok(StakeStateV2::Stake(meta, stake, _flags)) =
        bincode::deserialize::<StakeStateV2>(account.data())
    else {
        return;
    };
    let active_stake = stake
        .delegation
        .stake(epoch, stake_history, new_rate_epoch);
    if active_stake == 0 {
        return;
    }
    map.entry(stake.delegation.voter_pubkey)
        .or_default()
        .push(Delegation {
            stake_account_pubkey: *stake_account_pubkey,
            staker_pubkey: meta.authorized.staker,
            withdrawer_pubkey: meta.authorized.withdrawer,
            lamports_delegated: active_stake,
        });
}

/// Updates given map with new entry mapping withdraw authority to manager authority
/// if account is a StakePool.
fn update_stake_pool_voter_map(
    stake_pool_voter_map: &mut HashMap<AnchorPubkey, AnchorPubkey>,
    account: &AccountSharedData,
    stake_pool_pubkey: &Pubkey,
) {
    if to_anchor_pubkey(*account.owner()) != spl_stake_pool::id() {
        return;
    }

    // Check discriminator: first byte should be 1 (AccountType::StakePool)
    let data = account.data();
    if data.is_empty() || data[0] != AccountType::StakePool as u8 {
        return;
    }

    if let Ok(stake_pool) = StakePool::deserialize(&mut &account.data()[..]) {
        let (withdraw_authority, _) = find_withdraw_authority_program_address(
            &spl_stake_pool::id(),
            &to_anchor_pubkey(*stake_pool_pubkey),
        );
        if stake_pool.manager == AnchorPubkey::default() {
            return;
        }

        stake_pool_voter_map.insert(withdraw_authority, stake_pool.manager);
    }
}

/// Creates a MetaMerkleSnapshot from the given bank.
pub fn generate_meta_merkle_snapshot(bank: &Arc<Bank>) -> Result<MetaMerkleSnapshot, Error> {
    assert!(bank.is_frozen());

    println!("Bank loaded for epoch: {:?}", bank.epoch());

    // Pre-process: Find all Stake Pools and map withdraw_authority to their voting wallet
    // (StakePool manager by default)
    let mut stake_pool_voter_map: HashMap<AnchorPubkey, AnchorPubkey> = HashMap::new();

    // Maps Marinade LST stake pool withdraw authority to its ops wallet.
    stake_pool_voter_map.insert(MARINADE_WITHDRAW_AUTHORITY, MARINADE_OPS_VOTING_WALLET);

    // Epoch/stake-history context needed to compute each delegation's active
    // stake during the single account scan below.
    let stake_history =
        from_account::<StakeHistory, _>(&bank.get_account(&stake_history::id()).unwrap()).unwrap();
    let epoch = bank.epoch();
    let new_rate_epoch = bank.new_warmup_cooldown_rate_epoch();

    // Delegations grouped by voter (validator) pubkey, rebuilt from stake-program
    // accounts (see `collect_stake_delegation`).
    let mut voter_pubkey_to_delegations: std::collections::HashMap<Pubkey, Vec<Delegation>> =
        std::collections::HashMap::new();

    // Scan the indexed stake pool program accounts
    let stake_pool_accounts = bank.get_filtered_indexed_accounts(
        &IndexKey::ProgramId(STAKE_POOL_PROGRAM_ID),
        |_| true,
        None,
    )?;
    for (pubkey, account) in &stake_pool_accounts {
        update_stake_pool_voter_map(&mut stake_pool_voter_map, account, pubkey);
    }
    println!("Stake Pools Count: {}", stake_pool_voter_map.len());

    // Scan the stake program accounts
    let stake_accounts = bank.get_filtered_indexed_accounts(
        &IndexKey::ProgramId(solana_stake_interface::program::id()),
        |_| true,
        None,
    )?;
    for (pubkey, account) in &stake_accounts {
        collect_stake_delegation(
            &mut voter_pubkey_to_delegations,
            account,
            pubkey,
            epoch,
            &stake_history,
            new_rate_epoch,
        );
    }

    let mut vote_accounts_count = 0;
    let mut stake_account_count = 0;

    // 1. Generate leaf nodes for MetaMerkleTree.
    let (meta_merkle_leaves, stake_merkle_leaves_collection) = voter_pubkey_to_delegations
        .iter()
        .filter_map(|(voter_pubkey, delegations)| {
            // Track total stake delegated to this vote account across all stake accounts.
            let mut vote_account_stake = 0;

            // 1. Create leaf nodes for StakeMerkleTree.
            let mut stake_merkle_leaves = delegations
                .iter()
                .map(|delegation| {
                    let mut voting_wallet = to_anchor_pubkey(delegation.withdrawer_pubkey);

                    // Overwrite voting wallet if stake account has a withdraw authority that is
                    // mapped to a different wallet. Otherwise, use the withdrawer authority.
                    if let Some(manager) = stake_pool_voter_map.get(&voting_wallet) {
                        voting_wallet = *manager;
                    }

                    vote_account_stake += delegation.lamports_delegated;
                    stake_account_count += 1;
                    StakeMerkleLeaf {
                        voting_wallet,
                        stake_account: to_anchor_pubkey(delegation.stake_account_pubkey),
                        active_stake: delegation.lamports_delegated,
                    }
                })
                .collect::<Vec<StakeMerkleLeaf>>();

            // 2. Sort leaves by stake account key.
            stake_merkle_leaves.sort_by_key(|leaf| leaf.stake_account);

            // 3. Build StakeMerkleTree to get a root node.
            let hashed_nodes: Vec<[u8; 32]> = stake_merkle_leaves
                .iter()
                .map(|n| n.hash().to_bytes())
                .collect();
            let stake_merkle = MerkleTree::new(&hashed_nodes[..], true);

            let voting_wallet = get_validator_identity(bank, voter_pubkey);
            if voting_wallet.is_none() {
                println!(
                    "Missing vote account {}, setting voting wallet to default",
                    voter_pubkey
                );
            }

            // 4. Build MetaMerkleLeaf using root node of StakeMerkleTree.
            let meta_merkle_leaf = MetaMerkleLeaf {
                vote_account: to_anchor_pubkey(*voter_pubkey),
                voting_wallet: to_anchor_pubkey(voting_wallet.unwrap_or_default()),
                stake_merkle_root: stake_merkle.get_root().unwrap().to_bytes(),
                active_stake: vote_account_stake,
            };

            vote_accounts_count += 1;

            Some((meta_merkle_leaf, stake_merkle_leaves))
        })
        .collect::<(Vec<MetaMerkleLeaf>, Vec<Vec<StakeMerkleLeaf>>)>();

    // 2. Sort leaves by vote account key.
    let mut combined: Vec<(MetaMerkleLeaf, Vec<StakeMerkleLeaf>)> = meta_merkle_leaves
        .into_iter()
        .zip(stake_merkle_leaves_collection)
        .collect();
    combined.sort_by_key(|(leaf, _)| leaf.vote_account);
    let (meta_merkle_leaves, stake_merkle_leaves_collection): (Vec<_>, Vec<_>) =
        combined.into_iter().unzip();

    // 3. Build MetaMerkleTree to get a root node.
    let hashed_nodes: Vec<[u8; 32]> = meta_merkle_leaves
        .iter()
        .map(|n| n.hash().to_bytes())
        .collect();
    let meta_merkle = MerkleTree::new(&hashed_nodes[..], true);

    // 4. Generate MetaMerkleLeafBundle with proof.
    let meta_merkle_bundles = meta_merkle_leaves
        .into_iter()
        .zip(stake_merkle_leaves_collection)
        .enumerate()
        .map(
            |(i, (meta_merkle_leaf, stake_merkle_leaves))| MetaMerkleLeafBundle {
                meta_merkle_leaf,
                stake_merkle_leaves,
                proof: Some(get_proof(&meta_merkle, i)),
            },
        )
        .collect();

    println!("Vote Accounts Count: {}", vote_accounts_count);
    println!("Stake Accounts Count: {}", stake_account_count);

    Ok(MetaMerkleSnapshot {
        root: meta_merkle.get_root().unwrap().to_bytes(),
        leaf_bundles: meta_merkle_bundles,
        slot: bank.slot(),
    })
}
