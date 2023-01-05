#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

set -e

SETUP_SCRIPTS_DIR=${PWD}/scripts
CHAINSPECFILE="chain-spec.json"

# Run setup script
echo "run scripts to set up pallets..."
npm i
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

echo "done"
