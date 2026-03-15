use std::fs;
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use dirs::home_dir;
use serde::{Deserialize, Serialize};

use crate::constants::{
    DEFAULT_DEVNET_PROGRAM_ID, DEFAULT_MAINNET_PROGRAM_ID, DEFAULT_MAINNET_RPC_URL,
    DEFAULT_TESTNET_PROGRAM_ID, DEFAULT_TESTNET_RPC_URL,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserType {
    Validator,
    Staker,
}

impl std::fmt::Display for UserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserType::Validator => write!(f, "Validator"),
            UserType::Staker => write!(f, "Staker"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub user_type: Option<UserType>,
    pub identity_keypair_path: Option<String>,
    pub staker_keypair_path: Option<String>,
    pub network: String,
    pub rpc_url: Option<String>,
    pub operator_api_url: String,
    #[serde(default)]
    pub program_id: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            user_type: None,
            identity_keypair_path: None,
            staker_keypair_path: Some(default_staker_keypair_path()),
            network: "mainnet".to_string(),
            rpc_url: None,
            operator_api_url: String::new(),
            program_id: None,
        }
    }
}

fn default_staker_keypair_path() -> String {
    if let Some(home) = home_dir() {
        home.join(".config")
            .join("solana")
            .join("id.json")
            .to_string_lossy()
            .to_string()
    } else {
        "~/.config/solana/id.json".to_string()
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let home = home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
        Ok(home.join(".svmgov"))
    }

    pub fn config_file_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Config> {
        let config_path = Self::config_file_path()?;

        if !config_path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&config_path).map_err(|e| {
            anyhow!(
                "Failed to read config file {}: {}",
                config_path.display(),
                e
            )
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            anyhow!(
                "Failed to parse config file {}: {}",
                config_path.display(),
                e
            )
        })?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        let config_path = Self::config_file_path()?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| {
                anyhow!(
                    "Failed to create config directory {}: {}",
                    config_dir.display(),
                    e
                )
            })?;
        }

        let toml_string = toml::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize config: {}", e))?;

        fs::write(&config_path, toml_string).map_err(|e| {
            anyhow!(
                "Failed to write config file {}: {}",
                config_path.display(),
                e
            )
        })?;

        Ok(())
    }

    pub fn get_rpc_url(&self) -> String {
        self.rpc_url
            .clone()
            .unwrap_or_else(|| get_default_rpc_url(&self.network))
    }

    pub fn get_identity_keypair_path(&self) -> Option<String> {
        match &self.user_type {
            Some(UserType::Validator) => self.identity_keypair_path.clone(),
            Some(UserType::Staker) => self.staker_keypair_path.clone(),
            None => None,
        }
    }
}

pub fn get_default_rpc_url(network: &str) -> String {
    match network.to_lowercase().as_str() {
        "mainnet" => DEFAULT_MAINNET_RPC_URL.to_string(),
        "testnet" => DEFAULT_TESTNET_RPC_URL.to_string(),
        _ => DEFAULT_MAINNET_RPC_URL.to_string(),
    }
}

pub fn get_default_program_id(network: &str) -> String {
    match network.to_lowercase().as_str() {
        "mainnet" => DEFAULT_MAINNET_PROGRAM_ID.to_string(),
        "testnet" => DEFAULT_TESTNET_PROGRAM_ID.to_string(),
        "devnet" => DEFAULT_DEVNET_PROGRAM_ID.to_string(),
        _ => DEFAULT_MAINNET_PROGRAM_ID.to_string(),
    }
}
