#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

set -e

NODE_DB_DIR=${PWD}/db

# Compile
echo "compiling..."
cargo build --release

# Run dev node
echo "start the dev node up..."
./target/release/node-template --dev --ws-external --base-path "$NODE_DB_DIR" > subsrate_node_log 2>&1 &

sleep 5

echo "node started up"
