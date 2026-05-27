use anchor_client::solana_sdk::signature::Signer;
use anchor_client::{
    solana_sdk::{bs58, commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair},
    Client, Cluster, Program,
};
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cli::{generate_meta_merkle_snapshot, utils::*, MetaMerkleSnapshot};
use log::info;
use ncn_snapshot::{Ballot, BallotBox, ConsensusResult, MetaMerkleProof, ProgramConfig};
use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::HashMap, fs, process::Command, thread, time::Duration};
use tip_router_operator_cli::{
    cli::SnapshotPaths,
    ledger_utils::{get_bank_from_ledger, get_bank_from_snapshot_at_slot},
};
use tokio::runtime::Builder;

/// Solana Governance Voter Snapshot CLI.
///
/// Operator-facing tool for generating stake snapshots (ledger snapshot ->
/// MetaMerkleSnapshot) and interacting with the on-chain `ncn-snapshot`
/// program (initializing config, managing the operator whitelist, casting and
/// removing votes, finalizing ballots, setting tie-breakers, and inspecting
/// on-chain state).
#[derive(Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the keypair file that pays transaction fees and rent.
    ///
    /// This signer is used as the fee payer for all on-chain instructions
    /// issued by this CLI. It does not need to match the program authority
    /// or operator authority.
    #[arg(short, long, env, default_value = "/")]
    pub payer_path: PathBuf,

    /// Path to the keypair file that signs privileged actions.
    ///
    /// Only consulted by subcommands that submit on-chain transactions or
    /// sign messages off-chain. The role this keypair plays is documented
    /// in the description of each such subcommand
    /// (see `<subcommand> --help`). Read-only and snapshot-only
    /// subcommands ignore this flag. The default `/` is a placeholder and
    /// must be overridden whenever the chosen subcommand actually needs to
    /// sign.
    #[arg(short, long, env, default_value = "/")]
    pub authority_path: PathBuf,

    /// Operator pubkey (base58) used to tag generated snapshots.
    ///
    /// Stamped into the snapshot metadata so downstream tooling can
    /// attribute a snapshot to a specific operator. Defaults to the system
    /// program address as a placeholder when not provided.
    #[arg(short, long, env, default_value = "11111111111111111111111111111111")]
    pub operator_address: String,

    /// Solana JSON-RPC endpoint used for all on-chain interactions.
    ///
    /// Example: `https://api.mainnet-beta.solana.com`,
    /// `https://api.devnet.solana.com`, or a local validator URL.
    #[arg(short, long, env, default_value = "http://localhost:8899")]
    pub rpc_url: String,

    /// Path to a Solana validator ledger directory.
    ///
    /// Required by snapshot-related subcommands (`snapshot-slot`,
    /// `generate-meta-merkle`, `await-snapshot`). This should point at the
    /// validator's `--ledger` directory containing rocksdb and snapshots.
    #[arg(short, long, env)]
    pub ledger_path: Option<PathBuf>,

    /// Directory containing full Solana ledger snapshots
    /// (`snapshot-<slot>-<hash>.tar.zst`).
    ///
    /// Defaults to `--ledger-path` when not provided. Used by
    /// `snapshot-slot` to locate the base full snapshot to replay from.
    #[arg(short, long, env)]
    pub full_snapshots_path: Option<PathBuf>,

    /// One or more accounts-db directories used during ledger replay.
    ///
    /// Pass a comma-separated list to spread accounts across multiple
    /// disks. When omitted, the ledger directory is used as the single
    /// accounts path.
    #[arg(long, env)]
    pub account_paths: Option<Vec<PathBuf>>,

    /// Directory where generated snapshots (full + incremental) are written
    /// and read.
    ///
    /// `snapshot-slot` writes new snapshots here, and `generate-meta-merkle`
    /// expects to find the snapshot for the target slot in this directory.
    #[arg(short, long, env)]
    pub backup_snapshots_dir: Option<PathBuf>,

    /// Solana cluster name passed through to bank loading.
    ///
    /// One of: `mainnet`, `devnet`, `testnet`, `development`. Affects
    /// cluster-specific feature activation during ledger replay.
    #[arg(long, env, default_value = "mainnet")]
    pub cluster: String,

    /// Optional priority fee (in micro-lamports per compute unit) attached
    /// to outgoing transactions.
    ///
    /// When set, a `ComputeBudgetInstruction::SetComputeUnitPrice` is
    /// prepended to each transaction. Useful for landing transactions
    /// during periods of congestion.
    #[arg(long, env)]
    pub micro_lamports: Option<u64>,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn get_snapshot_paths(&self) -> SnapshotPaths {
        let ledger_path = self.ledger_path.clone().unwrap();
        let account_paths = self.account_paths.clone();
        let account_paths = account_paths.map_or_else(|| vec![ledger_path.clone()], |paths| paths);
        let full_snapshots_path = self.full_snapshots_path.clone();
        let full_snapshots_path = full_snapshots_path.map_or(ledger_path.clone(), |path| path);
        let backup_snapshots_dir = self.backup_snapshots_dir.clone().unwrap();
        SnapshotPaths {
            ledger_path,
            account_paths,
            full_snapshots_path,
            incremental_snapshots_path: backup_snapshots_dir.clone(),
            backup_snapshots_dir,
        }
    }
}

