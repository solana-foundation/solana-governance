.PHONY: help install-ncn-cli install-verifier-service install-all

help:
	@echo "Available targets:"
	@echo "  make install-ncn-cli          - Install ncn-cli globally"
	@echo "  make install-verifier-service - Build and install verifier-service"
	@echo "  make install-all              - Run both installers"

install-ncn-cli:
	bash ncn/scripts/install-ncn-cli.sh

install-verifier-service:
	bash ncn/scripts/install-verifier-service.sh

install-all:
	bash ncn/scripts/install-ncn-cli.sh
	bash ncn/scripts/install-verifier-service.sh
