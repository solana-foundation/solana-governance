use std::io::{self, Write};

use anchor_client::{
    solana_account_decoder::UiAccountEncoding,
    solana_client::{
        nonblocking::rpc_client::RpcClient,
        rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        rpc_filter::{Memcmp, RpcFilterType},
    },
    solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey},
};
use anchor_lang::{AccountDeserialize, Discriminator};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

use crate::{
    constants::DEFAULT_RPC_URL,
    svmgov_program::{
        self,
        accounts::{GlobalConfig, Proposal, Vote, VoteOverride},
    },
    utils::{
        phase::{PhaseInputs, ProposalPhase},
        utils::derive_global_config_pda,
    },
};

const IDL: &str = include_str!("../../idls/svmgov_program.json");

// Only fixed Borsh fields can be used to derive RPC byte offsets. Fail closed
// if a future schema introduces variable-length or unsupported fields.
fn account_layout(idl: &Value, name: &str, discriminator_len: usize) -> Result<(usize, usize)> {
    let fields = idl["types"]
        .as_array()
        .context("IDL types missing")?
        .iter()
        .find(|ty| ty["name"] == name)
        .and_then(|ty| ty["type"]["fields"].as_array())
        .with_context(|| format!("IDL fields missing for {name}"))?;
    let mut size = discriminator_len;
    let mut proposal_offset = None;
    for field in fields {
        if field["name"] == "proposal" {
            ensure!(
                field["type"] == "pubkey",
                "{name}.proposal must be a public key"
            );
            proposal_offset = Some(size);
        }
        size += match field["type"].as_str() {
            Some("pubkey") => 32,
            Some("u64" | "i64") => 8,
            Some("u32" | "i32") => 4,
            Some("u16" | "i16") => 2,
            Some("u8" | "i8" | "bool") => 1,
            Some("u128" | "i128") => 16,
            _ => bail!("Unsupported IDL field type in {name}: {}", field["type"]),
        };
    }
    Ok((size, proposal_offset.context("IDL proposal field missing")?))
}

async fn fetch_votes<T: AccountDeserialize + Discriminator>(
    rpc: &RpcClient,
    idl: &Value,
    name: &str,
    proposal: Pubkey,
) -> Result<Vec<T>> {
    let (size, offset) = account_layout(idl, name, T::DISCRIMINATOR.len())?;
    let accounts = rpc
        .get_program_accounts_with_config(
            &svmgov_program::ID,
            RpcProgramAccountsConfig {
                filters: Some(vec![
                    RpcFilterType::DataSize(size as u64),
                    RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, T::DISCRIMINATOR.to_vec())),
                    RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                        offset,
                        proposal.to_bytes().to_vec(),
                    )),
                ]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(CommitmentConfig::finalized()),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await
        .with_context(|| format!("Failed to fetch {name} accounts"))?;
    accounts
        .into_iter()
        .map(|(key, account)| {
            ensure!(
                account.owner == svmgov_program::ID,
                "Unexpected owner for {key}"
            );
            ensure!(
                account.data.len() == size,
                "Unexpected account size for {key}"
            );
            T::try_deserialize(&mut account.data.as_slice())
                .with_context(|| format!("Failed to decode {name} account {key}"))
        })
        .collect()
}

async fn fetch_account<T: AccountDeserialize>(rpc: &RpcClient, key: &Pubkey) -> Result<T> {
    let account = rpc.get_account(key).await?;
    ensure!(
        account.owner == svmgov_program::ID,
        "Unexpected account owner for {key}"
    );
    T::try_deserialize(&mut account.data.as_slice()).context("Failed to decode program account")
}

#[derive(Debug, Default, PartialEq)]
struct Totals {
    categories: [u128; 3],
    stake: u128,
    overridden: u128,
    count: usize,
}

impl Totals {
    fn add(&mut self, categories: [u64; 3], stake: u64, overridden: u64) {
        for (total, amount) in self.categories.iter_mut().zip(categories) {
            *total += u128::from(amount);
        }
        self.stake += u128::from(stake);
        self.overridden += u128::from(overridden);
        self.count += 1;
    }

