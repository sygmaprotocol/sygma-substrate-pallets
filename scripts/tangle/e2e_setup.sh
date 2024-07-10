#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

set -e

echo ${PWD}

# go into project dir which is required by the launching script
cd tangle

# Run dev node
echo "start the dev node up..."
./scripts/run-standalone-local.sh --clean > substrate_node.log 2>&1 &

echo "waiting for dev node start..."
sleep 60

# Run setup script
echo "run scripts to set up pallets..."
npm i --prefix ./scripts/sygma-setup
node ./scripts/sygma-setup/setup.js

sleep 10

# Stop the node
echo "stopping the dev node..."
nPid=`pgrep -f "tangle"`
if [ "$nPid" ]
then
  echo "terminating dev node"
  kill $nPid
fi

echo "done"