#[derive(clap::Subcommand, Clone)]
pub enum Commands {
    /// Generate a Solana ledger snapshot for a specific target slot.
    ///
    /// Replays the validator bank up to `--slot` and writes a full snapshot
    /// into `--backup-snapshots-dir`. Requires `--ledger-path` and
    /// `--full-snapshots-path` (or defaults derived from `--ledger-path`).
    SnapshotSlot {
        /// Target slot to snapshot. Must be a slot that has already been
        /// rooted in the ledger.
        #[arg(long, env, help = "Target slot to snapshot")]
        slot: u64,
    },
    /// Build a MetaMerkleSnapshot from an existing full ledger snapshot.
    ///
    /// Loads the bank at `--slot` from `--backup-snapshots-dir` and emits a
    /// compressed `meta_merkle-<slot>.zip` containing the stake-weighted
    /// merkle tree used for on-chain voting.
    GenerateMetaMerkle {
        /// Target slot. A full snapshot for this exact slot must already
        /// exist in `--backup-snapshots-dir`.
        #[arg(long, env, help = "Target slot for the MetaMerkleSnapshot")]
        slot: u64,

        /// Directory in which `meta_merkle-<slot>.zip` will be written.
        #[arg(
            long,
            env,
            default_value = "./",
            help = "Directory to write the compressed MetaMerkleSnapshot to"
        )]
        save_path: PathBuf,
    },
    /// Print the merkle root, snapshot hash, and a signature over them from a
    /// MetaMerkleSnapshot file.
    ///
    /// Useful for sharing the snapshot identity with other operators before
    /// voting on-chain. `--authority-path` signs an off-chain message
    /// containing the snapshot slot and meta merkle root; no on-chain
    /// transaction is sent.
    LogMetaMerkleHash {
        /// Path to a MetaMerkleSnapshot file (compressed `.zip` or raw).
        #[arg(long, env, help = "Path to the MetaMerkleSnapshot file to read")]
        read_path: PathBuf,

        /// Whether the input file is the compressed `.zip` produced by
        /// `generate-meta-merkle`.
        #[arg(
            long,
            default_value = "true",
            help = "Set to true when `--read-path` points to a compressed `.zip`"
        )]
        is_compressed: bool,
    },
    /// Wait for a target slot to pass and snapshot it from on-disk snapshots.
    ///
    /// Polls `--snapshots-dir` until a full + incremental snapshot pair
    /// covering `--slot` is available, copies them and the relevant ledger
    /// range into the backup directories, replays to produce a snapshot at
    /// `--slot`, and optionally generates the MetaMerkleSnapshot.
    AwaitSnapshot {
        /// Polling interval, in minutes, between directory scans.
        #[arg(
            long,
            help = "Polling interval (in minutes) between snapshot directory scans"
        )]
        scan_interval: u64,

        /// Target slot to snapshot once it has been rooted.
        #[arg(long, help = "Target slot to snapshot once it has been rooted")]
        slot: u64,

        /// Directory to scan for live validator snapshots.
        #[arg(long, help = "Directory to scan for live validator snapshots")]
        snapshots_dir: PathBuf,

        /// Directory into which the matching full + incremental snapshots
        /// are copied and the new snapshot at `--slot` is written.
        #[arg(
            long,
            help = "Directory to copy matching snapshots into and write the new snapshot to"
        )]
        backup_snapshots_dir: PathBuf,

        /// Directory into which the relevant ledger range is copied via
        /// `agave-ledger-tool`.
        #[arg(
            long,
            help = "Directory to copy the ledger range required for replay into"
        )]
        backup_ledger_dir: PathBuf,

        /// Absolute path to the `agave-ledger-tool` binary used to copy the
        /// ledger range.
        #[arg(long, help = "Absolute path to the `agave-ledger-tool` binary")]
        agave_ledger_tool_path: PathBuf,

        /// Path to the live validator ledger directory
        /// (the same path passed to the validator's `-l`/`--ledger` flag).
        #[arg(
            long,
            help = "Path to the live validator ledger directory (validator's `-l`)"
        )]
        ledger_path: PathBuf,

        /// When set, also generate the MetaMerkleSnapshot after the full
        /// snapshot at `--slot` has been written.
        #[arg(
            long,
            help = "Also generate the MetaMerkleSnapshot after taking the snapshot"
        )]
        generate_meta_merkle: bool,
    },
    /// Initialize the on-chain `ProgramConfig` singleton.
    ///
    /// Must be run once per deployment. The signer of `--authority-path`
    /// becomes the initial program authority and is recorded as
    /// `ProgramConfig.authority`.
    InitProgramConfig {},
    /// Add or remove operator pubkeys from the on-chain whitelist.
    ///
    /// Only whitelisted operators are allowed to cast votes.
    /// `--authority-path` must be the current program authority (enforced
    /// by `has_one = authority` on `ProgramConfig`).
    UpdateOperatorWhitelist {
        /// Comma-separated operator pubkeys (base58) to add to the whitelist.
        #[arg(
            short,
            long,
            value_delimiter = ',',
            value_parser = parse_pubkey,
            help = "Comma-separated operator pubkeys (base58) to add to the whitelist"
        )]
        add: Option<Vec<Pubkey>>,

        /// Comma-separated operator pubkeys (base58) to remove from the
        /// whitelist.
        #[arg(
            short,
            long,
            value_delimiter = ',',
            value_parser = parse_pubkey,
            help = "Comma-separated operator pubkeys (base58) to remove from the whitelist"
        )]
        remove: Option<Vec<Pubkey>>,
    },
    /// Update mutable fields on the on-chain `ProgramConfig`.
    ///
    /// All arguments are optional; only the provided fields are updated.
    /// `--authority-path` must be the current program authority (enforced
    /// by `has_one = authority` on `ProgramConfig`). Updating
    /// `--proposed-authority` starts a two-step authority handover that
    /// must be completed via `finalize-proposed-authority`.
    UpdateProgramConfig {
        /// Proposed new program authority (base58). Becomes active only
        /// after `finalize-proposed-authority` is run by this pubkey.
        #[arg(
            long,
            env,
            help = "Proposed new program authority (base58); activated via `finalize-proposed-authority`"
        )]
        proposed_authority: Option<Pubkey>,

        /// Minimum stake-weighted consensus threshold, expressed in basis
        /// points (10000 = 100%). Example: `6000` for 60%.
        #[arg(
            long,
            help = "Minimum consensus threshold in basis points (e.g. 6000 for 60%)"
        )]
        min_consensus_threshold_bps: Option<u16>,

        /// New tie-breaker admin pubkey (base58). This admin can resolve a
        /// stalled ballot box via `set-tie-breaker`.
        #[arg(
            long,
            value_parser = parse_pubkey,
            help = "New tie-breaker admin pubkey (base58)"
        )]
        tie_breaker_admin: Option<Pubkey>,

        /// Vote duration in seconds. Operators have this long after a
        /// ballot box is created to cast votes.
        #[arg(long, help = "Voting window duration, in seconds")]
        vote_duration: Option<i64>,
    },
    /// Complete a pending two-step authority handover.
    ///
    /// `--authority-path` must be the *proposed* authority previously
    /// staged via `update-program-config --proposed-authority`; the
    /// on-chain constraint requires `signer == proposed_authority`. On
    /// success the signer becomes the active program authority and
    /// `proposed_authority` is cleared.
    FinalizeProposedAuthority {},
    /// Finalize the winning ballot for a snapshot slot.
    ///
    /// Closes voting for `--snapshot-slot` once consensus has been reached
    /// (or the vote window has expired with a clear winner) and writes the
    /// `ConsensusResult` PDA. The on-chain instruction is permissionless,
    /// so `--payer-path` is used purely as the fee payer for the new
    /// PDA and does not need to hold any privileged role. The `--authority-path`
    /// is not used for this command.
    FinalizeBallot {
        /// Snapshot slot identifying the ballot box to finalize.
        #[arg(long, help = "Snapshot slot identifying the ballot box to finalize")]
        snapshot_slot: u64,
    },
    /// Print the on-chain `BallotBox` for a snapshot slot.
    GetBallot {
        /// Snapshot slot identifying the ballot box to fetch.
        #[arg(long, help = "Snapshot slot identifying the ballot box to fetch")]
        snapshot_slot: u64,
    },
    /// Print the on-chain `ProgramConfig` singleton.
    GetProgramConfig {},
    /// Print the operator whitelist from `ProgramConfig`.
    GetOperatorWhitelist {},
    /// Print a single operator's vote within a ballot box.
    GetOperatorVote {
        /// Snapshot slot identifying the ballot box.
        #[arg(long, help = "Snapshot slot identifying the ballot box")]
        snapshot_slot: u64,
        /// Operator pubkey (base58) whose vote should be looked up.
        #[arg(
            long,
            value_parser = parse_pubkey,
            help = "Operator pubkey (base58) whose vote should be looked up"
        )]
        operator: Pubkey,
    },
    /// Print the `ConsensusResult` PDA for a snapshot slot.
    GetConsensusResult {
        /// Snapshot slot identifying the consensus result to fetch.
        #[arg(long, help = "Snapshot slot identifying the consensus result to fetch")]
        snapshot_slot: u64,
    },
    /// Print the `MetaMerkleProof` PDA for a vote account.
    GetProof {
        /// Snapshot slot identifying the consensus result the proof belongs
        /// to.
        #[arg(
            long,
            help = "Snapshot slot identifying the consensus result the proof belongs to"
        )]
        snapshot_slot: u64,
        /// Validator vote account (base58) whose proof should be fetched.
        #[arg(
            long,
            value_parser = parse_pubkey,
            help = "Validator vote account (base58) whose proof should be fetched"
        )]
        vote_account: Pubkey,
    },
    /// Check whether a ballot box exists for a snapshot slot.
    BallotExists {
        /// Snapshot slot to check for a `BallotBox` PDA.
        #[arg(long, help = "Snapshot slot to check for a `BallotBox` PDA")]
        snapshot_slot: u64,
    },
    /// Print a combined status report: program config, ballot box, and
    /// consensus result for a snapshot slot.
    Status {
        /// Snapshot slot to summarize.
        #[arg(long, help = "Snapshot slot to summarize")]
        snapshot_slot: u64,
    },
    /// Cast a vote on a ballot box using an explicit root + hash.
    ///
    /// Prefer `cast-vote-from-snapshot` when voting from a local snapshot.
    /// `--authority-path` signs as a whitelisted operator; the handler
    /// verifies the signer appears in
    /// `ProgramConfig.whitelisted_operators` before recording the vote in
    /// the `BallotBox`.
    CastVote {
        /// Snapshot slot identifying the ballot box to vote in.
        #[arg(long, help = "Snapshot slot identifying the ballot box to vote in")]
        snapshot_slot: u64,

        /// Meta merkle tree root, base-58 encoded (32 bytes).
        #[arg(
            long,
            value_parser = parse_base_58_32,
            help = "Meta merkle tree root, base-58 encoded (32 bytes)"
        )]
        root: [u8; 32],

        /// SHA-256 hash of the MetaMerkleSnapshot file, base-58 encoded
        /// (32 bytes).
        #[arg(
            long,
            value_parser = parse_base_58_32,
            help = "SHA-256 hash of the MetaMerkleSnapshot file, base-58 encoded (32 bytes)"
        )]
        hash: [u8; 32],
    },
    /// Cast a vote on a ballot box using a local MetaMerkleSnapshot file.
    ///
    /// Reads the root and computes the snapshot hash from `--read-path`,
    /// then submits the vote. `--authority-path` signs as a whitelisted
    /// operator; the handler verifies the signer appears in
    /// `ProgramConfig.whitelisted_operators` before recording the vote in
    /// the `BallotBox`.
    CastVoteFromSnapshot {
        /// Snapshot slot identifying the ballot box to vote in.
        #[arg(long, help = "Snapshot slot identifying the ballot box to vote in")]
        snapshot_slot: u64,

        /// Path to a MetaMerkleSnapshot file (compressed `.zip` or raw)
        /// produced by `generate-meta-merkle`.
        #[arg(long, env, help = "Path to the MetaMerkleSnapshot file to vote with")]
        read_path: PathBuf,

        /// Whether the input file is the compressed `.zip` produced by
        /// `generate-meta-merkle`.
        #[arg(
            long,
            default_value = "true",
            help = "Set to true when `--read-path` points to a compressed `.zip`"
        )]
        is_compressed: bool,
    },
    /// Remove the caller's vote from a ballot box.
    ///
    /// Only permitted before consensus is reached and before the vote
    /// window expires. `--authority-path` must be the operator that
    /// originally cast the vote; the handler removes only that signer's
    /// tally entry.
    RemoveVote {
        /// Snapshot slot identifying the ballot box to remove the vote from.
        #[arg(
            long,
            help = "Snapshot slot identifying the ballot box to remove the vote from"
        )]
        snapshot_slot: u64,
    },
    /// Resolve a stalled ballot by writing an explicit winning ballot.
    ///
    /// The provided `--root` / `--hash` are not required to match any cast
    /// ballot. `--authority-path` must be the tie-breaker admin recorded
    /// in `ProgramConfig` (enforced by `has_one = tie_breaker_admin`).
    SetTieBreaker {
        /// Snapshot slot identifying the ballot box to tie-break.
        #[arg(long, help = "Snapshot slot identifying the ballot box to tie-break")]
        snapshot_slot: u64,

        /// Meta merkle tree root, base-58 encoded (32 bytes).
        #[arg(
            long,
            value_parser = parse_base_58_32,
            help = "Tie-breaking meta merkle tree root, base-58 encoded (32 bytes)"
        )]
        root: [u8; 32],

        /// SHA-256 hash of the corresponding MetaMerkleSnapshot, base-58
        /// encoded (32 bytes).
        #[arg(
            long,
            value_parser = parse_base_58_32,
            help = "Tie-breaking snapshot hash (SHA-256), base-58 encoded (32 bytes)"
        )]
        hash: [u8; 32],
    },
    /// Reset a bricked ballot box.
    ///
    /// Permitted only when the ballot box's vote window has not yet
    /// expired, consensus has not been reached, and ballot tallies are at
    /// their maximum capacity. Clears tallies so voting can restart.
    /// `--authority-path` must be the tie-breaker admin recorded in
    /// `ProgramConfig` (enforced by `has_one = tie_breaker_admin`).
    ResetBallotBox {
        /// Snapshot slot identifying the ballot box to reset.
        #[arg(long, help = "Snapshot slot identifying the ballot box to reset")]
        snapshot_slot: u64,
    },
    /// Dump the raw `Debug` representation of an on-chain account.
    ///
    /// Selects the account type via `--ty`; some types require additional
    /// arguments such as `--snapshot-slot` and/or `--vote-account`.
    Log {
        /// Snapshot slot identifying the ballot box, consensus result, or
        /// proof to fetch. Required for every `--ty` except
        /// `program-config`.
        #[arg(
            long,
            help = "Snapshot slot of the ballot box, consensus result, or proof (required for all `--ty` except `program-config`)"
        )]
        snapshot_slot: Option<u64>,

        /// Validator vote account (base58). Required when `--ty proof`.
        #[arg(
            long,
            value_parser = parse_pubkey,
            help = "Validator vote account (base58); required when `--ty proof`"
        )]
        vote_account: Option<Pubkey>,

        /// Account type to dump.
        #[arg(
            long,
            value_parser = parse_log_type,
            help = "Account type to dump: `program-config` | `ballot-box` | `consensus-result` | `proof`"
        )]
        ty: LogType,
    },
}

