set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Check TypeScript and Rust formatting without modifying files.
fmt:
    pnpm --dir frontend run format:check
    cargo fmt --all -- --check
    cargo fmt --manifest-path ncn/Cargo.toml --all -- --check
    cargo fmt --manifest-path svmgov/cli/Cargo.toml --all -- --check
    cargo fmt --manifest-path ncn-router/Cargo.toml --all -- --check
    cargo fmt --manifest-path squads-client/Cargo.toml --all -- --check

# Run the frontend static analysis checks.
lint:
    pnpm --dir frontend run lint
