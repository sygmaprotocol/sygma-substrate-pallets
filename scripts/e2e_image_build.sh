#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

set -e

SETUP_SCRIPTS_DIR=${PWD}/scripts
NODE_DB_DIR=${PWD}/db
DOCKERFILE_DIR=${PWD}/Dockerfile_e2e
CHAINSPECFILE="chain-spec.json"

# Run setup script
echo "run scripts to set up pallets..."
node $SETUP_SCRIPTS_DIR/js/setup.js

sleep 10

# Run chain snapshot after setup
echo "set up is done, now export the chain state..."
./target/release/node-template export-state > $CHAINSPECFILE

# Stop the node
echo "stopping the dev node..."
nPid=`pgrep -f "node-template"`
if [ "$nPid" ]
then
  echo "terminating dev node"
  kill $nPid
fi

# E2E preconfigured docker image build
echo "building the e2e test docker image..."
echo "dockerfile in use: $DOCKERFILE_DIR"
docker build --file "$DOCKERFILE_DIR" -t sygma_substrate_pallets_e2e_preconfigured .

echo "cleanup..."
rm -rf "$NODE_DB_DIR"
rm -f $CHAINSPECFILE
rm -f subsrate_node_log

echo "done"
