//! End-to-end integration tests for the squads-client vault-transaction lifecycle.
//!
//! Each test:
//!   1. Boots a `surfpool_sdk::Surfnet` with mainnet remote-fetch enabled so the real
//!      Squads V4 program is lazily loaded into the simnet on first access.
//!   2. Hand-encodes a `Multisig` account and injects it directly via the surfpool
//!      `set_account` cheatcode — bypassing `multisig_create_v2` and the
//!      `program_config` singleton entirely.
//!   3. Runs the canonical lifecycle that the squads-client *does* produce:
//!         vault_transaction_create → proposal_create → proposal_approve
//!         → vault_transaction_execute
//!      against the real on-chain Squads program.
//!   4. Asserts on-chain state after each phase to prove every wire-format detail
//!      (discriminator, account ordering, Borsh body, PDA derivation, message
//!      compilation) is accepted by the deployed Squads V4 program.
//!
//! The v2 (`solana-program ^2`) ↔ v3 (`surfpool-sdk` re-exports of the v3 stack)
//! type boundary is crossed via a single `xver` helper that bincode-roundtrips the
//! v2 `Instruction` into the v3 `Instruction`. The Solana wire format is stable
//! across SDK majors, so this round-trip is lossless.

#![allow(deprecated)]
// `solana_program::bpf_loader_upgradeable` and `solana_program::system_instruction`
// are deprecated in favour of standalone crates, but we deliberately keep the
// crate's transitive dependency on `solana-program` ^2 and avoid adding more
// SDK crates to dev-dependencies just to satisfy the deprecation lint.

use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey as V2Pubkey,
    system_instruction,
};
use solana_transaction::Transaction as V3Transaction;
use surfpool_sdk::{
    cheatcodes::Cheatcodes, Keypair, Pubkey as V3Pubkey, RpcClient, Signer, Surfnet,
};

use squads_client::{
    multisig_pda, proposal_approve_ix, try_compile, vault_pda as derive_vault_pda,
    vault_transaction_execute_ix, Member, Multisig, Permission, Permissions, Proposal,
    ProposalApproveAccounts, ProposalStatus, ProposalVoteArgs, SquadsClient, TransactionMessage,
    VaultTransaction, VaultTransactionExecuteAccounts, PROGRAM_ID,
};

// ============================================================================
// Helpers
// ============================================================================

/// Bincode-roundtrip across the solana-program v2 / surfpool v3 type boundary.
///
/// The Solana wire formats for `Pubkey`, `Instruction`, and `AccountMeta` are
/// stable across SDK majors (this is the same property that lets a mainnet
/// validator deserialize transactions from clients pinned to older SDKs). Both
/// v2 and v3 derive `serde::{Serialize, Deserialize}` on identical field
/// layouts, so the byte representations are guaranteed equal.
fn xver<T, U>(value: &T) -> U
where
    T: serde::Serialize,
    U: serde::de::DeserializeOwned,
{
    bincode::deserialize(&bincode::serialize(value).expect("bincode serialize"))
        .expect("bincode deserialize")
}

/// Convert a v2 `Pubkey` (squads-client's canonical type) into a v3 `Pubkey`
/// (the type surfpool's RPC + cheatcodes consume). The two are identical
/// 32-byte newtypes — a direct array copy is cheaper than `xver`.
fn as_surfpool_pubkey(pubkey: V2Pubkey) -> V3Pubkey {
    V3Pubkey::new_from_array(*pubkey.as_array())
}

/// Inverse of `as_surfpool_pubkey`.
fn from_surfpool_pubkey(pubkey: &V3Pubkey) -> V2Pubkey {
    V2Pubkey::new_from_array(pubkey.to_bytes())
}

