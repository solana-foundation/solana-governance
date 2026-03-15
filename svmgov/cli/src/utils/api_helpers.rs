use std::str::FromStr;

use anchor_lang::prelude::Pubkey;
use anyhow::{Result, anyhow};
use gov_v1::{ConsensusResult, MetaMerkleLeaf, MetaMerkleProof, StakeMerkleLeaf};
use log::info;
use serde::{Deserialize, Serialize};


/// Summary endpoint response structure (/voter/:voting_wallet)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoterSummaryResponse {
    pub network: String,
    pub snapshot_slot: u64,
    pub voting_wallet: String,
    pub vote_accounts: Vec<VoteAccountSummary>,
    pub stake_accounts: Vec<StakeAccountSummary>,
}

/// Vote account summary in voter response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteAccountSummary {
    pub vote_account: String,
    pub active_stake: u64,
}

/// Stake account summary in voter response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeAccountSummary {
    pub stake_account: String,
    pub active_stake: u64,
    pub vote_account: String,
}

/// Vote account proof endpoint response structure (/proof/vote_account/:vote_account)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteAccountProofResponse {
    pub network: String,
    pub snapshot_slot: u64,
    pub meta_merkle_leaf: MetaMerkleLeafData,
    pub meta_merkle_proof: Vec<String>,
}

/// Stake account proof endpoint response structure (/proof/stake_account/:stake_account)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeAccountProofResponse {
    pub network: String,
    pub snapshot_slot: u64,
    pub stake_merkle_leaf: StakeMerkleLeafData,
    pub stake_merkle_proof: Vec<String>,
    pub vote_account: String,
}

/// Meta merkle leaf data structure (for vote account proofs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaMerkleLeafData {
    pub voting_wallet: String,
    pub vote_account: String,
    pub stake_merkle_root: String,
    pub active_stake: u64,
}

/// Stake merkle leaf data structure (for stake account proofs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeMerkleLeafData {
    pub voting_wallet: String,
    pub stake_account: String,
    pub active_stake: u64,
}

/// Get voter summary with all vote and stake accounts
/// Endpoint: GET /voter/:voting_wallet?snapshot_slot=...
pub async fn get_voter_summary(
    wallet: &Pubkey,
    snapshot_slot: Option<u64>,
) -> Result<VoterSummaryResponse> {
    let base_url = get_api_base_url()?;
    let mut url = format!("{}/voter/{}", base_url, wallet);

    if let Some(slot) = snapshot_slot {
        url.push_str(&format!("?snapshot_slot={}", slot));
    }

    log::debug!("Fetching voter summary from: {}", url);

    let response = reqwest::get(&url).await?;
    let summary: VoterSummaryResponse = response.json().await?;

    log::debug!(
        "Got voter summary for {}: {} vote accounts, {} stake accounts",
        wallet,
        summary.vote_accounts.len(),
        summary.stake_accounts.len()
    );

    Ok(summary)
}

/// Get merkle proof for a vote account
/// Endpoint: GET /proof/vote_account/:vote_account?snapshot_slot=...
pub async fn get_vote_account_proof(
    vote_account: &str,
    snapshot_slot: u64,
    network: &str,
) -> Result<VoteAccountProofResponse> {
    let base_url = get_api_base_url()?;
    let url = format!(
        "{}/proof/vote_account/{}?slot={}&network={}",
        base_url, vote_account, snapshot_slot, network
    );

    log::debug!("Fetching vote account proof from: {}", url);

    let response = reqwest::get(&url).await?;
    info!("Response: {:?}", url);
    let proof: VoteAccountProofResponse = response.json().await?;

    log::debug!(
        "Got vote account proof: leaf stake={}, proof elements={}",
        proof.meta_merkle_leaf.active_stake,
        proof.meta_merkle_proof.len()
    );

    Ok(proof)
}

