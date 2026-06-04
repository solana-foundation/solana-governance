# INTEGRATION TEST PLAN: SQUADS VAULT TRANSACTION LIFECYCLE

## Scope

End-to-end validate that the instruction bytes emitted by squads-client are accepted by the real on-chain Squads V4 program for the full vault-transaction lifecycle:

vault_transaction_create → proposal_create → proposal_approve → vault_transaction_execute

Test harness: surfpool-sdk 1.3 with mainnet remote-fetch enabled (already proven in squads-client/tests/integration.rs:11-31 — the Squads program loads on demand). State setup: hand-encoded Multisig bytes injected via Cheatcodes::set_account, bypassing multisig_create_v2 and the program_config singleton entirely. Type-version bridge: bincode round-trip (xver), not field-wise conversion.

All tests live in squads-client/tests/integration.rs (extend the existing file — do not create new ones).

## Prerequisites — minimal crate-level additions

These two items unlock the test surface. Both have value beyond testing.

### P1. Add vault_transaction_execute builder

Live location: squads-client/src/instructions.rs, after proposal_approve_ix at squads-client/src/instructions.rs:248-266. Follow the same pattern.

Required surface:

pub struct VaultTransactionExecuteAccounts {
pub multisig: Pubkey,
pub proposal: Pubkey,
pub transaction: Pubkey,
pub member: Pubkey, // signer, must hold Permission::Execute
}

pub fn vault_transaction_execute_ix(
program_id: &Pubkey,
accounts: VaultTransactionExecuteAccounts,
message_account_keys: &[AccountMeta], // remaining_accounts, in compiled-message order
) -> std::io::Result<Instruction>

Discriminator: sha256("global:vault_transaction_execute")[..8]. Args struct: empty — the body is just the 8 discriminator bytes. Fixed accounts (in order): multisig (read), proposal (mut), transaction (read), member (signer, mut). Followed by remaining_accounts: the keys from the stored TransactionMessage in the exact order they appear in TransactionMessage::account_keys (squads-client/src/message/mod.rs:60), with each entry's is_signer = false (Squads provides signatures via CPI) and is_writable matching the writability bucket the key landed in during try_compile.

Pin the discriminator in squads-client/tests/regression.rs alongside the existing three (squads-client/tests/regression.rs:20-42):

#[test]
fn instruction_discriminator_vault_transaction_execute_pinned() {
assert_eq!(
instruction_discriminator("vault_transaction_execute"),
/_ 8 bytes verified via:
python3 -c "import hashlib;
print(hashlib.sha256(b'global:vault_transaction_execute').digest()[:8].hex())" _/
);
}

### P2. Add bincode = "1" to [dev-dependencies]

squads-client/Cargo.toml:31-34 already has rand, tokio, surfpool-sdk. Add bincode = "1" so the type-version bridge has a stable serializer.

## Test-file helpers — the only shims

Append to the top of squads-client/tests/integration.rs, above test_surfpool_loads_squads.

### H1. xver<T, U> — wire-format type bridge (the one shim that matters)

/// Bincode-roundtrip across the solana-program v2 / surfpool v3 type boundary.
/// Relies on the Solana wire format being stable across SDK majors — see
/// `Pubkey`, `Instruction`, `AccountMeta`, `Message`, `Transaction` impls.
fn xver<T, U>(value: &T) -> U
where
T: serde::Serialize,
U: serde::de::DeserializeOwned,
{
bincode::deserialize(&bincode::serialize(value).unwrap()).unwrap()
}

Used at the v2→v3 boundary for Instruction (each item in built.instructions from squads-client/src/client.rs:160-167). The existing as_surfpool_pubkey at squads-client/tests/integration.rs:6-8 stays — it's faster than xver for the 32-byte primitive and already proven.

### H2. encode_multisig — direct copy from the unit-test serializer

