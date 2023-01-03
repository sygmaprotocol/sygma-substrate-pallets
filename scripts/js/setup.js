require('dotenv').config();

const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
const {cryptoWaitReady} = require('@polkadot/util-crypto');
const BN = require('bn.js');

const bn1e12 = new BN(10).pow(new BN(12));

async function setFee(api, asset, amount, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set basic fee. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
            .sudo(api.tx.sygmaBasicFeeHandler.setFee(asset, amount))
            .signAndSend(sudo, {nonce: nonce, era: 0}, (result) => {
                console.log(`Current status is ${result.status}`);
                if (result.status.isInBlock) {
                    console.log(
                        `Transaction included at blockHash ${result.status.asInBlock}`
                    );
                    if (finalization) {
                        console.log('Waiting for finalization...');
                    } else {
                        unsub();
                        resolve();
                    }
                } else if (result.status.isFinalized) {
                    console.log(
                        `Transaction finalized at blockHash ${result.status.asFinalized}`
                    );
                    unsub();
                    resolve();
                } else if (result.isError) {
                    console.log(`Transaction Error`);
                    reject(`Transaction Error`);
                }
            });
    });
}

async function setMpcKey(api, mpcPubKey, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set MPC key. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
            .sudo(api.tx.sygmaBridge.setMpcKey(mpcPubKey))
            .signAndSend(sudo, {nonce: nonce, era: 0}, (result) => {
                console.log(`Current status is ${result.status}`);
                if (result.status.isInBlock) {
                    console.log(
                        `Transaction included at blockHash ${result.status.asInBlock}`
                    );
                    if (finalization) {
                        console.log('Waiting for finalization...');
                    } else {
                        unsub();
                        resolve();
                    }
                } else if (result.status.isFinalized) {
                    console.log(
                        `Transaction finalized at blockHash ${result.status.asFinalized}`
                    );
                    unsub();
                    resolve();
                } else if (result.isError) {
                    console.log(`Transaction Error`);
                    reject(`Transaction Error`);
                }
            });
    });
}

async function queryBridgePauseStatus(api) {
    let result = await api.query.sygmaBridge.isPaused();
    return result.toJSON()
}

async function main() {
    const sygmaPalletProvider = new WsProvider(process.env.PALLETWSENDPOINT || 'ws://127.0.0.1:9944');
    const api = await ApiPromise.create({
        provider: sygmaPalletProvider,
    });

    await cryptoWaitReady();
    const keyring = new Keyring({type: 'sr25519'});
    const sudo = keyring.addFromUri('//Alice');
    const asset = 0 // Concrete(MultiLocation::new(0, Here));
    const basicFeeAmount = bn1e12.mul(new BN(200)); // 200 * 10 ** 12
    const mpcPubKey = process.env.MPCPUBKEY || '222222222222222222222222222222222'; // TODO: load from EVN or replace the hardcoded mpc key with the actual one from relayer

    await setMpcKey(api, mpcPubKey, true, sudo);
    await setFee(api, asset, basicFeeAmount, true, sudo);

    // bridge should be unpaused by the end of the setup
    if (!await queryBridgePauseStatus(api)) console.log('🚀 Sygma substrate pallet setup is done! 🚀');

    // It is unnecessary to set up access separators here since ALICE will be the sudo account and all methods with access control logic are already setup in this script.
    // so that on Relayer, E2E test cases are only about public extrinsic such as deposit, executionProposal, retry .etc
}

main().catch(console.error).finally(() => process.exit());