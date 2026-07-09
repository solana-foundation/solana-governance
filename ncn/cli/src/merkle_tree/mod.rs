//! Vendored slice of `jito-tip-router/meta_merkle_tree`.
//!
//! Only the three items actually used by this workspace are copied here:
//! [`MerkleTree`], [`get_proof`], and [`Delegation`]. The full upstream crate
//! pulls in the heavy Jito dependency graph (jito-vault-core, wincode, shank,
//! …); this slice depends only on `solana-program`, `serde`, and `fast-math`.

pub mod delegation;
pub mod merkle_tree;
pub mod utils;

pub use delegation::Delegation;
pub use merkle_tree::MerkleTree;
pub use utils::get_proof;
