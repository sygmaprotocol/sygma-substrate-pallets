require('dotenv').config();

const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
const {cryptoWaitReady} = require('@polkadot/util-crypto');

// this is the dummy proposal that used to verify if proposal execution works on pallet
const proposal = {
    origin_domain_id: 1,
    deposit_nonce: 2,
    resource_id: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0],
    data: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90, 243, 16, 122, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 0, 1, 1, 0, 212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88, 133, 76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125]
}
// signer is the mpc address 0x1c5541A79AcC662ab2D2647F3B141a3B7Cdb2Ae4
const signature = [180, 250, 104, 54, 47, 69, 174, 209, 145, 226, 25, 32, 184, 96, 142, 125, 103, 53, 60, 180, 107, 207, 80, 188, 9, 138, 218, 97, 50, 132, 193, 10, 6, 15, 186, 139, 6, 21, 63, 39, 157, 144, 81, 12, 81, 165, 215, 213, 200, 105, 198, 105, 115, 193, 42, 183, 145, 118, 52, 47, 45, 198, 165, 5, 28];
const mpcAddress = "0x1c5541a79acc662ab2d2647f3b141a3b7cdb2ae4";
const aliceAddress = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

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

async function queryMPCAddress(api) {
    let result = await api.query.sygmaBridge.mpcAddr();
    return result.toJSON()
}

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
        console.error("mpc address not match");
        return
    }

    console.log(`sudo address ${sudo.address}`)

    const balanceBefore = await queryAssetBalance(api, assetID || 2000, aliceAddress);
    console.log('asset balance before: ', balanceBefore.balance);
    await executeProposal(api, [proposal], signature, true, sudo);
    const balanceAfter = await queryAssetBalance(api, assetID || 2000, aliceAddress);
    console.log('asset balance after: ', balanceAfter.balance);

    if (balanceAfter.balance !== balanceBefore.balance) {
        console.log('proposal execution test is passing✅');
        return
    }
    console.error('proposal execution test failed❌')
}

main().catch(console.error).finally(() => process.exit());
