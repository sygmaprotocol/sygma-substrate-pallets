// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

require('dotenv').config();

const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
const {cryptoWaitReady} = require('@polkadot/util-crypto');
const {
    executeProposal,
    queryAssetBalance,
    queryBalance,
    queryMPCAddress
} = require("./util");

// these are the dummy proposals that used to verify if proposal execution works on pallet
// bridge amount from relayer  is 100000000000000
const proposal_usdc = {
    origin_domain_id: 1,
    deposit_nonce: 2,
    resource_id: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0],
    data: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90, 243, 16, 122, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 0, 1, 1, 0, 212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88, 133, 76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125]
}
const proposal_native = {
    origin_domain_id: 1,
    deposit_nonce: 3,
    resource_id: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    data: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90, 243, 16, 122, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 0, 1, 1, 0, 212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88, 133, 76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125]
}

// signer is the mpc address 0x1c5541A79AcC662ab2D2647F3B141a3B7Cdb2Ae4
const signature_usdc = [180, 250, 104, 54, 47, 69, 174, 209, 145, 226, 25, 32, 184, 96, 142, 125, 103, 53, 60, 180, 107, 207, 80, 188, 9, 138, 218, 97, 50, 132, 193, 10, 6, 15, 186, 139, 6, 21, 63, 39, 157, 144, 81, 12, 81, 165, 215, 213, 200, 105, 198, 105, 115, 193, 42, 183, 145, 118, 52, 47, 45, 198, 165, 5, 28];
const signature_native = [57, 218, 225, 125, 128, 217, 23, 82, 49, 217, 8, 197, 110, 174, 42, 157, 129, 43, 22, 63, 215, 213, 100, 179, 17, 170, 23, 95, 72, 80, 78, 181, 108, 176, 60, 138, 137, 29, 157, 138, 244, 0, 5, 180, 128, 243, 48, 99, 175, 53, 140, 245, 162, 111, 36, 65, 89, 208, 41, 69, 209, 149, 247, 149, 28];

const mpcAddress = "0x1c5541a79acc662ab2d2647f3b141a3b7cdb2ae4";
const aliceAddress = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

// this script take one param as the USDC assetID, default is 2000 if not provided
async function main() {
    const [assetID] = process.argv.slice(2);
    if (!assetID) {
        console.log("assetID is not provided, using default assetID 2000")
    }
    const sygmaPalletProvider = new WsProvider(process.env.PALLETWSENDPOINT || 'ws://127.0.0.1:9944');
    const api = await ApiPromise.create({
        provider: sygmaPalletProvider,
    });

    await cryptoWaitReady();
    const keyring = new Keyring({type: 'sr25519'});
    const sudo = keyring.addFromUri('//Alice');

    // make sure mpc address matches
    const registeredMpcAddr = await queryMPCAddress(api);
    if (registeredMpcAddr !== mpcAddress) {
        throw Error("mpc address not match")
    }

    console.log(`sudo address ${sudo.address}`)

    // USDC
    const usdcBalanceBefore = await queryAssetBalance(api, assetID || 2000, aliceAddress);
    console.log('usdc asset balance before: ', usdcBalanceBefore.balance);
    await executeProposal(api, [proposal_usdc], signature_usdc, false, sudo);
    const usdcbalanceAfter = await queryAssetBalance(api, assetID || 2000, aliceAddress);
    console.log('usdc asset balance after: ', usdcbalanceAfter.balance);

    if (usdcbalanceAfter.balance === usdcBalanceBefore.balance) {
        throw Error('proposal execution test failed(proposal of USDC)')
    }

    // Native asset
    const nativeBalanceBefore = await queryBalance(api, aliceAddress);
    console.log('native asset balance before: ', nativeBalanceBefore.data.free);
    await executeProposal(api, [proposal_native], signature_native, false, sudo);
    const nativeBalanceAfter = await queryBalance(api, aliceAddress);
    console.log('native asset balance after: ', nativeBalanceAfter.data.free);

    // this fee is proposal execution fee for sygma pallet standalone node with hardcoded dummy proposal
    // its value might be different when running on other parachain, so need to be adjusted accordingly
    const fee = BigInt(293974317);
    const amount = BigInt(100000000);
    const before_num = BigInt(nativeBalanceBefore.data.free.replaceAll(',', ''));
    const after_num = BigInt(nativeBalanceAfter.data.free.replaceAll(',', ''));

    if (after_num !== before_num + amount - fee) {
        throw Error('proposal execution test failed(proposal of Native asset)')
    }

    console.log('proposal execution test pass✅');
}

main().catch(console.error).finally(() => process.exit());
