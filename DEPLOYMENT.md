# Production Deployment Checklist — Solana Governance

Covers the **two on-chain programs** (`svmgov`, `ncn-snapshot`), the **off-chain services**
(verifier-service, ncn-router / meta-cron, frontend), the **admin-side values** you must
decide, and the **contract initialization** order.

## How the pieces fit (read first)

- `svmgov` = governance (proposals, support, votes). Admin is **hardcoded** in the program.
- `ncn-snapshot` = operator consensus on stake snapshots. Authority is **whoever signs init**.
- They are coupled by program ID: `svmgov.support_proposal` CPIs into
  `ncn-snapshot.init_ballot_box`, and `ncn` checks the proposal PDA belongs to `svmgov`
  (`InvalidProposal`), while `svmgov` checks the snapshot program ID
  (`InvalidSnapshotProgram`). **Both IDs must be synced everywhere before building, or CPIs
  fail.**
- Off-chain: operators run **verifier-service**; **ncn-meta-cron** builds verifier
  whitelists; **ncn-router** serves/redirects to verifiers; **frontend** reads on-chain
  config + verifier proofs.

---

## Phase 0 — Pre-flight

- [ ] Toolchain pinned: Rust `1.89.0`, Solana CLI ≥ 3.0, Anchor (matching `ncn/Anchor.toml` /
      `svmgov/program/Anchor.toml`).
- [ ] Decide the target network and confirm `networks.toml`:
  - [ ] ⚠️ `networks.toml` currently uses **public RPCs** (`api.mainnet-beta.solana.com`) and
        the **same program IDs + same jito commit for every network**. Replace mainnet
        `rpc_url` with a real provider (Helius/Triton/etc.) before prod, and confirm program
        IDs are the ones you actually control.
  - [ ] `jito_tip_router_commit` = `d60e3eb…` is the intended release commit.
- [ ] Deploy/upgrade authority keypairs secured (program upgrade authority — decide multisig
      vs single key, ideally **squads/multisig** for mainnet).
- [ ] **svmgov admin key**: `ADMIN_PUBKEY = BjHS1TPhG47CJGyghwKYrDZeHwmqh9frBk4Ba3uSXeRy` is
      hardcoded (`svmgov/program/programs/svmgov_program/src/constants.rs:4`). ✅ `admin.json`
      in the repo matches it — but that file is a **plaintext secret committed locally**;
      confirm it's gitignored and move custody to a secure signer for mainnet. If you want a
      different admin, you must change the constant and rebuild.
- [ ] Fund deploy + admin + ncn authority keypairs with SOL.

## Phase 1 — Build & sync program IDs

- [ ] `make bootstrap` — clones/pins `jito-tip-router` to the commit in `networks.toml`.
- [ ] `make sync-dry-run NETWORK=mainnet` — review every program-ID / RPC rewrite.
- [ ] `make sync NETWORK=mainnet` — rewrites IDs + RPCs across programs, both CLIs, frontend,
      ncn-router (`scripts/sync-program-ids.sh`).
- [ ] `make build-programs` — builds `svmgov` + `ncn` and copies IDL into `svmgov/cli/idls/`
      and `frontend/src/chain/idl/`.
- [ ] Confirm built `declare_id!` in both `lib.rs` == `networks.toml` IDs == on-chain target.

## Phase 2 — Deploy the programs

- [ ] Deploy `ncn-snapshot` (`anchor deploy` / `solana program deploy`) to the
      `ncn_snapshot_program_id` address.
- [ ] Deploy `svmgov` to the `svmgov_program_id` address.
- [ ] Set program **upgrade authority** to the agreed custody (multisig).
- [ ] Record deployed addresses; confirm they match `docs/.../program-ids` and
      `networks.toml`.

## Phase 3 — Initialize contracts (order matters)

**svmgov** (signer must be `admin.json`):
- [ ] `init-global-config` (one-time, admin-gated) — sets all params below in one shot.
- [ ] `init-index` — creates `ProposalIndex` (permissionless, but do it now; required before
      any proposal).
- [ ] `show-global-config` — verify written values.

**ncn-snapshot** (signer = `--authority-path`, becomes the authority):
- [ ] `init-program-config` — ⚠️ sets **only `authority`**; `min_consensus_threshold_bps`,
      `vote_duration`, `tie_breaker_admin` are left **zero/unset** and the program is not
      usable until configured.
- [ ] `update-program-config --min-consensus-threshold-bps <…> --vote-duration <…>
      --tie-breaker-admin <…>` — **must run before any voting** (threshold must be 1–10000,
      vote_duration > 0).
- [ ] `update-operator-whitelist --add <op1,op2,…>` — add the production operator set
      (max 64).
- [ ] `log --ty program-config` — verify authority, threshold, vote_duration,
      tie_breaker_admin, whitelist.

## Phase 4 — Admin values to decide (fill these in before Phase 3)

**svmgov `init-global-config`:**

