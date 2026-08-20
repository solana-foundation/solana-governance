use std::{str::FromStr, time::Duration};

use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, Result};
use log::info;
use ncn_snapshot::{MetaMerkleLeaf, MetaMerkleProof, StakeMerkleLeaf};
use serde::{Deserialize, Serialize};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Turn a non-success response into an actionable error. A stale operator that
/// has not uploaded the proposal's snapshot answers 404, which would otherwise
/// reach `response.json()` and surface as an opaque deserialization failure.
async fn ensure_ok(
    response: reqwest::Response,
    what: &str,
    base_url: &str,
) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!(
            "The operator API at {base_url} has no {what} for this snapshot slot (404). It may not \
             have uploaded this snapshot yet — retry, or point --operator-api-url at another operator."
        ));
    }
    Err(anyhow!(
        "The operator API at {base_url} returned {status} for the {what}."
    ))
}

/// The operator fleet is externally operated, and some endpoints have been
/// observed accepting connections without ever responding. `reqwest::get` uses a
/// client with no timeout, which leaves a validator hanging with no error at
/// all, so every request goes through a client that is bounded.
fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))
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

/// Rejects a proof whose leaf describes a vote account other than the one
/// requested. The leaf seeds the `MetaMerkleProof` PDA and is what the program
/// verifies against the consensus root, so its identity is load-bearing — a
/// mismatch would otherwise surface much later as an on-chain constraint
/// failure.
fn ensure_vote_account_matches(
    proof: &VoteAccountProofResponse,
    requested: &str,
    base_url: &str,
) -> Result<()> {
    let returned = &proof.meta_merkle_leaf.vote_account;
    if returned != requested {
        return Err(anyhow!(
            "The operator API at {base_url} returned a proof for vote account {returned} but \
             {requested} was requested."
        ));
    }
    Ok(())
}

/// Rejects a proof whose leaf describes a stake account other than the one
/// requested. See [`ensure_vote_account_matches`] for why the leaf's identity
/// matters.
fn ensure_stake_account_matches(
    proof: &StakeAccountProofResponse,
    requested: &str,
    base_url: &str,
) -> Result<()> {
    let returned = &proof.stake_merkle_leaf.stake_account;
    if returned != requested {
        return Err(anyhow!(
            "The operator API at {base_url} returned a proof for stake account {returned} but \
             {requested} was requested."
        ));
    }
    Ok(())
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

    let response = http_client()?
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to reach the operator API at {}: {}", base_url, e))?;
    let response = ensure_ok(response, "vote account proof", &base_url).await?;
    let proof: VoteAccountProofResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Malformed vote account proof from {}: {}", base_url, e))?;

    ensure_vote_account_matches(&proof, vote_account, &base_url)?;

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

    let response = http_client()?
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to reach the operator API at {}: {}", base_url, e))?;
    let response = ensure_ok(response, "stake account proof", &base_url).await?;
    let proof: StakeAccountProofResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Malformed stake account proof from {}: {}", base_url, e))?;

    ensure_stake_account_matches(&proof, stake_account, &base_url)?;

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
        return Ok(crate::constants::DEFAULT_OPERATOR_API_URL.to_string());
    }

    info!("API base URL (from config): {}", config.operator_api_url);
    Ok(config.operator_api_url)
}

/// Convert API MetaMerkleLeafData to ncn_snapshot MetaMerkleLeaf
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

/// Convert API StakeMerkleLeafData to ncn_snapshot StakeMerkleLeaf
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

/// Convert API VoteAccountSummary to ncn_snapshot MetaMerkleLeaf
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

/// Convert API StakeAccountSummary to ncn_snapshot StakeMerkleLeaf
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

/// TryFrom implementation to convert ncn_snapshot StakeMerkleLeaf to IDL-compatible StakeMerkleLeaf type
impl TryFrom<StakeMerkleLeaf> for crate::svmgov_program::types::StakeMerkleLeaf {
    type Error = anyhow::Error;

    fn try_from(ncn_snapshot_leaf: StakeMerkleLeaf) -> Result<Self, Self::Error> {
        Ok(Self {
            voting_wallet: ncn_snapshot_leaf.voting_wallet,
            stake_account: ncn_snapshot_leaf.stake_account,
            active_stake: ncn_snapshot_leaf.active_stake,
        })
    }
}

/// Convert API StakeMerkleLeafData directly to IDL-compatible StakeMerkleLeaf type
pub fn convert_stake_merkle_leaf_data_to_idl_type(
    stake_merkle_leaf_data: &StakeMerkleLeafData,
) -> Result<crate::svmgov_program::types::StakeMerkleLeaf> {
    // First convert to ncn_snapshot type, then to IDL type
    let ncn_snapshot_leaf: StakeMerkleLeaf = stake_merkle_leaf_data.try_into()?;
    ncn_snapshot_leaf.try_into()
}

