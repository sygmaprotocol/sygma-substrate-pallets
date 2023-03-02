#!/usr/bin/env bash
# The Licensed Work is (c) 2022 Sygma
# SPDX-License-Identifier: LGPL-3.0-only

echo "cleanup..."
rm -rf ./scripts/phala-subbridge/code
rm -rf ./scripts/phala-subbridge/node
rm -f subbridge_node_launching.log

echo "done"
