use anchor_client::{solana_sdk::signature::Signature, ClientError};

pub fn assert_client_err(res: Result<Signature, ClientError>, msg: &str) {
    assert!(res.unwrap_err().to_string().contains(msg))
}