/// Get merkle proof for a stake account
/// Endpoint: GET /proof/stake_account/:stake_account?snapshot_slot=...
pub async fn get_stake_account_proof(
    stake_account: &str,
    snapshot_slot: u64,
    network: &str,
) -> Result<StakeAccountProofResponse> {
    let base_url = get_api_base_url()?;
    let url = format!(
        "{}/proof/stake_account/{}?network={}&slot={}",
        base_url, stake_account, network, snapshot_slot
    );

    log::debug!("Fetching stake account proof from: {}", url);

    let response = reqwest::get(&url).await?;
    let proof: StakeAccountProofResponse = response.json().await?;

    log::debug!(
        "Got stake account proof: leaf stake={}, proof elements={}",
        proof.stake_merkle_leaf.active_stake,
        proof.stake_merkle_proof.len()
    );

    Ok(proof)
}

/// Get the base API URL from config
fn get_api_base_url() -> anyhow::Result<String> {
    let config = crate::config::Config::load()?;

    if config.operator_api_url.is_empty() {
        anyhow::bail!(
            "operator-api-url is not set. Please run: svmgov config set operator-api-url <URL>"
        );
    }

    info!("API base URL (from config): {}", config.operator_api_url);
    Ok(config.operator_api_url)
}

/// Convert API MetaMerkleLeafData to gov_v1 MetaMerkleLeaf
impl TryFrom<&MetaMerkleLeafData> for MetaMerkleLeaf {
    type Error = anyhow::Error;

    fn try_from(api_data: &MetaMerkleLeafData) -> Result<Self, Self::Error> {
        let stake_merkle_root_bytes = bs58::decode(&api_data.stake_merkle_root)
            .into_vec()
            .map_err(|e| anyhow!("Invalid stake_merkle_root: {}", e))?;

        if stake_merkle_root_bytes.len() != 32 {
            return Err(anyhow!("stake_merkle_root must be 32 bytes"));
        }

        let mut stake_merkle_root = [0u8; 32];
        stake_merkle_root.copy_from_slice(&stake_merkle_root_bytes);

        Ok(Self {
            voting_wallet: Pubkey::from_str(&api_data.voting_wallet)
                .map_err(|e| anyhow!("Invalid voting_wallet pubkey: {}", e))?,
            vote_account: Pubkey::from_str(&api_data.vote_account)
                .map_err(|e| anyhow!("Invalid vote_account pubkey: {}", e))?,
            stake_merkle_root,
            active_stake: api_data.active_stake,
        })
    }
}

/// Convert API StakeMerkleLeafData to gov_v1 StakeMerkleLeaf
impl TryFrom<&StakeMerkleLeafData> for StakeMerkleLeaf {
    type Error = anyhow::Error;

    fn try_from(api_data: &StakeMerkleLeafData) -> Result<Self, Self::Error> {
        Ok(Self {
            voting_wallet: Pubkey::from_str(&api_data.voting_wallet)
                .map_err(|e| anyhow!("Invalid voting_wallet pubkey: {}", e))?,
            stake_account: Pubkey::from_str(&api_data.stake_account)
                .map_err(|e| anyhow!("Invalid stake_account pubkey: {}", e))?,
            active_stake: api_data.active_stake,
        })
    }
}

/// Convert API VoteAccountSummary to gov_v1 MetaMerkleLeaf
impl TryFrom<&VoteAccountSummary> for MetaMerkleLeaf {
    type Error = anyhow::Error;

    fn try_from(api_data: &VoteAccountSummary) -> Result<Self, Self::Error> {
        Ok(Self {
            voting_wallet: Pubkey::default(), // Not available in summary
            vote_account: Pubkey::from_str(&api_data.vote_account)
                .map_err(|e| anyhow!("Invalid vote_account pubkey: {}", e))?,
            stake_merkle_root: [0u8; 32], // Not available in summary
            active_stake: api_data.active_stake,
        })
    }
}

