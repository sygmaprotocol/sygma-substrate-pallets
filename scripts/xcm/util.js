// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

const {WsProvider} = require("@polkadot/api");
require('dotenv').config();

// those account are configured in the substrate-node runtime, and are only applicable for sygma pallet standalone node,
// other parachain might have different runtime config so those account address need to be adjusted accordingly
const FeeReserveAccount = "5ELLU7ibt5ZrNEYRwohtaRBDBa3TzcWwwPELBPSWWd2mbgv3";
const NativeTokenTransferReserveAccount = "5EYCAe5jLbHcAAMKvLFSXgCTbPrLgBJusvPwfKcaKzuf5X5e";
const OtherTokenTransferReserveAccount = "5EYCAe5jLbHcAAMKvLFiGhk3htXY8jQncbLTDGJQnpnPMAVp";

// UsdcAssetId: AssetId defined in runtime.rs
const usdcAssetID = 2000;
const usdcMinBalance = 100;
const usdcName = "USDC test asset";
const usdcSymbol = "USDC";
const usdcDecimal = 12;

// asset hub parachain
const assetHubProvider = new WsProvider(process.env.ASSETHUBENDPOINT || 'ws://127.0.0.1:9910');
// bridge hub parachain
const bridgeHubProvider = new WsProvider(process.env.BRIDGEHUBENDPOINT || 'ws://127.0.0.1:8943');

async function transferBalance(api, who, value, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to transfer balance of ${who} to ${value}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.balances.transferKeepAlive(who, value)
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

async function setFeeHandler(api, domainID, asset, feeHandlerType, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set fee handler on domainID ${domainID}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sygmaFeeHandlerRouter.setFeeHandler(domainID, asset, feeHandlerType)
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

async function setFee(api, domainID, asset, amount, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set basic fee on domainID ${domainID}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sygmaBasicFeeHandler.setFee(domainID, asset, amount)
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

async function setFeeRate(api, domainID, asset, feeRate, feeRateLowerBound, feeRateUpperBound, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set percentage fee rate on domainID ${domainID}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sygmaPercentageFeeHandler.setFeeRate(domainID, asset, feeRate, feeRateLowerBound, feeRateUpperBound)
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
        const unsub = await api.tx.sygmaBridge.setMpcAddress(mpcAddr)
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

async function queryBridgePauseStatus(api, domainID) {
    let result = await api.query.sygmaBridge.isPaused(domainID);
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

async function registerDomain(api, domainID, chainID, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to register domainID ${domainID} with chainID ${chainID}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sygmaBridge.registerDomain(domainID, chainID)
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

function getNativeAssetId(api) {
    return api.createType('StagingXcmV3MultiassetAssetId', {
        Concrete: api.createType('StagingXcmV3MultiLocation', {
            parents: 0,
            interior: api.createType('StagingXcmV3Junctions', 'Here')
        })
    })
}

function getNativeMultiAsset(api, amount) {
    return api.createType('StagingXcmV3MultiAsset', {
        id: getNativeAssetId(api),
        fun: api.createType('StagingXcmV3MultiassetFungibility', {
            Fungible: api.createType('Compact<U128>', amount)
        })
    })
}

function getAssetDepositDest(api) {
    return api.createType('StagingXcmV3MultiLocation', {
        parents: 0,
        interior: api.createType('StagingXcmV3Junctions', {
            X3: [
                api.createType('StagingXcmV3Junction', {
                    GeneralKey: {
                        length: 5,
                        data: '0x7379676d61000000000000000000000000000000000000000000000000000000'
                    }
                }),
                api.createType('StagingXcmV3Junction', {
                    GeneralIndex: '1'
                }),
                api.createType('StagingXcmV3Junction', {
                    GeneralKey: {
                        length: 20,
                        data: '0x1abd6948e422a1b6ced1ba28ba72ca562333df01000000000000000000000000'
                    }
                }),
            ]
        })
    })
}

function getUSDCAssetId(api) {
    return api.createType('StagingXcmV3MultiassetAssetId', {
        Concrete: api.createType('StagingXcmV3MultiLocation', {
            parents: 1,
            interior: api.createType('StagingXcmV3Junctions', {
                X3: [
                    api.createType('StagingXcmV3Junction', {
                        Parachain: api.createType('Compact<U32>', 2005)
                    }),
                    api.createType('StagingXcmV3Junction', {
                        // 0x7379676d61 is general key of "sygma" defined in sygma substrate pallet runtime for testing
                        // see UsdcLocation definition in runtime.rs
                        GeneralKey: {
                            length: 5,
                            data: '0x7379676d61000000000000000000000000000000000000000000000000000000'
                        }
                    }),
                    api.createType('StagingXcmV3Junction', {
                        // 0x75736463 is general key of "usdc" defined in sygma substrate pallet runtime for testing
                        // see UsdcLocation definition in runtime.rs
                        GeneralKey: {
                            length: 4,
                            data: '0x7573646300000000000000000000000000000000000000000000000000000000'
                        }
                    }),
                ]
            })
        })
    })
}

function getUSDCMultiAsset(api, amount) {
    return api.createType('StagingXcmV3MultiAsset', {
        id: getUSDCAssetId(api),
        fun: api.createType('StagingXcmV3MultiassetFungibility', {
            Fungible: api.createType('Compact<U128>', amount)
        })
    })
}

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

async function depositLocal(api, asset, dest, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to deposit. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sygmaBridge.deposit(asset, dest)
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

async function queryBalance(api, account) {
    let result = await api.query.system.account(account);
    return result.toHuman()
}

async function queryMPCAddress(api) {
    let result = await api.query.sygmaBridge.mpcAddr();
    return result.toJSON()
}

module.exports = {
    assetHubProvider,
    bridgeHubProvider,
    getNativeAssetId,
    getNativeMultiAsset,
    getAssetDepositDest,
    getUSDCAssetId,
    getUSDCMultiAsset,
    depositLocal,
    registerDomain,
    mintAsset,
    setAssetMetadata,
    createAsset,
    queryBridgePauseStatus,
    setMpcAddress,
    setFee,
    setFeeRate,
    setFeeHandler,
    transferBalance,
    executeProposal,
    queryAssetBalance,
    queryBalance,
    queryMPCAddress,
    FeeReserveAccount,
    NativeTokenTransferReserveAccount,
    OtherTokenTransferReserveAccount,
    usdcAssetID,
    usdcMinBalance,
    usdcName,
    usdcSymbol,
    usdcDecimal,
}
