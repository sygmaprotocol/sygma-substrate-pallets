#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

set -e

MAIN_DIR=${PWD}
SETUP_SCRIPTS_DIR=${PWD}/scripts/phala-subbridge

# clone polkadot and khala repo
mkdir $SETUP_SCRIPTS_DIR/code
git clone https://github.com/paritytech/polkadot.git $SETUP_SCRIPTS_DIR/code/polkadot
git clone https://github.com/Phala-Network/khala-parachain.git $SETUP_SCRIPTS_DIR/code/khala-parachain

# build the polkadot and khala-node
git -C $SETUP_SCRIPTS_DIR/code/polkadot checkout release-v0.9.37
cd $SETUP_SCRIPTS_DIR/code/polkadot && cargo build --release
cd $MAIN_DIR
cp $SETUP_SCRIPTS_DIR/code/polkadot/target/release/polkadot $SETUP_SCRIPTS_DIR/code/khala-parachain/polkadot-launch/bin

git -C $SETUP_SCRIPTS_DIR/code/khala-parachain checkout sygma-integration
cd $SETUP_SCRIPTS_DIR/code/khala-parachain && cargo build --release --features=all-runtimes
cd $MAIN_DIR
cp $SETUP_SCRIPTS_DIR/code/khala-parachain/target/release/khala-node $SETUP_SCRIPTS_DIR/code/khala-parachain/polkadot-launch/bin

cp $SETUP_SCRIPTS_DIR/khala-e2e.config.json $SETUP_SCRIPTS_DIR/code/khala-parachain/polkadot-launch

mkdir $SETUP_SCRIPTS_DIR/node
cp -r $SETUP_SCRIPTS_DIR/code/khala-parachain/polkadot-launch/ $SETUP_SCRIPTS_DIR/node/polkadot-launch
cp -r $SETUP_SCRIPTS_DIR/code/khala-parachain/scripts/ $SETUP_SCRIPTS_DIR/node/scripts

echo "clean up the code dir..."
rm -rf $SETUP_SCRIPTS_DIR/code

yarn --cwd $SETUP_SCRIPTS_DIR/node/polkadot-launch
yarn --cwd $SETUP_SCRIPTS_DIR/node/polkadot-launch start khala-e2e.config.json > subbridge_node_launching.log 2>&1 &