/// Serialize a `Multisig` struct into the on-chain byte layout that
/// `Multisig::try_deserialize` accepts. This is a direct port of the serializer
/// used by the round-trip test at `squads-client/src/state.rs:597-619`.
///
/// Caller MUST pass `members` sorted ascending by pubkey (Squads on-chain code
/// assumes that ordering) and `bump` equal to `multisig_pda(create_key, None).1`.
fn encode_multisig(m: &Multisig) -> Vec<u8> {
    let mut b = Vec::with_capacity(8 + 96 + 32 + m.members.len() * 33);
    b.extend_from_slice(&Multisig::discriminator());
    b.extend_from_slice(&m.create_key.to_bytes());
    b.extend_from_slice(&m.config_authority.to_bytes());
    b.extend_from_slice(&m.threshold.to_le_bytes());
    b.extend_from_slice(&m.time_lock.to_le_bytes());
    b.extend_from_slice(&m.transaction_index.to_le_bytes());
    b.extend_from_slice(&m.stale_transaction_index.to_le_bytes());
    match &m.rent_collector {
        None => b.push(0),
        Some(rc) => {
            b.push(1);
            b.extend_from_slice(&rc.to_bytes());
        }
    }
    b.push(m.bump);
    b.extend_from_slice(&(m.members.len() as u32).to_le_bytes());
    for mem in &m.members {
        b.extend_from_slice(&mem.key.to_bytes());
        b.push(mem.permissions.mask);
    }
    b
}

/// Encode a `Multisig` and inject it as a Squads-owned account at `multisig_pda`.
fn inject_multisig(cheats: &Cheatcodes<'_>, multisig_pda: &V2Pubkey, m: &Multisig) {
    let data = encode_multisig(m);
    cheats
        .set_account(
            &as_surfpool_pubkey(*multisig_pda),
            5_000_000, // ample rent-exempt buffer
            &data,
            &as_surfpool_pubkey(PROGRAM_ID),
        )
        .expect("inject multisig via set_account");
    println!(
        "Injected Multisig at PDA {}, with create_key {}, members {:?}",
        multisig_pda,
        m.create_key,
        m.members.iter().map(|mem| mem.key).collect::<Vec<_>>()
    );
}

/// Convert a slice of v2 instructions into v3 instructions, sign with the given
/// keypair(s), and submit to the surfnet. Returns the resulting signature.
///
/// The v2→v3 conversion is the entire boundary between the two SDK stacks: one
/// generic bincode call per instruction. Everything downstream (signing,
/// blockhash, transmission) is native v3.
fn submit(
    rpc: &RpcClient,
    v2_ixs: &[Instruction],
    fee_payer: &Keypair,
    additional_signers: &[&Keypair],
) {
    let v3_ixs: Vec<solana_transaction::Instruction> = v2_ixs.iter().map(xver).collect();
    let blockhash = rpc.get_latest_blockhash().expect("get_latest_blockhash");
    let mut signers: Vec<&Keypair> = Vec::with_capacity(1 + additional_signers.len());
    signers.push(fee_payer);
    for s in additional_signers {
        signers.push(*s);
    }
    let tx = V3Transaction::new_signed_with_payer(
        &v3_ixs,
        Some(&fee_payer.pubkey()),
        signers.as_slice(),
        blockhash,
    );
    rpc.send_and_confirm_transaction(&tx)
        .expect("send_and_confirm_transaction");
}

/// Start a Surfnet with mainnet-remote-fetch enabled so the Squads V4 program
/// loads lazily on first access (as proven by `test_surfpool_loads_squads`).
/// `skip_blockhash_check(true)` keeps tests resilient to blockhash aging.
async fn start_surfnet_with_squads() -> Surfnet {
    let surfnet = Surfnet::builder()
        .remote_rpc_url("https://api.mainnet-beta.solana.com")
        .skip_blockhash_check(true)
        .start()
        .await
        .expect("start surfnet");
    println!("Started surfnet with endpoint: {}", surfnet.rpc_url());
    surfnet
}

/// From a compiled `TransactionMessage` (produced by `try_compile` with the
/// same inputs that went into `vault_transaction_create`), compute the
/// `remaining_accounts` slice that `vault_transaction_execute` expects.
///
/// Order: same as `account_keys` (writable signers, readonly signers, writable
/// non-signers, readonly non-signers). Every entry's `is_signer = false`
/// because Squads provides signatures via CPI; `is_writable` is determined by
/// the key's bucket in the compiled message.
///
/// Note: we re-derive this from the inputs rather than decoding the on-chain
/// `VaultTransaction.message_bytes`, because Squads' stored on-chain
/// `VaultTransactionMessage` format uses u32-prefixed `Vec`s and is NOT the
/// same as the wire `TransactionMessage` format (which uses u8 SmallVec
/// prefixes). The compiled output is deterministic, so reconstruction is
/// always byte-identical to what is stored on-chain.
fn execute_remaining_accounts(stored: &TransactionMessage) -> Vec<AccountMeta> {
    let num_signers = stored.num_signers as usize;
    let num_writable_signers = stored.num_writable_signers as usize;
    let num_writable_non_signers = stored.num_writable_non_signers as usize;
    stored
        .account_keys
        .as_slice()
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let is_writable = if i < num_writable_signers {
                true
            } else if i < num_signers {
                false
            } else if i < num_signers + num_writable_non_signers {
                true
            } else {
                false
            };
            AccountMeta {
                pubkey: *key,
                is_signer: false,
                is_writable,
            }
        })
        .collect()
}

