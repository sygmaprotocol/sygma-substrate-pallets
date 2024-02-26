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

# run unit test with benchmark
test-benchmark:
	cargo test --features runtime-benchmarks

# license-check Checks for missing license crates
license-check:
	@echo "  >  \033[Checking for license headers...\033[0m "
	cargo-deny -L error check license

############################## Local node ############################

# build the binaries locally
# this will build both standalone node binary and the parachain node binary
build:
	cargo build --release

# build the binaries locally with benchmark
# this will build both standalone node binary and the parachain node binary
build-benchmark:
	cargo build --release --features runtime-benchmarks

# launch the standalone node in dev mode
start-dev:
	./target/release/standalone-node-template --dev --rpc-external

# run setup js script to setup the local substrate node
# substrate node is required, run make start-dev first
run-setup:
	node ./scripts/js/setup.js

# build-docker-image builds the docker image without setup the chain
build-docker-image:
	docker build -t sygma-substrate-pallet .

# start-docker-image launches the docker image without setup the chain
start-docker-image:
	docker run -p 9944:9944 -it sygma-substrate-pallet --dev --rpc-external

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

##################### Zombienet ##################

# prepare parachain-node binary and polkadot-sdk binary
build-zombienet: build
	./zombienet/scripts/zombienet_prepare.sh

# launch the parachain node in local zombienet with relay chain and parachain
start-zombienet:
	./zombienet/zombienet spawn -p native ./zombienet/local_zombienet.toml
