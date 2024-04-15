#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

set -e

cd polkadot-sdk/local-test

NODE_DB_DIR=${PWD}/db
echo "node db dir:${NODE_DB_DIR}"

# Run zombienet node
echo "spin up the zombienet..."
./zombienet spawn --provider native ./bridge_hub_rococo_local_network.toml -d "$NODE_DB_DIR" > zombienet_node.log 2>&1 &

echo "waiting for zombienet start..."
sleep 120

# back to ./script
# doing this will keep the zombienet toml file as is, so that no zombienet file changes required when debugging manually in terminal
cd ..
cd ..

SETUP_SCRIPTS_DIR=${PWD}
CHAINSPECFILE="chain-spec.json"

# Run setup script
echo "run scripts to set up the zombienet..."
npm i --prefix $SETUP_SCRIPTS_DIR/xcm
node $SETUP_SCRIPTS_DIR/xcm/setup.js

sleep 20

# Stop the node
echo "stopping the zombienet..."
nPid=`pgrep -f "zombienet"`
if [ "$nPid" ]
then
  echo "terminating zombienet nodes"
  kill $nPid
fi

echo "done"