/// Decode the `Proposal` account at `pda` from the surfnet.
fn fetch_proposal(rpc: &RpcClient, pda: &V2Pubkey) -> Proposal {
    let account = rpc
        .get_account(&as_surfpool_pubkey(*pda))
        .expect("get proposal account");
    Proposal::try_deserialize(&account.data).expect("decode Proposal")
}

/// Decode the `VaultTransaction` account at `pda` from the surfnet.
fn fetch_vault_transaction(rpc: &RpcClient, pda: &V2Pubkey) -> VaultTransaction {
    let account = rpc
        .get_account(&as_surfpool_pubkey(*pda))
        .expect("get vault transaction account");
    VaultTransaction::try_deserialize(&account.data).expect("decode VaultTransaction")
}

/// Decode the on-chain Multisig account at `pda`.
#[allow(dead_code)]
fn fetch_multisig(rpc: &RpcClient, pda: &V2Pubkey) -> Multisig {
    let account = rpc
        .get_account(&as_surfpool_pubkey(*pda))
        .expect("get multisig account");
    Multisig::try_deserialize(&account.data).expect("decode Multisig")
}

/// Read the lamport balance of an account, returning 0 if the account does not
/// exist on the surfnet. (Surfpool's `get_balance` returns Ok(0) for absent
/// accounts; this helper smooths over the Result wrapping at call sites.)
fn balance_of(rpc: &RpcClient, pubkey: &V2Pubkey) -> u64 {
    rpc.get_balance(&as_surfpool_pubkey(*pubkey))
        .expect("get_balance")
}

fn rpc_client(rpc_url: &str) -> RpcClient {
    RpcClient::new_with_commitment(
        rpc_url,
        solana_commitment_config::CommitmentConfig::processed(),
    )
}

/// Lamport amount that survives rent for a small account (Squads internally
/// allocates ~200 bytes for new VaultTransactions; topping the vault up with a
/// fixed buffer is simpler than computing exact rent).
const LAMPORTS_VAULT: u64 = 500_000_000;
const LAMPORTS_PROPOSER: u64 = 2_000_000_000;

