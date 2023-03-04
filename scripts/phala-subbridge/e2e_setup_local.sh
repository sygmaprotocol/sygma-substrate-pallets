#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

set -e

echo "waiting for relay chain nodes and parachain nodes start..."
sleep 60

SETUP_SCRIPTS_DIR=${PWD}/scripts/phala-subbridge/node/scripts/js

# Run setup script
echo "run scripts to set up sygma pallets..."
npm i --prefix $SETUP_SCRIPTS_DIR
node $SETUP_SCRIPTS_DIR/setup_sygma.js

sleep 10

# Stop the nodes
echo "stopping the dev nodes..."
nPid=`pgrep -f "polkadot"`
if [ "$nPid" ]
then
  echo "terminating relay chain nodes"
  kill $nPid
fi

nPid=`pgrep -f "khala-node"`
if [ "$nPid" ]
then
  echo "terminating parachain nodes"
  kill $nPid
fi

echo "clean up the setup scripts..."
rm -rf ${PWD}/scripts/phala-subbridge/node/scripts

echo "done"
