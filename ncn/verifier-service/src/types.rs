//! Types for HTTP requests and responses

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NetworkQuery {
    pub network: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VoterQuery {
    pub network: Option<String>,
    pub slot: u64,
}
