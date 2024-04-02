// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

require('dotenv').config();

const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
const {cryptoWaitReady} = require('@polkadot/util-crypto');
const {
    assetHubProvider,
    bridgeHubProvider,
    getNativeMultiAsset,
    getUSDCMultiAsset,
    getUSDCMultiAssetX2,
    getUSDCAssetIdX2,
    getTTTMultiAsset,
    getAHNMultiAsset,
    getAssetDepositDest,
    teleportTokenViaXCM,
    getAssetHubTeleportDest,
    getAssetHubTeleportBeneficiary,
    getAssetHubTeleportBeneficiaryToSygma,
    getAssetHubTeleportAsset,
    getAssetHubTeleportWeightLimit,
    deposit,
    subEvents,
    queryBalance,
    queryAssetBalance,
    FeeReserveAccount,
    NativeTokenTransferReserveAccount,
    OtherTokenTransferReserveAccount,
    usdcAssetID,
    usdcMinBalance,
    usdcName,
    usdcSymbol,
    usdcDecimal,
    ahnAssetID,
    tttAssetID,
    tttMinBalance,
    tttName,
    tttSymbol,
    tttDecimal,
} = require("./util");

async function main() {
    const assetHubApi = await ApiPromise.create({
        provider: assetHubProvider,
    });
    const bridgeHubApi = await ApiPromise.create({
        provider: bridgeHubProvider,
    });

    // prepare keyring
    await cryptoWaitReady();
    const keyring = new Keyring({type: 'sr25519'});
    const sudo = keyring.addFromUri('//Alice');

    // collection of failed testcases
    let failedTestcases = [];

    // run testcases
    await testcase1(bridgeHubApi, sudo, failedTestcases);
    await testcase2(bridgeHubApi, sudo, failedTestcases);
    await testcase3(assetHubApi, bridgeHubApi, sudo, failedTestcases);
    await testcase4(assetHubApi, bridgeHubApi, sudo, failedTestcases);
    await testcase5(assetHubApi, bridgeHubApi, sudo, failedTestcases);
    await testcase6(assetHubApi, bridgeHubApi, sudo, failedTestcases);

    // checking if any testcase failed
    for (const item of failedTestcases) {
        console.error('\x1b[31m%s\x1b[0m\n', item);
        return
    }
    console.log('\x1b[32m%s\x1b[0m', "All testcases pass");
}

function str2BigInt(a) {
    return BigInt(a.replaceAll(',', ''));
}

// testcase 1: Native token deposit on Bridge hub, dest on relayer
async function testcase1(bridgeHubApi, sudo, failedTestcases) {
    const nativeBalanceBeforeAlice = await queryBalance(bridgeHubApi, sudo.address);
    console.log('Alice native asset balance before: ', nativeBalanceBeforeAlice.data.free);

    const nativeBalanceBeforeNativeTokenTransferReserveAccount = await queryBalance(bridgeHubApi, NativeTokenTransferReserveAccount);
    console.log('token reserve account native asset balance before: ', nativeBalanceBeforeNativeTokenTransferReserveAccount.data.free);

    const nativeBalanceBeforeFeeAccount = await queryBalance(bridgeHubApi, FeeReserveAccount);
    console.log('fee account native asset balance before: ', nativeBalanceBeforeFeeAccount.data.free);

    const depositAmount = 10000000000000; // 10 tokens
    await deposit(bridgeHubApi, getNativeMultiAsset(bridgeHubApi, depositAmount), getAssetDepositDest(bridgeHubApi), true, sudo)

    const nativeBalanceAfterAlice = await queryBalance(bridgeHubApi, sudo.address);
    console.log('Alice native asset balance after: ', nativeBalanceAfterAlice.data.free);

    const nativeBalanceAfterNativeTokenTransferReserveAccount = await queryBalance(bridgeHubApi, NativeTokenTransferReserveAccount);
    console.log('token reserve account native asset balance after: ', nativeBalanceAfterNativeTokenTransferReserveAccount.data.free);

    const nativeBalanceAfterFeeAccount = await queryBalance(bridgeHubApi, FeeReserveAccount);
    console.log('fee account native asset balance after: ', nativeBalanceAfterFeeAccount.data.free);

    // Alice balance should be deducted by 10 + tx fee, so the before - after should be greater than 10 tokens
    if (str2BigInt(nativeBalanceBeforeAlice.data.free) - str2BigInt(nativeBalanceAfterAlice.data.free) <= BigInt(depositAmount)) {
        failedTestcases.push("testcase 1 failed: Alice balance not match")
    }
    // balance reserve account should add deposit amount - fee which is 9,500,000,000,000
    if (str2BigInt(nativeBalanceAfterNativeTokenTransferReserveAccount.data.free) - str2BigInt(nativeBalanceBeforeNativeTokenTransferReserveAccount.data.free) !== BigInt(9500000000000)) {
        failedTestcases.push("testcase 1 failed: NativeTokenTransferReserveAccount balance not match")
    }
    // fee account should add 500,000,000,000
    if (str2BigInt(nativeBalanceAfterFeeAccount.data.free) - str2BigInt(nativeBalanceBeforeFeeAccount.data.free) !== BigInt(500000000000)) {
        failedTestcases.push("testcase 1 failed: FeeAccount balance not match")
    }
}