The byte layout is already documented and round-trip-verified at squads-client/src/state.rs:597-619 (the multisig_full_roundtrip test). Lift that into a test helper:

fn encode_multisig(m: &squads_client::Multisig) -> Vec<u8> {
let mut b = vec![];
b.extend_from_slice(&squads_client::Multisig::discriminator());
b.extend_from_slice(&m.create_key.to_bytes());
b.extend_from_slice(&m.config_authority.to_bytes());
b.extend_from_slice(&m.threshold.to_le_bytes());
b.extend_from_slice(&m.time_lock.to_le_bytes());
b.extend_from_slice(&m.transaction_index.to_le_bytes());
b.extend_from_slice(&m.stale_transaction_index.to_le_bytes());
match &m.rent_collector {
None => b.push(0),
Some(rc) => { b.push(1); b.extend_from_slice(&rc.to_bytes()); }
}
b.push(m.bump);
b.extend_from_slice(&(m.members.len() as u32).to_le_bytes());
for mem in &m.members { // CALLER must pass members sorted by pubkey asc.
b.extend_from_slice(&mem.key.to_bytes());
b.push(mem.permissions.mask);
}
b
}

Invariants the caller must satisfy (verified by test setup, not by this helper):

• m.members sorted ascending by Pubkey::to_bytes() (Squads on-chain assumes this ordering).
• m.bump equals multisig_pda(&m.create_key, None).1 — the canonical PDA bump.
• m.threshold ∈ (0, num_voters].

### H3. inject_multisig — combine encoding + cheatcode

fn inject*multisig(cheats: &Cheatcodes<'*>, multisig_pda: &Pubkey, m: &squads_client::Multisig) {
let data = encode_multisig(m);
cheats.set_account(
&as_surfpool_pubkey(\*multisig_pda),
5_000_000, // generous rent-exempt buffer
&data,
&as_surfpool_pubkey(squads_client::PROGRAM_ID), // OWNED BY SQUADS V4
).unwrap();
}

### H4. submit — sign + send a v2-built instruction list against surfpool's v3 RPC

async fn submit(
⠦ Researching 1:24m · Ctrl+C to interrupt
v2_ixs: &[solana_program::instruction::Instruction],
fee_payer: &surfpool_sdk::Keypair,
additional_signers: &[&surfpool_sdk::Keypair],
) -> v3::Signature {
let v3_ixs: Vec<v3::Instruction> = v2_ixs.iter().map(xver).collect();
let blockhash = rpc.get_latest_blockhash().unwrap();
let signers: Vec<&surfpool_sdk::Keypair> =
std::iter::once(fee_payer).chain(additional_signers.iter().copied()).collect();
let tx = v3::Transaction::new_signed_with_payer(
&v3_ixs,
Some(&fee_payer.pubkey()),
&signers,
blockhash,
);
rpc.send_and_confirm_transaction(&tx).unwrap()
}

This is the entire boundary code. Three lines do the v2→v3 conversion. Everything downstream is native v3.

### H5. start_surfnet_with_squads — shared bootstrap

async fn start_surfnet_with_squads() -> surfpool_sdk::Surfnet {
surfpool_sdk::Surfnet::builder()
.remote_rpc_url("https://api.mainnet-beta.solana.com")
.start().await.unwrap()
// First reference to PROGRAM_ID lazy-fetches Squads V4 — already proven at
// squads-client/tests/integration.rs:22-31.
}

Total helper line count: 80 lines, of which 3 are the v2/v3 type bridge (xver).

## Test catalog

Lifecycle tests, in priority order. Each test sets up its own surfnet (Surfnet is cheap to spin per the SDK docs at surfpool_sdk::Surfnet::start).

### Test 1 — lifecycle_1of1_transfer_executes (the must-pass core test)

Topology: 1 member with Initiate | Vote | Execute permissions, threshold = 1, autonomous (config_authority = Pubkey::default()).

