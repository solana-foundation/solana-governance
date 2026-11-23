set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

ncn_router_infra := "infra/ncn-router"
ncn_router_bootstrap := ncn_router_infra + "/bootstrap"
ncn_router_state_bucket := env_var_or_default("NCN_ROUTER_TF_STATE_BUCKET", "ncn-router-terraform-state")
ncn_router_plan := env_var_or_default("NCN_ROUTER_TF_PLAN", "/tmp/ncn-router.tfplan")
ncn_router_doppler_project := "solana-governance"
ncn_router_doppler_config := "prd_infra"

# Run paced requests through the router and against every configured verifier.
[positional-arguments]
ncn-network-loadtest *args:
    cargo run --locked --manifest-path ncn-router/Cargo.toml --bin ncn-network-loadtest -- "$@"

# Exercise reporting and network failures using local HTTP fixtures.
ncn-network-loadtest-test:
    cargo test --locked --manifest-path ncn-router/Cargo.toml --bin ncn-network-loadtest

# Check formatting and lint the load test binary.
ncn-network-loadtest-check:
    rustfmt --edition 2021 --check ncn-router/src/bin/network_loadtest.rs
    cargo clippy --locked --manifest-path ncn-router/Cargo.toml --bin ncn-network-loadtest -- -D warnings

# Create the remote-state bucket once with the infrastructure service account.
ncn-router-tf-bootstrap:
    doppler run --project {{ncn_router_doppler_project}} --config {{ncn_router_doppler_config}} -- bash -euo pipefail -c ': "${NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY:?Set NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY in Doppler prd_infra}"; export GOOGLE_CREDENTIALS="$NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY"; unset NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY; TF_VAR_state_bucket_name="{{ncn_router_state_bucket}}" terraform -chdir={{ncn_router_bootstrap}} init -input=false; TF_VAR_state_bucket_name="{{ncn_router_state_bucket}}" terraform -chdir={{ncn_router_bootstrap}} apply -input=false'

# Format both NCN router Terraform stacks.
ncn-router-tf-fmt:
    doppler run --project {{ncn_router_doppler_project}} --config {{ncn_router_doppler_config}} -- bash -euo pipefail -c ': "${NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY:?Set NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY in Doppler prd_infra}"; export GOOGLE_CREDENTIALS="$NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY"; unset NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY; terraform fmt -recursive {{ncn_router_infra}}'

# Validate without contacting the remote-state bucket.
ncn-router-tf-validate:
    doppler run --project {{ncn_router_doppler_project}} --config {{ncn_router_doppler_config}} -- bash -euo pipefail -c ': "${NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY:?Set NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY in Doppler prd_infra}"; export GOOGLE_CREDENTIALS="$NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY"; unset NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY; terraform -chdir={{ncn_router_bootstrap}} init -backend=false -input=false; terraform -chdir={{ncn_router_bootstrap}} validate; terraform -chdir={{ncn_router_infra}} init -backend=false -input=false; terraform -chdir={{ncn_router_infra}} validate'

# Save a production infrastructure plan outside the repository.
ncn-router-tf-plan:
    doppler run --project {{ncn_router_doppler_project}} --config {{ncn_router_doppler_config}} -- bash -euo pipefail -c ': "${NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY:?Set NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY in Doppler prd_infra}"; export GOOGLE_CREDENTIALS="$NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY"; unset NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY; terraform -chdir={{ncn_router_infra}} init -input=false -reconfigure -backend-config="bucket={{ncn_router_state_bucket}}"; terraform -chdir={{ncn_router_infra}} plan -input=false -out={{ncn_router_plan}}'

# Apply the plan saved by ncn-router-tf-plan.
ncn-router-tf-apply:
    test -f {{ncn_router_plan}}
    doppler run --project {{ncn_router_doppler_project}} --config {{ncn_router_doppler_config}} -- bash -euo pipefail -c ': "${NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY:?Set NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY in Doppler prd_infra}"; export GOOGLE_CREDENTIALS="$NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY"; unset NCN_ROUTER_TERRAFORM_SERVICE_ACCOUNT_KEY; terraform -chdir={{ncn_router_infra}} apply -input=false {{ncn_router_plan}}'