// testcase 2: Foreign token TTT deposit on Bridge hub, dest on relayer
async function testcase2(bridgeHubApi, sudo, failedTestcases) {
    let tttBalanceBeforeAlice = await queryAssetBalance(bridgeHubApi, tttAssetID, sudo.address);
    console.log('Alice TTT asset balance before: ', tttBalanceBeforeAlice.balance);

    const tttBalanceBeforeOtherTokenTransferReserveAccount = await queryAssetBalance(bridgeHubApi, tttAssetID, OtherTokenTransferReserveAccount);
    console.log('OtherTokenTransferReserveAccount TTT asset balance before: ', tttBalanceBeforeOtherTokenTransferReserveAccount.balance);

    const tttBalanceBeforeFeeReserveAccount = await queryAssetBalance(bridgeHubApi, tttAssetID, FeeReserveAccount);
    console.log('FeeReserveAccountAddress TTT asset balance before: ', tttBalanceBeforeFeeReserveAccount.balance);

    const tttDepositAmount = 10000000000000; // 10 tokens
    await deposit(bridgeHubApi, getTTTMultiAsset(bridgeHubApi, tttDepositAmount), getAssetDepositDest(bridgeHubApi), true, sudo)

    let tttBalanceAfterAlice = await queryAssetBalance(bridgeHubApi, tttAssetID, sudo.address);
    console.log('Alice TTT asset balance after: ', tttBalanceAfterAlice.balance);

    const tttBalanceAfterOtherTokenTransferReserveAccount = await queryAssetBalance(bridgeHubApi, tttAssetID, OtherTokenTransferReserveAccount);
    console.log('OtherTokenTransferReserveAccount TTT asset balance after: ', tttBalanceAfterOtherTokenTransferReserveAccount.balance);

    const tttBalanceAfterFeeReserveAccount = await queryAssetBalance(bridgeHubApi, tttAssetID, FeeReserveAccount);
    console.log('FeeReserveAccountAddress TTT asset balance after: ', tttBalanceAfterFeeReserveAccount.balance);

    // Alice should be deducted by 10 TTT tokens
    if (str2BigInt(tttBalanceBeforeAlice.balance) - str2BigInt(tttBalanceAfterAlice.balance) !== BigInt(tttDepositAmount)) {
        failedTestcases.push("testcase 2 failed: Alice TTT token balance not match")
    }
    // OtherTokenTransferReserveAccount should add 0 because TTT is a non-reserve token on Bridge hub
    if (str2BigInt(tttBalanceBeforeOtherTokenTransferReserveAccount.balance) !== str2BigInt(tttBalanceAfterOtherTokenTransferReserveAccount.balance)) {
        failedTestcases.push("testcase 2 failed: OtherTokenTransferReserveAccount TTT token balance not match")
    }
    // FeeReserveAccount should add fee which is 500000000000
    if (str2BigInt(tttBalanceAfterFeeReserveAccount.balance) - str2BigInt(tttBalanceBeforeFeeReserveAccount.balance) !== BigInt(500000000000)) {
        failedTestcases.push("testcase 2 failed: FeeReserveAccount TTT token balance not match")
    }
}

