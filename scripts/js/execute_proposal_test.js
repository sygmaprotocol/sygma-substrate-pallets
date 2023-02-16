require('dotenv').config();

const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
const {cryptoWaitReady} = require('@polkadot/util-crypto');

const proposal = {
    origin_domain_id: 1,
    deposit_nonce: 0,
    resource_id: new Uint8Array([0, 177, 78, 7, 29, 218, 208, 177, 43, 229, 172, 166, 223, 252, 95, 37, 132, 234, 21, 141, 155, 12, 231, 62, 20, 55, 17, 94, 151, 163, 42, 62]),
    data: new Uint8Array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 24, 78, 114, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
}
// signer should be the same MPC address as setup.js
// 0xe51258e9a4ea837e53f4898c64c417c6c4800aa8
const signature = [113, 25, 147, 244, 240, 122, 231, 247, 95, 241, 104, 112, 28, 225, 244, 217, 171, 141, 254, 135, 243, 85, 50, 72, 198, 247, 246, 149, 33, 219, 61, 223, 119, 97, 41, 158, 15, 77, 58, 97, 196, 44, 109, 202, 115, 8, 21, 15, 85, 139, 70, 128, 227, 250, 101, 143, 229, 180, 128, 31, 209, 47, 109, 12, 1];

async function executeProposal(api, proposalList, signature, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to execute dummy proposal for testing : (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sygmaBridge.executeProposal(proposalList, signature)
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

async function queryAssetBalance(api, assetID, account) {
    let result = await api.query.assets.account(assetID, account);
    return result.toHuman()
}

async function main() {
    const sygmaPalletProvider = new WsProvider(process.env.PALLETWSENDPOINT || 'ws://127.0.0.1:9944');
    const api = await ApiPromise.create({
        provider: sygmaPalletProvider,
    });

    await cryptoWaitReady();
    const keyring = new Keyring({type: 'sr25519'});
    const sudo = keyring.addFromUri('//Alice');

    const balanceBefore = await queryAssetBalance(api,2000, sudo.address)
    console.log('asset balance before: ', balanceBefore.balance);

    await executeProposal(api, [proposal], signature.buffer, true, sudo);

    const balanceAfter = await queryAssetBalance(api,2000, sudo.address)
    console.log('asset balance after: ', balanceAfter.balance);
}

main().catch(console.error).finally(() => process.exit());
