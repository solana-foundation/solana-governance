use rand::{seq::SliceRandom, thread_rng};
use reqwest::Client;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::{interval, MissedTickBehavior};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Quick-and-dirty CLI via envs
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://18.224.114.193".to_string());
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "./governance.db".to_string());
    let network = std::env::var("NETWORK").unwrap_or_else(|_| "testnet".to_string());
    let slot: u64 = std::env::var("SLOT")
        .ok()
        .and_then(|v| v.parse().ok())
        .expect("SLOT env is required (u64)");
    let duration_secs: u64 = std::env::var("DURATION_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let concurrency: usize = std::env::var("CONCURRENCY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(64);
    let target_rps: Option<u64> = std::env::var("TARGET_RPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0);
    // Endpoint selection: comma list of voter,vote_proof,stake_proof
    let endpoints_csv =
        std::env::var("ENDPOINTS").unwrap_or_else(|_| "voter,vote_proof,stake_proof".to_string());
    let mut selected_labels: Vec<String> = Vec::new();
    for part in endpoints_csv.split(',').map(|s| s.trim().to_lowercase()) {
        match part.as_str() {
            "voter" => selected_labels.push("voter".to_string()),
            "vote_proof" => selected_labels.push("vote_proof".to_string()),
            "stake_proof" => selected_labels.push("stake_proof".to_string()),
            _ => {}
        }
    }
    if selected_labels.is_empty() {
        anyhow::bail!("ENDPOINTS produced no valid entries");
    }
    // Optional weights like "voter=1,vote_proof=2,stake_proof=1"
    let weights_map: std::collections::HashMap<String, u32> = std::env::var("ENDPOINT_WEIGHTS")
        .ok()
        .map(|s| {
            s.split(',')
                .filter_map(|kv| {
                    let mut it = kv.split('=');
                    let k = it.next()?.trim().to_lowercase();
                    let v: u32 = it.next()?.trim().parse().ok()?;
                    Some((k, v.max(1)))
                })
                .collect()
        })
        .unwrap_or_default();
    // Build a simple sampling bag of endpoint indices per weight
    let mut pick_bag: Vec<usize> = Vec::new();
    for (idx, name) in selected_labels.iter().enumerate() {
        let w = *weights_map.get(name).unwrap_or(&1);
        for _ in 0..w {
            pick_bag.push(idx);
        }
    }
    if pick_bag.is_empty() {
        anyhow::bail!("No endpoints to pick from");
    }

    println!("BASE_URL={}", base_url);
    println!("DB_PATH={}", db_path);
    println!("NETWORK={} SLOT={}", network, slot);
    println!(
        "DURATION_SECS={} CONCURRENCY={} {}",
        duration_secs,
        concurrency,
        target_rps
            .map(|r| format!("TARGET_RPS={}", r))
            .unwrap_or_else(|| "(best-effort firehose)".to_string())
    );
    println!("ENDPOINTS={}", selected_labels.join(","));

    // Load IDs from DB
    let pool = SqlitePool::connect(&format!("sqlite:{}", db_path)).await?;
    let vote_accounts: Vec<String> =
        sqlx::query("SELECT vote_account FROM vote_accounts LIMIT 5000")
            .map(|row: SqliteRow| row.get::<String, _>("vote_account"))
            .fetch_all(&pool)
            .await?;
    let stake_accounts: Vec<String> =
        sqlx::query("SELECT stake_account FROM stake_accounts LIMIT 5000")
            .map(|row: SqliteRow| row.get::<String, _>("stake_account"))
            .fetch_all(&pool)
            .await?;
    // derive wallets from either table
    let voting_wallets: Vec<String> =
        sqlx::query("SELECT DISTINCT voting_wallet FROM vote_accounts LIMIT 5000")
            .map(|row: SqliteRow| row.get::<String, _>("voting_wallet"))
            .fetch_all(&pool)
            .await?;

    println!(
        "Loaded {} vote, {} stake, {} wallets",
        vote_accounts.len(),
        stake_accounts.len(),
        voting_wallets.len()
    );
    if vote_accounts.is_empty() && stake_accounts.is_empty() {
        anyhow::bail!("No accounts found in DB at {}", db_path);
    }

    let client = Client::builder()
        .pool_idle_timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(10_000)
        .tcp_nodelay(true)
        .timeout(Duration::from_secs(15))
        .build()?;

    let start_at = Instant::now();
    let end_at = start_at + Duration::from_secs(duration_secs);
    let sem = Arc::new(Semaphore::new(concurrency));
    let mut rng = thread_rng();

    let mut tasks = Vec::with_capacity(concurrency * 2);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let issued = Arc::new(AtomicU64::new(0));

    let labels_for_stats = selected_labels.clone();
    let issued_for_stats = issued.clone();
    let stats_handle = tokio::spawn(async move {
        let mut ok = 0u64;
        let mut err = 0u64;
        let mut ok_per: Vec<u64> = vec![0; labels_for_stats.len()];
        let mut err_per: Vec<u64> = vec![0; labels_for_stats.len()];
        let mut latencies_ms: Vec<u128> = Vec::new();
        while let Some((success, ms, idx)) = rx.recv().await {
            if success {
                ok += 1;
                ok_per[idx] += 1;
            } else {
                err += 1;
                err_per[idx] += 1;
            }
            latencies_ms.push(ms);
        }
        latencies_ms.sort_unstable();
        let p = |q: f64| -> u128 {
            if latencies_ms.is_empty() {
                return 0;
            }
            let idx = ((latencies_ms.len() as f64 - 1.0) * q).round() as usize;
            latencies_ms[idx]
        };
        let completed = ok + err;
        let issued_total = issued_for_stats.load(Ordering::Relaxed);
        let elapsed = start_at.elapsed().as_secs_f64();
        let qps = if elapsed > 0.0 {
            completed as f64 / elapsed
        } else {
            0.0
        };
        println!(
            "Summary: issued={} completed={} ok={} err={} p50={}ms p90={}ms p99={}ms qps={:.1}",
            issued_total,
            completed,
            ok,
            err,
            p(0.50),
            p(0.90),
            p(0.99),
            qps
        );
        for (i, name) in labels_for_stats.iter().enumerate() {
            println!(
                "  {}: ok={} err={} total={}",
                name,
                ok_per[i],
                err_per[i],
                ok_per[i] + err_per[i]
            );
        }
    });

    // Producer loop
    if let Some(rps) = target_rps {
        let mut ticker = interval(Duration::from_nanos(1_000_000_000 / rps));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        while Instant::now() < end_at {
            ticker.tick().await;
            let permit = sem.clone().acquire_owned().await.unwrap();
            issued.fetch_add(1, Ordering::Relaxed);
            // Pick an endpoint and id (weighted by pick_bag)
            let idx = *pick_bag.choose(&mut rng).unwrap();
            let name = selected_labels[idx].as_str();
            let (url, label_idx) = match name {
                "voter" => {
                    if voting_wallets.is_empty() {
                        continue;
                    }
                    let wallet = voting_wallets.choose(&mut rng).unwrap();
                    let url = format!(
                        "{}/voter/{}?network={}&slot={}",
                        base_url, wallet, network, slot
                    );
                    (url, idx)
                }
                "vote_proof" => {
                    if vote_accounts.is_empty() {
                        continue;
                    }
                    let acc = vote_accounts.choose(&mut rng).unwrap();
                    let url = format!(
                        "{}/proof/vote_account/{}?network={}&slot={}",
                        base_url, acc, network, slot
                    );
                    (url, idx)
                }
                _ => {
                    if stake_accounts.is_empty() {
                        continue;
                    }
                    let acc = stake_accounts.choose(&mut rng).unwrap();
                    let url = format!(
                        "{}/proof/stake_account/{}?network={}&slot={}",
                        base_url, acc, network, slot
                    );
                    (url, idx)
                }
            };

            let client_ref = client.clone();
            let tx_ref = tx.clone();
            let permit_ref = permit;
            tokio::spawn(async move {
                let started = Instant::now();
                let resp = client_ref.get(&url).send().await;
                let elapsed = started.elapsed().as_millis();
                let ok = match &resp {
                    Ok(r) => r.status().is_success(),
                    Err(_) => false,
                };
                let _ = tx_ref.send((ok, elapsed, label_idx));
                drop(permit_ref);
                if !ok {
                    match resp {
                        Ok(r) => eprintln!("err {}ms {} status={}", elapsed, url, r.status()),
                        Err(e) => eprintln!("err {}ms {} net={}", elapsed, url, e),
                    }
                }
            });
        }
    } else {
        while Instant::now() < end_at {
            let permit = sem.clone().acquire_owned().await.unwrap();
            issued.fetch_add(1, Ordering::Relaxed);
            // Pick an endpoint and id (weighted by pick_bag)
            let idx = *pick_bag.choose(&mut rng).unwrap();
            let name = selected_labels[idx].as_str();
            let (url, label_idx) = match name {
                "voter" => {
                    if voting_wallets.is_empty() {
                        continue;
                    }
                    let wallet = voting_wallets.choose(&mut rng).unwrap();
                    (
                        format!(
                            "{}/voter/{}?network={}&slot={}",
                            base_url, wallet, network, slot
                        ),
                        idx,
                    )
                }
                "vote_proof" => {
                    if vote_accounts.is_empty() {
                        continue;
                    }
                    let acc = vote_accounts.choose(&mut rng).unwrap();
                    let url = format!(
                        "{}/proof/vote_account/{}?network={}&slot={}",
                        base_url, acc, network, slot
                    );
                    (url, idx)
                }
                _ => {
                    if stake_accounts.is_empty() {
                        continue;
                    }
                    let acc = stake_accounts.choose(&mut rng).unwrap();
                    let url = format!(
                        "{}/proof/stake_account/{}?network={}&slot={}",
                        base_url, acc, network, slot
                    );
                    (url, idx)
                }
            };

            let client_ref = client.clone();
            let tx_ref = tx.clone();
            let permit_ref = permit;
            tasks.push(tokio::spawn(async move {
                let started = Instant::now();
                let resp = client_ref.get(&url).send().await;
                let elapsed = started.elapsed().as_millis();
                let ok = match &resp {
                    Ok(r) => r.status().is_success(),
                    Err(_) => false,
                };
                let _ = tx_ref.send((ok, elapsed, label_idx));
                drop(permit_ref);
                if !ok {
                    match resp {
                        Ok(r) => eprintln!("err {}ms {} status={}", elapsed, url, r.status()),
                        Err(e) => eprintln!("err {}ms {} net={}", elapsed, url, e),
                    }
                }
            }));
        }
    }

    // Close the stats channel so the summary prints
    drop(tx);

    // Wait a bit for tasks to finish
    for t in tasks {
        let _ = t.await;
    }

    // Ensure stats are printed before exit
    let _ = stats_handle.await;

    Ok(())
}
