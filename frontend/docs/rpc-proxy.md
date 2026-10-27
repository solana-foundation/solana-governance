# Solana RPC proxy

Browser Solana clients use `POST /api/rpc?cluster=mainnet|testnet|devnet`.
The route forwards requests to the matching server-only `SOLANA_RPC_*`
environment variable, so provider credentials are never included in the
client bundle or browser network requests.

The proxy is unauthenticated and intentionally narrow:

- JSON-RPC batches and cross-origin browser requests are rejected.
- Only methods in `RPC_METHODS` (`src/lib/rpcProxy.ts`) are accepted.
- `getProgramAccounts` is restricted to the governance, snapshot, and stake
  programs used by the application.
- Request and transaction sizes are capped.
- Per-instance read and write rate limits provide a basic safety net. Add
  distributed CDN/WAF rate limiting and usage alerts in production.

Safe reads use the shared Next.js remote cache with method-specific lifetimes.
Latest blockhashes, signature status, simulation, and submission are never
cached. A successful submission invalidates the cluster's cached reads.

Transaction signing remains in the user's wallet. The signed bytes are sent
through the proxy with `sendTransaction`; confirmation polls
`getSignatureStatuses` over HTTP so web3.js does not open a direct WebSocket
connection to an upstream RPC provider.
