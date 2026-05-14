.PHONY: help bootstrap build-ncn build-svmgov build-programs \
        install-ncn-cli install-verifier-service install-all \
        sync sync-mainnet sync-mainnet-staging sync-testnet sync-testnet-staging sync-localnet \
        sync-dry-run

help:
	@echo "Available targets:"
	@echo ""
	@echo "  bootstrap                - Clone/update jito-tip-router to the commit pinned in networks.toml"
	@echo ""
	@echo "  build-ncn                - bootstrap + anchor build the ncn program"
	@echo "  build-svmgov             - anchor build the svmgov program"
	@echo "  build-programs           - build both programs"
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

bootstrap:
	bash scripts/setup-jito-tip-router.sh

build-ncn:
	bash scripts/build-program.sh ncn

build-svmgov:
	bash scripts/build-program.sh svmgov

build-programs:
	bash scripts/build-program.sh all

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
