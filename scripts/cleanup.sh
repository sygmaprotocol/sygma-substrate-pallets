#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

NODE_DB_DIR=${PWD}/db
CHAINSPECFILE="chain-spec.json"

echo "cleanup..."
rm -rf "$NODE_DB_DIR"
rm -f $CHAINSPECFILE
rm -f subsrate_node.log

echo "done"
