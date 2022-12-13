const {ApiPromise, WsProvider} = require('@polkadot/api');
const {cryptoWaitReady} = require('@polkadot/util-crypto');
const BN = require('bn.js');

const mpcPubKey = '000000000000000000000000000000000'; // TODO: replace mpc key with the actual one from relayer
const bn1e12 = new BN(10).pow(new BN(12));

async function configSygmaPallet(api, fee, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to config sygma substrate pallet. (nonce: ${nonce}) ---`
        );

        // Setup steps
        const setMpcKey = api.tx.sudo.sudo(api.tx.sygmaBridge.setMpcKey(mpcPubKey));
        const setFee = api.tx.sudo.sudo(api.tx.chainBridge.updateFee(0, fee));

        const unsub = await api.tx.utility.batch([setMpcKey, setFee])
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

async function main() {
    const sygmaPalletProvider = new WsProvider('ws://127.0.0.1:9944');
    const api = await ApiPromise.create({
        provider: sygmaPalletProvider,
    });

    await cryptoWaitReady();
    const keyring = new Keyring({type: 'sr25519'});
    const sudo = keyring.addFromUri('//Alice');

    await configSygmaPallet(api, bn1e12.mul(new BN(300)), true, sudo);
}

main().catch(console.error).finally(() => process.exit());