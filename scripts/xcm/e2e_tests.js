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
    getAssetDepositDest,
    deposit,
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
    // await testcase3(assetHubApi, sudo, failedTestcases);

    // checking if any testcase failed
    for (const item of failedTestcases) {
        console.error('\x1b[31m%s\x1b[0m', item);
        return
    }
    console.log('\x1b[32m%s\x1b[0m', "All testcases pass");
}

// testcase 1: Native token deposit on Bridge hub, dest on relayer
async function testcase1(bridgeHubApi, sudo, failedTestcases) {
    const nativeBalanceBeforeAlice = await queryBalance(bridgeHubApi, sudo.address);
    console.log('Alice native asset balance before: ', nativeBalanceBeforeAlice.data.free);

    const nativeBalanceBeforeNativeTokenTransferReserveAccount = await queryBalance(bridgeHubApi, NativeTokenTransferReserveAccount);
    console.log('token reserve account native asset balance before: ', nativeBalanceBeforeNativeTokenTransferReserveAccount.data.free);

    const nativeBalanceBeforeFeeAccount = await queryBalance(bridgeHubApi, FeeReserveAccount);
    console.log('fee account native asset balance before: ', nativeBalanceBeforeFeeAccount.data.free);

    const depositAmount = 10000000000000;
    await deposit(bridgeHubApi, getNativeMultiAsset(bridgeHubApi, depositAmount), getAssetDepositDest(bridgeHubApi), true, sudo)

    const nativeBalanceAfterAlice = await queryBalance(bridgeHubApi, sudo.address);
    console.log('Alice native asset balance after: ', nativeBalanceAfterAlice.data.free);

    const nativeBalanceAfterNativeTokenTransferReserveAccount = await queryBalance(bridgeHubApi, NativeTokenTransferReserveAccount);
    console.log('token reserve account native asset balance after: ', nativeBalanceAfterNativeTokenTransferReserveAccount.data.free);

    const nativeBalanceAfterFeeAccount = await queryBalance(bridgeHubApi, FeeReserveAccount);
    console.log('fee account native asset balance after: ', nativeBalanceAfterFeeAccount.data.free);

    // Alice balance should be deducted by 10 + tx fee, so the before - after should be greater than 10 tokens
    if (nativeBalanceBeforeAlice.data.free - nativeBalanceAfterAlice.data.free <= depositAmount) {
        failedTestcases.push("testcase 1 failed: Alice balance not match")
    }
    // balance reserve account should add deposit amount - fee which is 9,500,000,000,000
    if (nativeBalanceBeforeNativeTokenTransferReserveAccount.data.free - nativeBalanceAfterNativeTokenTransferReserveAccount.data.free === 9500000000000) {
        failedTestcases.push("testcase 1 failed: NativeTokenTransferReserveAccount balance not match")
    }
    // fee account should add 9,500,000,000,000
    if (nativeBalanceAfterFeeAccount.data.free - nativeBalanceBeforeFeeAccount.data.free === 9500000000000) {
        failedTestcases.push("testcase 1 failed: FeeAccount balance not match")
    }
}

