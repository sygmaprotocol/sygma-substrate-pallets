#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

set -e

MAIN_DIR=${PWD}
SETUP_SCRIPTS_DIR=${PWD}/scripts/phala-subbridge

# clone polkadot and khala repo
mkdir $SETUP_SCRIPTS_DIR/code

git clone https://github.com/Phala-Network/khala-parachain.git $SETUP_SCRIPTS_DIR/code/khala-parachain
git -C $SETUP_SCRIPTS_DIR/code/khala-parachain checkout sygma-integration

# download prebuild the polkadot and build khala-node
wget https://github.com/paritytech/polkadot/releases/download/v0.9.37/polkadot
mv polkadot $SETUP_SCRIPTS_DIR/code/khala-parachain/polkadot-launch/bin
chmod +x $SETUP_SCRIPTS_DIR/code/khala-parachain/polkadot-launch/bin/polkadot

cd $SETUP_SCRIPTS_DIR/code/khala-parachain && cargo build --release --features=all-runtimes
cd $MAIN_DIR
cp $SETUP_SCRIPTS_DIR/code/khala-parachain/target/release/khala-node $SETUP_SCRIPTS_DIR/code/khala-parachain/polkadot-launch/bin

cp $SETUP_SCRIPTS_DIR/khala-e2e.config.json $SETUP_SCRIPTS_DIR/code/khala-parachain/polkadot-launch

mkdir $SETUP_SCRIPTS_DIR/node
cp -r $SETUP_SCRIPTS_DIR/code/khala-parachain/polkadot-launch/ $SETUP_SCRIPTS_DIR/node/polkadot-launch
cp -r $SETUP_SCRIPTS_DIR/code/khala-parachain/scripts/ $SETUP_SCRIPTS_DIR/node/scripts

rm -rf $SETUP_SCRIPTS_DIR/code

yarn --cwd $SETUP_SCRIPTS_DIR/node/polkadot-launch
yarn --cwd $SETUP_SCRIPTS_DIR/node/polkadot-launch start khala-e2e.config.json > subbridge_node_launching.log 2>&1 &
