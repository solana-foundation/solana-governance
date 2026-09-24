# NCN router infrastructure

Terraform provisions the production NCN router at `ncn-governance.solana.com` in the
`ncn-router` GCP project. CI applies infrastructure with one service account, then builds and
deploys an immutable container digest with a separate service account.

## Trust boundaries

| Identity | Scope | Access |
|---|---|---|
| `ncn-router-terraform` | Infrastructure CI only | APIs, Compute Engine, Artifact Registry administration, service-account administration/use, project IAM, and state objects |
| `ncn-router-deployer` | Application deployment CI only | Write the `ncn-router` repository, SSH through IAP on TCP 22, OS Admin Login, manage the existing `ncn-router` VM, and use its runtime service account |
| `ncn-router-runtime` | VM only; created by Terraform | Read the `ncn-router` repository and write Cloud Logging entries |

Never grant these identities `roles/editor`, `roles/owner`, or `roles/iap.admin`. Terraform
applies deployer access at the lowest supported resource:

- `roles/artifactregistry.writer` on the `ncn-router` repository.
- `roles/compute.instanceAdmin.v1` on the existing `ncn-router` VM. Instance scope prevents
  the deployer from creating VMs.
- `roles/iap.tunnelResourceAccessor` on the VM's IAP tunnel, conditioned on
  `destination.port == 22`.
- `roles/iam.serviceAccountUser` on `ncn-router-runtime`.
- `roles/compute.osAdminLogin` at the project. `gcloud compute ssh` needs
  `compute.projects.get`; the instance and IAP bindings still gate entry to the VM.

## Create the two CI identities

An administrator must create both identities before Terraform runs because they authenticate
Terraform and deployment jobs.

```bash
gcloud iam service-accounts create ncn-router-terraform \
  --project=ncn-router \
  --display-name='NCN router Terraform CI'

gcloud iam service-accounts create ncn-router-deployer \
  --project=ncn-router \
  --display-name='NCN router deployment CI'
```

Grant the Terraform identity its bootstrap roles:

```bash
for role in \
  roles/storage.admin \
  roles/serviceusage.serviceUsageAdmin \
  roles/compute.admin \
  roles/artifactregistry.admin \
  roles/iam.serviceAccountAdmin \
  roles/iam.serviceAccountUser \
  roles/resourcemanager.projectIamAdmin
do
  gcloud projects add-iam-policy-binding ncn-router \
    --member='serviceAccount:ncn-router-terraform@ncn-router.iam.gserviceaccount.com' \
    --role="$role"
done
```

Do not manually grant deployer roles. The main stack creates the repository, VM, and runtime
service account first, then applies the resource-scoped deployer bindings.

Create one JSON key for each CI identity. Store the key contents in Doppler immediately, then
remove local copies. Key rotation requires updating Doppler before disabling the previous key.

## Configure Doppler and GitHub OIDC

Use the existing `solana-governance` Doppler project with two production configurations:

| Config | Secret |
|---|---|
| `prd_infra` | `NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY` |
| `prd` | `NCN_ROUTER_DEPLOY_SERVICE_ACCOUNT_KEY` |
| `prd` | `CLOUDFLARE_ORIGIN_CA_CERTIFICATE` |
| `prd` | `CLOUDFLARE_ORIGIN_CA_PRIVATE_KEY` |
| `prd` | `SOLANA_RPC_URL_MAINNET` |
| `prd` | `SOLANA_RPC_URL_TESTNET` |

Configure a Doppler service identity to trust GitHub OIDC for this repository. Add its
non-secret identity ID as the GitHub Actions variable `DOPPLER_SERVICE_IDENTITY_ID`. Do not add
GCP keys, RPC URLs, or the Origin CA private key to GitHub secrets or variables.

The infrastructure job reads only `prd_infra`. The deployment job runs in a separate runner and
reads only `prd`.

## Bootstrap remote state

Sign in to the Doppler CLI with access to `solana-governance/prd_infra`, then create the state
bucket once:

```bash
doppler login
just ncn-router-tf-bootstrap
```

The Justfile runs Terraform through `doppler run` and maps
`NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY` to `GOOGLE_CREDENTIALS` in the child process. It never
loads the application secrets from `prd` into Terraform.

The bucket uses uniform access, public-access prevention, object versioning, 7-day soft delete,
`deletion_policy = "PREVENT"`, and Terraform `prevent_destroy`. The bootstrap stack grants
`roles/storage.objectAdmin` on that bucket to the Terraform identity.

