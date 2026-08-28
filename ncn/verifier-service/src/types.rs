//! Types for HTTP requests and responses

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NetworkQuery {
    pub network: Option<String>,
    /// When set, return that snapshot instead of the newest one.
    pub slot: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct VoterQuery {
    pub network: Option<String>,
    pub slot: u64,
}
