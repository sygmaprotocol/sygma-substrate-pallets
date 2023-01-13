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

async function setMpcAddress(api, mpcAddr, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set MPC address. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
            .sudo(api.tx.sygmaBridge.setMpcAddress(mpcAddr))
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

async function createAsset(api, id, admin, minBalance, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to create asset: (nonce: ${nonce}) ---`
        );

        const unsub = await api.tx.assets.create(id, admin, minBalance)
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

async function setAssetMetadata(api, id, name, symbol, decimals, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to register asset metadata: (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.assets.setMetadata(id, name, symbol, decimals)
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

async function mintAsset(api, id, recipient, amount, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to mint asset to ${recipient}: (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.assets.mint(id, recipient, amount)
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
    const sygmaPalletProvider = new WsProvider(process.env.PALLETWSENDPOINT || 'ws://127.0.0.1:9944');
    const api = await ApiPromise.create({
        provider: sygmaPalletProvider,
    });

    await cryptoWaitReady();
    const keyring = new Keyring({type: 'sr25519'});
    const sudo = keyring.addFromUri('//Alice');

    const mpcAddr = process.env.MPCADDR || '0x1c5541A79AcC662ab2D2647F3B141a3B7Cdb2Ae4';
    await setMpcAddress(api, mpcAddr, true, sudo);

    const basicFeeAmount = bn1e12.mul(new BN(1)); // 1 * 10 ** 12

    // create native test asset
    const nativeAssetID = 1000;
    const nativeAdmin = sudo.address;
    const nativeMinBalance = 100;
    const nativeName = "Native test asset";
    const nativeSymbol = "NATIVE";
    const nativeDecimal = 12;
    await createAsset(api, nativeAssetID, nativeAdmin, nativeMinBalance, true, sudo);
    await setAssetMetadata(api, nativeAssetID, nativeName, nativeSymbol, nativeDecimal, true, sudo);
    await mintAsset(api, nativeAssetID, nativeAdmin, 100000000000000, true, sudo); // mint 100 native to Alice

    const nativeAsset = 0; // TODO: I suppose 0 means `Concrete(MultiLocation::new(0, Here);` ?
                           // TODO: and how to link this nativeAsset with NativeLocation: MultiLocation that defined in the runtime?

    // set fee for native currency
    await setFee(api, nativeAsset, basicFeeAmount, true, sudo);

    // create USDC test asset (foreign asset)
    const usdcAssetID = 2000;
    const usdcAdmin = sudo.address;
    const usdcMinBalance = 100;
    const usdcName = "USDC test asset";
    const usdcSymbol = "USDC";
    const usdcDecimal = 12;
    await createAsset(api, usdcAssetID, usdcAdmin, usdcMinBalance, true, sudo);
    await setAssetMetadata(api, usdcAssetID, usdcName, usdcSymbol, usdcDecimal, true, sudo);
    await mintAsset(api, usdcAssetID, usdcAdmin, 100000000000000, true, sudo); // mint 100 USDC to Alice

    const usdcAsset = 1; // TODO: how to assign usdcAsset param here to represent the UsdcLocation: MultiLocation that defined in the runtime?
                        //  TODO: and how to link this created usdcAsset with UsdcLocation: MultiLocation that defined in the runtime?
    // set fee for USDC
    await setFee(api, usdcAsset, basicFeeAmount, true, sudo);

    // bridge should be unpaused by the end of the setup
    if (!await queryBridgePauseStatus(api)) console.log('🚀 Sygma substrate pallet setup is done! 🚀');

    // It is unnecessary to set up access segregator here since ALICE will be the sudo account and all methods with access control logic are already setup in this script.
    // so that on Relayer, E2E test cases are only about public extrinsic such as deposit, executionProposal, retry .etc
}

main().catch(console.error).finally(() => process.exit());