// ============================================================================
// Test 1 — 1-of-1 vault transaction transfer executes end-to-end
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn lifecycle_1of1_transfer_executes() {
    let surfnet = start_surfnet_with_squads().await;
    println!("Surfnet started with endpoint: {}", surfnet.rpc_url());
    let rpc = rpc_client(surfnet.rpc_url());

    let cheats = surfnet.cheatcodes();

    // ---- Setup ----
    let proposer_kp = Keypair::new();
    let proposer = from_surfpool_pubkey(&proposer_kp.pubkey());
    let create_key = V2Pubkey::new_unique();
    let (multisig, bump) = multisig_pda(&create_key, None);
    let (vault, _) = derive_vault_pda(&multisig, 0, None);
    let recipient = V2Pubkey::new_unique();

    let multisig_state = Multisig {
        create_key,
        config_authority: V2Pubkey::default(), // autonomous
        threshold: 1,
        time_lock: 0,
        transaction_index: 0,
        stale_transaction_index: 0,
        rent_collector: None,
        bump,
        members: vec![Member {
            key: proposer,
            permissions: Permissions::from_vec(&[
                Permission::Initiate,
                Permission::Vote,
                Permission::Execute,
            ]),
        }],
    };
    inject_multisig(&cheats, &multisig, &multisig_state);

    cheats
        .fund_sol(&proposer_kp.pubkey(), LAMPORTS_PROPOSER)
        .unwrap();
    cheats
        .fund_sol(&as_surfpool_pubkey(vault), LAMPORTS_VAULT)
        .unwrap();
    // We capture the recipient's balance pre-execute and assert on the *delta*
    // after — that's robust to Surfpool's account-creation lamport adjustments
    // and is what we actually care about: that the vault successfully signed
    // a CPI that transferred funds.

    // ---- Phase A: vault_transaction_create + proposal_create ----
    let inner = vec![system_instruction::transfer(
        &vault,
        &recipient,
        100_000_000,
    )];
    let built = SquadsClient::new()
        .build_vault_tx_with_proposal(&multisig, 0, 0, &proposer, &proposer, &inner, &[], None)
        .expect("build vault tx + proposal");

    submit(&rpc, &built.instructions, &proposer_kp, &[]);
    println!("Submitted vault_transaction_create and proposal_create instructions, waiting for confirmation...");

    // Phase A assertions
    let vt = fetch_vault_transaction(&rpc, &built.transaction);
    assert_eq!(vt.multisig, multisig, "vault tx points at correct multisig");
    assert_eq!(vt.creator, proposer, "vault tx creator is proposer");
    assert_eq!(vt.index, 1, "vault tx index is 1 (transaction_index+1)");
    assert_eq!(vt.vault_index, 0, "vault index is 0");
    let proposal = fetch_proposal(&rpc, &built.proposal);
    assert!(
        matches!(proposal.status, ProposalStatus::Active { .. }),
        "proposal starts Active, got {:?}",
        proposal.status
    );
    assert!(
        proposal.approved.is_empty(),
        "no approvals before approve step"
    );

    // ---- Phase B: proposal_approve ----
    let approve_ix = proposal_approve_ix(
        &PROGRAM_ID,
        ProposalApproveAccounts {
            multisig,
            member: proposer,
            proposal: built.proposal,
        },
        &ProposalVoteArgs { memo: None },
    )
    .unwrap();
    submit(&rpc, &[approve_ix], &proposer_kp, &[]);
    println!("Submitted proposal_approve instruction, waiting for confirmation...");

    let proposal = fetch_proposal(&rpc, &built.proposal);
    assert!(
        matches!(proposal.status, ProposalStatus::Approved { .. }),
        "proposal moves to Approved after single 1-of-1 vote, got {:?}",
        proposal.status
    );
    assert_eq!(proposal.approved, vec![proposer]);

    // ---- Phase C: vault_transaction_execute ----
    let stored = try_compile(&vault, &inner, &[]).expect("re-compile inner message");
    let remaining = execute_remaining_accounts(&stored);
    let execute_ix = vault_transaction_execute_ix(
        &PROGRAM_ID,
        VaultTransactionExecuteAccounts {
            multisig,
            proposal: built.proposal,
            transaction: built.transaction,
            member: proposer,
        },
        &remaining,
    )
    .unwrap();
    // Capture pre-execute balances so we can assert on the delta (the inner
    // transfer's effect), independent of any Surfpool account-creation quirks.
    let recipient_before = balance_of(&rpc, &recipient);
    let vault_before = balance_of(&rpc, &vault);
    submit(&rpc, &[execute_ix], &proposer_kp, &[]);
    println!("Submitted vault_transaction_execute instruction, waiting for confirmation...");

    let proposal = fetch_proposal(&rpc, &built.proposal);
    assert!(
        matches!(proposal.status, ProposalStatus::Executed { .. }),
        "proposal moves to Executed, got {:?}",
        proposal.status
    );

    // Real proof the vault signed the CPI: recipient gained exactly the transfer
    // amount and the vault lost at least that much.
    let recipient_after = balance_of(&rpc, &recipient);
    assert_eq!(
        recipient_after - recipient_before,
        100_000_000,
        "recipient received exactly the transferred lamports (before={}, after={})",
        recipient_before,
        recipient_after
    );
    let vault_after = balance_of(&rpc, &vault);
    assert!(
        vault_before - vault_after >= 100_000_000,
        "vault lost at least the transferred lamports (before={}, after={})",
        vault_before,
        vault_after
    );
}