// testcase 3: Foreign token(USDC) deposit on Asset hub, dest on Bridge hub
async function testcase3(assetHubApi, bridgeHubApi, sudo, failedTestcases) {
    const usdcBalanceBeforeAliceAssethub = await queryAssetBalance(assetHubApi, usdcAssetID, sudo.address);
    console.log('Alice USDC asset balance on Asset hub before: ', usdcBalanceBeforeAliceAssethub.balance);

    const usdcBalanceBeforeAliceBridgehub = await queryAssetBalance(bridgeHubApi, usdcAssetID, sudo.address);
    console.log('Alice USDC asset balance on Bridge hub before: ', usdcBalanceBeforeAliceBridgehub.balance);

    const depositAmount = 10000000000000; // 10 tokens
    const beneficiary = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d" // Alice
    await teleportTokenViaXCM(
        assetHubApi,
        {
            dest: getAssetHubTeleportDest(assetHubApi),
            beneficiary: getAssetHubTeleportBeneficiary(assetHubApi, beneficiary),
            assets: getAssetHubTeleportAsset(assetHubApi, getUSDCMultiAssetX2(assetHubApi, depositAmount)),
            feeAssetItem: 0,
            weightLimit: getAssetHubTeleportWeightLimit(assetHubApi),
            fromParachainID: 1000,
            toParachainID: 1013
        },
        true, sudo)

    const usdcBalanceAfterAliceAssethub = await queryAssetBalance(assetHubApi, usdcAssetID, sudo.address);
    console.log('Alice USDC asset balance on Asset hub after: ', usdcBalanceAfterAliceAssethub.balance);

    const usdcBalanceAfterAliceBridgehub = await queryAssetBalance(bridgeHubApi, usdcAssetID, sudo.address);
    console.log('Alice USDC asset balance on Bridge hub after: ', usdcBalanceAfterAliceBridgehub.balance);

    // Alice USDC balance should be deducted by 10, so before - after should be equal to 10 tokens on asset hub
    if (str2BigInt(usdcBalanceBeforeAliceAssethub.balance) - str2BigInt(usdcBalanceAfterAliceAssethub.balance) !== BigInt(depositAmount)) {
        failedTestcases.push("testcase 3 failed: Alice USDC balance on Asset hub not match")
    }

    // Alice USDC balance should be added by 10 - tx fee, so after - before should be less than 10 tokens
    if (str2BigInt(usdcBalanceAfterAliceBridgehub.balance) - str2BigInt(usdcBalanceBeforeAliceBridgehub.balance) >= BigInt(depositAmount)) {
        failedTestcases.push("testcase 3 failed: Alice USDC balance on Bridge hub not match")
    }
}

