use agave_snapshots::snapshot_config::SnapshotConfig;
use borsh::BorshSerialize;
use cli::generate_meta_merkle_snapshot;
use ncn_ledger::ledger_utils::get_bank_from_snapshot_at_slot;
use solana_accounts_db::{
    accounts_db::AccountsDbConfig,
    accounts_index::{AccountIndex, AccountSecondaryIndexes},
};
use solana_runtime::{
    bank::Bank,
    genesis_utils::{create_genesis_config_with_vote_accounts, ValidatorVoteKeypairs},
    snapshot_bank_utils::bank_to_full_snapshot_archive,
};
use solana_sdk::signer::Signer;
use std::{collections::HashSet, fs, sync::Arc};

#[test]
fn agave_snapshot_roundtrip_preserves_governance_stake_and_proofs() {
    let temp = tempfile::tempdir().unwrap();
    let ledger = temp.path().join("ledger");
    let snapshots = temp.path().join("snapshots");
    let bank_snapshots = temp.path().join("bank-snapshots");
    let seed_accounts = temp.path().join("seed-accounts");
    let loaded_accounts = temp.path().join("loaded-accounts");
    for path in [
        &snapshots,
        &bank_snapshots,
        &seed_accounts,
        &loaded_accounts,
    ] {
        fs::create_dir_all(path).unwrap();
    }

    let validators = [
        ValidatorVoteKeypairs::new_rand(),
        ValidatorVoteKeypairs::new_rand(),
    ];
    let stakes = [1_000_000_000, 2_000_000_000];
    let genesis =
        create_genesis_config_with_vote_accounts(10_000_000_000, &validators, stakes.to_vec());
    genesis.genesis_config.write(&ledger).unwrap();
    let bank = Arc::new(Bank::new_from_genesis(
        &genesis.genesis_config,
        Arc::default(),
        vec![seed_accounts],
        None,
        AccountsDbConfig {
            account_indexes: Some(AccountSecondaryIndexes {
                indexes: HashSet::from([AccountIndex::ProgramId]),
                ..AccountSecondaryIndexes::default()
            }),
            ..AccountsDbConfig::default()
        },
        None,
        None,
        Arc::default(),
        None,
        None,
    ));
    bank.fill_bank_with_ticks_for_tests();
    Bank::calculate_and_set_block_id_for_dcou(&bank);

    let expected = generate_meta_merkle_snapshot(&bank).unwrap();
    assert_eq!(expected.leaf_bundles.len(), validators.len());
    for (validator, stake) in validators.iter().zip(stakes) {
        let leaf = &expected
            .leaf_bundles
            .iter()
            .find(|bundle| {
                bundle.meta_merkle_leaf.vote_account.to_bytes()
                    == validator.vote_keypair.pubkey().to_bytes()
            })
            .unwrap()
            .meta_merkle_leaf;
        assert_eq!(leaf.active_stake, stake);
        assert_eq!(
            leaf.voting_wallet.to_bytes(),
            validator.node_keypair.pubkey().to_bytes()
        );
    }

    bank_to_full_snapshot_archive(
        &SnapshotConfig {
            full_snapshot_archives_dir: snapshots.clone(),
            incremental_snapshot_archives_dir: snapshots.clone(),
            bank_snapshots_dir: bank_snapshots.clone(),
            use_registered_io_uring_buffers: false,
            ..SnapshotConfig::new_load_only()
        },
        &bank,
    )
    .unwrap();

    let loaded = Arc::new(
        get_bank_from_snapshot_at_slot(
            bank.slot(),
            &snapshots,
            &bank_snapshots,
            vec![loaded_accounts],
            &ledger,
        )
        .unwrap(),
    );
    assert_eq!(loaded.hash(), bank.hash());
    let actual = generate_meta_merkle_snapshot(&loaded).unwrap();
    assert_eq!(actual.try_to_vec().unwrap(), expected.try_to_vec().unwrap());
}
