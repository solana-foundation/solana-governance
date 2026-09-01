//! Types for HTTP requests and responses

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NetworkQuery {
    pub network: Option<String>,
    /// Which snapshot to describe. Omitted means the newest.
    pub slot: Option<u64>,
}

/// Query for the batch metadata endpoint.
#[derive(Debug, Deserialize)]
pub struct MetasQuery {
    pub network: Option<String>,
    /// Comma-separated snapshot slots.
    pub slots: String,
}

#[derive(Debug, Deserialize)]
pub struct VoterQuery {
    pub network: Option<String>,
    pub slot: u64,
}
