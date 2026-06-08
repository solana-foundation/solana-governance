use anchor_client::{solana_sdk::signature::Signature, ClientError};

pub fn assert_client_err(res: Result<Signature, ClientError>, msg: &str) {
    assert!(res.unwrap_err().to_string().contains(msg))
}

/// Asserts that `res` is an `Err` whose `Display` representation contains `msg`. Works for
/// any result/error type (used for routed commands that return `anyhow::Error`).
pub fn assert_err_contains<T, E: std::fmt::Display>(res: Result<T, E>, msg: &str) {
    match res {
        Ok(_) => panic!("expected an error containing '{}', got Ok(_)", msg),
        Err(e) => assert!(
            e.to_string().contains(msg),
            "error '{}' does not contain '{}'",
            e,
            msg
        ),
    }
}