Inner instruction: system_program::transfer(vault → recipient, 100_000_000).

Setup phase:

1. surfnet = start_surfnet_with_squads().await.
2. Derive create*key = Pubkey::new_unique(), then (multisig, bump) = multisig_pda(&create_key, None) (squads-client/src/pda.rs:37-42) and
   (vault, *) = vault_pda(&multisig, 0, None) (squads-client/src/pda.rs:46-55).
3. Build a Multisig struct with transaction_index = 0, stale_transaction_index = 0, members = [Member { key: proposer, permissions: 0b111 }],
   threshold = 1, bump = bump.
4. inject_multisig(&cheats, &multisig, &m).
5. cheats.fund_sol(&proposer_kp.pubkey(), 1_000_000_000) — proposer keypair signs three transactions.
6. cheats.fund_sol(&as_surfpool_pubkey(vault), 500_000_000) — vault needs SOL to transfer from.
7. cheats.set_account(&recipient_pk, 0, &[], &system_program::ID) — explicit zero starting balance for the assertion at step 11d.

Phase A — vault_transaction_create + proposal_create (bundled by build_vault_tx_with_proposal):

1. built = SquadsClient::new().build_vault_tx_with_proposal(...) with current_transaction_index = 0, creator = rent_payer = proposer,
   inner_instructions = [system_instruction::transfer(&vault, &recipient, 100_000_000)], memo = None.
2. submit(&rpc, &built.instructions, &proposer_kp, &[]).await.

Phase A assertions:

• rpc.get*account(&xver::<*, v3::Pubkey>(&built.transaction)) returns Ok — VaultTransaction account exists.
• Decode via VaultTransaction::try_deserialize(&account.data) (squads-client/src/state.rs:477-510); assert creator == proposer, index == 1,
vault_index == 0, bump == built.transaction's canonical bump.
• Decode the Proposal PDA via Proposal::try_deserialize (squads-client/src/state.rs:399-424); assert status matches ProposalStatus::Active { ..
} and approved.is_empty().

Phase B — proposal_approve:

1. Build proposal_approve_ix (squads-client/src/instructions.rs:248-266) with member = proposer, proposal = built.proposal, no memo.
2. submit(&rpc, &[approve_ix], &proposer_kp, &[]).await.

Phase B assertions:

• Re-fetch Proposal; assert status matches ProposalStatus::Approved { .. } and approved == vec![proposer].

Phase C — vault_transaction_execute:

1. Compute the remaining_accounts from the compiled inner message. For this fixed inner instruction the list is:
   vec![
    AccountMeta::new(vault, false),               // writable, NOT signer (vault signs via CPI)
    AccountMeta::new(recipient, false),           // writable
    AccountMeta::new_readonly(system_program::ID, false),
]
1. Build vault_transaction_execute_ix (P1) with member = proposer.
1. submit(&rpc, &[execute_ix], &proposer_kp, &[]).await.

