# Solana Validator Governance System

A decentralized governance platform enabling Solana validators and stakers to create, vote on, and finalize on-chain proposals using stake-weighted voting and merkle proof verification.

## Overview

The Solana Governance System provides two governance tracks:

- **SVM Governance (`svmgov/`)** — Validator and staker proposals with stake-weighted voting, vote overrides for delegated stake, and on-chain finalization
- **NCN Governance (`ncn/`)** — Node Consensus Network ballot voting with merkle proof verification, operator whitelisting, and a verifier service for automated snapshot processing

Both tracks use stake weight to determine voting power, ensuring that governance decisions reflect the economic commitments of participants.

## Quick Start

| I want to... | Go to |
|---|---|
| **Vote as a validator** | [Validator Guide](./docs/src/content/svmgov/validators/index.mdx) |
| **Vote as a staker** | [Staker Guide](./docs/src/content/svmgov/stakers/index.mdx) |
| **Run the NCN verifier** | [Verifier Service](./ncn/verifier-service/README.md) |
| **Use the CLI** | [CLI Reference](./docs/src/content/ncn/cli/index.mdx) |
| **Read the full docs** | [Documentation](./docs/) |

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

## Contributing

Issues and pull requests are welcome. See the [open issues](https://github.com/solana-foundation/solana-governance/issues) for areas that need work.

## License

See [LICENSE](./LICENSE) for details.
