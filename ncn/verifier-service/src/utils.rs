//! Shared utility functions for the verifier service

use axum::http::StatusCode;
use tracing::info;

/// Validate that the network is one of the supported values
pub fn validate_network(network: &str) -> Result<(), StatusCode> {
    match network {
        "devnet" | "testnet" | "mainnet" => Ok(()),
        _ => {
            info!(
                "Invalid network '{}'. Must be one of: devnet, testnet, mainnet",
                network
            );
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Parse an environment variable into a type implementing FromStr, with a default fallback
pub fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_network_valid() {
        assert!(validate_network("devnet").is_ok());
        assert!(validate_network("testnet").is_ok());
        assert!(validate_network("mainnet").is_ok());
    }

    #[test]
    fn test_validate_network_invalid() {
        assert!(validate_network("localnet").is_err());
        assert!(validate_network("invalid").is_err());
        assert!(validate_network("DEVNET").is_err()); // Case-sensitive
        assert!(validate_network("").is_err());
    }
}
