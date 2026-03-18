// Default RPC endpoints
pub const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
pub const DEFAULT_WSS_URL: &str = "wss://api.mainnet-beta.solana.com";

// Network-specific default RPC URLs
pub const DEFAULT_MAINNET_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
pub const DEFAULT_TESTNET_RPC_URL: &str = "https://api.testnet.solana.com";

// Network-specific default program IDs
// Note: These are the governance program IDs. Update when mainnet is deployed.
pub const DEFAULT_MAINNET_PROGRAM_ID: &str = "EKwRPoyRactBV2z2XhUSVU1YbZuyTVq4kU5U5dM2JyZY"; // Using testnet ID until mainnet is deployed
pub const DEFAULT_TESTNET_PROGRAM_ID: &str = "EKwRPoyRactBV2z2XhUSVU1YbZuyTVq4kU5U5dM2JyZY";
pub const DEFAULT_DEVNET_PROGRAM_ID: &str = "GoVpHPV3EY89hwKJjfw19jTdgMsGKG4UFSE2SfJqTuhc";

// Voting constants
pub const BASIS_POINTS_TOTAL: u64 = 10_000;

// UI constants
pub const SPINNER_TICK_DURATION_MS: u64 = 100;

// Environment variable names
pub const SVMGOV_KEY_ENV: &str = "SVMGOV_KEY";
pub const SVMGOV_RPC_ENV: &str = "SVMGOV_RPC";

pub const DISCUSSION_EPOCHS: u64 = 3;
pub const VOTING_EPOCHS: u64 = 3;
pub const SNAPSHOT_EPOCH_EXTENSION: u64 = 1;
