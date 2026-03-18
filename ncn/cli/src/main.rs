use anchor_client::{
    solana_sdk::{
        bs58,
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair},
    },
    Client, Cluster, Program,
};
use anyhow::{anyhow, Result};
use clap::Parser;
use cli::{generate_meta_merkle_snapshot, utils::*, MetaMerkleSnapshot};
use ncn_snapshot::{Ballot, BallotBox, ConsensusResult, MetaMerkleProof, ProgramConfig};
use log::info;
use anchor_client::solana_sdk::signature::Signer;
use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::HashMap, fs, process::Command, thread, time::Duration};
use tip_router_operator_cli::{
    cli::SnapshotPaths,
    ledger_utils::{get_bank_from_ledger, get_bank_from_snapshot_at_slot},
};
use tokio::runtime::Builder;

#[derive(Clone, Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(short, long, env, default_value = "/")]
    pub payer_path: PathBuf,

    #[arg(short, long, env, default_value = "/")]
    pub authority_path: PathBuf,

    #[arg(short, long, env, default_value = "11111111111111111111111111111111")]
    pub operator_address: String,

    #[arg(short, long, env, default_value = "http://localhost:8899")]
    pub rpc_url: String,

    #[arg(short, long, env)]
    pub ledger_path: Option<PathBuf>,

    #[arg(short, long, env)]
    pub full_snapshots_path: Option<PathBuf>,

    #[arg(long, env)]
    pub account_paths: Option<Vec<PathBuf>>,

    #[arg(short, long, env)]
    pub backup_snapshots_dir: Option<PathBuf>,

    #[arg(long, env, default_value = "mainnet")]
    pub cluster: String,

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
    SnapshotSlot {
        #[arg(long, env)]
        slot: u64,
    },
    GenerateMetaMerkle {
        #[arg(long, env)]
        slot: u64,

        #[arg(
            long,
            env,
            default_value = "./",
            help = "Path to save meta merkle tree"
        )]
        save_path: PathBuf,
    },
    LogMetaMerkleHash {
        #[arg(long, env, help = "Path to read meta merkle tree")]
        read_path: PathBuf,

        #[arg(long, default_value = "true")]
        is_compressed: bool,
    },
    AwaitSnapshot {
        #[arg(long, help = "Scan interval in minutes")]
        scan_interval: u64,

        #[arg(long, help = "Target slot to snapshot")]
        slot: u64,

        #[arg(long, help = "Directory to scan for snapshots")]
        snapshots_dir: PathBuf,

        #[arg(long, help = "Directory to copy matching snapshots to")]
        backup_snapshots_dir: PathBuf,

        #[arg(long, help = "Directory to copy ledger range to")]
        backup_ledger_dir: PathBuf,

        #[arg(long, help = "Path to agave-ledger-tool binary")]
        agave_ledger_tool_path: PathBuf,

        #[arg(long, help = "Path to live ledger directory (-l)")]
        ledger_path: PathBuf,

        #[arg(long, help = "Generate MetaMerkleSnapshot after snapshot")]
        generate_meta_merkle: bool,
    },
    InitProgramConfig {},
    UpdateOperatorWhitelist {
        #[arg(short, long, value_delimiter = ',', value_parser = parse_pubkey)]
        add: Option<Vec<Pubkey>>,

        #[arg(short, long, value_delimiter = ',', value_parser = parse_pubkey)]
        remove: Option<Vec<Pubkey>>,
    },
    UpdateProgramConfig {
        #[arg(long, env)]
        proposed_authority: Option<Pubkey>,

        #[arg(long)]
        min_consensus_threshold_bps: Option<u16>,

        #[arg(long, value_parser = parse_pubkey)]
        tie_breaker_admin: Option<Pubkey>,

        #[arg(long)]
        vote_duration: Option<i64>,
    },
    FinalizeProposedAuthority {},
    FinalizeBallot {
        #[arg(long, help = "Snapshot slot of ballot box")]
        snapshot_slot: u64,
    },
    CastVote {
        #[arg(long, help = "Snapshot slot of ballot box")]
        snapshot_slot: u64,

        #[arg(long, value_parser = parse_base_58_32, help = "Meta merkle tree root, base-58 encoded.")]
        root: [u8; 32],

        #[arg(long, value_parser = parse_base_58_32, help = "SHA256 hash of the meta merkle snapshot, base-58 encoded.")]
        hash: [u8; 32],
    },
    CastVoteFromSnapshot {
        #[arg(long, help = "Snapshot slot of ballot box")]
        snapshot_slot: u64,

        #[arg(long, env, help = "Path to read meta merkle tree")]
        read_path: PathBuf,

        #[arg(long, default_value = "true")]
        is_compressed: bool,
    },
    RemoveVote {
        #[arg(long, help = "Snapshot slot of ballot box")]
        snapshot_slot: u64,
    },
    SetTieBreaker {
        #[arg(long, help = "Snapshot slot of ballot box")]
        snapshot_slot: u64,

        #[arg(long, value_parser = parse_base_58_32, help = "Meta merkle tree root, base-58 encoded.")]
        root: [u8; 32],

        #[arg(long, value_parser = parse_base_58_32, help = "SHA256 hash of the meta merkle snapshot, base-58 encoded.")]
        hash: [u8; 32],
    },
    ResetBallotBox {
        #[arg(long, help = "Snapshot slot of ballot box")]
        snapshot_slot: u64,
    },
    Log {
        #[arg(long, help = "Snapshot slot of ballot box or consensus result")]
        snapshot_slot: Option<u64>,

        #[arg(long, value_parser = parse_pubkey)]
        vote_account: Option<Pubkey>,

        #[arg(long, value_parser = parse_log_type, help = "Account type: program-config | ballot-box | consensus-result | proof")]
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

    fn cast_vote_shared(cli: Cli, snapshot_slot: u64, root: [u8; 32], hash: [u8; 32]) -> Result<()> {
        let payer = read_keypair_file(&cli.payer_path).unwrap();
        let authority = read_keypair_file(&cli.authority_path).unwrap();
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

        info!("== Voted For Ballot Box (snapshot_slot: {}) ==", snapshot_slot);
        info!("Merkle Root: {}", bs58::encode(root).into_string());
        info!("Snapshot Hash: {}", bs58::encode(hash).into_string());

        Ok(())
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
                    let data: BallotBox =
                        program.account(BallotBox::pda(snapshot_slot.expect("Missing --snapshot-slot argument")).0)?;
                    println!("{:?}", data);
                }
                LogType::ConsensusResult => {
                    let data: ConsensusResult = program
                        .account(ConsensusResult::pda(snapshot_slot.expect("Missing --snapshot-slot argument")).0)?;
                    println!("{:?}", data);
                }
                LogType::MetaMerkleProof => {
                    let consensus_result_pda =
                        ConsensusResult::pda(snapshot_slot.expect("Missing --snapshot-slot argument")).0;
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

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
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

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
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

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
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

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
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
        Commands::CastVote { snapshot_slot, root, hash } => cast_vote_shared(cli, snapshot_slot, root, hash)?,
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

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
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
        Commands::SetTieBreaker { snapshot_slot, root, hash } => {
            info!("SetTieBreaker...");

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
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

            let payer = read_keypair_file(&cli.payer_path).unwrap();
            let authority = read_keypair_file(&cli.authority_path).unwrap();
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

            let payer = read_keypair_file(&cli.payer_path).unwrap();
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
            let authority = read_keypair_file(&cli.authority_path).unwrap();
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
                        "Target slot {} not yet passed; sleeping for {} minutes...",
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
