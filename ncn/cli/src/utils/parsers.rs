use anchor_client::solana_sdk::pubkey::Pubkey;
use solana_sdk::bs58;
use std::str::FromStr;

/// Parse a string in base58 format to a Pubkey.
pub fn parse_pubkey(s: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(s).map_err(|e| format!("invalid pubkey: {e}"))
}

/// Parse a string in base58 format to a 32-byte array.
pub fn parse_base_58_32(s: &str) -> Result<[u8; 32], String> {
    let bytes = bs58::decode(s)
        .into_vec()
        .map_err(|e| format!("Invalid base58: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("Expected 32 bytes, got {}", bytes.len()));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

pub fn parse_log_type(s: &str) -> Result<LogType, String> {
    match s.to_lowercase().as_str() {
        "program-config" => Ok(LogType::ProgramConfig),
        "ballot-box" => Ok(LogType::BallotBox),
        "consensus-result" => Ok(LogType::ConsensusResult),
        "proof" => Ok(LogType::MetaMerkleProof),
        _ => Err(format!("invalid log type: {}", s)),
    }
}

#[derive(Clone, Debug)]
pub enum LogType {
    ProgramConfig,
    BallotBox,
    ConsensusResult,
    MetaMerkleProof,
}

// Snapshot filename parsers
pub fn parse_full_snapshot_start_slot(name: &str) -> Option<u64> {
    if !name.starts_with("snapshot-") || !name.ends_with(".tar.zst") {
        return None;
    }
    let trimmed = &name["snapshot-".len()..name.len() - ".tar.zst".len()];
    let mut parts = trimmed.splitn(2, '-');
    let start = parts.next()?;
    start.parse::<u64>().ok()
}

pub fn parse_incremental_snapshot_slots(name: &str) -> Option<(u64, u64)> {
    if !name.starts_with("incremental-snapshot-") || !name.ends_with(".tar.zst") {
        return None;
    }
    let trimmed = &name["incremental-snapshot-".len()..name.len() - ".tar.zst".len()];
    let mut parts = trimmed.splitn(3, '-');
    let start = parts.next()?.parse::<u64>().ok()?;
    let end = parts.next()?.parse::<u64>().ok()?;
    Some((start, end))
}
