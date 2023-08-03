#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

set -e

NODE_DB_DIR=${PWD}/db

# Run dev node
echo "start the dev node up..."
./node-template --dev --rpc-external --base-path "$NODE_DB_DIR" > substrate_node.log 2>&1 &

echo "waiting for dev nodes start..."
sleep 60

SETUP_SCRIPTS_DIR=${PWD}
CHAINSPECFILE="chain-spec.json"

# Run setup script
echo "run scripts to set up pallets..."
npm i --prefix $SETUP_SCRIPTS_DIR/scripts/js
node $SETUP_SCRIPTS_DIR/scripts/js/setup.js

sleep 10

# Run chain snapshot after setup
echo "set up is done, now export the chain state..."
./node-template export-state > $CHAINSPECFILE

# Stop the node
echo "stopping the dev node..."
nPid=`pgrep -f "node-template"`
if [ "$nPid" ]
then
  echo "terminating dev node"
  kill $nPid
fi

echo "done"
