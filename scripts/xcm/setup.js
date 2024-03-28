// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

require('dotenv').config();

const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
const {cryptoWaitReady} = require('@polkadot/util-crypto');
const {
    relayChainProvider,
    assetHubProvider,
    bridgeHubProvider,
    transferBalance,
    setFeeHandler,
    setMpcAddress,
    registerDomain,
    setFee,
    setFeeRate,
    getNativeAssetId,
    createAsset,
    setAssetMetadata,
    mintAsset,
    getUSDCAssetId,
    queryBridgePauseStatus,
    FeeReserveAccount,
    NativeTokenTransferReserveAccount,
    OtherTokenTransferReserveAccount,
    hrmpChannelRequest,
    getHRMPChannelMessage,
    getHRMPChannelDest,
    delay,
    usdcAssetID,
    usdcMinBalance,
    usdcName,
    usdcSymbol,
    usdcDecimal,
} = require("./util");

const BN = require('bn.js');
const bn1e12 = new BN(10).pow(new BN(12));
const bn1e18 = new BN(10).pow(new BN(18));
const bn1e20 = new BN(10).pow(new BN(20));

const feeHandlerType = {
    BasicFeeHandler: "BasicFeeHandler",
    PercentageFeeHandler: "PercentageFeeHandler",
    DynamicFeeHandler: "DynamicFeeHandler"
}

const supportedDestDomains = [
    {
        domainID: 1,
        chainID: 1
    }
]