// testcase 4: Native token deposit on Asset hub, dest on Bridge hub
async function testcase4(assetHubApi, bridgeHubApi, sudo, failedTestcases) {
    const beneficiaryAddressOnBridgehub = "5GYrSdyt7wydaQiqsnrvq11neaC2eTUBXCnXhSJKpUPT3hXP";
    const beneficiary = "0xc668b505f6a7012a50dca169757c629651bfd6cefbfc24301dea2d2cc0ab2732" // Alice_extension

    const nativeBalanceBeforeAliceAssethub = await queryBalance(assetHubApi, sudo.address);
    console.log('Alice native asset balance on Asset hub before: ', nativeBalanceBeforeAliceAssethub.data.free);

    const nativeBalanceBeforeAliceBridgehub = await queryAssetBalance(bridgeHubApi, ahnAssetID, beneficiaryAddressOnBridgehub);
    console.log('Alice assethub\'s native asset balance on Bridge hub before: ', nativeBalanceBeforeAliceBridgehub.balance);

    const depositAmount = 2000000000000; // 2 tokens
    await teleportTokenViaXCM(
        assetHubApi,
        {
            dest: getAssetHubTeleportDest(assetHubApi),
            beneficiary: getAssetHubTeleportBeneficiary(assetHubApi, beneficiary),
            assets: getAssetHubTeleportAsset(assetHubApi, getNativeMultiAsset(assetHubApi, depositAmount)),
            feeAssetItem: 0,
            weightLimit: getAssetHubTeleportWeightLimit(assetHubApi),
            fromParachainID: 1000,
            toParachainID: 1013
        },
        true, sudo)

    const nativeBalanceAfterAliceAssethub = await queryBalance(assetHubApi, sudo.address);
    console.log('Alice native asset balance on Asset hub after: ', nativeBalanceAfterAliceAssethub.data.free);

    const nativeBalanceAfterAliceBridgehub = await queryAssetBalance(bridgeHubApi, ahnAssetID, beneficiaryAddressOnBridgehub);
    console.log('Alice assethub\'s native asset balance on Bridge hub after: ', nativeBalanceAfterAliceBridgehub.balance);

    // Alice native token balance should be deducted by 2 and some tx fee, so before - after should be greater than 2 tokens on asset hub
    if (str2BigInt(nativeBalanceBeforeAliceAssethub.data.free) - str2BigInt(nativeBalanceAfterAliceAssethub.data.free) <= BigInt(depositAmount)) {
        failedTestcases.push("testcase 4 failed: Alice native asset balance on Asset hub not match")
    }

    // Alice native asset token balance should be added by 2 - tx fee on bridge hub, so after - before should be less than 2 tokens
    if (str2BigInt(nativeBalanceAfterAliceBridgehub.balance) - str2BigInt(nativeBalanceBeforeAliceBridgehub.balance) >= BigInt(depositAmount)) {
        failedTestcases.push("testcase 4 failed: Alice native asset token balance on Bridge hub not match")
    }
}

// testcase 5: Foreign token(USDC) deposit on Asset hub, dest on sygma via Bridge hub
async function testcase5(assetHubApi, bridgeHubApi, sudo, failedTestcases) {
    const events = [];
    await subEvents(bridgeHubApi, events);

    const usdcBalanceBeforeAliceAssethub = await queryAssetBalance(assetHubApi, usdcAssetID, sudo.address);
    console.log('Alice USDC asset balance on Asset hub before: ', usdcBalanceBeforeAliceAssethub.balance);

    const usdcBalanceBeforeOtherTokenTransferReserveAccount = await queryAssetBalance(bridgeHubApi, usdcAssetID, OtherTokenTransferReserveAccount);
    console.log('OtherTokenTransferReserveAccount USDC asset balance on Bridge hub before: ', usdcBalanceBeforeOtherTokenTransferReserveAccount.balance);

    const depositAmount = 5000000000000; // 5 tokens
    await teleportTokenViaXCM(
        assetHubApi,
        {
            dest: getAssetHubTeleportDest(assetHubApi),
            beneficiary: getAssetHubTeleportBeneficiaryToSygma(assetHubApi), // EVM address: 0x1abd6948e422a1b6ced1ba28ba72ca562333df01
            assets: getAssetHubTeleportAsset(assetHubApi, getUSDCMultiAssetX2(assetHubApi, depositAmount)),
            feeAssetItem: 0,
            weightLimit: getAssetHubTeleportWeightLimit(assetHubApi),
            fromParachainID: 1000,
            toParachainID: 1013
        },
        true, sudo)

    const usdcBalanceAfterAliceAssethub = await queryAssetBalance(assetHubApi, usdcAssetID, sudo.address);
    console.log('Alice USDC asset balance on Asset hub after: ', usdcBalanceAfterAliceAssethub.balance);

    const usdcBalanceAfterOtherTokenTransferReserveAccount = await queryAssetBalance(bridgeHubApi, usdcAssetID, OtherTokenTransferReserveAccount);
    console.log('OtherTokenTransferReserveAccount USDC asset balance on Bridge hub after: ', usdcBalanceAfterOtherTokenTransferReserveAccount.balance);

    // Alice USDC balance should be deducted by 5, so before - after should be equal to 10 tokens on asset hub
    if (str2BigInt(usdcBalanceBeforeAliceAssethub.balance) - str2BigInt(usdcBalanceAfterAliceAssethub.balance) !== BigInt(depositAmount)) {
        failedTestcases.push("testcase 5 failed: Alice USDC balance on Asset hub not match")
    }

    // OtherTokenTransferReserveAccount USDC balance should be added by 5 - tx fee, so after - before should be less than 5 tokens
    if (str2BigInt(usdcBalanceAfterOtherTokenTransferReserveAccount.balance) - str2BigInt(usdcBalanceBeforeOtherTokenTransferReserveAccount.balance) >= BigInt(depositAmount)) {
        failedTestcases.push("testcase 5 failed: OtherTokenTransferReserveAccount USDC balance on Bridge hub not match")
    }

    // checking if any sygma events emitted
    for (const e of events) {
        console.log('sygma pallets event emitted: \x1b[32m%s\x1b[0m\n', e);
    }
    if (events.length === 0) {
        failedTestcases.push("testcase 5 failed: sygma pallets event not emitted");
    }
}

