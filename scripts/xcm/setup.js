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
    getAHNMultiAsset,
    getAHNAssetId,
    getTTTMultiAsset,
    getTTTAssetId,
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
    ahnAssetID,
    ahnMinBalance,
    ahnName,
    ahnSymbol,
    ahnDecimal,
    tttAssetID,
    tttMinBalance,
    tttName,
    tttSymbol,
    tttDecimal,
    bhnAssetID,
    bhnMinBalance,
    bhnName,
    bhnSymbol,
    bhnDecimal,
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
    await transferBalance(relayChainApi, sovereignAccount1000, bn1e12.mul(new BN(10)), true, sudo); // set balance to 10 native asset
    await transferBalance(relayChainApi, sovereignAccount1013, bn1e12.mul(new BN(10)), true, sudo); // set balance to 10 native asset

    console.log('======= Relaychain setup is done =======');

    // checking if parachains start to producing blocks
    console.log("wait for the parachain stars producing blocks...")
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
    // AHN token admin
    const ahnAdmin = sudo.address;
    // BHN token admin
    const bhnAdmin = sudo.address;
    // TTT token admin
    const tttAdmin = sudo.address;

    const extensionAliceAccount = "5GYrSdyt7wydaQiqsnrvq11neaC2eTUBXCnXhSJKpUPT3hXP"

    // create USDC test asset (foreign asset) on asset hub
    await createAsset(assetHubApi, usdcAssetID, usdcAdmin, usdcMinBalance, true, sudo);
    await setAssetMetadata(assetHubApi, usdcAssetID, usdcName, usdcSymbol, usdcDecimal, true, sudo);
    await mintAsset(assetHubApi, usdcAssetID, usdcAdmin, bn1e12.mul(new BN(100)), true, sudo); // mint 100 USDC to Alice

    // create USDC test asset (foreign asset) on bridge hub
    await createAsset(bridgeHubApi, usdcAssetID, usdcAdmin, usdcMinBalance, true, sudo);
    await setAssetMetadata(bridgeHubApi, usdcAssetID, usdcName, usdcSymbol, usdcDecimal, true, sudo);
    await mintAsset(bridgeHubApi, usdcAssetID, usdcAdmin, bn1e12.mul(new BN(100)), true, sudo); // mint 100 USDC to Alice

    // create Asset Hub Native(AHN) test asset (foreign asset) on bridge hub
    // this is for mapping the Asset Hub Native asset on Bridge hub
    await createAsset(bridgeHubApi, ahnAssetID, ahnAdmin, ahnMinBalance, true, sudo);
    await setAssetMetadata(bridgeHubApi, ahnAssetID, ahnName, ahnSymbol, ahnDecimal, true, sudo);
    await mintAsset(bridgeHubApi, ahnAssetID, ahnAdmin, bn1e12.mul(new BN(100)), true, sudo); // mint 100 AHN to Alice

    // create Bridge Hub Native(BHN) test asset (foreign asset) on Asset hub
    // this is for mapping the Bridge Hub Native asset on Asset hub
    await createAsset(assetHubApi, bhnAssetID, bhnAdmin, bhnMinBalance, true, sudo);
    await setAssetMetadata(assetHubApi, bhnAssetID, bhnName, bhnSymbol, bhnDecimal, true, sudo);
    await mintAsset(assetHubApi, bhnAssetID, bhnAdmin, bn1e12.mul(new BN(100)), true, sudo); // mint 100 BHN to Alice

    // create TTT test asset (foreign asset) on bridge hub
    // this is for mapping the local foreign token on Bridge hub
    await createAsset(bridgeHubApi, tttAssetID, tttAdmin, tttMinBalance, true, sudo);
    await setAssetMetadata(bridgeHubApi, tttAssetID, tttName, tttSymbol, tttDecimal, true, sudo);
    await mintAsset(bridgeHubApi, tttAssetID, tttAdmin, bn1e12.mul(new BN(100)), true, sudo); // mint 100 TTT to Alice

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
        // set fee for TTT token for local token tests on bridge hub
        await setFeeHandler(bridgeHubApi, domain.domainID, getTTTAssetId(bridgeHubApi), feeHandlerType.PercentageFeeHandler, true, sudo)
        await setFeeRate(bridgeHubApi, domain.domainID, getTTTAssetId(bridgeHubApi), percentageFeeRate, feeRateLowerBound, feeRateUpperBound,true, sudo);

        // USDC as a foreign token is used for xcm related testcases
        await setFeeHandler(bridgeHubApi, domain.domainID, getUSDCAssetId(bridgeHubApi), feeHandlerType.BasicFeeHandler, true, sudo)
        await setFee(bridgeHubApi, domain.domainID, getUSDCAssetId(bridgeHubApi), basicFeeAmount,true, sudo);

        // AHN as a Native token of asset hub is used for xcm related testcases
        await setFeeHandler(bridgeHubApi, domain.domainID, getAHNAssetId(bridgeHubApi), feeHandlerType.BasicFeeHandler, true, sudo)
        await setFee(bridgeHubApi, domain.domainID, getAHNAssetId(bridgeHubApi), basicFeeAmount,true, sudo);
    }

    // transfer some native asset to FeeReserveAccount and TransferReserveAccount as Existential Deposit(aka ED) on bridge hub
    await transferBalance(bridgeHubApi, FeeReserveAccount, bn1e12.mul(new BN(10)), true, sudo); // set balance to 10 native asset
    await transferBalance(bridgeHubApi, NativeTokenTransferReserveAccount, bn1e12.mul(new BN(10)), true, sudo); // set balance to 10 native asset reserved account
    await transferBalance(bridgeHubApi, OtherTokenTransferReserveAccount, bn1e12.mul(new BN(10)), true, sudo); // set balance to 10 other asset reserved account

    // mint 1 TTT to reserve and fee account so that in the testcase they will not have null as balance
    await mintAsset(bridgeHubApi, tttAssetID, OtherTokenTransferReserveAccount, bn1e12.mul(new BN(1)), true, sudo); // mint 1 TTT to OtherTokenTransferReserveAccount
    await mintAsset(bridgeHubApi, tttAssetID, FeeReserveAccount, bn1e12.mul(new BN(1)), true, sudo); // mint 1 TTT to FeeReserveAccount

    // mint 100 USDC to reserve account so that the testcase 7 will have some init funds
    await mintAsset(bridgeHubApi, usdcAssetID, OtherTokenTransferReserveAccount, bn1e12.mul(new BN(100)), true, sudo); // mint 100 USDC to OtherTokenTransferReserveAccount

    // mint 10 USDC to the sibling sovereignaccount of 1013 on asset hub
    // USDC is reserved on asset hub, so in testcase 9, the reserved token from siblingSovereignAccount1013 will be transferred to the recipient on Asset hub
    const siblingSovereignAccount1013= "5Eg2fntRRwLinojmk3sh5xscp7F3S6Zzm5oDVtoLTALKiypR";
    await transferBalance(assetHubApi, siblingSovereignAccount1013, bn1e12.mul(new BN(10)), true, sudo); // make sure the sibling sovereignaccount of 1013 on asset hub exists
    await mintAsset(assetHubApi, usdcAssetID, siblingSovereignAccount1013, bn1e12.mul(new BN(10)), true, sudo);

    // set up MPC address(will also unpause all registered domains) on bridge hub
    if (mpcAddr) {
        console.log(`set up mpc address: ${mpcAddr}`);
        await setMpcAddress(bridgeHubApi, mpcAddr, true, sudo);
        // bridge should be unpaused by the end of the setup
        for (const domain of supportedDestDomains) {
            if (!await queryBridgePauseStatus(bridgeHubApi, domain.domainID)) console.log(`DestDomainID: ${domain.domainID} is ready✅`);
        }
    }

    // transfer native asset to the sovereignaccount of the other
    await transferBalance(assetHubApi, sovereignAccount1013, bn1e12.mul(new BN(1)), true, sudo); // set balance to 1 native asset
    await transferBalance(bridgeHubApi, sovereignAccount1000, bn1e12.mul(new BN(1)), true, sudo); // set balance to 1 native asset

    // transfer native asset to extension alice account on bridge hub
    // this is for teleport native asset of Asset hub(AHN) -> Bridge hub testcase
    await transferBalance(bridgeHubApi, extensionAliceAccount, bn1e12.mul(new BN(1)), true, sudo); // set balance to 1 native asset

    // mint 10 AHN to extensionAliceAccount, used in testcase 4
    await mintAsset(bridgeHubApi, ahnAssetID, extensionAliceAccount, bn1e12.mul(new BN(10)), true, sudo);
    // mint 10 AHN to OtherTokenTransferReserveAccount, used in testcase 6
    await mintAsset(bridgeHubApi, ahnAssetID, OtherTokenTransferReserveAccount, bn1e12.mul(new BN(10)), true, sudo); // mint 10 AHN to OtherTokenTransferReserveAccount


    // transfer native asset to FungiblesTransactor CheckingAccount on both parachains
    // this is part of the parachain launching setup, ideally should be done by parachain team after launching, but in our testing env we are using brand-new chain, so we need to set this up.
    const CheckingAccount = "5EYCAe5ijiYgWYWi1fs8Xz1td1djEtJVVnNfzvDRP4VtLL7Y";
    await transferBalance(assetHubApi, CheckingAccount, bn1e12.mul(new BN(1)), true, sudo); // set balance to 1 native asset
    await transferBalance(bridgeHubApi, CheckingAccount, bn1e12.mul(new BN(1)), true, sudo); // set balance to 1 native asset

    // some other addresses need to exist as well, they are tmp accounts in pallets
    const sygmaXCMTransactorAccount = "5ExVnaLuWGe8WqCpaY4jg65kMz5hefx5A2covME3RhE4Y1m1";
    const sygmaXCMTransactorAccount2 = "5D6gSNWpCcRowidpVC2k3FzmrfJjHX1Wu2NuBgsi717qtL5Y"; // when transfer BHN from sygma relayer to asset hub via bridge hub, this account received the BHN from NativeReservedAcoount on bridge hub
    await transferBalance(bridgeHubApi, sygmaXCMTransactorAccount, bn1e12.mul(new BN(10)), true, sudo); // set balance to 10 native asset
    await transferBalance(bridgeHubApi, sygmaXCMTransactorAccount2, bn1e12.mul(new BN(10)), true, sudo); // set balance to 10 native asset

    console.log('======= Parachain setup is done =======');

    console.log('======= HRMP channel setup begin =======');
    // setup HRMP channel between two parachains
    // init HRMP channel open request from 1000 to 1013
    const openHRMPChannelRequestEncodedData1000To1013 = "0x3c00f50300000800000000001000";
    await hrmpChannelRequest(assetHubApi, getHRMPChannelDest(assetHubApi), getHRMPChannelMessage(assetHubApi, openHRMPChannelRequestEncodedData1000To1013, 1000), 1000, 1013, true, sudo);
    console.log("wait processing on the relay chain...")
    await delay(10000);
    // accept HRMP channel open request on 1013
    const acceptHRMPChannelRequestEncodedData1000To1013 = "0x3c01e8030000";
    await hrmpChannelRequest(bridgeHubApi, getHRMPChannelDest(bridgeHubApi), getHRMPChannelMessage(bridgeHubApi, acceptHRMPChannelRequestEncodedData1000To1013, 1013), 1000, 1013, true, sudo);

    await delay(5000);

    // init HRMP channel open request from 1013 to 1000
    const openHRMPChannelRequestEncodedData1013To1000 = "0x3c00e80300000800000000001000";
    await hrmpChannelRequest(bridgeHubApi, getHRMPChannelDest(bridgeHubApi), getHRMPChannelMessage(bridgeHubApi, openHRMPChannelRequestEncodedData1013To1000, 1013), 1013, 1000, true, sudo);
    console.log("wait processing on the relay chain...")
    await delay(10000);
    // accept HRMP channel open request on 1000
    const acceptHRMPChannelRequestEncodedData1013To1000 = "0x3c01f5030000";
    await hrmpChannelRequest(assetHubApi, getHRMPChannelDest(assetHubApi), getHRMPChannelMessage(assetHubApi, acceptHRMPChannelRequestEncodedData1013To1000, 1000), 1013, 1000, true, sudo);

    console.log('======= HRMP Channel setup is done! =======');

    console.log('🚀 setup is done! 🚀');
}

main().catch(console.error).finally(() => process.exit());
