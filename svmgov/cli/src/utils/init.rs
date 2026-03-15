use std::path::Path;

use anyhow::{Result, anyhow};
use inquire::{Select, Text};

use crate::config::{Config, UserType};

pub async fn run_init() -> Result<()> {
    println!("Welcome to svmgov CLI setup!");
    println!();

    let mut config = Config::load().unwrap_or_default();

    // Ask user type
    let user_type_options = vec!["Validator", "Staker"];
    let user_type_choice = Select::new("Are you a validator or staker?", user_type_options.clone())
        .prompt()
        .map_err(|e| anyhow!("Failed to get user input: {}", e))?;

    let user_type = if user_type_choice == "Validator" {
        UserType::Validator
    } else {
        UserType::Staker
    };

    config.user_type = Some(user_type.clone());

    // Handle validator setup
    if user_type == UserType::Validator {
        let default_path = config.identity_keypair_path.clone().unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| {
                    h.join(".config")
                        .join("solana")
                        .join("id.json")
                        .to_string_lossy()
                        .to_string()
                })
                .unwrap_or_else(|| "~/.config/solana/id.json".to_string())
        });

        let prompt_msg = format!(
            "Enter the path to your validator identity keypair (default: {}):",
            default_path
        );
        let identity_path_input = Text::new(&prompt_msg)
            .with_help_message("Path to the JSON keypair file")
            .prompt()
            .map_err(|e| anyhow!("Failed to get input: {}", e))?;

        let identity_path = if identity_path_input.trim().is_empty() {
            default_path
        } else {
            identity_path_input
        };

        let path = Path::new(&identity_path);
        if !path.exists() {
            println!(
                "Warning: The specified keypair file does not exist: {}",
                identity_path
            );
            println!(
                "You may need to create it or update the path later using 'svmgov config set identity-keypair <path>'"
            );
        }

        config.identity_keypair_path = Some(identity_path);
    } else {
        // Handle staker setup
        let default_path = config.staker_keypair_path.clone().unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| {
                    h.join(".config")
                        .join("solana")
                        .join("id.json")
                        .to_string_lossy()
                        .to_string()
                })
                .unwrap_or_else(|| "~/.config/solana/id.json".to_string())
        });

        let prompt_msg = format!(
            "Enter the path to your staker keypair (default: {}):",
            default_path
        );
        let staker_path_input = Text::new(&prompt_msg)
            .with_help_message("Path to the JSON keypair file")
            .prompt()
            .map_err(|e| anyhow!("Failed to get input: {}", e))?;

        let staker_path = if staker_path_input.trim().is_empty() {
            default_path
        } else {
            staker_path_input
        };

        let path = Path::new(&staker_path);
        if !path.exists() {
            println!(
                "Warning: The specified keypair file does not exist: {}",
                staker_path
            );
            println!(
                "You may need to create it or update the path later using 'svmgov config set staker-keypair <path>'"
            );
        }

        config.staker_keypair_path = Some(staker_path);
    }

    // Ask for network preference
    let network_options = vec!["mainnet", "testnet"];
    let network_choice = Select::new(
        "Which network do you want to use by default?",
        network_options.clone(),
    )
    .with_starting_cursor(0)
    .prompt()
    .map_err(|e| anyhow!("Failed to get input: {}", e))?;

    config.network = network_choice.to_string();

    // Save config
    config.save()?;

    println!();
    println!("✓ Configuration saved successfully!");
    println!("  User type: {}", user_type);
    println!("  Network: {}", config.network);
    if let Some(path) = config.get_identity_keypair_path() {
        println!("  Keypair path: {}", path);
    }
    println!();
    println!("You can update your configuration anytime using 'svmgov config'");

    Ok(())
}