async function main() {
    // relay chain
    const relayChainApi = await ApiPromise.create({
        provider: relayChainProvider,
    });
    // asset hub parachain
    const assetHubApi = await ApiPromise.create({
        provider: assetHubProvider,
    });
    // bridge hub parachain
    const bridgeHubApi = await ApiPromise.create({
        provider: bridgeHubProvider,
    });

    // prepare keyring
    await cryptoWaitReady();
    const keyring = new Keyring({type: 'sr25519'});
    const sudo = keyring.addFromUri('//Alice');

    console.log('======= Relaychain setup begin =======');

    // relaychain setup
    // sovereignaccount of parachain 1000 on relaychain:
    const sovereignAccount1000 = "5Ec4AhPZk8STuex8Wsi9TwDtJQxKqzPJRCH7348Xtcs9vZLJ";
    // sovereignaccount of parachain 1013 on relaychain:
    const sovereignAccount1013 = "5Ec4AhPcMD9pfD1dC3vbyKXoZdZjigWthS9nEwGqaSJksLJv";
    // transfer native asset to parachain sovereignaccount on relay chain
    await transferBalance(relayChainApi, sovereignAccount1000, bn1e12.mul(new BN(100)), true, sudo); // set balance to 100 native asset
    await transferBalance(relayChainApi, sovereignAccount1013, bn1e12.mul(new BN(100)), true, sudo); // set balance to 100 native asset

    console.log('======= Relaychain setup is done =======');

    // checking if parachains start to producing blocks
    console.log("wait for the parachain stars producting blocks...")
    let currentBlockNumber = 0;
    while (currentBlockNumber < 1) {
        const signedBlock = await assetHubApi.rpc.chain.getBlock();
        const blockNumber = signedBlock.block.header.number.toHuman();
        await delay(1000);
        currentBlockNumber = blockNumber
    }

    console.log('======= Parchain setup begin =======');
    // USDC token admin
    const usdcAdmin = sudo.address;

    // create USDC test asset (foreign asset) on asset hub
    await createAsset(assetHubApi, usdcAssetID, usdcAdmin, usdcMinBalance, true, sudo);
    await setAssetMetadata(assetHubApi, usdcAssetID, usdcName, usdcSymbol, usdcDecimal, true, sudo);
    await mintAsset(assetHubApi, usdcAssetID, usdcAdmin, bn1e12.mul(new BN(100)), true, sudo); // mint 100 USDC to Alice

    // create USDC test asset (foreign asset) on bridge hub
    await createAsset(bridgeHubApi, usdcAssetID, usdcAdmin, usdcMinBalance, true, sudo);
    await setAssetMetadata(bridgeHubApi, usdcAssetID, usdcName, usdcSymbol, usdcDecimal, true, sudo);
    await mintAsset(bridgeHubApi, usdcAssetID, usdcAdmin, bn1e12.mul(new BN(100)), true, sudo); // mint 100 USDC to Alice

    // make sure access segregator is set up for Alice before setting up all sygma pallet!
    // sygma config
    const basicFeeAmount = bn1e12.mul(new BN(1)); // 1 * 10 ** 12
    const percentageFeeRate = 500; // 5%
    const feeRateLowerBound = 0;
    const feeRateUpperBound = bn1e12.mul(new BN(1000)); // 1000 * 10 ** 12
    const mpcAddr = process.env.MPCADDR || "0x1c5541A79AcC662ab2D2647F3B141a3B7Cdb2Ae4";

    // register sygma on bridge hub parachain
    // register dest domains
    for (const domain of supportedDestDomains) {
        await registerDomain(bridgeHubApi, domain.domainID, domain.chainID, true, sudo);
    }
    // set fee rate for native asset for domains
    for (const domain of supportedDestDomains) {
        await setFeeHandler(bridgeHubApi, domain.domainID, getNativeAssetId(bridgeHubApi), feeHandlerType.PercentageFeeHandler, true, sudo)
        await setFeeRate(bridgeHubApi, domain.domainID, getNativeAssetId(bridgeHubApi), percentageFeeRate, feeRateLowerBound, feeRateUpperBound, true, sudo);
    }
    // set fee for tokens with domains on bridge hub
    for (const domain of supportedDestDomains) {
        await setFeeHandler(bridgeHubApi, domain.domainID, getUSDCAssetId(bridgeHubApi), feeHandlerType.PercentageFeeHandler, true, sudo)
        await setFeeRate(bridgeHubApi, domain.domainID, getUSDCAssetId(bridgeHubApi), percentageFeeRate, feeRateLowerBound, feeRateUpperBound, true, sudo);
    }

    // transfer some native asset to FeeReserveAccount and TransferReserveAccount as Existential Deposit(aka ED) on bridge hub
    await transferBalance(bridgeHubApi, FeeReserveAccount, bn1e12.mul(new BN(10000)), true, sudo); // set balance to 10000 native asset
    await transferBalance(bridgeHubApi, NativeTokenTransferReserveAccount, bn1e12.mul(new BN(10000)), true, sudo); // set balance to 10000 native asset reserved account
    await transferBalance(bridgeHubApi, OtherTokenTransferReserveAccount, bn1e12.mul(new BN(10000)), true, sudo); // set balance to 10000 other asset reserved account

    // TODO: create another token on bridge hub for testcase 1 and 2, USDC will be used for xcm transfer testcase
    // mint 1 USDC to reserve and fee account so that in the testcase they will not have null as balance
    await mintAsset(bridgeHubApi, usdcAssetID, OtherTokenTransferReserveAccount, bn1e12.mul(new BN(1)), true, sudo); // mint 1 USDC to OtherTokenTransferReserveAccount
    await mintAsset(bridgeHubApi, usdcAssetID, FeeReserveAccount, bn1e12.mul(new BN(1)), true, sudo); // mint 1 USDC to FeeReserveAccount

    // set up MPC address(will also unpause all registered domains) on bridge hub
    if (mpcAddr) {
        console.log(`set up mpc address: ${mpcAddr}`);
        await setMpcAddress(bridgeHubApi, mpcAddr, true, sudo);
        // bridge should be unpaused by the end of the setup
        for (const domain of supportedDestDomains) {
            if (!await queryBridgePauseStatus(bridgeHubApi, domain.domainID)) console.log(`DestDomainID: ${domain.domainID} is ready✅`);
        }
    }
    console.log('🚀 Sygma substrate pallet setup is done! 🚀');

    // transfer native asset to parachain sovereignaccount on each parachain
    await transferBalance(assetHubApi, sovereignAccount1013, bn1e12.mul(new BN(1)), true, sudo); // set balance to 1 native asset
    await transferBalance(bridgeHubApi, sovereignAccount1000, bn1e12.mul(new BN(1)), true, sudo); // set balance to 1 native asset
    console.log('======= Parachain setup is done =======');

    console.log('======= HRMP channel setup begin =======');
    // setup HRMP channel between two parachains
    // init HRMP channel open request from 1000 to 1013
    const openHRMPChannelRequestEncodedData1000To1013 = "0x3c00f50300000800000000001000";
    await hrmpChannelRequest(assetHubApi, getHRMPChannelDest(assetHubApi), getHRMPChannelMessage(assetHubApi, openHRMPChannelRequestEncodedData1000To1013, 1000), 1000, 1013, true, sudo);
    console.log("wait processing on the relay chain...")
    await delay(15000);
    // accept HRMP channel open request on 1013
    const acceptHRMPChannelRequestEncodedData1000To1013 = "0x3c01e8030000";
    await hrmpChannelRequest(bridgeHubApi, getHRMPChannelDest(bridgeHubApi), getHRMPChannelMessage(bridgeHubApi, acceptHRMPChannelRequestEncodedData1000To1013, 1013), 1000, 1013, true, sudo);

    await delay(15000);

    // init HRMP channel open request from 1013 to 1000
    const openHRMPChannelRequestEncodedData1013To1000 = "0x3c00e80300000800000000001000";
    await hrmpChannelRequest(bridgeHubApi, getHRMPChannelDest(bridgeHubApi), getHRMPChannelMessage(bridgeHubApi, openHRMPChannelRequestEncodedData1013To1000, 1013), 1013, 1000, true, sudo);
    console.log("wait processing on the relay chain...")
    await delay(15000);
    // accept HRMP channel open request on 1000
    const acceptHRMPChannelRequestEncodedData1013To1000 = "0x3c01f5030000";
    await hrmpChannelRequest(assetHubApi, getHRMPChannelDest(assetHubApi), getHRMPChannelMessage(assetHubApi, acceptHRMPChannelRequestEncodedData1013To1000, 1000), 1013, 1000, true, sudo);

    // transfer native asset to FungiblesTransactor CheckingAccount on both parachains
    const CheckingAccount = "5EYCAe5ijiYgWYWi1fs8Xz1td1djEtJVVnNfzvDRP4VtLL7Y";
    await transferBalance(assetHubApi, CheckingAccount, bn1e12.mul(new BN(1)), true, sudo); // set balance to 1 native asset
    await transferBalance(bridgeHubApi, CheckingAccount, bn1e12.mul(new BN(1)), true, sudo); // set balance to 1 native asset

    console.log('🚀 HRMP Channel setup is done! 🚀');
}


main().catch(console.error).finally(() => process.exit());
