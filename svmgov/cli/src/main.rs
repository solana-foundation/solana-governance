mod config;
mod constants;
mod instructions;
mod utils;

use anchor_client::anchor_lang::declare_program;
use anyhow::Result;
use clap::{Parser, Subcommand};

use config::Config;
use constants::*;
use utils::{
    commands,
    config_command::{ConfigSubcommand, handle_config_command},
    init,
    utils::*,
};

declare_program!(govcontract);

// anchor idl init --provider.cluster http://86.109.14.141:8899 --provider.wallet /path/to/wallet.json -f target/idl/my_program.json 4igPvJuaCVUCwqaQ3q7L8Y5JL5G1vsDCfLGMMoNthmSt

#[derive(Parser)]
#[command(
    name = "svmgov",
    version,
    about = "A simple CLI to help creating and voting on validator governance proposals.",
    long_about = "svmgov is a command-line tool for interacting with the Solana Validator Governance program. \
                    It allows users to create proposals, support proposals, cast votes, tally votes, and view proposals and votes.\n\n\
                    Environment variables can be used for global options: SVMGOV_KEY for --identity-keypair and SVMGOV_RPC for --rpc-url. \
                    Flags override env vars if both are provided.\n\n\
                    To get started, use one of the subcommands below. For example, to list all proposals:\n\
                    $ svmgov --rpc-url https://api.mainnet-beta.solana.com proposal \"EKwRPoyRactBV2z2XhUSVU1YbZuyTVq4kU5U5dM2JyZY\"\n\n\
                    For more information on each subcommand, use --help, e.g., `svmgov create-proposal --help`."
)]
struct Cli {
    /// Path to the identity keypair JSON file.
    /// This argument is global, meaning it can be used with any subcommand.
    #[arg(
        short,
        long,
        help = "Path to the identity keypair JSON file (or set via SVMGOV_KEY env var)",
        global = true,
        env = SVMGOV_KEY_ENV
    )]
    identity_keypair: Option<String>,

    /// Custom rpc url. This argument is also global and can be used with any subcommand.
    #[arg(
        short,
        long,
        help = "Custom rpc url (or set via SVMGOV_RPC env var)",
        global = true,
        env = SVMGOV_RPC_ENV
    )]
    rpc_url: Option<String>,

    /// Subcommands for the CLI
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(
        about = "Create a proposal to vote on",
        long_about = "This command creates a new governance proposal with the help of the Solana Validator Governance program. \
                      It requires a title and a GitHub link for the proposal description, and optionally a unique seed to derive the proposal's address (PDA). \
                      The identity keypair is required to sign the transaction, and an optional RPC URL can be provided to connect to the chain.\n\n\
                      Examples:\n\
                      $ svmgov --identity-keypair /path/to/key.json create-proposal --title \"New Governance Rule\" --description \"https://github.com/repo/proposal\"\n\
                      $ svmgov --identity-keypair /path/to/key.json --rpc-url https://api.mainnet-beta.solana.com create-proposal --seed 42 --title \"New Governance Rule\" --description \"https://github.com/repo/proposal\""
    )]
    CreateProposal {
        /// Optional unique seed for the proposal (used to derive the PDA).
        #[arg(long, help = "Unique seed for the proposal (optional)")]
        seed: Option<u64>,

        /// Title of the proposal.
        #[arg(long, help = "Proposal title")]
        title: String,

        /// GitHub link for the proposal description.
        #[arg(long, help = "GitHub link for the proposal description")]
        description: String,

        /// Network for fetching merkle proofs
        #[arg(long, help = "Network for fetching merkle proofs")]
        network: String,
    },

    #[command(
        about = "Support a proposal to vote on",
        long_about = "This command allows an eligible validator to support a governance proposal, making it available for voting. \
                      It requires the proposal ID and the validator's identity keypair to sign the transaction. \
                      An optional RPC URL can be provided to connect to the chain.\n\n\
                      Example:\n\
                      $ svmgov --identity-keypair /path/to/key.json --rpc-url https://api.mainnet-beta.solana.com support-proposal --proposal-id \"123\""
    )]
    SupportProposal {
        #[arg(long, help = "Proposal ID")]
        proposal_id: String,

        /// Network for fetching merkle proofs
        #[arg(long, help = "Network for fetching merkle proofs")]
        network: String,
    },

    #[command(
        about = "Cast a vote on a proposal",
        long_about = "This command casts a vote on a live governance proposal. \
                      Voters specify how to allocate their stake weight across 'For', 'Against', and 'Abstain' using basis points, which must sum to 10,000 (representing 100% of their stake). \
                      It requires the proposal ID and the identity keypair to sign the vote. An optional RPC URL can be provided to connect to the chain.\n\n\
                      Example:\n\
                      $ svmgov --identity-keypair /path/to/key.json --rpc-url https://api.mainnet-beta.solana.com cast-vote --proposal-id 123 --for-votes 6000 --against-votes 3000 --abstain-votes 1000"
    )]
    /// Voters submit their votes via the smart contract, specifying how they allocate their
    /// stake weight across the three options. For example, a voter with 100 SOL might assign
    /// 6,000 basis points (60%) to "for," 3,000 (30%) to "against," and 1,000 (10%) to "abstain."
    /// Each voter’s allocation must sum to 10,000 basis points (100% of their stake).
    /// svmgov --identity-keypair /path/to/key.json cast-vote --proposal-id "123" --for-votes 6000 --against-votes 3000 --abstain-votes 1000
    CastVote {
        /// Proposal ID for which the vote is being cast (proposal Pubkey).
        #[arg(long, help = "Proposal ID")]
        proposal_id: String,

        /// Basis points for 'For' vote.
        #[arg(long, help = "Basis points for 'For'")]
        for_votes: u64,

        /// Basis points for 'Against' vote.
        #[arg(long, help = "Basis points for 'Against'")]
        against_votes: u64,

        /// Basis points for 'Abstain' vote.
        #[arg(long, help = "Basis points for 'Abstain'")]
        abstain_votes: u64,

        /// Network for fetching merkle proofs
        #[arg(long, help = "Network for fetching merkle proofs")]
        network: String,
    },

    #[command(
        about = "Modify an existing vote on a proposal",
        long_about = "This command modifies an existing vote on a live governance proposal. \
                      Voters can update how they allocate their stake weight across 'For', 'Against', and 'Abstain' using basis points, which must sum to 10,000 (representing 100% of their stake). \
                      It requires the proposal ID and the identity keypair to sign the modification. An optional RPC URL can be provided to connect to the chain.\n\n\
                      Example:\n\
                      $ svmgov --identity-keypair /path/to/key.json --rpc-url https://api.mainnet-beta.solana.com modify-vote --proposal-id 123 --for-votes 7000 --against-votes 2000 --abstain-votes 1000"
    )]
    ModifyVote {
        /// Proposal ID for which the vote is being modified (proposal Pubkey).
        #[arg(long, help = "Proposal ID")]
        proposal_id: String,

        /// Basis points for 'For' vote.
        #[arg(long, help = "Basis points for 'For'")]
        for_votes: u64,

        /// Basis points for 'Against' vote.
        #[arg(long, help = "Basis points for 'Against'")]
        against_votes: u64,

        /// Basis points for 'Abstain' vote.
        #[arg(long, help = "Basis points for 'Abstain'")]
        abstain_votes: u64,

        /// Network for fetching merkle proofs
        #[arg(long, help = "Network for fetching merkle proofs")]
        network: String,
    },

    #[command(
        about = "Finalize a proposal after voting period has ended",
        long_about = "This command sends a transaction to finalize a governance proposal after its voting period has ended. \
                      It requires the proposal ID and the identity keypair to interact with the chain. \
                      An optional RPC URL can be provided to connect to the chain. \
                      The proposal must be in a finalized state (voting period ended) to be finalized.\n\n\
                      Example:\n\
                      $ svmgov --identity-keypair /path/to/key.json --rpc-url https://api.mainnet-beta.solana.com finalize-proposal --proposal-id \"123\""
    )]
    FinalizeProposal {
        /// Proposal ID to finalize.
        #[arg(long, help = "Proposal ID")]
        proposal_id: String,
    },

    #[command(
        about = "Display a proposal and its details",
        long_about = "This command retrieves and displays a governance proposal and its details from the Solana Validator Governance program. \
                      An optional RPC URL can be provided to connect to the chain; otherwise, a default URL is used.\n\n\
                      Examples:\n\
                      $ svmgov --rpc-url https://api.mainnet-beta.solana.com proposal \"EKwRPoyRactBV2z2XhUSVU1YbZuyTVq4kU5U5dM2JyZY\""
    )]
    Proposal {
        /// Proposal ID to display
        proposal_id: String,
    },

    #[command(
        about = "List all proposals",
        long_about = "This command retrieves and displays all governance proposals from the Solana Validator Governance program. \
                      An optional RPC URL can be provided to connect to the chain; otherwise, a default URL is used.\n\n\
                      Examples:\n\
                      $ svmgov list-proposals\n\
                      $ svmgov list-proposals --status active\n\
                      $ svmgov list-proposals --limit 5 --json true\n\
                      $ svmgov --rpc-url https://api.mainnet-beta.solana.com list-proposals"
    )]
    ListProposals {
        /// Filter proposals by status: active (voting), finalized, or support
        #[arg(long, help = "Filter by status: active, finalized, or support")]
        status: Option<String>,

        /// Limit the number of proposals to display
        #[arg(long, help = "Limit the number of proposals to display")]
        limit: Option<usize>,

        /// Output results in JSON format
        #[arg(long, help = "Output results in JSON format (use --json or --json true)", num_args = 0..=1, default_missing_value = "true")]
        json: Option<String>,
    },

    #[command(
        about = "Initialize the proposal index pda",
        long_about = "This command allows anyone to initialize the proposal index pda which will follow proposal creation \
                      An optional RPC URL can be provided to connect to the chain.\n\n\
                      Example:\n\
                      $ svmgov --identity-keypair /path/to/key.json --rpc-url https://api.mainnet-beta.solana.com init-index"
    )]
    InitIndex {},

    #[command(
        about = "Override validator vote with delegator vote",
        long_about = "This command allows a delegator to override their validator's vote on a proposal. \
                      The CLI fetches snapshot data from the operator API and submits the override. \
                      Requires the proposal ID and a stake account delegated by the signer. You may explicitly pass a stake \
                      account using --stake-account <PUBKEY> (base58). If omitted, the CLI selects the first stake account \
                      from the voter summary.\n\n\
                      Examples:\n\
                      # Auto-select first stake account from summary\n\
                      $ svmgov --identity-keypair /path/to/key.json cast-vote-override --proposal-id \"123\" --for-votes 6000 --against-votes 3000 --abstain-votes 1000\n\
                      # Use an explicit stake account\n\
                      $ svmgov --identity-keypair /path/to/key.json cast-vote-override --proposal-id \"123\" --for-votes 6000 --against-votes 3000 --abstain-votes 1000 --stake-account <STAKE_PUBKEY>"
    )]
    CastVoteOverride {
        /// Proposal ID for which to override the vote
        #[arg(long, help = "Proposal ID")]
        proposal_id: String,

        /// Basis points for 'For' vote
        #[arg(
            long,
            help = "Basis points for 'For' (must sum to 10,000 with other votes)"
        )]
        for_votes: u64,

        /// Basis points for 'Against' vote
        #[arg(
            long,
            help = "Basis points for 'Against' (must sum to 10,000 with other votes)"
        )]
        against_votes: u64,

        /// Basis points for 'Abstain' vote
        #[arg(
            long,
            help = "Basis points for 'Abstain' (must sum to 10,000 with other votes)"
        )]
        abstain_votes: u64,

        /// Optional specific stake account to use for override
        #[arg(
            long,
            help = "Stake account to use for override (base58 pubkey). If omitted, the first stake account from the voter summary will be used."
        )]
        stake_account: String,

        /// Network for fetching merkle proofs
        #[arg(long, help = "Network for fetching merkle proofs")]
        network: String,

        /// Staker keypair for signing the transaction
        #[arg(long, help = "Staker keypair for signing the transaction")]
        staker_keypair: String,

        /// Vote account pubkey for the validator
        #[arg(long, help = "Vote account pubkey (base58) for the validator")]
        vote_account: String,
    },

    #[command(
        about = "Modify an existing vote override on a proposal",
        long_about = "This command allows a delegator to modify their existing vote override on a proposal. \
                      The CLI fetches snapshot data from the operator API and submits the modification. \
                      Requires the proposal ID and a stake account delegated by the signer. You may explicitly pass a stake \
                      account using --stake-account <PUBKEY> (base58). If omitted, the CLI selects the first stake account \
                      from the voter summary.\n\n\
                      Examples:\n\
                      # Auto-select first stake account from summary\n\
                      $ svmgov --identity-keypair /path/to/key.json modify-vote-override --proposal-id \"123\" --for-votes 5000 --against-votes 3000 --abstain-votes 2000\n\
                      # Use an explicit stake account\n\
                      $ svmgov --identity-keypair /path/to/key.json modify-vote-override --proposal-id \"123\" --for-votes 5000 --against-votes 3000 --abstain-votes 2000 --stake-account <STAKE_PUBKEY>"
    )]
    ModifyVoteOverride {
        /// Proposal ID for which to modify the vote override
        #[arg(long, help = "Proposal ID")]
        proposal_id: String,

        /// Basis points for 'For' vote
        #[arg(
            long,
            help = "Basis points for 'For' (must sum to 10,000 with other votes)"
        )]
        for_votes: u64,

        /// Basis points for 'Against' vote
        #[arg(
            long,
            help = "Basis points for 'Against' (must sum to 10,000 with other votes)"
        )]
        against_votes: u64,

        /// Basis points for 'Abstain' vote
        #[arg(
            long,
            help = "Basis points for 'Abstain' (must sum to 10,000 with other votes)"
        )]
        abstain_votes: u64,

        /// Stake account to use for override modification
        #[arg(
            long,
            help = "Stake account to use for override modification (base58 pubkey)"
        )]
        stake_account: String,

        /// Network for fetching merkle proofs
        #[arg(long, help = "Network for fetching merkle proofs")]
        network: String,

        /// Staker keypair for signing the transaction
        #[arg(long, help = "Staker keypair for signing the transaction")]
        staker_keypair: String,

        /// Vote account pubkey for the validator
        #[arg(long, help = "Vote account pubkey (base58) for the validator")]
        vote_account: String,
    },

    #[command(
        about = "Initialize the CLI configuration",
        long_about = "This command sets up the initial configuration for svmgov CLI. \
                      It will ask you whether you are a validator or staker, and prompt for \
                      the appropriate keypair paths and network preferences.\n\n\
                      Example:\n\
                      $ svmgov init"
    )]
    Init,

    #[command(
        about = "Manage CLI configuration",
        long_about = "This command allows you to view and modify configuration settings. \
                      Use 'config show' to view all settings, 'config get <key>' to get a specific \
                      value, or 'config set <key> <value>' to set a value.\n\n\
                      Examples:\n\
                      $ svmgov config show\n\
                      $ svmgov config set network testnet\n\
                      $ svmgov config get rpc-url"
    )]
    Config {
        #[command(subcommand)]
        subcommand: ConfigSubcommand,
    },
}

