# Production Deployment Checklist — Solana Governance

Covers the **two on-chain programs** (`svmgov`, `ncn-snapshot`), the **off-chain services**
(verifier-service, ncn-router / meta-cron, frontend), the **admin-side values** you must
decide, and the **contract initialization** order.

## How the pieces fit (read first)

- `svmgov` = governance (proposals, support, votes). The config **admin** is **not**
  hardcoded: it is set at `init-global-config` to the program's **upgrade authority** and is
  transferable on-chain afterward (two-step `nominate-admin` / `accept-admin`).
- `ncn-snapshot` = operator consensus on stake snapshots. Authority is **whoever signs init**. The authority is transferable on-chain afterward (two-step `update-program-config` / `finalize-proposed-authority`).
- They are coupled by program ID: `svmgov.support_proposal` CPIs into
  `ncn-snapshot.init_ballot_box`, and `ncn` authorizes the opening proposal PDA against the
  svmgov program stored in `ProgramConfig.svmgov_program_pubkey`. That value is **set at
  `init-program-config` (not baked into the ncn program)** and can be retargeted later with
  `update-program-config --svmgov-program-id`, so a wrong ID no longer bricks the deployment.
  The declared program IDs (`declare_id!`) and IDLs are still kept in sync by `make sync`, so
  set the IDs correctly before building.
- ⚠️ **This protection is one-directional — the reverse reference (svmgov → ncn) is
  compile-time, not config.** svmgov's ballot-box-opening paths (`support_proposal`, and
  `flush_merkle_root` for reset/recovery) hard-require the ncn program and its `ProgramConfig`
  to equal `ncn_snapshot::ID`, baked into the svmgov binary at build (kept current by
  `make sync`). There is **no `GlobalConfig` field to retarget it**. A normal in-place upgrade
  of ncn-snapshot keeps the same program ID and is safe; but **redeploying ncn-snapshot under a
  new program ID** locks out new ballot boxes (`support_proposal` rejects the new program) and
  can only be fixed by re-syncing the ID, rebuilding, and **upgrading the svmgov program** — a
  config update is not enough, and it is unrecoverable if svmgov has been made immutable. Treat
  the ncn-snapshot program ID as effectively permanent for a given svmgov deployment, and keep
  svmgov upgradeable unless you are certain it will never change.
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
      vs single key, ideally **squads/multisig** for mainnet). Note: the svmgov **upgrade
      authority is also the bootstrap admin** — whoever signs `init-global-config` must be the
      program's upgrade authority (verified on-chain against `ProgramData`) and becomes the
      stored `GlobalConfig.admin`.
