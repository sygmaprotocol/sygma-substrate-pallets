#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

set -e

POLKADOT_SDK_VERSION='v1.2.0-rc1'

POLKADOT_SDK_DIR=${PWD}/zombienet/polkadot-sdk
ZOMBIENET_DIR=${PWD}/zombienet

# fetch and build polkadot-sdk
#git clone https://github.com/paritytech/polkadot-sdk.git ${POLKADOT_SDK_DIR} && cd ${POLKADOT_SDK_DIR} && git checkout ${POLKADOT_SDK_VERSION} && cargo build --release

# copy polkadot binaries
cp ${POLKADOT_SDK_DIR}/target/release/polkadot ${ZOMBIENET_DIR}/polkadot
cp ${POLKADOT_SDK_DIR}/target/release/polkadot-execute-worker ${ZOMBIENET_DIR}/polkadot-execute-worker
cp ${POLKADOT_SDK_DIR}/target/release/polkadot-prepare-worker ${ZOMBIENET_DIR}/polkadot-prepare-worker

# grant execution permission
chmod +x ${ZOMBIENET_DIR}/polkadot
chmod +x ${ZOMBIENET_DIR}/polkadot-execute-worker
chmod +x ${ZOMBIENET_DIR}/polkadot-prepare-worker

# fetch zombienet binary
echo "Please fetch zombienet binary based on your OS to this location: ${ZOMBIENET_DIR}/"
echo "Zombienet Docs: https://github.com/paritytech/zombienet?tab=readme-ov-file"
echo "Zombienet binary can be found: https://github.com/paritytech/zombienet/releases"
echo "Make sure to grand execution permission to zombienet binary"

echo "Cleaning up..."
rm -rf ${POLKADOT_SDK_DIR}

echo "done"
