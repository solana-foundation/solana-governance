// Structural constants that never change
pub const ANCHOR_DISCRIMINATOR: usize = 8;
pub const BASIS_POINTS_MAX: u64 = 10_000;

// Borsh size of the `delegate_signer: Option<Pubkey>` field appended to `Vote`
// and `VoteOverride`. Records created before it existed are exactly this much
// shorter and are grown by `migrate_vote_records`.
pub const DELEGATE_SIGNER_SPACE: usize = 1 + 32;

// Account sizing constants - upper bounds for Proposal account allocation
// These are NOT governance parameters; they define the max possible account size
pub const MAX_TITLE_ACCOUNT_SIZE: usize = 200;
pub const MAX_DESC_ACCOUNT_SIZE: usize = 500;

// Hard upper bound on the admin-configurable `max_supporters` cap. Every support
// call must re-tally the whole supporter list (read zero-copy from the proposal
// account) in a single transaction; 2000 staked validators is now the maximum
// after SIMD-0357: Alpenglow VAT implementation has been activated on all
// clusters.
pub const MAX_SUPPORTERS_LIMIT: u32 = 2_000;
