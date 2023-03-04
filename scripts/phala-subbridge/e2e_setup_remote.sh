#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

set -e

SETUP_SCRIPTS_DIR=${PWD}

echo $SETUP_SCRIPTS_DIR

yarn --cwd $SETUP_SCRIPTS_DIR start khala-e2e.config.json > subbridge_node_launching.log 2>&1 &

echo "waiting for relay chain nodes and parachain nodes start..."
sleep 60

# Run setup script
echo "run scripts to set up sygma pallets..."
npm i --prefix $SETUP_SCRIPTS_DIR/scripts
node $SETUP_SCRIPTS_DIR/scripts/setup_sygma.js