    fn sum(&self) -> u128 {
        self.categories.iter().sum()
    }

    fn output(&self) -> Value {
        let categories: Vec<_> = self
            .categories
            .iter()
            .map(|&amount| {
                json!({
                    "lamports": amount.to_string(),
                    "sol": sol(amount),
                    "percent": percent(amount, self.sum()),
                })
            })
            .collect();
        json!({
            "for": categories[0], "against": categories[1], "abstain": categories[2],
            "account_count": self.count,
            "stake_lamports": self.stake.to_string(),
            "stake_minus_vote_lamports": (self.stake as i128 - self.overridden as i128 - self.sum() as i128).to_string(),
        })
    }
}

fn aggregate(
    proposal_key: Pubkey,
    proposal: &Proposal,
    votes: &[Vote],
    overrides: &[VoteOverride],
) -> Result<(Totals, Totals)> {
    let mut validators = Totals::default();
    for vote in votes {
        ensure!(
            vote.proposal == proposal_key,
            "Validator vote belongs to another proposal"
        );
        ensure!(
            vote.override_lamports <= vote.stake,
            "Validator overridden stake exceeds original stake"
        );
        validators.add(
            [
                vote.for_votes_lamports,
                vote.against_votes_lamports,
                vote.abstain_votes_lamports,
            ],
            vote.stake,
            vote.override_lamports,
        );
    }
    let mut stakers = Totals::default();
    for vote in overrides {
        ensure!(
            vote.proposal == proposal_key,
            "Staker override belongs to another proposal"
        );
        stakers.add(
            [
                vote.for_votes_lamports,
                vote.against_votes_lamports,
                vote.abstain_votes_lamports,
            ],
            vote.stake_amount,
            0,
        );
    }
    for (i, expected) in [
        proposal.for_votes_lamports,
        proposal.against_votes_lamports,
        proposal.abstain_votes_lamports,
    ]
    .into_iter()
    .enumerate()
    {
        ensure!(
            validators.categories[i] + stakers.categories[i] == u128::from(expected),
            "Vote category {i} does not reconcile with the proposal; voting may have changed during the query. Retry after voting ends"
        );
    }
    ensure!(
        validators.count + stakers.count == proposal.vote_count as usize,
        "Vote account count does not reconcile with the proposal"
    );
    Ok((validators, stakers))
}

fn sol(lamports: u128) -> String {
    format!(
        "{}.{:09}",
        lamports / 1_000_000_000,
        lamports % 1_000_000_000
    )
}

fn percent(amount: u128, total: u128) -> String {
    if total == 0 {
        return "0.0000".to_owned();
    }
    let rounded = (amount * 1_000_000 + total / 2) / total;
    format!("{}.{:04}", rounded / 10_000, rounded % 10_000)
}

