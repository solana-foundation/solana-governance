//! Cross-program integration tests for the Solana governance system.
//!
//! These tests deploy BOTH on-chain programs — `svmgov_program` and
//! `ncn_snapshot` — onto an in-process Surfpool ephemeral network and drive the
//! proposal creation/support flow, including svmgov's CPI into ncn-snapshot.
//!
//! The harness ([`harness::start_surfnet_with_programs`]) brings up the surfnet
//! and deploys both programs; tests interact with it over RPC.

mod harness;

#[cfg(test)]
mod full_flow;

#[cfg(test)]
mod epoch_stake_cheatcode;

#[cfg(test)]
mod proposal_validation;
