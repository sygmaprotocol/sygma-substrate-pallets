// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

require('dotenv').config();

async function setBalance(api, who, value, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set balance of ${who} to ${value}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
            .sudo(api.tx.balances.forceSetBalance(who, value))
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
        const unsub = await api.tx.sudo
            .sudo(api.tx.sygmaFeeHandlerRouter.setFeeHandler(domainID, asset, feeHandlerType))
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
        const unsub = await api.tx.sudo
            .sudo(api.tx.sygmaBasicFeeHandler.setFee(domainID, asset, amount))
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

async function setFeeRate(api, domainID, asset, feeRate, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set percentage fee rate on domainID ${domainID}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
            .sudo(api.tx.sygmaPercentageFeeHandler.setFeeRate(domainID, asset, feeRate))
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
        const unsub = await api.tx.sudo
            .sudo(api.tx.sygmaBridge.registerDomain(domainID, chainID))
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

function getUSDCAssetId(api) {
    return api.createType('XcmV3MultiassetAssetId', {
        Concrete: api.createType('XcmV3MultiLocation', {
            parents: 1,
            interior: api.createType('XcmV3Junctions', {
                X3: [
                    api.createType('XcmV3Junction', {
                        Parachain: api.createType('Compact<U32>', 2004)
                    }),
                    api.createType('XcmV3Junction', {
                        // 0x7379676d61 is general key of USDC defined in sygma substrate pallet runtime for testing
                        // see UsdcLocation definition in runtime.rs
                        GeneralKey: {
                            length: 5,
                            data: '0x7379676d61000000000000000000000000000000000000000000000000000000'
                        }
                    }),
                    api.createType('XcmV3Junction', {
                        // 0x75736463 is general key of USDC defined in sygma substrate pallet runtime for testing
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

function getERC20TSTAssetId(api) {
    return api.createType('XcmV3MultiassetAssetId', {
        Concrete: api.createType('XcmV3MultiLocation', {
            parents: 1,
            interior: api.createType('XcmV3Junctions', {
                X3: [
                    api.createType('XcmV3Junction', {
                        Parachain: api.createType('Compact<U32>', 2004)
                    }),
                    api.createType('XcmV3Junction', {
                        GeneralKey: {
                            length: 5,
                            data: '0x7379676d61000000000000000000000000000000000000000000000000000000'
                        } // sygma
                    }),
                    api.createType('XcmV3Junction', {
                        GeneralKey: {
                            length: 8,
                            data: '0x6572633230747374000000000000000000000000000000000000000000000000'
                        } // erc20tst
                    }),
                ]
            })
        })
    })
}

function getERC20TSTD20AssetId(api) {
    return api.createType('XcmV3MultiassetAssetId', {
        Concrete: api.createType('XcmV3MultiLocation', {
            parents: 1,
            interior: api.createType('XcmV3Junctions', {
                X3: [
                    api.createType('XcmV3Junction', {
                        Parachain: api.createType('Compact<U32>', 2004)
                    }),
                    api.createType('XcmV3Junction', {
                        GeneralKey: {
                            length: 5,
                            data: '0x7379676d61000000000000000000000000000000000000000000000000000000'
                        } // sygma
                    }),
                    api.createType('XcmV3Junction', {
                        GeneralKey: {
                            length: 11,
                            data: '0x6572633230747374643230000000000000000000000000000000000000000000'
                        } // erc20tstd20
                    }),
                ]
            })
        })
    })
}

function getNativeAssetId(api) {
    return api.createType('XcmV3MultiassetAssetId', {
        Concrete: api.createType('XcmV3MultiLocation', {
            parents: 0,
            interior: api.createType('XcmV3Junctions', 'Here')
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
    getNativeAssetId,
    getUSDCAssetId,
    getERC20TSTAssetId,
    getERC20TSTD20AssetId,
    registerDomain,
    mintAsset,
    setAssetMetadata,
    createAsset,
    queryBridgePauseStatus,
    setMpcAddress,
    setFee,
    setFeeRate,
    setFeeHandler,
    setBalance,
    executeProposal,
    queryAssetBalance,
    queryBalance,
    queryMPCAddress
}