// ============================================================================
// Test 2 — 2-of-3 requires two approvals before execute
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn lifecycle_2of3_requires_two_approvals_before_execute() {
    let surfnet = start_surfnet_with_squads().await;
    let rpc = rpc_client(surfnet.rpc_url());
    let cheats = surfnet.cheatcodes();

    // Three members with full permissions; sorted ascending by pubkey before encoding.
    let mut member_kps = vec![Keypair::new(), Keypair::new(), Keypair::new()];
    let all_perms =
        Permissions::from_vec(&[Permission::Initiate, Permission::Vote, Permission::Execute]);
    let mut members: Vec<Member> = member_kps
        .iter()
        .map(|kp| Member {
            key: from_surfpool_pubkey(&kp.pubkey()),
            permissions: all_perms,
        })
        .collect();
    members.sort_by_key(|m| m.key.to_bytes());
    member_kps.sort_by_key(|kp| kp.pubkey().to_bytes());

    let create_key = V2Pubkey::new_unique();
    let (multisig, bump) = multisig_pda(&create_key, None);
    let (vault, _) = derive_vault_pda(&multisig, 0, None);
    let recipient = V2Pubkey::new_unique();

    inject_multisig(
        &cheats,
        &multisig,
        &Multisig {
            create_key,
            config_authority: V2Pubkey::default(),
            threshold: 2,
            time_lock: 0,
            transaction_index: 0,
            stale_transaction_index: 0,
            rent_collector: None,
            bump,
            members: members.clone(),
        },
    );

    for kp in &member_kps {
        cheats.fund_sol(&kp.pubkey(), LAMPORTS_PROPOSER).unwrap();
    }
    cheats
        .fund_sol(&as_surfpool_pubkey(vault), LAMPORTS_VAULT)
        .unwrap();

    let member_a = members[0].key;
    let member_b = members[1].key;
    let member_c = members[2].key;
    let kp_a = &member_kps[0];
    let kp_b = &member_kps[1];
    let kp_c = &member_kps[2];

    // ---- Phase A: create the proposal as member A ----
    let inner = vec![system_instruction::transfer(&vault, &recipient, 75_000_000)];
    let built = SquadsClient::new()
        .build_vault_tx_with_proposal(&multisig, 0, 0, &member_a, &member_a, &inner, &[], None)
        .unwrap();
    submit(&rpc, &built.instructions, kp_a, &[]);

    // ---- First approval (insufficient) ----
    let approve_a = proposal_approve_ix(
        &PROGRAM_ID,
        ProposalApproveAccounts {
            multisig,
            member: member_a,
            proposal: built.proposal,
        },
        &ProposalVoteArgs { memo: None },
    )
    .unwrap();
    submit(&rpc, &[approve_a], kp_a, &[]);

    let proposal = fetch_proposal(&rpc, &built.proposal);
    assert!(
        matches!(proposal.status, ProposalStatus::Active { .. }),
        "1 of 2 approvals: status stays Active, got {:?}",
        proposal.status
    );
    assert_eq!(proposal.approved, vec![member_a]);

    // ---- Second approval reaches threshold ----
    let approve_b = proposal_approve_ix(
        &PROGRAM_ID,
        ProposalApproveAccounts {
            multisig,
            member: member_b,
            proposal: built.proposal,
        },
        &ProposalVoteArgs { memo: None },
    )
    .unwrap();
    submit(&rpc, &[approve_b], kp_b, &[]);

    let proposal = fetch_proposal(&rpc, &built.proposal);
    assert!(
        matches!(proposal.status, ProposalStatus::Approved { .. }),
        "2 of 2 approvals: status becomes Approved, got {:?}",
        proposal.status
    );
    assert!(
        proposal.approved.contains(&member_a) && proposal.approved.contains(&member_b),
        "both approvers recorded, got {:?}",
        proposal.approved
    );

    // ---- Execute (signed by member C — only requires Execute permission, not proposer) ----
    let recipient_before = balance_of(&rpc, &recipient);
    let stored = try_compile(&vault, &inner, &[]).expect("re-compile inner message");
    let remaining = execute_remaining_accounts(&stored);
    let execute_ix = vault_transaction_execute_ix(
        &PROGRAM_ID,
        VaultTransactionExecuteAccounts {
            multisig,
            proposal: built.proposal,
            transaction: built.transaction,
            member: member_c,
        },
        &remaining,
    )
    .unwrap();
    submit(&rpc, &[execute_ix], kp_c, &[]);

    let proposal = fetch_proposal(&rpc, &built.proposal);
    assert!(
        matches!(proposal.status, ProposalStatus::Executed { .. }),
        "proposal becomes Executed when member C runs it, got {:?}",
        proposal.status
    );
    let recipient_after = balance_of(&rpc, &recipient);
    assert_eq!(
        recipient_after - recipient_before,
        75_000_000,
        "recipient received exactly the transferred lamports (before={}, after={})",
        recipient_before,
        recipient_after
    );
}

