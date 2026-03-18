mod cron_job;

use std::env;

fn main() {
    // Load .env if present.
    let _ = dotenvy::dotenv();
    let args: Vec<String> = env::args().skip(1).collect();

    let mut network_filter: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--network" => {
                i += 1;
                let v = args.get(i).unwrap_or_else(|| {
                    eprintln!("Missing value for --network");
                    std::process::exit(1);
                });
                network_filter = Some(v.to_string());
            }
            v => {
                eprintln!("Unknown argument: {}", v);
                eprintln!("Usage: cargo run --bin ncn-meta-cron [--network mainnet|testnet]");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    cron_job::run(network_filter);
}