/// Generate MetaMerkleProof PDA for a given consensus result and vote account
pub fn generate_meta_merkle_proof_pda(
    consensus_result_pda: &Pubkey,
    vote_account: &Pubkey,
) -> Result<Pubkey> {
    let (pda, _bump) = MetaMerkleProof::pda(consensus_result_pda, vote_account);
    Ok(pda)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(stake_merkle_root: &str) -> MetaMerkleLeafData {
        MetaMerkleLeafData {
            voting_wallet: "11111111111111111111111111111111".to_string(),
            vote_account: "11111111111111111111111111111111".to_string(),
            stake_merkle_root: stake_merkle_root.to_string(),
            active_stake: 1,
        }
    }

    /// The vote paths used to feed these strings to `Pubkey::from_str_const`, a
    /// const decoder that panics on bad input, so a malformed operator response
    /// aborted the CLI with a stack trace instead of an error a validator could
    /// act on. Every rejection below must be an `Err`, never a panic.
    #[test]
    fn a_malformed_stake_merkle_root_is_an_error_not_a_panic() {
        for bad in [
            "",
            "abc",
            "not-base58-0OIl",
            "1111111111111111111111111111111",
        ] {
            let result = MetaMerkleLeaf::try_from(&leaf(bad));
            assert!(result.is_err(), "expected an error for {bad:?}");
        }
    }

    #[test]
    fn a_valid_leaf_still_converts() {
        // 32 zero bytes in base58 — the shape a real snapshot root has.
        let ok = MetaMerkleLeaf::try_from(&leaf("11111111111111111111111111111111"));
        assert!(ok.is_ok());
        assert_eq!(ok.unwrap().active_stake, 1);
    }

    #[test]
    fn a_malformed_proof_hash_is_an_error_not_a_panic() {
        for bad in ["", "abc", "not-base58-0OIl"] {
            let result = convert_merkle_proof_strings(&[bad.to_string()]);
            assert!(result.is_err(), "expected an error for {bad:?}");
        }
    }

    #[test]
    fn an_empty_proof_is_valid() {
        // A single-leaf tree yields no sibling hashes; that is not an error.
        assert_eq!(convert_merkle_proof_strings(&[]).unwrap().len(), 0);
    }

    const OTHER_ACCOUNT: &str = "SysvarC1ock11111111111111111111111111111111";

    fn vote_account_proof(vote_account: &str) -> VoteAccountProofResponse {
        VoteAccountProofResponse {
            network: "mainnet".to_string(),
            snapshot_slot: 1,
            meta_merkle_leaf: MetaMerkleLeafData {
                vote_account: vote_account.to_string(),
                ..leaf("11111111111111111111111111111111")
            },
            meta_merkle_proof: vec![],
        }
    }

    fn stake_account_proof(stake_account: &str) -> StakeAccountProofResponse {
        StakeAccountProofResponse {
            network: "mainnet".to_string(),
            snapshot_slot: 1,
            stake_merkle_leaf: StakeMerkleLeafData {
                voting_wallet: "11111111111111111111111111111111".to_string(),
                stake_account: stake_account.to_string(),
                active_stake: 1,
            },
            stake_merkle_proof: vec![],
            vote_account: "11111111111111111111111111111111".to_string(),
        }
    }

    #[test]
    fn a_vote_account_proof_for_a_different_account_is_rejected() {
        let requested = "11111111111111111111111111111111";
        assert!(
            ensure_vote_account_matches(&vote_account_proof(requested), requested, "http://x")
                .is_ok()
        );

        let err =
            ensure_vote_account_matches(&vote_account_proof(OTHER_ACCOUNT), requested, "http://x")
                .expect_err("a proof for another account must be rejected");
        // The message has to name both accounts, or the operator cannot debug it.
        let msg = err.to_string();
        assert!(msg.contains(requested), "{msg}");
        assert!(msg.contains("SysvarC1ock"), "{msg}");
    }

    #[test]
    fn a_stake_account_proof_for_a_different_account_is_rejected() {
        let requested = "11111111111111111111111111111111";
        assert!(
            ensure_stake_account_matches(&stake_account_proof(requested), requested, "http://x")
                .is_ok()
        );

        let err = ensure_stake_account_matches(
            &stake_account_proof(OTHER_ACCOUNT),
            requested,
            "http://x",
        )
        .expect_err("a proof for another account must be rejected");
        let msg = err.to_string();
        assert!(msg.contains(requested), "{msg}");
        assert!(msg.contains("SysvarC1ock"), "{msg}");
    }

    #[test]
    fn a_wrong_length_hash_is_rejected() {
        // Decodes as valid base58 but is not 32 bytes — the case a length-blind
        // decoder would silently accept or truncate.
        let short = bs58::encode([0u8; 31]).into_string();
        assert!(convert_merkle_proof_strings(&[short]).is_err());
    }
}
