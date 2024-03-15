// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

require('dotenv').config();

const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
const {cryptoWaitReady} = require('@polkadot/util-crypto');
const {
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
        await setFeeRate(bridgeHubApi, domain.domainID, getUSDCAssetId(bridgeHubApi), percentageFeeRate, feeRateLowerBound, feeRateUpperBound,true, sudo);
    }

    // transfer some native asset to FeeReserveAccount and TransferReserveAccount as Existential Deposit(aka ED) on bridge hub
    await transferBalance(bridgeHubApi, FeeReserveAccount, bn1e12.mul(new BN(10000)), true, sudo); // set balance to 10000 native asset
    await transferBalance(bridgeHubApi, NativeTokenTransferReserveAccount, bn1e12.mul(new BN(10000)), true, sudo); // set balance to 10000 native asset reserved account
    await transferBalance(bridgeHubApi, OtherTokenTransferReserveAccount, bn1e12.mul(new BN(10000)), true, sudo); // set balance to 10000 other asset reserved account

    // mint 1 USDC to reserve and fee acount so that in the testcase they will not have null as balance
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
}

main().catch(console.error).finally(() => process.exit());
