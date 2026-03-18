# Solana Validator Governance System

A decentralized governance platform for Solana validators featuring merkle proof verification, stake-weighted voting, and real-time event monitoring.

## Project Structure

### [`frontend/`](./frontend)
Next.js web interface for monitoring governance proposals, viewing active proposals, voting status, and real-time results.

### [`svmgov/program/`](./svmgov/program)
Anchor/Solana program implementing the on-chain governance logic — proposals, voting, merkle verification, and config management.

### [`svmgov/cli/`](./svmgov/cli)
Rust CLI tool for interacting with the governance program — creating proposals, casting votes, and managing governance operations from the terminal.

### [`ncn/`](./ncn)
NCN (Node Consensus Network) governance program — on-chain ballot voting, merkle proof verification, operator whitelisting, and a verifier service. Includes its own CLI and integration tests.

### [`docs/`](./docs)
Documentation site built with Nextra covering validator and staker workflows.
