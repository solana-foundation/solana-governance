use anchor_lang::Id;
use anyhow::{Result, anyhow};

use crate::config::{Config, UserType, get_default_rpc_url};
use crate::svmgov_program::program::SvmgovProgram;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum ConfigSubcommand {
    /// Set a configuration value
    Set {
        /// Configuration key to set (network, rpc-url, operator-api-url, program-id, identity-keypair, staker-keypair)
        key: String,
        /// Value to set
        value: String,
    },
    /// Get a configuration value
    Get {
        /// Configuration key to get
        key: String,
    },
    /// Show all configuration values
    Show,
}

pub async fn handle_config_command(cmd: ConfigSubcommand) -> Result<()> {
    match cmd {
        ConfigSubcommand::Set { key, value } => handle_set(&key, &value).await,
        ConfigSubcommand::Get { key } => handle_get(&key).await,
        ConfigSubcommand::Show => handle_show().await,
    }
}

async fn handle_set(key: &str, value: &str) -> Result<()> {
    let mut config = Config::load()?;

    match key.to_lowercase().as_str() {
        "network" => {
            let network_lower = value.to_lowercase();
            if network_lower != "mainnet" && network_lower != "testnet" && network_lower != "devnet"
            {
                return Err(anyhow!(
                    "Network must be either 'mainnet', 'testnet', or 'devnet'"
                ));
            }
            config.network = network_lower.clone();

            // Always update RPC URL to network default
            let old_rpc_url = config.rpc_url.clone();
            let new_rpc_url = get_default_rpc_url(&network_lower);
            config.rpc_url = Some(new_rpc_url.clone());

            // Warn if RPC URL was changed from a custom value
            if let Some(old_url) = old_rpc_url {
                if old_url != new_rpc_url {
                    println!(
                        "⚠️  Warning: RPC URL has been updated from '{}' to '{}' (network default)",
                        old_url, new_rpc_url
                    );
                }
            }

        }
        "rpc-url" => {
            config.rpc_url = Some(value.to_string());

            // Check if the new RPC URL matches the current network default
            let current_default = get_default_rpc_url(&config.network);
            if value != current_default {
                println!(
                    "Warning: Custom RPC URL does not match the '{}' network default ({}).",
                    config.network, current_default
                );
                println!(
                    "If you've switched networks, also run: svmgov config set network <mainnet|testnet|devnet>"
                );
            }
        }
        "operator-api-url" => {
            config.operator_api_url = value.to_string();
        }
        "identity-keypair" => {
            if config.user_type != Some(UserType::Validator) {
                return Err(anyhow!(
                    "identity-keypair is only valid for validators. Use 'staker-keypair' for stakers."
                ));
            }
            config.identity_keypair_path = Some(value.to_string());
        }
        "staker-keypair" => {
            if config.user_type != Some(UserType::Staker) {
                return Err(anyhow!(
                    "staker-keypair is only valid for stakers. Use 'identity-keypair' for validators."
                ));
            }
            config.staker_keypair_path = Some(value.to_string());
        }
        _ => {
            return Err(anyhow!(
                "Unknown config key: {}. Valid keys are: network, rpc-url, operator-api-url, identity-keypair, staker-keypair",
                key
            ));
        }
    }

    config.save()?;
    println!("✓ Configuration updated: {} = {}", key, value);
    Ok(())
}

async fn handle_get(key: &str) -> Result<()> {
    let config = Config::load()?;

    let value = match key.to_lowercase().as_str() {
        "network" => config.network,
        "rpc-url" => config.get_rpc_url(),
        "operator-api-url" => {
            if config.operator_api_url.is_empty() {
                "not set".to_string()
            } else {
                config.operator_api_url
            }
        }
        "program-id" => SvmgovProgram::id().to_string(),
        "identity-keypair" => config
            .identity_keypair_path
            .unwrap_or_else(|| "not set".to_string()),
        "staker-keypair" => config
            .staker_keypair_path
            .unwrap_or_else(|| "not set".to_string()),
        "user-type" => config
            .user_type
            .map(|u| u.to_string())
            .unwrap_or_else(|| "not set".to_string()),
        _ => {
            return Err(anyhow!(
                "Unknown config key: {}. Valid keys are: network, rpc-url, operator-api-url, program-id, identity-keypair, staker-keypair, user-type",
                key
            ));
        }
    };

    println!("{} = {}", key, value);
    Ok(())
}

async fn handle_show() -> Result<()> {
    let config = Config::load()?;

    println!("Current configuration:");
    println!(
        "  user-type: {}",
        config
            .user_type
            .as_ref()
            .map(|u| u.to_string())
            .unwrap_or_else(|| "not set".to_string())
    );

    if let Some(path) = &config.identity_keypair_path {
        println!("  identity-keypair: {}", path);
    }

    if let Some(path) = &config.staker_keypair_path {
        println!("  staker-keypair: {}", path);
    }

    println!("  network: {}", config.network);
    println!("  rpc-url: {}", config.get_rpc_url());
    println!("  program-id: {} (from IDL)", SvmgovProgram::id());

    if config.operator_api_url.is_empty() {
        println!("  operator-api-url: not set (required)");
    } else {
        println!("  operator-api-url: {}", config.operator_api_url);
    }

    Ok(())
}
