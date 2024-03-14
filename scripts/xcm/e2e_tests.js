// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

require('dotenv').config();

const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
const {cryptoWaitReady} = require('@polkadot/util-crypto');
const {
    getNativeMultiAsset,
    getUSDCMultiAsset,
    getAssetDepositDest,
    depositLocal,
} = require("./util");

async function main() {
    // asset hub parachain
    const assetHubProvider = new WsProvider(process.env.ASSETHUBENDPOINT || 'ws://127.0.0.1:9910');
    const assetHubApi = await ApiPromise.create({
        provider: assetHubProvider,
    });

    // bridge hub parachain
    const bridgeHubProvider = new WsProvider(process.env.BRIDGEHUBENDPOINT || 'ws://127.0.0.1:8943');
    const bridgeHubApi = await ApiPromise.create({
        provider: bridgeHubProvider,
    });

    // prepare keyring
    await cryptoWaitReady();
    const keyring = new Keyring({type: 'sr25519'});
    const sudo = keyring.addFromUri('//Alice');

    // testcase 1: Native token deposit on Bridge hub, dest on relayer
    await depositLocal(bridgeHubApi, getNativeMultiAsset(bridgeHubApi, 10000000000000), getAssetDepositDest(bridgeHubApi), true, sudo)

    // testcase 2: Foreign token deposit on Bridge hub, dest on relayer
    await depositLocal(bridgeHubApi, getUSDCMultiAsset(bridgeHubApi, 10000000000000), getAssetDepositDest(bridgeHubApi), true, sudo)

}

main().catch(console.error).finally(() => process.exit());