fn main() -> Result<()> {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .is_test(false)
        .try_init();

    let runtime = Builder::new_multi_thread().enable_all().build()?;
    let _enter = runtime.enter();
    let cli = Cli::parse();

    fn load_client_program(payer: &Keypair, rpc_url: String) -> Program<&Keypair> {
        let client: Client<&Keypair> = Client::new_with_options(
            Cluster::Custom(rpc_url.clone(), rpc_url),
            payer,
            CommitmentConfig::confirmed(),
        );
        client.program(ncn_snapshot::id()).unwrap()
    }

    fn cast_vote_shared(
        cli: Cli,
        snapshot_slot: u64,
        root: [u8; 32],
        hash: [u8; 32],
    ) -> Result<()> {
        let payer = read_signer_keypair(&cli.payer_path, "--payer-path")
            .context("loading payer keypair")?;
        let authority = read_signer_keypair(&cli.authority_path, "--authority-path")
            .context("loading authority keypair")?;
        let program = load_client_program(&payer, cli.rpc_url);

        let tx_sender = &TxSender {
            program: &program,
            micro_lamports: cli.micro_lamports,
            payer: &payer,
            authority: &authority,
        };
        let ballot_box_pda = BallotBox::pda(snapshot_slot).0;
        let tx = send_cast_vote(
            tx_sender,
            ballot_box_pda,
            Ballot {
                meta_merkle_root: root,
                snapshot_hash: hash,
            },
        )?;
        info!("Transaction sent: {}", tx);

        info!(
            "== Voted For Ballot Box (snapshot_slot: {}) ==",
            snapshot_slot
        );
        info!("Merkle Root: {}", bs58::encode(root).into_string());
        info!("Snapshot Hash: {}", bs58::encode(hash).into_string());

        Ok(())
    }

    fn format_ballot(ballot: &Ballot) -> (String, String) {
        (
            bs58::encode(ballot.meta_merkle_root).into_string(),
            bs58::encode(ballot.snapshot_hash).into_string(),
        )
    }

    fn print_program_config(config: &ProgramConfig) {
        println!("Program Config");
        println!("  Authority: {}", config.authority);
        match config.proposed_authority {
            Some(authority) => println!("  Proposed Authority: {}", authority),
            None => println!("  Proposed Authority: (none)"),
        }
        println!(
            "  Min Consensus Threshold (bps): {}",
            config.min_consensus_threshold_bps
        );
        println!("  Tie Breaker Admin: {}", config.tie_breaker_admin);
        println!("  Vote Duration: {}", config.vote_duration);
        println!(
            "  Whitelisted Operators: {}",
            config.whitelisted_operators.len()
        );
    }

    fn print_ballot_box(ballot_box: &BallotBox) {
        println!("Ballot Box");
        println!("  Snapshot Slot: {}", ballot_box.snapshot_slot);
        println!("  Epoch: {}", ballot_box.epoch);
        println!("  Slot Created: {}", ballot_box.slot_created);
        println!(
            "  Slot Consensus Reached: {}",
            ballot_box.slot_consensus_reached
        );
        println!(
            "  Min Consensus Threshold (bps): {}",
            ballot_box.min_consensus_threshold_bps
        );
        println!(
            "  Vote Expiry Timestamp: {}",
            ballot_box.vote_expiry_timestamp
        );
        println!(
            "  Tie Breaker Consensus: {}",
            ballot_box.tie_breaker_consensus
        );
        println!(
            "  Total Operator Votes: {}",
            ballot_box.operator_votes.len()
        );
        println!(
            "  Total Ballot Tallies: {}",
            ballot_box.ballot_tallies.len()
        );

        let (winning_root, winning_hash) = format_ballot(&ballot_box.winning_ballot);
        println!("Winning Ballot");
        println!("  Meta Merkle Root (base58): {}", winning_root);
        println!("  Snapshot Hash (base58): {}", winning_hash);

        println!("Ballot Tallies");
        if ballot_box.ballot_tallies.is_empty() {
            println!("  (none)");
        } else {
            for tally in &ballot_box.ballot_tallies {
                let (root, hash) = format_ballot(&tally.ballot);
                println!("  - Index: {}", tally.index);
                println!("    Tally: {}", tally.tally);
                println!("    Root (base58): {}", root);
                println!("    Hash (base58): {}", hash);
            }
        }

        println!("Operator Votes");
        if ballot_box.operator_votes.is_empty() {
            println!("  (none)");
        } else {
            for vote in &ballot_box.operator_votes {
                println!("  - Operator: {}", vote.operator);
                println!("    Slot Voted: {}", vote.slot_voted);
                println!("    Ballot Index: {}", vote.ballot_index);
            }
        }
    }

    match cli.command {
        // === On-chain Instructions ===
        Commands::Log {
            snapshot_slot,
            vote_account,
            ty,
        } => {
            let temp = Keypair::new();
            let program = load_client_program(&temp, cli.rpc_url);

            match ty {
                LogType::ProgramConfig => {
                    let data: ProgramConfig = program.account(ProgramConfig::pda().0)?;
                    println!("{:?}", data);
                }
                LogType::BallotBox => {
                    let data: BallotBox = program.account(
                        BallotBox::pda(snapshot_slot.expect("Missing --snapshot-slot argument")).0,
                    )?;
                    println!("{:?}", data);
                }
                LogType::ConsensusResult => {
                    let data: ConsensusResult = program.account(
                        ConsensusResult::pda(
                            snapshot_slot.expect("Missing --snapshot-slot argument"),
                        )
                        .0,
                    )?;
                    println!("{:?}", data);
                }
                LogType::MetaMerkleProof => {
                    let consensus_result_pda = ConsensusResult::pda(
                        snapshot_slot.expect("Missing --snapshot-slot argument"),
                    )
                    .0;
                    let data: MetaMerkleProof = program.account(
                        MetaMerkleProof::pda(
                            &consensus_result_pda,
                            &vote_account.expect("Missing --vote-account argument"),
                        )
                        .0,
                    )?;
                    println!("{:?}", data);
                }
            }
        }
        Commands::InitProgramConfig {} => {
            info!("InitProgramConfig...");

            let payer = read_signer_keypair(&cli.payer_path, "--payer-path")
                .context("loading payer keypair")?;
            let authority = read_signer_keypair(&cli.authority_path, "--authority-path")
                .context("loading authority keypair")?;
            let program = load_client_program(&payer, cli.rpc_url);

            let tx_sender = &TxSender {
                program: &program,
                micro_lamports: cli.micro_lamports,
                payer: &payer,
                authority: &authority,
            };
            let tx = send_init_program_config(tx_sender)?;
            info!("Transaction sent: {}", tx);
        }
        Commands::UpdateOperatorWhitelist { add, remove } => {
            info!("UpdateOperatorWhitelist...");

            let payer = read_signer_keypair(&cli.payer_path, "--payer-path")
                .context("loading payer keypair")?;
            let authority = read_signer_keypair(&cli.authority_path, "--authority-path")
                .context("loading authority keypair")?;
            let program = load_client_program(&payer, cli.rpc_url);

            let tx_sender = &TxSender {
                program: &program,
                micro_lamports: cli.micro_lamports,
                payer: &payer,
                authority: &authority,
            };
            let tx = send_update_operator_whitelist(tx_sender, add, remove)?;
            info!("Transaction sent: {}", tx);
        }
        Commands::UpdateProgramConfig {
            proposed_authority,
            min_consensus_threshold_bps,
            tie_breaker_admin,
            vote_duration,
        } => {
            info!("UpdateProgramConfig...");

            let payer = read_signer_keypair(&cli.payer_path, "--payer-path")
                .context("loading payer keypair")?;
            let authority = read_signer_keypair(&cli.authority_path, "--authority-path")
                .context("loading authority keypair")?;
            let program = load_client_program(&payer, cli.rpc_url);

            let tx_sender = &TxSender {
                program: &program,
                micro_lamports: cli.micro_lamports,
                payer: &payer,
                authority: &authority,
            };
            let tx = send_update_program_config(
                tx_sender,
                proposed_authority,
                min_consensus_threshold_bps,
                tie_breaker_admin,
                vote_duration,
            )?;
            info!("Transaction sent: {}", tx);
        }
        Commands::FinalizeProposedAuthority {} => {
            info!("FinalizeProposedAuthority...");

            let payer = read_signer_keypair(&cli.payer_path, "--payer-path")
                .context("loading payer keypair")?;
            let authority = read_signer_keypair(&cli.authority_path, "--authority-path")
                .context("loading authority keypair")?;
            let program = load_client_program(&payer, cli.rpc_url);

            let tx_sender = &TxSender {
                program: &program,
                micro_lamports: cli.micro_lamports,
                payer: &payer,
                authority: &authority,
            };
            let tx = send_finalize_proposed_authority(tx_sender)?;
            info!("Transaction sent: {}", tx);
        }
        Commands::CastVote {
            snapshot_slot,
            root,
            hash,
        } => cast_vote_shared(cli, snapshot_slot, root, hash)?,
        Commands::CastVoteFromSnapshot {
            snapshot_slot,
            ref read_path,
            is_compressed,
        } => {
            let snapshot = MetaMerkleSnapshot::read(read_path.clone(), is_compressed)?;
            info!("Using snapshot for slot {}", snapshot.slot);

            let snapshot_hash =
                MetaMerkleSnapshot::snapshot_hash(read_path.clone(), is_compressed)?;
            cast_vote_shared(cli, snapshot_slot, snapshot.root, snapshot_hash.to_bytes())?;
        }
        Commands::RemoveVote { snapshot_slot } => {
            info!("RemoveVote...");

            let payer = read_signer_keypair(&cli.payer_path, "--payer-path")
                .context("loading payer keypair")?;
            let authority = read_signer_keypair(&cli.authority_path, "--authority-path")
                .context("loading authority keypair")?;
            let program = load_client_program(&payer, cli.rpc_url);

            let ballot_box_pda = BallotBox::pda(snapshot_slot).0;
            let tx_sender = &TxSender {
                program: &program,
                micro_lamports: cli.micro_lamports,
                payer: &payer,
                authority: &authority,
            };
            let tx = send_remove_vote(tx_sender, ballot_box_pda)?;
            info!("Transaction sent: {}", tx);
        }
        Commands::SetTieBreaker {
            snapshot_slot,
            root,
            hash,
        } => {
            info!("SetTieBreaker...");

            let payer = read_signer_keypair(&cli.payer_path, "--payer-path")
                .context("loading payer keypair")?;
            let authority = read_signer_keypair(&cli.authority_path, "--authority-path")
                .context("loading authority keypair")?;
            let program = load_client_program(&payer, cli.rpc_url);
            let ballot_box_pda = BallotBox::pda(snapshot_slot).0;

            let tx_sender = &TxSender {
                program: &program,
                micro_lamports: cli.micro_lamports,
                payer: &payer,
                authority: &authority,
            };
            let ballot = Ballot {
                meta_merkle_root: root,
                snapshot_hash: hash,
            };
            let tx = send_set_tie_breaker(tx_sender, ballot_box_pda, ballot)?;
            info!("Transaction sent: {}", tx);
        }
        Commands::ResetBallotBox { snapshot_slot } => {
            info!("ResetBallotBox...");

            let payer = read_signer_keypair(&cli.payer_path, "--payer-path")
                .context("loading payer keypair")?;
            let authority = read_signer_keypair(&cli.authority_path, "--authority-path")
                .context("loading authority keypair")?;
            let program = load_client_program(&payer, cli.rpc_url);
            let ballot_box_pda = BallotBox::pda(snapshot_slot).0;

            let tx_sender = &TxSender {
                program: &program,
                micro_lamports: cli.micro_lamports,
                payer: &payer,
                authority: &authority,
            };
            let tx = send_reset_ballot_box(tx_sender, ballot_box_pda)?;
            info!("Transaction sent: {}", tx);
        }
        Commands::FinalizeBallot { snapshot_slot } => {
            info!("FinalizeBallot...");

            let payer = read_signer_keypair(&cli.payer_path, "--payer-path")
                .context("loading payer keypair")?;
            let program = load_client_program(&payer, cli.rpc_url);

            let ballot_box_pda = BallotBox::pda(snapshot_slot).0;
            let consensus_result_pda = ConsensusResult::pda(snapshot_slot).0;
            let tx_sender = &TxSender {
                program: &program,
                micro_lamports: cli.micro_lamports,
                payer: &payer,
                authority: &payer,
            };
            let tx = send_finalize_ballot(tx_sender, ballot_box_pda, consensus_result_pda)?;
            info!("Transaction sent: {}", tx);
        }
        Commands::GetBallot { snapshot_slot } => {
            let temp = Keypair::new();
            let program = load_client_program(&temp, cli.rpc_url);
            let ballot_box: BallotBox = program.account(BallotBox::pda(snapshot_slot).0)?;
            print_ballot_box(&ballot_box);
        }
        Commands::GetProgramConfig {} => {
            let temp = Keypair::new();
            let program = load_client_program(&temp, cli.rpc_url);
            let config: ProgramConfig = program.account(ProgramConfig::pda().0)?;
            print_program_config(&config);
        }
        Commands::GetOperatorWhitelist {} => {
            let temp = Keypair::new();
            let program = load_client_program(&temp, cli.rpc_url);
            let config: ProgramConfig = program.account(ProgramConfig::pda().0)?;
            println!("Operator Whitelist");
            if config.whitelisted_operators.is_empty() {
                println!("  (none)");
            } else {
                for (index, operator) in config.whitelisted_operators.iter().enumerate() {
                    println!("  - [{}] {}", index, operator);
                }
            }
        }
        Commands::GetOperatorVote {
            snapshot_slot,
            operator,
        } => {
            let temp = Keypair::new();
            let program = load_client_program(&temp, cli.rpc_url);
            let ballot_box: BallotBox = program.account(BallotBox::pda(snapshot_slot).0)?;
            let maybe_vote = ballot_box
                .operator_votes
                .iter()
                .find(|vote| vote.operator == operator);

            println!("Operator Vote");
            println!("  Snapshot Slot: {}", snapshot_slot);
            println!("  Operator: {}", operator);
            match maybe_vote {
                Some(vote) => {
                    println!("  Found: true");
                    println!("  Slot Voted: {}", vote.slot_voted);
                    println!("  Ballot Index: {}", vote.ballot_index);
                }
                None => {
                    println!("  Found: false");
                }
            }
        }
        Commands::GetConsensusResult { snapshot_slot } => {
            let temp = Keypair::new();
            let program = load_client_program(&temp, cli.rpc_url);
            let result: ConsensusResult = program.account(ConsensusResult::pda(snapshot_slot).0)?;
            let (root, hash) = format_ballot(&result.ballot);
            println!("Consensus Result");
            println!("  Snapshot Slot: {}", result.snapshot_slot);
            println!("  Tie Breaker Consensus: {}", result.tie_breaker_consensus);
            println!("  Meta Merkle Root (base58): {}", root);
            println!("  Snapshot Hash (base58): {}", hash);
        }
        Commands::GetProof {
            snapshot_slot,
            vote_account,
        } => {
            let temp = Keypair::new();
            let program = load_client_program(&temp, cli.rpc_url);
            let consensus_result_pda = ConsensusResult::pda(snapshot_slot).0;
            let proof_pda = MetaMerkleProof::pda(&consensus_result_pda, &vote_account).0;
            let proof: MetaMerkleProof = program.account(proof_pda)?;
            println!("Meta Merkle Proof");
            println!("  PDA: {}", proof_pda);
            println!("  Payer: {}", proof.payer);
            println!("  Consensus Result: {}", proof.consensus_result);
            println!("  Vote Account: {}", proof.meta_merkle_leaf.vote_account);
            println!("  Voting Wallet: {}", proof.meta_merkle_leaf.voting_wallet);
            println!(
                "  Stake Merkle Root (base58): {}",
                bs58::encode(proof.meta_merkle_leaf.stake_merkle_root).into_string()
            );
            println!("  Active Stake: {}", proof.meta_merkle_leaf.active_stake);
            println!("  Proof Nodes: {}", proof.meta_merkle_proof.len());
            println!("  Close Timestamp: {}", proof.close_timestamp);
        }
        Commands::BallotExists { snapshot_slot } => {
            let temp = Keypair::new();
            let program = load_client_program(&temp, cli.rpc_url);
            let ballot_pda = BallotBox::pda(snapshot_slot).0;
            let exists = program.account::<BallotBox>(ballot_pda).is_ok();
            println!("Ballot Exists");
            println!("  Snapshot Slot: {}", snapshot_slot);
            println!("  PDA: {}", ballot_pda);
            println!("  Exists: {}", exists);
        }
        Commands::Status { snapshot_slot } => {
            let temp = Keypair::new();
            let program = load_client_program(&temp, cli.rpc_url.clone());

            println!("Status");
            println!("  RPC URL: {}", cli.rpc_url);
            println!("  Cluster: {}", cli.cluster);
            println!("  Program ID: {}", ncn_snapshot::id());
            println!("  Snapshot Slot: {}", snapshot_slot);

            let config: ProgramConfig = program
                .account(ProgramConfig::pda().0)
                .context("failed to fetch ProgramConfig")?;
            println!();
            print_program_config(&config);

            let ballot_pda = BallotBox::pda(snapshot_slot).0;
            let ballot_box = program.account::<BallotBox>(ballot_pda);
            println!();
            match ballot_box {
                Ok(ballot_box) => {
                    print_ballot_box(&ballot_box);
                }
                Err(_) => {
                    println!("Ballot Box");
                    println!("  Snapshot Slot: {}", snapshot_slot);
                    println!("  PDA: {}", ballot_pda);
                    println!("  Exists: false");
                }
            }

            let consensus_pda = ConsensusResult::pda(snapshot_slot).0;
            let consensus = program.account::<ConsensusResult>(consensus_pda);
            println!();
            match consensus {
                Ok(consensus) => {
                    let (root, hash) = format_ballot(&consensus.ballot);
                    println!("Consensus Result");
                    println!("  PDA: {}", consensus_pda);
                    println!("  Exists: true");
                    println!(
                        "  Tie Breaker Consensus: {}",
                        consensus.tie_breaker_consensus
                    );
                    println!("  Meta Merkle Root (base58): {}", root);
                    println!("  Snapshot Hash (base58): {}", hash);
                }
                Err(_) => {
                    println!("Consensus Result");
                    println!("  PDA: {}", consensus_pda);
                    println!("  Exists: false");
                }
            }
        }
        // === Snapshot Processing ===
        Commands::SnapshotSlot { slot } => {
            info!("Snapshotting slot...");

            let save_snapshot = true;
            let SnapshotPaths {
                ledger_path,
                account_paths,
                full_snapshots_path,
                incremental_snapshots_path: _,
                backup_snapshots_dir,
            } = cli.get_snapshot_paths();

            get_bank_from_ledger(
                cli.operator_address,
                &ledger_path,
                account_paths,
                full_snapshots_path,
                backup_snapshots_dir.clone(),
                &slot,
                save_snapshot,
                backup_snapshots_dir,
                &cli.cluster,
            );
        }
        Commands::GenerateMetaMerkle {
            slot,
            ref save_path,
        } => {
            // Start timer
            let start_time = std::time::Instant::now();
            let SnapshotPaths {
                ledger_path,
                account_paths,
                full_snapshots_path: _,
                incremental_snapshots_path: _,
                backup_snapshots_dir,
            } = cli.get_snapshot_paths();

            // We can safely expect to use the backup_snapshots_dir as the full snapshot path because
            //  _get_bank_from_snapshot_at_slot_ expects the snapshot at the exact `slot` to have
            //  already been taken.
            let bank = get_bank_from_snapshot_at_slot(
                slot,
                &backup_snapshots_dir,
                &backup_snapshots_dir,
                account_paths,
                ledger_path.as_path(),
            )?;

            let meta_merkle_snapshot = generate_meta_merkle_snapshot(&Arc::new(bank))?;

            let file_path = PathBuf::from(save_path).join(format!("meta_merkle-{}.zip", slot));
            meta_merkle_snapshot.save_compressed(file_path)?;

            // Stop timer
            let end_time = std::time::Instant::now();
            let duration = end_time.duration_since(start_time);
            info!("Time taken: {:?}", duration);
        }
        Commands::LogMetaMerkleHash {
            read_path,
            is_compressed,
        } => {
            let authority = read_signer_keypair(&cli.authority_path, "--authority-path")
                .context("loading authority keypair")?;
            let snapshot = MetaMerkleSnapshot::read(read_path.clone(), is_compressed)?;
            let snapshot_hash = MetaMerkleSnapshot::snapshot_hash(read_path, is_compressed)?;

            let encoded_root = bs58::encode(snapshot.root).into_string();
            let encoded_hash = bs58::encode(snapshot_hash.to_bytes()).into_string();

            let mut message = Vec::new();
            message.extend_from_slice(&snapshot.slot.to_le_bytes());
            message.extend_from_slice(&encoded_root.as_bytes());
            let signature = authority.sign_message(&message);

            println!("Signature: {}", bs58::encode(signature).into_string());
            println!("Slot: {}", snapshot.slot);
            println!("Merkle Root: {}", encoded_root);
            println!("Snapshot Hash: {}", encoded_hash);
        }
        Commands::AwaitSnapshot {
            scan_interval,
            slot,
            snapshots_dir,
            backup_snapshots_dir,
            backup_ledger_dir,
            agave_ledger_tool_path,
            ledger_path,
            generate_meta_merkle,
        } => {
            // Validate paths upfront before entering the scan loop
            validate_executable_path(&agave_ledger_tool_path)?;
            validate_directory_path(&ledger_path)?;

            info!(
                "AwaitSnapshot starting: scan_interval={}m target_slot={} snapshot_dir={:?} backup_snapshot_dir={:?} backup_ledger_dir={:?}",
                scan_interval,
                slot,
                snapshots_dir,
                backup_snapshots_dir,
                backup_ledger_dir
            );

            // Loop until we find a matching pair of snapshot files
            let sleep_duration = Duration::from_secs(scan_interval.saturating_mul(60));
            loop {
                // Map of full snapshots by start slot: (start_slot, (name, path))
                let mut full_by_start: HashMap<u64, (String, PathBuf)> = HashMap::new();

                // Best matching incremental snapshot: (start_slot, end_slot, name, path)
                let mut best_le: Option<(u64, u64, String, PathBuf)> = None;

                // Flag to track if the target slot has elapsed
                let mut exists_ge: bool = false;

                match fs::read_dir(&snapshots_dir) {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            if let Ok(file_type) = entry.file_type() {
                                if !file_type.is_file() {
                                    continue;
                                }
                            }

                            let name_os = entry.file_name();
                            let name = name_os.to_string_lossy().to_string();

                            if let Some(start) = parse_full_snapshot_start_slot(&name) {
                                full_by_start.insert(start, (name.clone(), entry.path()));
                                continue;
                            }
                            if let Some((start, end)) = parse_incremental_snapshot_slots(&name) {
                                // Track proof that target_slot has elapsed
                                if end >= slot {
                                    exists_ge = true;
                                }
                                // Track the largest end <= target_slot across all incrementals
                                if end <= slot {
                                    match best_le {
                                        None => {
                                            best_le = Some((start, end, name.clone(), entry.path()))
                                        }
                                        Some((_, cur_end, _, _)) => {
                                            if end > cur_end {
                                                best_le =
                                                    Some((start, end, name.clone(), entry.path()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        info!(
                            "Failed to read snapshot directory {:?}: {}. Retrying in {}m",
                            snapshots_dir, err, scan_interval
                        );
                        thread::sleep(sleep_duration);
                        continue;
                    }
                }

                if !exists_ge {
                    info!(
                        "Incremental snapshot for target slot {} not yet found; sleeping for {} minutes...",
                        slot, scan_interval
                    );
                    thread::sleep(sleep_duration);
                    continue;
                }

                if let Some((start_slot, best_end_le, incr_name, incr_path)) = best_le {
                    if let Some((full_name, full_path)) = full_by_start.get(&start_slot) {
                        info!(
                            "Found matching snapshots: start_slot={} best_end_le={} (target_slot={})",
                            start_slot, best_end_le, slot
                        );

                        // Copy files to backup snapshot directory
                        let dest_full = backup_snapshots_dir.join(full_name);
                        let dest_incr = backup_snapshots_dir.join(&incr_name);
                        info!(
                            "Copying {} and {} to {:?}",
                            full_name, incr_name, backup_snapshots_dir
                        );
                        fs::create_dir_all(&backup_snapshots_dir)?;
                        fs::copy(full_path, &dest_full)?;
                        fs::copy(&incr_path, &dest_incr)?;

                        // Run agave-ledger-tool to copy ledger into backup directory
                        let end_copy_slot = slot.saturating_add(32);
                        info!(
                            "Running agave-ledger-tool: {} blockstore --ignore-ulimit-nofile-error -l {:?} copy --starting-slot {} --ending-slot {} --target-ledger {:?}",
                            agave_ledger_tool_path.display(),
                            ledger_path,
                            start_slot,
                            end_copy_slot,
                            backup_ledger_dir
                        );
                        let status = Command::new(&agave_ledger_tool_path)
                            .arg("blockstore")
                            .arg("--ignore-ulimit-nofile-error")
                            .arg("-l")
                            .arg(&ledger_path)
                            .arg("copy")
                            .arg("--starting-slot")
                            .arg(start_slot.to_string())
                            .arg("--ending-slot")
                            .arg(end_copy_slot.to_string())
                            .arg("--target-ledger")
                            .arg(&backup_ledger_dir)
                            .status()?;
                        if !status.success() {
                            return Err(anyhow!(
                                "agave-ledger-tool failed with status: {}",
                                status
                            ));
                        }

                        // Trigger snapshot creation using same flow as SnapshotSlot
                        info!(
                            "Starting snapshot at slot {} using backup ledger and snapshots dir...",
                            slot
                        );
                        let save_snapshot = true;
                        let account_paths = vec![backup_ledger_dir.clone()];
                        get_bank_from_ledger(
                            cli.operator_address,
                            &backup_ledger_dir,
                            account_paths,
                            backup_snapshots_dir.clone(),
                            backup_snapshots_dir.clone(),
                            &slot,
                            save_snapshot,
                            backup_snapshots_dir.clone(),
                            &cli.cluster,
                        );

                        if generate_meta_merkle {
                            info!("Generating MetaMerkleSnapshot for slot {}...", slot);
                            let mm_start = std::time::Instant::now();

                            let bank = get_bank_from_snapshot_at_slot(
                                slot,
                                &backup_snapshots_dir,
                                &backup_snapshots_dir,
                                vec![backup_ledger_dir.clone()],
                                backup_ledger_dir.as_path(),
                            )?;
                            let meta_merkle_snapshot =
                                generate_meta_merkle_snapshot(&Arc::new(bank))?;
                            let mm_file_path =
                                backup_snapshots_dir.join(format!("meta_merkle-{}.zip", slot));
                            meta_merkle_snapshot.save_compressed(mm_file_path.clone())?;

                            let mm_duration = mm_start.elapsed();
                            info!(
                                "Saved MetaMerkleSnapshot to {:?} (took {:?})",
                                mm_file_path, mm_duration
                            );
                        }

                        info!("Completed AwaitSnapshot flow. Exiting.");
                        break;
                    } else {
                        info!(
                            "Missing full snapshot for start_slot {}. Sleeping for {} minutes...",
                            start_slot, scan_interval
                        );
                        thread::sleep(sleep_duration);
                        continue;
                    }
                } else {
                    info!(
                        "No incremental snapshot with end <= target_slot found yet. Sleeping for {} minutes...",
                        scan_interval
                    );
                    thread::sleep(sleep_duration);
                    continue;
                }
            }
        }
    }
    Ok(())
}