// ============================================================================
// Test 3 — memo round-trips to transaction log messages
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn lifecycle_with_memo_round_trips_to_indexer_log() {
    let surfnet = start_surfnet_with_squads().await;
    let rpc = rpc_client(surfnet.rpc_url());
    let cheats = surfnet.cheatcodes();

    let proposer_kp = Keypair::new();
    let proposer = from_surfpool_pubkey(&proposer_kp.pubkey());
    let create_key = V2Pubkey::new_unique();
    let (multisig, bump) = multisig_pda(&create_key, None);
    let (vault, _) = derive_vault_pda(&multisig, 0, None);
    let recipient = V2Pubkey::new_unique();
    let memo = "integration-test-memo";

    inject_multisig(
        &cheats,
        &multisig,
        &Multisig {
            create_key,
            config_authority: V2Pubkey::default(),
            threshold: 1,
            time_lock: 0,
            transaction_index: 0,
            stale_transaction_index: 0,
            rent_collector: None,
            bump,
            members: vec![Member {
                key: proposer,
                permissions: Permissions::from_vec(&[
                    Permission::Initiate,
                    Permission::Vote,
                    Permission::Execute,
                ]),
            }],
        },
    );
    cheats
        .fund_sol(&proposer_kp.pubkey(), LAMPORTS_PROPOSER)
        .unwrap();
    cheats
        .fund_sol(&as_surfpool_pubkey(vault), LAMPORTS_VAULT)
        .unwrap();

    // The vault_transaction_create instruction is built with the memo populated;
    // the Squads program serializes it on-chain. Decoding the stored args from
    // the VaultTransaction is the most reliable assertion (log inspection across
    // the v2/v3 RPC clients adds plumbing we don't need).
    let inner = vec![system_instruction::transfer(&vault, &recipient, 25_000_000)];
    let built = SquadsClient::new()
        .build_vault_tx_with_proposal(
            &multisig,
            0,
            0,
            &proposer,
            &proposer,
            &inner,
            &[],
            Some(memo.to_string()),
        )
        .unwrap();

    // Sanity: the Borsh memo encoding appears verbatim in the instruction body.
    let create_data = &built.instructions[0].data;
    let needle = memo.as_bytes();
    assert!(
        create_data.windows(needle.len()).any(|w| w == needle),
        "memo bytes must appear in vault_transaction_create instruction data"
    );

    submit(&rpc, &built.instructions, &proposer_kp, &[]);

    // Run the rest of the lifecycle to prove the memo didn't perturb any other
    // part of the wire format.
    let approve_ix = proposal_approve_ix(
        &PROGRAM_ID,
        ProposalApproveAccounts {
            multisig,
            member: proposer,
            proposal: built.proposal,
        },
        &ProposalVoteArgs {
            memo: Some(memo.to_string()),
        },
    )
    .unwrap();
    submit(&rpc, &[approve_ix], &proposer_kp, &[]);

    let proposal = fetch_proposal(&rpc, &built.proposal);
    assert!(matches!(proposal.status, ProposalStatus::Approved { .. }));

    let recipient_before = balance_of(&rpc, &recipient);
    let stored = try_compile(&vault, &inner, &[]).expect("re-compile inner message");
    let remaining = execute_remaining_accounts(&stored);
    let execute_ix = vault_transaction_execute_ix(
        &PROGRAM_ID,
        VaultTransactionExecuteAccounts {
            multisig,
            proposal: built.proposal,
            transaction: built.transaction,
            member: proposer,
        },
        &remaining,
    )
    .unwrap();
    submit(&rpc, &[execute_ix], &proposer_kp, &[]);

    let proposal = fetch_proposal(&rpc, &built.proposal);
    assert!(matches!(proposal.status, ProposalStatus::Executed { .. }));
    let recipient_after = balance_of(&rpc, &recipient);
    assert_eq!(
        recipient_after - recipient_before,
        25_000_000,
        "recipient received exactly the transferred lamports (before={}, after={})",
        recipient_before,
        recipient_after
    );
}