// testcase 6: Native token deposit on Asset hub, dest on sygma via Bridge hub
async function testcase6(assetHubApi, bridgeHubApi, sudo, failedTestcases) {
    const events = [];
    await subEvents(bridgeHubApi, events);

    const nativeBalanceBeforeAliceAssethub = await queryBalance(assetHubApi, sudo.address);
    console.log('Alice native asset balance on Asset hub before: ', nativeBalanceBeforeAliceAssethub.data.free);

    const nativeBalanceBeforeOtherTokenTransferReserveAccount = await queryAssetBalance(bridgeHubApi, ahnAssetID, OtherTokenTransferReserveAccount);
    console.log('OtherTokenTransferReserveAccount AHN balance on Bridge hub before: ', nativeBalanceBeforeOtherTokenTransferReserveAccount.balance);

    const depositAmount = 2000000000000; // 5 tokens
    await teleportTokenViaXCM(
        assetHubApi,
        {
            dest: getAssetHubTeleportDest(assetHubApi),
            beneficiary: getAssetHubTeleportBeneficiaryToSygma(assetHubApi), // EVM address: 0x1abd6948e422a1b6ced1ba28ba72ca562333df01
            assets: getAssetHubTeleportAsset(assetHubApi, getNativeMultiAsset(assetHubApi, depositAmount)),
            feeAssetItem: 0,
            weightLimit: getAssetHubTeleportWeightLimit(assetHubApi),
            fromParachainID: 1000,
            toParachainID: 1013
        },
        true, sudo)

    const nativeBalanceAfterAliceAssethub = await queryBalance(assetHubApi, sudo.address);
    console.log('Alice native asset balance on Asset hub before: ', nativeBalanceAfterAliceAssethub.data.free);

    const nativeBalanceAfterOtherTokenTransferReserveAccount = await queryAssetBalance(bridgeHubApi, ahnAssetID, OtherTokenTransferReserveAccount);
    console.log('OtherTokenTransferReserveAccount AHN balance on Bridge hub after: ', nativeBalanceAfterOtherTokenTransferReserveAccount.balance);

    // Alice native token balance should be deducted by 2 and some tx fee, so before - after should be greater than 2 tokens on asset hub
    if (str2BigInt(nativeBalanceBeforeAliceAssethub.data.free) - str2BigInt(nativeBalanceAfterAliceAssethub.data.free) <= BigInt(depositAmount)) {
        failedTestcases.push("testcase 6 failed: Alice native asset balance on Asset hub not match")
    }

    // OtherTokenTransferReserveAccount AHN balance should be added by 2 - tx fee, so after - before should be less than 2 tokens
    if (str2BigInt(nativeBalanceAfterOtherTokenTransferReserveAccount.balance) - str2BigInt(nativeBalanceBeforeOtherTokenTransferReserveAccount.balance) >= BigInt(depositAmount)) {
        failedTestcases.push("testcase 6 failed: OtherTokenTransferReserveAccount AHN balance on Bridge hub not match")
    }

    // checking if any sygma events emitted
    for (const e of events) {
        console.log('sygma pallets event emitted: \x1b[32m%s\x1b[0m\n', e);
    }
    if (events.length === 0) {
        failedTestcases.push("testcase 6 failed: sygma pallets event not emitted");
    }
}


main().catch(console.error).finally(() => process.exit());

