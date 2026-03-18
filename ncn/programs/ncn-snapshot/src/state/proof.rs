use anchor_lang::{
    prelude::*,
    solana_program::hash::{hashv, Hash},
};

#[account]
#[derive(Debug)]
pub struct MetaMerkleProof {
    /// Payer wallet
    pub payer: Pubkey,
    /// ConsensusResult proof is created for.
    pub consensus_result: Pubkey,
    /// Meta merkle leaf
    pub meta_merkle_leaf: MetaMerkleLeaf,
    /// Meta merkle proof
    pub meta_merkle_proof: Vec<[u8; 32]>,
    /// Timestamp after which MetaMerkleProof can be closed permissionlessly.
    /// This is selected by the payer but our recommendation is to set to vote expiry time.
    pub close_timestamp: i64,
}

impl MetaMerkleProof {
    pub fn pda(consensus_result: &Pubkey, vote_account: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                b"MetaMerkleProof",
                consensus_result.as_ref(),
                vote_account.as_ref(),
            ],
            &crate::ID,
        )
    }

    pub fn init_space(meta_merkle_proof: Vec<[u8; 32]>) -> usize {
        72 + MetaMerkleLeaf::INIT_SPACE + 4 + 32 * meta_merkle_proof.len()
    }
}

#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize, InitSpace, PartialEq)]
pub struct MetaMerkleLeaf {
    /// Wallet designated for governance voting for the vote account.
    pub voting_wallet: Pubkey,
    /// Validator's vote account.
    pub vote_account: Pubkey,
    /// Root hash of the StakeMerkleTree, representing all active stake accounts
    /// delegated to the current vote account.
    pub stake_merkle_root: [u8; 32],
    /// Total active delegated stake under this vote account.
    pub active_stake: u64,
}

impl MetaMerkleLeaf {
    pub fn hash(&self) -> Hash {
        hashv(&[
            &self.voting_wallet.to_bytes(),
            &self.vote_account.to_bytes(),
            &self.stake_merkle_root,
            &self.active_stake.to_le_bytes(),
        ])
    }
}

#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct StakeMerkleLeaf {
    /// Wallet designated for governance voting for the stake account.
    pub voting_wallet: Pubkey,
    /// The stake account address.
    pub stake_account: Pubkey,
    /// Active delegated stake amount.
    pub active_stake: u64,
}

impl StakeMerkleLeaf {
    pub fn hash(&self) -> Hash {
        hashv(&[
            &self.voting_wallet.to_bytes(),
            &self.stake_account.to_bytes(),
            &self.active_stake.to_le_bytes(),
        ])
    }
}