Phase C assertions (the integration test's reason to exist):

• Re-fetch Proposal; assert status matches ProposalStatus::Executed { .. }.
• rpc.get_account(&xver(&recipient)) returns lamports == 100_000_000 — proves the vault actually signed the CPI and the wrapped instruction
ran.
• rpc.get_account(&xver(&vault)) returns lamports < 500_000_000 - 100_000_000 (the difference accounts for any account rent reallocation Squads
does — assert with <=, not ==).

Why this test is enough by itself: landing this single test validates every wire-format detail in the crate today — every discriminator (squads-client/src/discriminator.rs:10-23), every PDA derivation (squads-client/src/pda.rs:29-90), every account ordering, every Borsh arg encoding (squads-client/src/instructions.rs:43-63, squads-client/src/instructions.rs:165-172, squads-client/src/instructions.rs:221-233), the entire try_compile output (squads-client/src/message/mod.rs:181-246), and the new vault_transaction_execute_ix (P1).

### Test 2 — lifecycle_2of3_requires_two_approvals_before_execute

Topology: 3 members sorted by pubkey, each with Initiate | Vote | Execute, threshold = 2.

Lifecycle:

1. Same setup as Test 1, but with members.sort_by_key(|m| m.key.to_bytes()) before encoding.
2. Submit vault_transaction_create + proposal_create signed by member-A (proposer).
3. First approve by member-A. Assert Proposal.status still Active (1 approve < threshold 2).
4. Second approve by member-B. Assert Proposal.status == Approved, approved == [A, B] (sorted; check the on-chain ordering).
5. Submit vault_transaction_execute signed by member-C. Assert success.
6. Recipient balance == transfer amount.

Why this matters: validates that the crate's proposal_approve_ix correctly identifies the voting member (squads-client/src/instructions.rs:259-265), and confirms that any member with Execute permission can submit the execute step — not just the proposer. The single-member happy path can't distinguish those code paths.

### Test 3 — lifecycle_with_memo_round_trips_to_indexer_log

Topology: 1-of-1.

Lifecycle: Same as Test 1, but pass memo = Some("integration-test-memo") to build_vault_tx_with_proposal. After Phase A's submit, call rpc.get_transaction(&signature, UiTransactionEncoding::Json) and assert the memo string appears in the transaction's log messages.

Why this matters: the memo travels through VaultTransactionCreateArgs::memo (squads-client/src/instructions.rs:39-40) → on-chain emit! (Squads's indexer event) → transaction logs. Verifies the optional Borsh encoding at squads-client/src/instructions.rs:52-60 against the on-chain reader.

### Test 4 — lifecycle_multi_inner_ix_executes_atomically

Topology: 1-of-1.

Inner instructions: Two system_program::transfer calls to distinct recipients in the same vault transaction.

Lifecycle: Same flow as Test 1. Assert both recipients received their amounts after vault_transaction_execute.

Why this matters: stresses try_compile's multi-instruction path (squads-client/src/message/mod.rs:181-246). The single-transfer Test 1 can't distinguish between "compile correctly for 1 inner ix" and "compile correctly for N inner ixs."

## Out of scope (explicitly not in this batch)

These would expand the test plan beyond the stated scope ("validity of a vault transaction lifecycle"). Document them as follow-ups in commit message, don't try to land them in the same PR:

• Negative tests (non-member proposer, missing Initiate permission, missing Vote, missing Execute, threshold underflow, transaction_index race)
— useful but they test rejection paths, not lifecycle validity. The pure-Rust verify_proposer tests at squads-client/src/client.rs:333-392
already cover the local-detection path; the integration variants would re-verify on-chain rejection.
• multisig_create_v2 + program_config_init — explicitly excluded by the "write the original squads accounts directly to the surfnet"
requirement. These need a separate test suite if/when those builders are added.
• proposal_cancel, proposal_reject — lifecycle branches not covered by the create→approve→execute happy path.
• Address-lookup-table compression — try_compile accepts ALTs (squads-client/src/message/mod.rs:181-184) but every test above passes &[]. ALT
testing requires injecting a real ALT account via set_account and is a separate exercise.
• time_lock enforcement — would require Cheatcodes::time_travel_to_timestamp calls between approve and execute. Out of scope for "validity of
lifecycle"; it tests timing, not bytes.
• Versioned (v0) transactions — every test above uses legacy Transaction. v0 with ALTs is wire-stable through xver the same way, but adds
complexity.

## Implementation ordering

The four work items have dependencies. Land them in this order so each PR is independently testable:

1. P1 + regression test pin for vault_transaction_execute discriminator. No surfpool dep yet. Add the builder + the unit-test discriminator pin
   (squads-client/tests/regression.rs). Verifiable in isolation.
2. P2 + helpers (H1–H5) into squads-client/tests/integration.rs. Lean on the existing test_surfpool_loads_squads test as proof the runtime
   works. Add the helpers but no new tests yet.
3. Test 1 (lifecycle_1of1_transfer_executes). The reason for everything above. If this lands green, the crate's wire format is validated
   against the real Squads V4 deployment.
4. Tests 2–4 can land independently in any order. Each is self-contained.

## Success criteria

The test plan is "done" when:

• All four tests run in cargo test --test integration (or cargo test -p squads-client --test integration) against a fresh local Surfnet.
• Each test completes in under 10 seconds (Surfnet startup is the dominant cost; the four-tx lifecycle itself is sub-second).
• A regression in any single byte of the crate's wire emission — a discriminator, an account-ordering swap, a Borsh field-order change — causes
at least one of these tests to fail. (Verify this manually by intentionally swapping creator and rent_payer in vault_transaction_create_ix at
squads-client/src/instructions.rs:94-95 and confirming Test 1 fails before reverting.)

## Helper-code line budget summary

┌──────────────────────────────────────────┬───────────────────────────────────┬───────────────┬─────────────────────────────────────────────┐
│ Item │ Location │ Approx. lines │ Type │
├──────────────────────────────────────────┼───────────────────────────────────┼───────────────┼─────────────────────────────────────────────┤
│ vault_transaction_execute_ix + accounts │ squads-client/src/instructions.rs │ 40 │ crate functionality │
│ struct │ │ │ │
├──────────────────────────────────────────┼───────────────────────────────────┼───────────────┼─────────────────────────────────────────────┤
│ Discriminator regression test │ squads-client/tests/regression.rs │ 8 │ crate test │
├──────────────────────────────────────────┼───────────────────────────────────┼───────────────┼─────────────────────────────────────────────┤
│ xver<T, U> │ squads-client/tests/integration.r │ 10 │ shim │
│ │ s │ │ │
├──────────────────────────────────────────┼───────────────────────────────────┼───────────────┼─────────────────────────────────────────────┤
│ as_surfpool_pubkey │ squads-client/tests/integration.r │ already │ shim │
│ │ s:6-8 │ present │ │
├──────────────────────────────────────────┼───────────────────────────────────┼───────────────┼─────────────────────────────────────────────┤
│ encode_multisig │ squads-client/tests/integration.r │ 20 │ direct port of │
│ │ s │ │ squads-client/src/state.rs:597-619 │
├──────────────────────────────────────────┼───────────────────────────────────┼───────────────┼─────────────────────────────────────────────┤
│ inject_multisig │ squads-client/tests/integration.r │ 10 │ test setup │
│ │ s │ │ │
├──────────────────────────────────────────┼───────────────────────────────────┼───────────────┼─────────────────────────────────────────────┤
│ submit │ squads-client/tests/integration.r │ 15 │ test setup │
│ │ s │ │ │
├──────────────────────────────────────────┼───────────────────────────────────┼───────────────┼─────────────────────────────────────────────┤
│ start_surfnet_with_squads │ squads-client/tests/integration.r │ 5 │ test setup │
│ │ s │ │ │
├──────────────────────────────────────────┼───────────────────────────────────┼───────────────┼─────────────────────────────────────────────┤
│ Shims subtotal │ │ 13 lines │ │
├──────────────────────────────────────────┼───────────────────────────────────┼───────────────┼─────────────────────────────────────────────┤
│ Functionality subtotal (test-file) │ │ 50 lines │ │
├──────────────────────────────────────────┼───────────────────────────────────┼───────────────┼─────────────────────────────────────────────┤
│ Crate additions │ │ 48 lines │ │
└──────────────────────────────────────────┴───────────────────────────────────┴───────────────┴─────────────────────────────────────────────┘

The "minimal shim" requirement is met: 13 lines (xver + as_surfpool_pubkey) carries the entire v2↔v3 boundary for the whole test suite. Everything else is genuine test logic or genuine crate functionality.
