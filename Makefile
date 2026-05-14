.PHONY: help install-ncn-cli install-verifier-service install-all \
        sync sync-mainnet sync-mainnet-staging sync-testnet sync-testnet-staging sync-localnet \
        sync-dry-run

help:
	@echo "Available targets:"
	@echo ""
	@echo "  install-ncn-cli          - Install ncn-cli globally"
	@echo "  install-verifier-service - Build and install verifier-service"
	@echo "  install-all              - Run both installers"
	@echo ""
	@echo "  sync NETWORK=<name>      - Rewrite all program IDs in source to match networks.toml"
	@echo "  sync-dry-run NETWORK=<name>"
	@echo "                           - Show what 'sync' would change without writing"
	@echo "  sync-mainnet             - Shortcut for: make sync NETWORK=mainnet"
	@echo "  sync-mainnet-staging     - Shortcut for: make sync NETWORK=mainnet-staging"
	@echo "  sync-testnet             - Shortcut for: make sync NETWORK=testnet"
	@echo "  sync-testnet-staging     - Shortcut for: make sync NETWORK=testnet-staging"
	@echo "  sync-localnet            - Shortcut for: make sync NETWORK=localnet"

install-ncn-cli:
	bash ncn/scripts/install-ncn-cli.sh

install-verifier-service:
	bash ncn/scripts/install-verifier-service.sh

install-all:
	bash ncn/scripts/install-ncn-cli.sh
	bash ncn/scripts/install-verifier-service.sh

# --- Program ID sync (driven by /networks.toml) ---------------------------

NETWORK ?=

sync:
	@if [ -z "$(NETWORK)" ]; then \
		echo "error: NETWORK is required, e.g. 'make sync NETWORK=mainnet'" >&2; \
		exit 2; \
	fi
	bash scripts/sync-program-ids.sh $(NETWORK)

sync-dry-run:
	@if [ -z "$(NETWORK)" ]; then \
		echo "error: NETWORK is required, e.g. 'make sync-dry-run NETWORK=mainnet'" >&2; \
		exit 2; \
	fi
	bash scripts/sync-program-ids.sh $(NETWORK) --dry-run

sync-mainnet:
	bash scripts/sync-program-ids.sh mainnet

sync-mainnet-staging:
	bash scripts/sync-program-ids.sh mainnet-staging

sync-testnet:
	bash scripts/sync-program-ids.sh testnet

sync-testnet-staging:
	bash scripts/sync-program-ids.sh testnet-staging

sync-localnet:
	bash scripts/sync-program-ids.sh localnet