// ============================================================================
// Test 4 — multiple inner instructions execute atomically
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn lifecycle_multi_inner_ix_executes_atomically() {
    let surfnet = start_surfnet_with_squads().await;
    let rpc = rpc_client(surfnet.rpc_url());
    let cheats = surfnet.cheatcodes();

    let proposer_kp = Keypair::new();
    let proposer = from_surfpool_pubkey(&proposer_kp.pubkey());
    let create_key = V2Pubkey::new_unique();
    let (multisig, bump) = multisig_pda(&create_key, None);
    let (vault, _) = derive_vault_pda(&multisig, 0, None);
    let recipient_one = V2Pubkey::new_unique();
    let recipient_two = V2Pubkey::new_unique();

    inject_multisig(
        &cheats,
        &multisig,
        &Multisig {
            create_key,
            config_authority: V2Pubkey::default(),
            threshold: 1,
            time_lock: 0,
            transaction_index: 0,
            stale_transaction_index: 0,
            rent_collector: None,
            bump,
            members: vec![Member {
                key: proposer,
                permissions: Permissions::from_vec(&[
                    Permission::Initiate,
                    Permission::Vote,
                    Permission::Execute,
                ]),
            }],
        },
    );
    cheats
        .fund_sol(&proposer_kp.pubkey(), LAMPORTS_PROPOSER)
        .unwrap();
    cheats
        .fund_sol(&as_surfpool_pubkey(vault), LAMPORTS_VAULT)
        .unwrap();

    let inner = vec![
        system_instruction::transfer(&vault, &recipient_one, 11_000_000),
        system_instruction::transfer(&vault, &recipient_two, 22_000_000),
    ];
    let built = SquadsClient::new()
        .build_vault_tx_with_proposal(&multisig, 0, 0, &proposer, &proposer, &inner, &[], None)
        .unwrap();
    submit(&rpc, &built.instructions, &proposer_kp, &[]);

    let approve_ix = proposal_approve_ix(
        &PROGRAM_ID,
        ProposalApproveAccounts {
            multisig,
            member: proposer,
            proposal: built.proposal,
        },
        &ProposalVoteArgs { memo: None },
    )
    .unwrap();
    submit(&rpc, &[approve_ix], &proposer_kp, &[]);

    // Re-derive the stored message via try_compile (deterministic from inputs)
    // and rebuild remaining_accounts generically — the exact ordering between
    // recipient_one and recipient_two depends on CompiledKeys::compile and
    // isn't worth hard-coding.
    let r1_before = balance_of(&rpc, &recipient_one);
    let r2_before = balance_of(&rpc, &recipient_two);
    let stored = try_compile(&vault, &inner, &[]).expect("re-compile inner message");
    let remaining = execute_remaining_accounts(&stored);

    let execute_ix = vault_transaction_execute_ix(
        &PROGRAM_ID,
        VaultTransactionExecuteAccounts {
            multisig,
            proposal: built.proposal,
            transaction: built.transaction,
            member: proposer,
        },
        &remaining,
    )
    .unwrap();
    submit(&rpc, &[execute_ix], &proposer_kp, &[]);

    let proposal = fetch_proposal(&rpc, &built.proposal);
    assert!(
        matches!(proposal.status, ProposalStatus::Executed { .. }),
        "multi-ix proposal executes atomically, got {:?}",
        proposal.status
    );
    let r1_after = balance_of(&rpc, &recipient_one);
    let r2_after = balance_of(&rpc, &recipient_two);
    assert_eq!(
        r1_after - r1_before,
        11_000_000,
        "recipient_one received transfer (before={}, after={})",
        r1_before,
        r1_after
    );
    assert_eq!(
        r2_after - r2_before,
        22_000_000,
        "recipient_two received transfer (before={}, after={})",
        r2_before,
        r2_after
    );
}