| Flag | Meaning | Decide |
|---|---|---|
| `--max-title-length` | proposal title chars | e.g. 50 |
| `--max-description-length` | desc chars (must be a `https://github.com` link) | e.g. 250 |
| `--max-support-epochs` | max epochs in support phase | ? |
| `--min-proposal-stake-lamports` | min stake to create a proposal | ? |
| `--cluster-support-pct-min-bps` | % cluster stake to activate voting (bps) | ? |
| `--discussion-epochs` | discussion epochs after activation | ? |
| `--voting-epochs` | active voting window (epochs) | ? |
| `--snapshot-epoch-extension` | extension epochs before snapshot slot | ? |
| `--snapshot-slot-offset` | slot offset from epoch start (can be negative) | ? |

**ncn `update-program-config`:**

| Flag | Meaning | Decide |
|---|---|---|
| `--min-consensus-threshold-bps` | fraction of operators for consensus (e.g. 6000 = 60%) | ? |
| `--vote-duration` | seconds a BallotBox stays open | ? |
| `--tie-breaker-admin` | resolves deadlocks / can reset bricked ballot box | ? (multisig?) |
| operator whitelist | the actual production operators | ? |

## Phase 5 — Off-chain services

**Verifier-service** (each operator; `ncn/verifier-service/`, Docker on EC2 per
`DEPLOYMENT.md`):
- [ ] `make install-verifier-service` (or run `src/scripts/setup.sh` on host).
- [ ] Required env: `OPERATOR_PUBKEY` (base58), `METRICS_AUTH_TOKEN`. Optional: `DB_PATH`
      (`/data/governance.db`), `PORT` (3000, host 80→3000), rate-limit vars,
      `NCN_SNAPSHOT_MAX_MB`.
- [ ] Storage ≥ 40 GB gp3; Elastic IP; SG: 22 (restricted), 80 from CF.
- [ ] Cloudflare proxy + rate-limit rules (`/upload`, `/proof/*`); TLS mode decided.
- [ ] DB cleanup cron (`cleanup.sh`: `DB`, `DAYS`, `SLOTS_PER_DAY`).
- [ ] Smoke: `curl /healthz`, `/version`, `docker logs verifier`.
- [ ] Operator's pubkey is in the **ncn on-chain whitelist** (Phase 3).

**ncn-router + ncn-meta-cron** (`ncn-router/`):
- [ ] `config.toml` lists the production verifier `name` + `verification_domain` set
      (currently 10 entries — confirm).
- [ ] Required env: **`NCN_PROGRAM_ID`** (must equal deployed `ncn_snapshot_program_id`). Set
      RPCs: `SOLANA_RPC_URL_MAINNET` / `_TESTNET`.
- [ ] Optional: `NCN_CONFIG`, `NCN_LOG`, `NCN_WHITELIST_MAINNET_PATH`/`_TESTNET_PATH`,
      `NCN_ROUTER_BIND_ADDR` (default `0.0.0.0:8080`), `NCN_ROUTER_MODE` (`redirect`/`proxy`),
      `NCN_ROUTER_PROXY_TIMEOUT_SECS`.
- [ ] Run `ncn-meta-cron` (regenerates whitelist every ~2h) and `ncn-router` as long-running
      services (systemd/supervisor — no Dockerfile shipped). Ensure cron's whitelist output
      path == router's read path.
- [ ] Verify router serves a verifier given `?network=mainnet`.

**Frontend** (`frontend/`, Next.js):
- [ ] Set `NEXT_PUBLIC_SOLANA_RPC_MAINNET` (+ testnet/devnet) to production RPCs;
      `NEXT_PUBLIC_SENTRY_DSN` / `SENTRY_AUTH_TOKEN` if using Sentry.
- [ ] Confirm IDL in `frontend/src/chain/idl/` is the freshly-built one (Phase 1) and program
      IDs match.
- [ ] `pnpm build` → deploy (Vercel default; `/api/governance/config` caches on-chain
      `GlobalConfig` for 1h).
- [ ] Verify dashboard loads config, proposals, and verifier-backed proofs.

## Phase 6 — End-to-end verification

- [ ] On a non-prod or staging slot: create proposal → support past threshold (triggers
      `init_ballot_box` CPI) → operators generate snapshot + `cast-vote` → consensus →
      `finalize-ballot` → validator `cast-vote` on svmgov via verifier proof →
      `finalize-proposal`.
- [ ] Confirm tie-breaker + `reset-ballot-box` paths work for the configured
      `tie_breaker_admin`.

## Phase 7 — Handover & custody

- [ ] Program upgrade authorities moved to multisig.
- [ ] ncn authority transfer (if needed): `update-program-config --proposed-authority <X>`
      then `finalize-proposed-authority` signed by X (two-step).
- [ ] svmgov admin is fixed by the hardcoded constant — to rotate it requires a program
      upgrade; document this.
- [ ] Secure/rotate `admin.json` and ncn authority keys out of any working tree; document key
      locations and the on-chain values set.

---

## Flags worth attention before running

1. `networks.toml` uses public RPCs and identical program IDs across all networks — fix for
   mainnet.
2. `init-program-config` leaves the ncn config at zeros, so the `update-program-config` step
   is mandatory, not optional.
3. `admin.json` is a committed plaintext keypair — confirm gitignore and custody.