Remove the temporary project-wide Storage Admin grant after bootstrap:

```bash
gcloud projects remove-iam-policy-binding ncn-router \
  --member='serviceAccount:ncn-router-terraform@ncn-router.iam.gserviceaccount.com' \
  --role='roles/storage.admin'
```

Keep the bootstrap state file secure. It is local by design and ignored by Git.

## Plan and apply locally

```bash
just ncn-router-tf-fmt
just ncn-router-tf-validate
just ncn-router-tf-plan
just ncn-router-tf-apply
```

`ncn-router-tf-plan` saves `/tmp/ncn-router.tfplan`. Override the bucket or plan path with
`NCN_ROUTER_TF_STATE_BUCKET` and `NCN_ROUTER_TF_PLAN`.

CI performs the same initialization, formatting, validation, and saved plan/apply before the
deployment job starts. It then runs `terraform plan -detailed-exitcode` and fails if any drift
remains. It also fails if either CI identity has
`Editor`, `Owner`, `Storage Admin`, or IAP Admin, or if the deployer has any project-level role
other than OS Admin Login.

## Point Cloudflare at the VM

Read the reserved address after apply:

```bash
terraform -chdir=infra/ncn-router output -raw external_ipv4_address
```

Create a proxied Cloudflare A record for `ncn-governance.solana.com` with that address. Issue a
Cloudflare Origin CA certificate that covers the hostname, store the PEM certificate and private
key in Doppler `prd`, and set Cloudflare SSL/TLS encryption mode to **Full (strict)**. Enable
**Global Authenticated Origin Pulls** for the zone before deploying: nginx requires Cloudflare's
client certificate and rejects direct origin traffic. The public Cloudflare Origin Pull CA is
bundled in this stack; it is distinct from the Origin CA certificate stored in Doppler.

The GCP firewall accepts TCP 443 only from Cloudflare's published IPv4 ranges. It accepts TCP 22
only from the IAP TCP-forwarding range. The VM exposes no application port directly.

## Deploy

Push a `v*` tag or run **Deploy NCN Router** manually. The workflow:

1. Applies infrastructure with `ncn-router-terraform` and `prd_infra`.
2. Builds both locked Rust binaries into a `linux/amd64` image and pushes it to Artifact Registry.
3. Resolves the pushed tag to a `sha256` digest.
4. Waits for VM startup provisioning to finish.
5. Sends the digest, RPC URLs, and Origin CA material over SSH standard input through IAP.
6. Installs runtime files as root with mode `0600` and starts separate cron/router systemd units.
7. Waits for fresh mainnet and testnet whitelist files, router redirects, and nginx readiness.
8. Restores the prior digest and runtime files if any readiness check fails.

Terraform state contains no RPC URLs, certificates, private keys, or application image digest.

## Verify the first deployment

Confirm no broad project roles were granted:

```bash
gcloud projects get-iam-policy ncn-router \
  --flatten='bindings[].members' \
  --filter='bindings.members:(ncn-router-terraform@ncn-router.iam.gserviceaccount.com OR ncn-router-deployer@ncn-router.iam.gserviceaccount.com)' \
  --format='table(bindings.role,bindings.members)'
```

Expected project bindings after bootstrap:

- Terraform: `serviceUsageAdmin`, `compute.admin`, `artifactregistry.admin`,
  `iam.serviceAccountAdmin`, `iam.serviceAccountUser`, and `projectIamAdmin`.
- Deployer: `compute.osAdminLogin` only.
- Neither identity: `editor`, `owner`, `storage.admin`, or `iap.admin`.

Inspect the bucket, repository, instance, IAP tunnel, and runtime service-account policies to
confirm the resource-scoped bindings. The resulting scopes establish that the deployer cannot
enable APIs, change project IAM, access Terraform state, create VMs, or administer repositories.

Run these operational checks after the first tag deployment:

```bash
curl --fail https://ncn-governance.solana.com/healthz

gcloud compute ssh ncn-router \
  --project=ncn-router \
  --zone=europe-west2-c \
  --tunnel-through-iap \
  --command='sudo systemctl status ncn-meta-cron ncn-router nginx --no-pager'
```

Reboot the VM and repeat the checks to verify systemd recovery. Test rollback with a controlled
image whose router process exits before readiness; the workflow must fail while the previous
digest returns to service. Run both a tag deployment and `workflow_dispatch` before relying on
the release path.

Review Cloud Audit Logs after the first deployments. Replace predefined roles with a custom role
only after the logs show a stable, smaller permission set.
