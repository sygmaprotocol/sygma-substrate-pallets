# run cargo clippy and cargo fmt
lint: fmt
	cargo fmt --all --check
	cargo clippy --all-targets -- -D warnings

# run cargo fmt
fmt:
	cargo fmt --all

# run unit test
test:
	cargo test

# license-check Checks for missing license crates
license-check:
	@echo "  >  \033[Checking for license headers...\033[0m "
	cargo deny check license

# build the binary locally
build:
	cargo build --release

############################## Local node ############################
# launch the local node in dev mode
start-dev:
	./target/release/node-template --dev --ws-external

# run setup js script to setup the local substrate node
# substrate node is required, run make start-dev first
run-setup:
	node ./scripts/js/setup.js

############################## E2E test image ###########################
# build-e2e-test-docker-image builds the e2e test docker image
build-e2e-test-docker-image:
	@echo "building the e2e test docker image..."
	@echo "dockerfile in use: Dockerfile_e2e"
	@echo "mpc address in env var: $(MPCADDR)"
	docker build --file ./Dockerfile_e2e -t sygma_substrate_pallets_e2e_preconfigured --build-arg mpc_address=$(MPCADDR) .

# run the preconfigured e2e docker image
start-e2e-image:
	 docker run -p 9944:9944 -it sygma_substrate_pallets_e2e_preconfigured

##################### Phala subbridge integration node E2E test image ##################
# build-subbridge-e2e-test-image builds the phala subbridge integrated sygma pallet e2e test docker image
# this e2e image is a relay chain + phala parachain with sygma pallets simulation env
build-subbridge-e2e-test-image:
	@echo "building the subbridge e2e test docker image..."
	@echo "dockerfile in use: Dockerfile_subbridge_e2e"
	docker build --file ./Dockerfile_subbridge_e2e -t sygma_substrate_pallets_subbridge_e2e_preconfigured .

# run the preconfigured e2e subbridge docker image
start-subbridge-e2e-image:
	 docker run -p 9944:9944 -it sygma_substrate_pallets_subbridge_e2e_preconfigured
