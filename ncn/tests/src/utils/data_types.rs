use anchor_client::solana_sdk::{pubkey::Pubkey, signature::Keypair};
use cli::MetaMerkleSnapshot;

pub struct ProgramTestContext {
    pub payer: Keypair,
    pub program_config_pda: Pubkey,
    pub operators: Vec<Keypair>,
    pub meta_merkle_snapshot: MetaMerkleSnapshot,
    pub snapshot_slot: u64,
}
