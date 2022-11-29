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