/// Fetches finalized proposal votes and prints an exact, reconciled stake breakdown.
/// JSON represents lamport amounts as decimal strings to preserve integer precision.
pub async fn vote_breakdown(
    rpc_url: Option<String>,
    proposal_key: Pubkey,
    json_output: bool,
) -> Result<()> {
    let idl: Value = serde_json::from_str(IDL)?;
    let rpc = RpcClient::new_with_commitment(
        rpc_url.unwrap_or_else(|| DEFAULT_RPC_URL.to_owned()),
        CommitmentConfig::finalized(),
    );
    let proposal: Proposal = fetch_account(&rpc, &proposal_key).await?;
    let config: GlobalConfig =
        fetch_account(&rpc, &derive_global_config_pda(&svmgov_program::ID)).await?;
    let epoch = rpc.get_epoch_info().await?.epoch;
    // Sequential requests keep getProgramAccounts load bounded on public RPCs.
    let votes = fetch_votes::<Vote>(&rpc, &idl, "Vote", proposal_key).await?;
    let overrides = fetch_votes::<VoteOverride>(&rpc, &idl, "VoteOverride", proposal_key).await?;
    let (validators, stakers) = aggregate(proposal_key, &proposal, &votes, &overrides)?;
    let total = Totals {
        categories: std::array::from_fn(|i| validators.categories[i] + stakers.categories[i]),
        stake: validators.stake - validators.overridden + stakers.stake,
        count: validators.count + stakers.count,
        overridden: 0,
    };
    let status = ProposalPhase::new(&PhaseInputs::new(&proposal, &config), epoch).label();
    let queried = chrono::Utc::now().to_rfc3339();
    let mut output = io::stdout().lock();
    if json_output {
        serde_json::to_writer_pretty(
            &mut output,
            &json!({
                "proposal": proposal_key.to_string(), "title": proposal.title,
                "program": svmgov_program::ID.to_string(), "commitment": "finalized",
                "queried_at": queried, "status": status,
                "total": total.output(), "validators": validators.output(), "staker_overrides": stakers.output(),
                "validator_vote_overridden": {
                    "lamports": validators.overridden.to_string(), "sol": sol(validators.overridden),
                    "original_stake_lamports": validators.stake.to_string(), "original_stake_sol": sol(validators.stake),
                    "percent": percent(validators.overridden, validators.stake),
                },
            }),
        )?;
        writeln!(output)?;
    } else {
        writeln!(
            output,
            "# {} — Vote breakdown\n\n- Proposal: `{proposal_key}`\n- Data commitment: Finalized\n- Queried: {queried}\n- Status: {status}\n",
            proposal.title
        )?;
        writeln!(
            output,
            "Amounts are in SOL; percentages are within each row. Zero-total rows show 0.0000%.\n"
        )?;
        writeln!(
            output,
            "| Source | For (SOL) | Against (SOL) | Abstain (SOL) |\n| --- | ---: | ---: | ---: |"
        )?;
        for (name, row) in [
            ("Total", &total),
            ("Validators", &validators),
            ("Staker overrides", &stakers),
        ] {
            let cells: Vec<_> = row
                .categories
                .iter()
                .map(|&amount| format!("{} ({}%)", sol(amount), percent(amount, row.sum())))
                .collect();
            writeln!(
                output,
                "| {name} | {} | {} | {} |",
                cells[0], cells[1], cells[2]
            )?;
        }
        writeln!(
            output,
            "\nValidator vote overridden: {} SOL / {} SOL = {}%",
            sol(validators.overridden),
            sol(validators.stake),
            percent(validators.overridden, validators.stake)
        )?;
        writeln!(
            output,
            "\nAccount reconciliation: {} validator votes + {} staker overrides = {} votes.",
            validators.count, stakers.count, total.count
        )?;
        writeln!(
            output,
            "\nStake minus category amounts (lamports): validators {}, staker overrides {}.",
            validators.stake as i128 - validators.overridden as i128 - validators.sum() as i128,
            stakers.stake as i128 - stakers.sum() as i128
        )?;
        writeln!(
            output,
            "\nThis is the rounding remainder after allocating stake to For + Against + Abstain. \
             The program uses integer arithmetic, so fractional lamports are rounded down.\n\n\
             - Validators: original stake - overridden stake - (For + Against + Abstain).\n\
             - Staker overrides: override stake - (For + Against + Abstain).\n\n\
             Small positive remainders are expected from rounding; 1 lamport = 0.000000001 SOL."
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::AccountSerialize;

    fn empty_account<T: AccountDeserialize + Discriminator>() -> T {
        let mut data = T::DISCRIMINATOR.to_vec();
        data.resize(1024, 0);
        T::try_deserialize(&mut data.as_slice()).unwrap()
    }

    #[test]
    fn filters_track_serialized_account_layouts_and_schema_changes() {
        let idl: Value = serde_json::from_str(IDL).unwrap();
        let proposal = Pubkey::new_unique();
        let mut vote: Vote = empty_account();
        vote.proposal = proposal;
        let mut override_vote: VoteOverride = empty_account();
        override_vote.proposal = proposal;
        let mut vote_bytes = Vec::new();
        vote.try_serialize(&mut vote_bytes).unwrap();
        let mut override_bytes = Vec::new();
        override_vote.try_serialize(&mut override_bytes).unwrap();
        for (name, discriminator, bytes) in [
            ("Vote", Vote::DISCRIMINATOR, vote_bytes),
            ("VoteOverride", VoteOverride::DISCRIMINATOR, override_bytes),
        ] {
            let (size, offset) = account_layout(&idl, name, discriminator.len()).unwrap();
            assert_eq!(bytes.len(), size);
            assert_eq!(&bytes[offset..offset + 32], proposal.as_ref());
            assert_eq!(&bytes[..discriminator.len()], discriminator);
        }
        let mut changed = idl.clone();
        let fields = changed["types"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|ty| ty["name"] == "Vote")
            .unwrap()["type"]["fields"]
            .as_array_mut()
            .unwrap();
        fields.insert(0, json!({"name": "new_field", "type": "pubkey"}));
        let (size, offset) = account_layout(&changed, "Vote", Vote::DISCRIMINATOR.len()).unwrap();
        let original = account_layout(&idl, "Vote", Vote::DISCRIMINATOR.len()).unwrap();
        assert_eq!((size, offset), (original.0 + 32, original.1 + 32));
        changed["types"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|ty| ty["name"] == "Vote")
            .unwrap()["type"]["fields"][0]["type"] = json!("string");
        assert!(account_layout(&changed, "Vote", Vote::DISCRIMINATOR.len()).is_err());
    }

    #[test]
    fn reconciles_effective_votes_and_overrides_including_unvoted_validators() {
        let key = Pubkey::new_unique();
        let mut vote: Vote = empty_account();
        vote.proposal = key;
        vote.stake = 1_000;
        vote.override_lamports = 200;
        vote.for_votes_lamports = 799;
        let mut voted_override: VoteOverride = empty_account();
        voted_override.proposal = key;
        voted_override.stake_amount = 200;
        voted_override.against_votes_lamports = 200;
        let mut unvoted_override: VoteOverride = empty_account();
        unvoted_override.proposal = key;
        unvoted_override.stake_amount = 100;
        unvoted_override.abstain_votes_lamports = 99;
        let votes = [vote];
        let overrides = [voted_override, unvoted_override];
        let mut proposal: Proposal = empty_account();
        proposal.for_votes_lamports = 799;
        proposal.against_votes_lamports = 200;
        proposal.abstain_votes_lamports = 99;
        proposal.vote_count = 3;
        let (validators, stakers) = aggregate(key, &proposal, &votes, &overrides).unwrap();
        assert_eq!(validators.output()["stake_minus_vote_lamports"], "1");
        assert_eq!(stakers.output()["stake_minus_vote_lamports"], "1");
        assert_eq!(validators.overridden, 200);
        assert_eq!(percent(validators.overridden, validators.stake), "20.0000");
        assert_eq!(stakers.categories, [0, 200, 99]);
        proposal.vote_count = 2;
        assert!(aggregate(key, &proposal, &votes, &overrides).is_err());
        proposal.vote_count = 3;
        proposal.for_votes_lamports += 1;
        assert!(aggregate(key, &proposal, &votes, &overrides).is_err());
        proposal.for_votes_lamports -= 1;
        assert!(aggregate(Pubkey::new_unique(), &proposal, &votes, &overrides).is_err());
    }

    #[test]
    fn preserves_large_lamport_amounts_and_rounds_percentages_without_floats() {
        let amount = 9_007_199_254_740_993u64;
        let mut totals = Totals::default();
        totals.add([amount, 1, 0], amount + 1, 0);
        assert_eq!(totals.output()["for"]["lamports"], amount.to_string());
        assert_eq!(sol(u128::from(amount)), "9007199.254740993");
        for value in [0, 1, amount, u64::MAX] {
            let formatted = sol(u128::from(value));
            let (whole, fraction) = formatted.split_once('.').unwrap();
            assert_eq!(
                whole.parse::<u128>().unwrap() * 1_000_000_000 + fraction.parse::<u128>().unwrap(),
                u128::from(value)
            );
        }
        assert_eq!(percent(1, 3), "33.3333");
        assert_eq!(percent(2, 3), "66.6667");
        assert_eq!(percent(1, 128), "0.7813");
        assert_eq!(percent(0, 0), "0.0000");
        let proposal: Proposal = empty_account();
        assert!(aggregate(Pubkey::new_unique(), &proposal, &[], &[]).is_ok());
    }
}
