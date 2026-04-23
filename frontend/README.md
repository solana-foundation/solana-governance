# Solana Governance Frontend

Next.js web interface for the Solana Validator Governance System. Provides a dashboard for viewing proposals, casting votes, and monitoring governance status.

## Features

- View active and historical governance proposals
- Cast and modify votes (validator and staker workflows)
- Real-time vote tracking and quorum progress
- Stake-weighted vote visualization
- Wallet integration via Solana wallet adapters

## Prerequisites

- Node.js (see `.nvmrc` for version)
- pnpm (package manager)
- A Solana RPC endpoint

## Getting Started

1. Copy the environment template:
   ```bash
   cp .env.example .env.local
   ```

2. Configure your RPC endpoint in `.env.local`

3. Install dependencies:
   ```bash
   pnpm install
   ```

4. Start the development server:
   ```bash
   pnpm dev
   ```

5. Open [http://localhost:3000](http://localhost:3000)

### Environment Variables

See [`.env.example`](./.env.example) for required configuration.

### Project Structure

- `src/app/` — Next.js app router pages
- `src/components/` — React components (governance, proposals, UI)
- `src/chain/` — Solana program interaction (IDL, instructions, types)
- `src/hooks/` — Custom React hooks for data fetching and state
- `src/data/` — API and data layer
- `src/contexts/` — React contexts (wallet, endpoints, modals)
