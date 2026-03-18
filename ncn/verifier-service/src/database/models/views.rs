//! API response view models

use serde::{Deserialize, Serialize};

/// View of VoteAccountRecord for summary endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteAccountSummary {
    pub vote_account: String,
    pub active_stake: u64,
}

/// View of StakeAccountRecord for summary endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeAccountSummary {
    pub stake_account: String,
    pub vote_account: String,
    pub active_stake: u64,
}
