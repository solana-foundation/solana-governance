use std::io::{self, Write};

use anchor_client::solana_sdk::bs58;
use anyhow::Result;
use ncn_snapshot::{
    Ballot, BallotBox, ConsensusResult, MetaMerkleProof, OperatorVote, ProgramConfig,
};
use serde_json::{json, Value};

pub fn print_json(value: &Value) -> Result<()> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, value)?;
    writeln!(stdout)?;
    Ok(())
}

pub fn program_config_json(config: &ProgramConfig) -> Value {
    json!({
        "authority": config.authority.to_string(),
        "proposed_authority": config.proposed_authority.map(|key| key.to_string()),
        "min_consensus_threshold_bps": config.min_consensus_threshold_bps,
        "tie_breaker_admin": config.tie_breaker_admin.to_string(),
        "svmgov_program_pubkey": config.svmgov_program_pubkey.to_string(),
        "whitelisted_operators": config.whitelisted_operators.iter().map(ToString::to_string).collect::<Vec<_>>(),
    })
}

fn ballot_json(ballot: &Ballot) -> Value {
    json!({
        "meta_merkle_root": bs58::encode(ballot.meta_merkle_root).into_string(),
        "snapshot_hash": bs58::encode(ballot.snapshot_hash).into_string(),
    })
}

pub fn operator_vote_json(vote: &OperatorVote) -> Value {
    json!({
        "operator": vote.operator.to_string(),
        "slot_voted": vote.slot_voted,
        "ballot_index": vote.ballot_index,
    })
}

pub fn ballot_box_json(ballot_box: &BallotBox) -> Value {
    json!({
        "bump": ballot_box.bump,
        "snapshot_slot": ballot_box.snapshot_slot,
        "epoch": ballot_box.epoch,
        "slot_created": ballot_box.slot_created,
        "slot_consensus_reached": ballot_box.slot_consensus_reached,
        "min_consensus_threshold_bps": ballot_box.min_consensus_threshold_bps,
        "vote_expiry_slot": ballot_box.vote_expiry_slot,
        "tie_breaker_consensus": ballot_box.tie_breaker_consensus,
        "winning_ballot": ballot_json(&ballot_box.winning_ballot),
        "operator_votes": ballot_box.operator_votes.iter().map(operator_vote_json).collect::<Vec<_>>(),
        "ballot_tallies": ballot_box.ballot_tallies.iter().map(|tally| json!({
            "index": tally.index,
            "tally": tally.tally,
            "ballot": ballot_json(&tally.ballot),
        })).collect::<Vec<_>>(),
        "voter_list": ballot_box.voter_list.iter().map(ToString::to_string).collect::<Vec<_>>(),
    })
}

pub fn consensus_result_json(result: &ConsensusResult) -> Value {
    json!({
        "snapshot_slot": result.snapshot_slot,
        "tie_breaker_consensus": result.tie_breaker_consensus,
        "ballot": ballot_json(&result.ballot),
    })
}

pub fn proof_json(proof: &MetaMerkleProof) -> Value {
    json!({
        "payer": proof.payer.to_string(),
        "consensus_result": proof.consensus_result.to_string(),
        "meta_merkle_leaf": {
            "vote_account": proof.meta_merkle_leaf.vote_account.to_string(),
            "voting_wallet": proof.meta_merkle_leaf.voting_wallet.to_string(),
            "stake_merkle_root": bs58::encode(proof.meta_merkle_leaf.stake_merkle_root).into_string(),
            "active_stake": proof.meta_merkle_leaf.active_stake,
        },
        "meta_merkle_proof": proof.meta_merkle_proof.iter().map(|node| bs58::encode(node).into_string()).collect::<Vec<_>>(),
        "close_timestamp": proof.close_timestamp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_client::solana_sdk::pubkey::Pubkey;
    use ncn_snapshot::{BallotTally, MetaMerkleLeaf};

    #[test]
    fn ballot_json_keeps_votes_and_tallies_with_base58_hashes() {
        let operator = Pubkey::new_unique();
        let ballot = Ballot {
            meta_merkle_root: [1; 32],
            snapshot_hash: [2; 32],
        };
        let ballot_box = BallotBox {
            bump: 255,
            epoch: 10,
            slot_created: 99,
            slot_consensus_reached: 101,
            min_consensus_threshold_bps: 6667,
            winning_ballot: ballot.clone(),
            operator_votes: vec![OperatorVote {
                operator,
                slot_voted: 101,
                ballot_index: 0,
            }],
            ballot_tallies: vec![BallotTally {
                index: 0,
                ballot,
                tally: 1,
            }],
            vote_expiry_slot: 200,
            snapshot_slot: 100,
            voter_list: vec![operator],
            tie_breaker_consensus: false,
        };
        let output = ballot_box_json(&ballot_box);
        assert_eq!(
            output["operator_votes"][0]["operator"],
            operator.to_string()
        );
        assert_eq!(output["voter_list"][0], operator.to_string());
        assert_eq!(output["ballot_tallies"][0]["tally"], 1);
        for field in ["meta_merkle_root", "snapshot_hash"] {
            let encoded = output["winning_ballot"][field].as_str().unwrap();
            assert_eq!(bs58::decode(encoded).into_vec().unwrap().len(), 32);
            assert_eq!(output["ballot_tallies"][0]["ballot"][field], encoded);
        }
    }

    #[test]
    fn proof_json_contains_usable_proof_nodes_and_exact_stake() {
        let vote_account = Pubkey::new_unique();
        let proof = MetaMerkleProof {
            payer: Pubkey::new_unique(),
            consensus_result: Pubkey::new_unique(),
            meta_merkle_leaf: MetaMerkleLeaf {
                voting_wallet: Pubkey::new_unique(),
                vote_account,
                stake_merkle_root: [3; 32],
                active_stake: u64::MAX,
            },
            meta_merkle_proof: vec![[4; 32], [5; 32]],
            close_timestamp: -1,
        };
        let output = proof_json(&proof);
        assert_eq!(
            output["meta_merkle_leaf"]["vote_account"],
            vote_account.to_string()
        );
        assert_eq!(
            output["meta_merkle_leaf"]["active_stake"].as_u64(),
            Some(u64::MAX)
        );
        let nodes: Vec<Vec<u8>> = output["meta_merkle_proof"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| bs58::decode(node.as_str().unwrap()).into_vec().unwrap())
            .collect();
        assert_eq!(nodes, vec![vec![4; 32], vec![5; 32]]);
        assert_eq!(output["close_timestamp"], -1);
    }
}