/// Convert API StakeAccountSummary to gov_v1 StakeMerkleLeaf
impl TryFrom<&StakeAccountSummary> for StakeMerkleLeaf {
    type Error = anyhow::Error;

    fn try_from(api_data: &StakeAccountSummary) -> Result<Self, Self::Error> {
        Ok(Self {
            voting_wallet: Pubkey::default(), // Not available in summary
            stake_account: Pubkey::from_str(&api_data.stake_account)
                .map_err(|e| anyhow!("Invalid stake_account pubkey: {}", e))?,
            active_stake: api_data.active_stake,
        })
    }
}

/// Helper function to convert merkle proof strings to bytes
pub fn convert_merkle_proof_strings(proof_strings: &[String]) -> Result<Vec<[u8; 32]>> {
    proof_strings
        .iter()
        .map(|s| {
            let bytes_result = bs58::decode(s).into_vec();

            let bytes = match bytes_result {
                Ok(b) => b,
                Err(e) => return Err(anyhow!("Invalid base58 merkle proof hash: {}", e)),
            };

            if bytes.len() != 32 {
                return Err(anyhow!(
                    "Merkle proof hash must be 32 bytes, got {}",
                    bytes.len()
                ));
            }

            let mut hash = [0u8; 32];
            hash.copy_from_slice(&bytes);
            Ok(hash)
        })
        .collect()
}

/// TryFrom implementation to convert gov_v1 StakeMerkleLeaf to IDL-compatible StakeMerkleLeaf type
impl TryFrom<StakeMerkleLeaf> for crate::govcontract::types::StakeMerkleLeaf {
    type Error = anyhow::Error;

    fn try_from(gov_v1_leaf: StakeMerkleLeaf) -> Result<Self, Self::Error> {
        Ok(Self {
            voting_wallet: gov_v1_leaf.voting_wallet,
            stake_account: gov_v1_leaf.stake_account,
            active_stake: gov_v1_leaf.active_stake,
        })
    }
}

/// Convert API StakeMerkleLeafData directly to IDL-compatible StakeMerkleLeaf type
pub fn convert_stake_merkle_leaf_data_to_idl_type(
    stake_merkle_leaf_data: &StakeMerkleLeafData,
) -> Result<crate::govcontract::types::StakeMerkleLeaf> {
    // First convert to gov_v1 type, then to IDL type
    let gov_v1_leaf: StakeMerkleLeaf = stake_merkle_leaf_data.try_into()?;
    gov_v1_leaf.try_into()
}

/// Generate ConsensusResult PDA for a given snapshot slot
pub fn generate_consensus_result_pda(snapshot_slot: u64) -> Result<Pubkey> {
    let (pda, _bump) = ConsensusResult::pda(snapshot_slot);
    Ok(pda)
}

/// Generate MetaMerkleProof PDA for a given consensus result and vote account
pub fn generate_meta_merkle_proof_pda(
    consensus_result_pda: &Pubkey,
    vote_account: &Pubkey,
) -> Result<Pubkey> {
    let (pda, _bump) = MetaMerkleProof::pda(consensus_result_pda, vote_account);
    Ok(pda)
}

/// Generate both ConsensusResult and MetaMerkleProof PDAs from VoteAccountProofResponse
pub fn generate_pdas_from_vote_proof_response(
    snapshot_slot: u64,
    response: &VoteAccountProofResponse,
) -> Result<(Pubkey, Pubkey)> {
    let consensus_pda = generate_consensus_result_pda(snapshot_slot)?;
    let vote_account = Pubkey::from_str(&response.meta_merkle_leaf.vote_account)
        .map_err(|e| anyhow!("Invalid vote_account pubkey in response: {}", e))?;
    let meta_proof = generate_meta_merkle_proof_pda(&consensus_pda, &vote_account)?;

    log::debug!(
        "Generated PDAs - consensus_result: {}, meta_merkle_proof: {}",
        consensus_pda,
        meta_proof
    );
    Ok((consensus_pda, meta_proof))
}
