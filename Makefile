lint: fmt
	cargo fmt --all --check
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

test:
	cargo test

build:
	cargo build --release

start-dev:
	./target/release/node-template --dev --ws-external

## license-check: Checks for missing license crates
license-check:
	@echo "  >  \033[Checking for license headers...\033[0m "
	cargo deny check license

build-e2e-test-docker-image:
	./scripts/start_substrate_node.sh
	./scripts/e2e_image_build.sh