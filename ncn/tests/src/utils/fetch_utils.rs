use anchor_client::{
    solana_client::rpc_config::RpcTransactionConfig,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{Keypair, Signature},
    },
    Program,
};

pub fn fetch_tx_block_details(program: &Program<&Keypair>, tx: Signature) -> (u64, i64) {
    let tx_details = program
        .rpc()
        .get_transaction_with_config(
            &tx,
            RpcTransactionConfig {
                encoding: None,
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: None,
            },
        )
        .unwrap();
    (tx_details.slot, tx_details.block_time.unwrap())
}