- [ ] **svmgov admin key**: Decide the `init-global-config` signer (=
      the program's upgrade authority — a single key or a squads vault) and fund it. The admin
      is rotatable on-chain afterward via the two-step `nominate-admin` / `accept-admin` flow,
      so it need not equal the long-term admin. (The repo's `admin.json` is gitignored and
      carries no special on-chain meaning anymore.)
- [ ] Fund deploy + svmgov admin + ncn authority keypairs with SOL.

## Phase 1 — Build & sync program IDs

- [ ] `make bootstrap` — clones/pins `jito-tip-router` to the commit in `networks.toml`.
- [ ] `make sync-dry-run NETWORK=mainnet` — review every program-ID / RPC rewrite.
- [ ] `make sync NETWORK=mainnet` — rewrites IDs + RPCs across programs, both CLIs, frontend,
      ncn-router (`scripts/sync-program-ids.sh`).
- [ ] `make build-programs` — builds `svmgov` + `ncn` and copies IDL into `svmgov/cli/idls/`
      and `frontend/src/chain/idl/`.
- [ ] Confirm built `declare_id!` in both `lib.rs` == `networks.toml` IDs == on-chain target.
- [ ] ⚠️ Confirm svmgov was rebuilt **after** `make sync` — svmgov bakes in the ncn program ID
      (`ncn_snapshot::ID`) at build time, and this svmgov→ncn pointer is **not stored in any
      account, not returned by any instruction, and not shown by `show-global-config`**. A bad
      or stale sync here is invisible on-chain and only surfaces at the first `support-proposal`
      (see Phase 7). Verify `ncn-snapshot/src/lib.rs` `declare_id!` == `networks.toml`
      `ncn_snapshot_program_id` before building, and ideally hash the deployed binary against a
      reproducible build.

## Phase 2 — Build the CLIs

Later phases are driven by two CLIs: **`svmgov`** (governance — `init-global-config`,
`create-proposal`, `support-proposal`, `cast-vote`, …) and **`ncn-cli`** (operator consensus —
`init-program-config`, `update-operator-whitelist`, snapshots, voting). Build/install both now.

- [ ] ⚠️ Build the CLIs **after** `make sync` + `make build-programs` (Phase 1). Like the svmgov
      program, the `svmgov` CLI links `ncn_snapshot::ID` at compile time plus the synced IDLs in
      `svmgov/cli/idls/`; a stale build aims `support-proposal` at the wrong ncn program.

**svmgov CLI** → installs the `svmgov` binary:

- [ ] `bash svmgov/cli/install.sh` — cleans `svmgov/cli/target`, runs `cargo build --release`,
      and copies the binary to `/usr/local/bin/svmgov` (falls back to `~/.local/bin/svmgov` when
      `/usr/local/bin` isn't writable), appending a PATH entry to your shell rc if needed.
- [ ] `svmgov --version` to confirm it's on `PATH`.

**ncn CLI** → installs the `ncn-cli` binary:

- [ ] `make install-ncn-cli` — ensures `jito-tip-router` is present (re-runs `make bootstrap`
      as needed), builds `cargo build --locked --release -p cli` from `ncn/` with
      `RUSTFLAGS=-C target-cpu=native` and the required compile-time program-ID env vars
      (`RESTAKING_PROGRAM_ID`, `VAULT_PROGRAM_ID`, `TIP_ROUTER_PROGRAM_ID` — mainnet defaults
      baked in; export your own first to override), then installs the binary as `ncn-cli`. Also
      appends a shell wrapper defaulting `RAYON_NUM_THREADS` / `ZSTD_NBTHREADS` / `RUST_LOG` for
      snapshot performance.
- [ ] Open a new shell (to load the wrapper) and run `ncn-cli --version` to confirm.

**No-install build** (locked-down hosts — skip global install, `sudo`, and shell-rc edits): run
`cargo build --release` in `svmgov/cli` (binary → `svmgov/cli/target/release/svmgov`), and
`cargo build --locked --release -p cli` in `ncn/` after `make bootstrap` (binary →
`ncn/target/release/cli`), exporting the three `*_PROGRAM_ID` vars first (see
`ncn/scripts/install-ncn-cli.sh` for defaults). Invoke by full path, or `cargo run --release
--bin cli --` for ncn.

## Phase 3 — Deploy the programs

- [ ] Deploy `ncn-snapshot` (`anchor deploy` / `solana program deploy`) to the
      `ncn_snapshot_program_id` address.
- [ ] Deploy `svmgov` to the `svmgov_program_id` address.
- [ ] Set program **upgrade authority** to the agreed custody (multisig).
- [ ] Record deployed addresses; confirm they match `docs/.../program-ids` and
      `networks.toml`.

## Phase 4 — Initialize contracts (order matters)

**svmgov** (`init-global-config` signer must be the program **upgrade authority**):

- [ ] `init-global-config` (one-time) — must be signed by the program's upgrade authority,
      who becomes the stored `GlobalConfig.admin`. Sets all params below in one shot.
      ⚠️ Run this **before** making the program immutable: if the upgrade authority is set to
      `None`, init can never succeed.
- [ ] `init-index` — creates `ProposalIndex` (permissionless, but do it now; required before
      any proposal).
- [ ] `show-global-config` — verify written values (incl. `admin` and any `pending_admin`).
- [ ] (optional) `nominate-admin` → `accept-admin` to hand the admin role to its long-term
      holder (e.g. a squads vault) if that differs from the upgrade authority that initialized.

**ncn-snapshot** (signer = `--authority-path`, becomes the authority):

- [ ] `init-program-config --svmgov-program-id <svmgov_program_id>` — sets `authority` **and**
      the `svmgov_program_pubkey` authorized to open ballot boxes (source it from
      `networks.toml`'s `svmgov_program_id`). ⚠️ `min_consensus_threshold_bps`, `vote_duration`,
      `tie_breaker_admin` are still left **zero/unset** and the program is not usable until
      configured.
- [ ] `update-program-config --min-consensus-threshold-bps <…> --vote-duration <…>
--tie-breaker-admin <…>` — **must run before any voting** (threshold must be 1–10000,
      vote_duration > 0). Can also pass `--svmgov-program-id <…>` to retarget the authorized
      svmgov program if it was set wrong or svmgov is redeployed (no ncn redeploy needed).
- [ ] `update-operator-whitelist --add <op1,op2,…>` — add the production operator set
      (max 64).
- [ ] `ncn-cli get-program-config` & `ncn-cli get-operator-whitelist` — verify authority, threshold, vote_duration,
      tie_breaker_admin, svmgov program, whitelist.

## Phase 5 — Admin values to decide (fill these in before Phase 4)

**svmgov `init-global-config`:**

| Flag                            | Meaning                                                                    | Decide   |
| ------------------------------- | -------------------------------------------------------------------------- | -------- |
| `--max-title-length`            | proposal title length, **in bytes** (1–200)                                | e.g. 50  |
| `--max-description-length`      | desc length **in bytes** (1–500); desc must be a `https://github.com` link | e.g. 250 |
| `--max-support-epochs`          | max epochs in support phase                                                | ?        |
| `--min-proposal-stake-lamports` | min stake to create a proposal                                             | ?        |
| `--cluster-support-pct-min-bps` | % cluster stake to activate voting (bps, 0–10000)                          | ?        |
| `--discussion-epochs`           | discussion epochs after activation                                         | ?        |
| `--voting-epochs`               | active voting window (epochs)                                              | ?        |
| `--snapshot-epoch-extension`    | extension epochs before snapshot slot                                      | ?        |
| `--snapshot-slot-offset`        | slot offset from epoch start (can be negative)                             | ?        |

**ncn `update-program-config`:**

| Flag                            | Meaning                                                                                       | Decide                                |
| ------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------- |
| `--min-consensus-threshold-bps` | fraction of operators for consensus (e.g. 6000 = 60%)                                         | ?                                     |
| `--vote-duration`               | seconds a BallotBox stays open                                                                | ?                                     |
| `--tie-breaker-admin`           | resolves deadlocks / can reset bricked ballot box                                             | ? (multisig?)                         |
| `--svmgov-program-id`           | svmgov program allowed to open ballot boxes (set at `init-program-config`; retargetable here) | = `networks.toml` `svmgov_program_id` |
| operator whitelist              | the actual production operators                                                               | ?                                     |

## Phase 6 — Off-chain services

**Verifier-service** (each operator; `ncn/verifier-service/`, Docker on EC2 per
`ncn/verifier-service/DEPLOYMENT.md`):

- [ ] `make install-verifier-service` (or run `src/scripts/setup.sh` on host).
- [ ] Required env: `OPERATOR_PUBKEY` (base58), `METRICS_AUTH_TOKEN`. Optional: `DB_PATH`
      (`/data/governance.db`), `PORT` (3000, host 80→3000), rate-limit vars,
      `NCN_SNAPSHOT_MAX_MB`.
- [ ] Storage ≥ 40 GB gp3; Elastic IP; SG: 22 (restricted), 80 from CF.
- [ ] Cloudflare proxy + rate-limit rules (`/upload`, `/proof/*`); TLS mode decided.
- [ ] DB cleanup cron (`cleanup.sh`: `DB`, `DAYS`, `SLOTS_PER_DAY`).
- [ ] Smoke: `curl /healthz`, `/version`, `docker logs verifier`.
- [ ] Operator's pubkey is in the **ncn on-chain whitelist** (Phase 4).

**ncn-router + ncn-meta-cron** (`ncn-router/`):

- [ ] `config.toml` lists the production verifier `name` + `verification_domain` set
      (currently 11 entries — confirm). The shared default router domain is
      `ncn-governance.solana.com`.
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
- [ ] Frontend NCN API defaults to `https://ncn-governance.solana.com`.
- [ ] Confirm IDL in `frontend/src/chain/idl/` is the freshly-built one (Phase 1) and program
      IDs match.
- [ ] `pnpm build` → deploy (Vercel default; `/api/governance/config` caches on-chain
      `GlobalConfig` for 1h).
- [ ] Verify dashboard loads config, proposals, and verifier-backed proofs.

## Phase 7 — End-to-end verification

- [ ] On a non-prod or staging slot: create proposal → support past threshold (triggers
      `init_ballot_box` CPI) → operators generate snapshot + `cast-vote` → consensus →
      `finalize-ballot` → validator `cast-vote` on svmgov via verifier proof →
      `finalize-proposal`.
- [ ] ⚠️ The **support-past-threshold step is the only validation of the svmgov→ncn pointer**
      (svmgov's baked-in `ncn_snapshot::ID`). That linkage cannot be checked on-chain
      beforehand — `support_proposal` is the first instruction to exercise it, so do **not**
      skip this step. A green deploy + init + create-proposal does not prove the two programs
      are correctly wired.
- [ ] Confirm tie-breaker + `reset-ballot-box` paths work for the configured
      `tie_breaker_admin`.
- [ ] `cast-vote` / `cast-vote-override` set the temporary `MetaMerkleProof` PDA's
      `close_timestamp` to the proposal's vote-expiry by default so it's reclaimable
      permissionlessly after voting — no action needed unless you want different close
      semantics (then pass `--close-timestamp <unix>`).

## Phase 8 — Handover & custody

- [ ] Program upgrade authorities moved to multisig.
- [ ] ncn authority transfer (if needed): `update-program-config --proposed-authority <X>`
      then `finalize-proposed-authority` signed by X (two-step).
- [ ] svmgov admin transfer (if needed): `nominate-admin --new-admin <X>` (current admin)
      then `accept-admin` signed by X (two-step). No program upgrade required.
- [ ] Secure/rotate the svmgov admin key and ncn authority keys out of any working tree;
      document key locations and the on-chain values set.

---

## Signing admin/authority transactions with Squads

Both CLIs accept global flags to route an instruction through a **Squads multisig vault**
instead of signing locally — use this for every privileged operation on mainnet:

- `--squads <MULTISIG_PUBKEY>` (required to enable), `--squads-vault-index <N>` (default `0`),
  `--squads-memo <text>`, `--squads-program-id <PUBKEY>` (only for non-canonical Squads
  deployments).
- The CLI builds a `vault_transaction_create` + `proposal_create` pair signed by your local
  keypair (the **proposing member**, which must hold the multisig's `Initiate` permission).
  It does **not** approve or execute — multisig members still approve and execute the proposal
  in Squads afterward.
- The on-chain authority must **be the vault PDA**. So set the program's upgrade authority /
  ncn authority / `tie_breaker_admin` / `GlobalConfig.admin` to the vault address first, then
  run the command with `--squads`.

Squads-compatible admin/authority commands:

- **svmgov:** `init-global-config` (vault must be the upgrade authority), `update-global-config`,
  `nominate-admin`, `accept-admin`.
- **ncn:** `init-program-config`, `update-program-config`, `update-operator-whitelist`,
  `set-tie-breaker`, `reset-ballot-box`, `finalize-proposed-authority`.

The CLI **refuses `--squads`** for commands whose on-chain check requires a specific
validator/operator hot key or is permissionless (svmgov `create-proposal` / `support-proposal`
/ `cast-vote` / `modify-vote` / `init-index` / `finalize-proposal`; ncn `cast-vote` /
`remove-vote` / `finalize-ballot`) — a vault PDA can't satisfy those, so run them with the
local keypair.

---

## Flags worth attention before running

1. `networks.toml` uses public RPCs and identical program IDs across all networks — fix for
   mainnet.
2. `init-program-config` records `authority` + `svmgov_program_pubkey` but leaves
   `min_consensus_threshold_bps` / `vote_duration` / `tie_breaker_admin` at zero, so the
   `update-program-config` step is mandatory, not optional.
3. svmgov `init-global-config` must be signed by the program's **upgrade authority** and must
   run **before** the program is made immutable — otherwise the config can never be
   initialized. The signer becomes the admin; rotate later via `nominate-admin` / `accept-admin`.
4. The ncn→svmgov program reference is retargetable on-chain, but the svmgov→ncn reference is
   **not** — svmgov bakes in `ncn_snapshot::ID` at build time. Redeploying ncn-snapshot under a
   **new** program ID locks out new ballot boxes and requires a svmgov rebuild + upgrade to fix
   (unrecoverable if svmgov is immutable). In-place ncn upgrades (same ID) are unaffected. See
   "How the pieces fit."