// testcase 2: Foreign token deposit on Bridge hub, dest on relayer
async function testcase2(bridgeHubApi, sudo, failedTestcases) {
    const usdcBalanceBeforeAlice = await queryAssetBalance(bridgeHubApi, usdcAssetID, sudo.address);
    console.log('Alice USDC asset balance before: ', usdcBalanceBeforeAlice.balance);

    const usdcBalanceBeforeOtherTokenTransferReserveAccount = await queryAssetBalance(bridgeHubApi, usdcAssetID, OtherTokenTransferReserveAccount);
    console.log('OtherTokenTransferReserveAccount USDC asset balance before: ', usdcBalanceBeforeOtherTokenTransferReserveAccount.balance);

    const usdcBalanceBeforeFeeReserveAccount = await queryAssetBalance(bridgeHubApi, usdcAssetID, FeeReserveAccount);
    console.log('FeeReserveAccountAddress USDC asset balance before: ', usdcBalanceBeforeFeeReserveAccount.balance);

    const usdcDepositAmount = 10000000000000;
    await deposit(bridgeHubApi, getUSDCMultiAsset(bridgeHubApi, usdcDepositAmount), getAssetDepositDest(bridgeHubApi), true, sudo)

    const usdcBalanceAfterAlice = await queryAssetBalance(bridgeHubApi, usdcAssetID, sudo.address);
    console.log('Alice USDC asset balance after: ', usdcBalanceAfterAlice.balance);

    const usdcBalanceAfterOtherTokenTransferReserveAccount = await queryAssetBalance(bridgeHubApi, usdcAssetID, OtherTokenTransferReserveAccount);
    console.log('OtherTokenTransferReserveAccount USDC asset balance after: ', usdcBalanceAfterOtherTokenTransferReserveAccount.balance);

    const usdcBalanceAfterFeeReserveAccount = await queryAssetBalance(bridgeHubApi, usdcAssetID, FeeReserveAccount);
    console.log('FeeReserveAccountAddress USDC asset balance after: ', usdcBalanceAfterFeeReserveAccount.balance);

    // Alice should be deducted by 10 USDC tokens
    if (usdcBalanceBeforeAlice.balance - usdcBalanceAfterAlice.balance === usdcDepositAmount) {
        failedTestcases.push("testcase 2 failed: Alice USDC token balance not match")
    }
    // OtherTokenTransferReserveAccount should add deposit amount - fee
    if (usdcBalanceBeforeOtherTokenTransferReserveAccount.balance - usdcBalanceAfterOtherTokenTransferReserveAccount.balance === usdcDepositAmount - 500000000000) {
        failedTestcases.push("testcase 2 failed: OtherTokenTransferReserveAccount USDC token balance not match")
    }
    // FeeReserveAccount should add fee which is 500000000000
    if (usdcBalanceAfterFeeReserveAccount.balance - usdcBalanceBeforeFeeReserveAccount.balance === 500000000000) {
        failedTestcases.push("testcase 2 failed: FeeReserveAccount USDC token balance not match")
    }
}

// testcase 3: Native token deposit on Asset hub, dest on Bridge hub
async function testcase3(assetHubApi, sudo, failedTestcases) {
    // const nativeBalanceBeforeAlice = await queryBalance(bridgeHubApi, sudo.address);
    // console.log('Alice native asset balance before: ', nativeBalanceBeforeAlice.data.free);
    //
    // const nativeBalanceBeforeNativeTokenTransferReserveAccount = await queryBalance(bridgeHubApi, NativeTokenTransferReserveAccount);
    // console.log('token reserve account native asset balance before: ', nativeBalanceBeforeNativeTokenTransferReserveAccount.data.free);
    //
    // const nativeBalanceBeforeFeeAccount = await queryBalance(bridgeHubApi, FeeReserveAccount);
    // console.log('fee account native asset balance before: ', nativeBalanceBeforeFeeAccount.data.free);

    const depositAmount = 10000000000000;
    await deposit(assetHubApi, getNativeMultiAsset(assetHubApi, depositAmount), getAssetDepositDest(assetHubApi), true, sudo)

    // const nativeBalanceAfterAlice = await queryBalance(bridgeHubApi, sudo.address);
    // console.log('Alice native asset balance after: ', nativeBalanceAfterAlice.data.free);
    //
    // const nativeBalanceAfterNativeTokenTransferReserveAccount = await queryBalance(bridgeHubApi, NativeTokenTransferReserveAccount);
    // console.log('token reserve account native asset balance after: ', nativeBalanceAfterNativeTokenTransferReserveAccount.data.free);
    //
    // const nativeBalanceAfterFeeAccount = await queryBalance(bridgeHubApi, FeeReserveAccount);
    // console.log('fee account native asset balance after: ', nativeBalanceAfterFeeAccount.data.free);
    //
    // // Alice balance should be deducted by 10 + tx fee, so the before - after should be greater than 10 tokens
    // if (nativeBalanceBeforeAlice.data.free - nativeBalanceAfterAlice.data.free <= depositAmount) {
    //     failedTestcases.push("testcase 1 failed: Alice balance not match")
    // }
    // // balance reserve account should add deposit amount - fee which is 9,500,000,000,000
    // if (nativeBalanceBeforeNativeTokenTransferReserveAccount.data.free - nativeBalanceAfterNativeTokenTransferReserveAccount.data.free === 9500000000000) {
    //     failedTestcases.push("testcase 1 failed: NativeTokenTransferReserveAccount balance not match")
    // }
    // // fee account should add 9,500,000,000,000
    // if (nativeBalanceAfterFeeAccount.data.free - nativeBalanceBeforeFeeAccount.data.free === 9500000000000) {
    //     failedTestcases.push("testcase 1 failed: FeeAccount balance not match")
    // }
}

main().catch(console.error).finally(() => process.exit());