fn merge_cli_with_config(cli: Cli, config: Config) -> Cli {
    // Merge identity_keypair: CLI arg > config (based on user_type) > None
    let identity_keypair = cli
        .identity_keypair
        .or_else(|| config.get_identity_keypair_path());

    // Merge rpc_url: CLI arg > config rpc_url > config network default > constants default
    let rpc_url = cli.rpc_url.or_else(|| {
        if config.rpc_url.is_some() {
            config.rpc_url.clone()
        } else {
            Some(config.get_rpc_url())
        }
    });

    Cli {
        identity_keypair,
        rpc_url,
        command: cli.command,
    }
}

async fn handle_command(cli: Cli) -> Result<()> {
    // Load config and merge with CLI args
    let config = Config::load().unwrap_or_default();
    let cli = merge_cli_with_config(cli, config);

    log::debug!(
        "Handling command: identity_keypair={:?}, rpc_url={:?}, command={:?}",
        cli.identity_keypair,
        cli.rpc_url,
        cli.command
    );

    match &cli.command {
        Commands::CreateProposal {
            seed,
            title,
            description,
            network,
        } => {
            instructions::create_proposal(
                title.to_string(),
                description.to_string(),
                *seed,
                cli.identity_keypair,
                cli.rpc_url,
                network.clone(),
            )
            .await?;
        }
        Commands::SupportProposal {
            proposal_id,
            network,
        } => {
            instructions::support_proposal(
                proposal_id.to_string(),
                cli.identity_keypair,
                cli.rpc_url,
                network.clone(),
            )
            .await?;
        }
        Commands::CastVote {
            proposal_id,
            for_votes,
            against_votes,
            abstain_votes,
            network,
        } => {
            instructions::cast_vote(
                proposal_id.to_string(),
                *for_votes,
                *against_votes,
                *abstain_votes,
                cli.identity_keypair,
                cli.rpc_url,
                network.clone(),
            )
            .await?;
        }
        Commands::ModifyVote {
            proposal_id,
            for_votes,
            against_votes,
            abstain_votes,
            network,
        } => {
            instructions::modify_vote(
                proposal_id.to_string(),
                *for_votes,
                *against_votes,
                *abstain_votes,
                cli.identity_keypair,
                cli.rpc_url,
                network.clone(),
            )
            .await?;
        }
        Commands::FinalizeProposal { proposal_id } => {
            instructions::finalize_proposal(
                proposal_id.to_string(),
                cli.identity_keypair,
                cli.rpc_url,
            )
            .await?;
        }
        Commands::Proposal { proposal_id } => {
            commands::get_proposal(cli.rpc_url.clone(), proposal_id).await?;
        }
        Commands::ListProposals {
            status,
            limit,
            json,
        } => {
            let json_bool = json.as_ref().map(|s| s.parse::<bool>().unwrap_or(true)).unwrap_or(false);
            commands::list_proposals(
                cli.rpc_url.clone(),
                status.clone(),
                *limit,
                json_bool,
            )
            .await?;
        }
        Commands::InitIndex {} => {
            instructions::initialize_index(cli.identity_keypair, cli.rpc_url).await?;
        }
        Commands::CastVoteOverride {
            proposal_id,
            for_votes,
            against_votes,
            abstain_votes,
            stake_account,
            network,
            staker_keypair,
            vote_account,
        } => {
            instructions::cast_vote_override(
                proposal_id.to_string(),
                *for_votes,
                *against_votes,
                *abstain_votes,
                staker_keypair.clone(),
                cli.rpc_url,
                stake_account.clone(),
                vote_account.clone(),
                network.clone(),
            )
            .await?;
        }
        Commands::ModifyVoteOverride {
            proposal_id,
            for_votes,
            against_votes,
            abstain_votes,
            stake_account,
            network,
            staker_keypair,
            vote_account,
        } => {
            instructions::modify_vote_override(
                proposal_id.to_string(),
                *for_votes,
                *against_votes,
                *abstain_votes,
                staker_keypair.clone(),
                cli.rpc_url,
                stake_account.clone(),
                vote_account.clone(),
                network.clone(),
            )
            .await?;
        }
        Commands::Init => {
            init::run_init().await?;
        }
        Commands::Config { subcommand } => {
            handle_config_command(subcommand.clone()).await?;
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    // env_logger::init();
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    dotenv::dotenv().ok();
    let cli = Cli::parse();

    tokio::runtime::Runtime::new()?.block_on(handle_command(cli))?;

    Ok(())
}
