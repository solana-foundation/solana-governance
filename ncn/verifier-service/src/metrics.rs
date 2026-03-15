use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::OnceCell;

use crate::database::constants::DEFAULT_DB_PATH;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum UploadOutcome {
    Success,
    BadRequest,
    Unauthorized,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ProofKind {
    Vote,
    Stake,
}

pub struct Metrics {
    upload_total: HashMap<UploadOutcome, u64>,
    proofs_not_found_total: HashMap<ProofKind, u64>,
}

static METRICS: OnceCell<Mutex<Metrics>> = OnceCell::new();

fn get() -> &'static Mutex<Metrics> {
    METRICS.get_or_init(|| {
        Mutex::new(Metrics {
            upload_total: HashMap::new(),
            proofs_not_found_total: HashMap::new(),
        })
    })
}

pub fn record_upload_outcome(outcome: UploadOutcome) {
    let mut m = get().lock().expect("metrics mutex poisoned");
    *m.upload_total.entry(outcome).or_insert(0) += 1;
}

pub fn record_proofs_not_found(kind: ProofKind) {
    let mut m = get().lock().expect("metrics mutex poisoned");
    *m.proofs_not_found_total.entry(kind).or_insert(0) += 1;
}

pub fn snapshot_as_json() -> serde_json::Value {
    use serde_json::json;
    let m = get().lock().expect("metrics mutex poisoned");

    let uploads: Vec<serde_json::Value> = m
        .upload_total
        .iter()
        .map(|(outcome, count)| {
            json!({
                "outcome": match outcome {
                    UploadOutcome::Success => "success",
                    UploadOutcome::BadRequest => "bad_request",
                    UploadOutcome::Unauthorized => "unauthorized",
                    UploadOutcome::Internal => "internal",
                },
                "count": count
            })
        })
        .collect();

    let not_found: Vec<serde_json::Value> = m
        .proofs_not_found_total
        .iter()
        .map(|(kind, count)| {
            json!({
                "kind": match kind { ProofKind::Vote => "vote", ProofKind::Stake => "stake" },
                "count": count
            })
        })
        .collect();

    let (db_path_str, db_bytes) = storage_db_info();
    let db_mb = db_bytes.map(|b| round2(bytes_to_mb(b)));
    let fs_free_mb = filesystem_free_mb_from_db_path(&db_path_str);

    json!({
        "upload_total": uploads,
        "proofs_not_found_total": not_found,
        "storage": {
            "db_path": db_path_str,
            "db_size_mb": db_mb,
            "free_storage_mb": fs_free_mb,
        }
    })
}

fn storage_db_info() -> (String, Option<u64>) {
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    let db_bytes =
        std::fs::metadata(&db_path)
            .ok()
            .and_then(|m| if m.is_file() { Some(m.len()) } else { None });

    (db_path, db_bytes)
}

fn bytes_to_mb(bytes: u64) -> f64 {
    let mb = 1024.0 * 1024.0;
    (bytes as f64) / mb
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn filesystem_free_mb_from_db_path(db_path: &str) -> Option<f64> {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    let path = std::path::Path::new(db_path);
    let mount = path.canonicalize().ok().and_then(|p| {
        disks
            .iter()
            .filter(|d| p.starts_with(d.mount_point()))
            .max_by_key(|d| d.mount_point().as_os_str().len())
    });

    if let Some(d) = mount {
        let available = bytes_to_mb(d.available_space());
        Some(round2(available))
    } else {
        None
    }
}
