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

const ahnAssetID = 2001;
const ahnMinBalance = 100;
const ahnName = "Asset Hub Native";
const ahnSymbol = "AHN";
const ahnDecimal = 12;

const tttAssetID = 2002;
const tttMinBalance = 100;
const tttName = "Test Token Tub";
const tttSymbol = "TTT";
const tttDecimal = 12;

// relay chain
const relayChainProvider = new WsProvider(process.env.RELAYCHAINENDPOINT || 'ws://127.0.0.1:9942');
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
function getAHNAssetId(api) {
    return api.createType('StagingXcmV3MultiassetAssetId', {
        Concrete: api.createType('StagingXcmV3MultiLocation', {
            parents: 1,
            interior: api.createType('StagingXcmV3Junctions', {
                X1:
                    api.createType('StagingXcmV3Junction', {
                        Parachain: api.createType('Compact<U32>', 1000)
                    })
            })
        })
    })
}

function getAHNMultiAsset(api, amount) {
    return api.createType('StagingXcmV3MultiAsset', {
        id: getAHNAssetId(api),
        fun: api.createType('StagingXcmV3MultiassetFungibility', {
            Fungible: api.createType('Compact<U128>', amount)
        })
    })
}

// return USDC assetID with parachain(full X3)
function getUSDCAssetId(api) {
    return api.createType('StagingXcmV3MultiassetAssetId', {
        Concrete: api.createType('StagingXcmV3MultiLocation', {
            parents: 1,
            interior: api.createType('StagingXcmV3Junctions', {
                X3: [
                    api.createType('StagingXcmV3Junction', {
                        Parachain: api.createType('Compact<U32>', 1000)
                    }),
                    api.createType('StagingXcmV3Junction', {
                        PalletInstance: 50
                    }),
                    api.createType('StagingXcmV3Junction', {
                        GeneralIndex: 2000
                    }),
                ]
            })
        })
    })
}

// return USDC assetID but without parachain(only X2)
function getUSDCAssetIdX2(api) {
    return api.createType('StagingXcmV3MultiassetAssetId', {
        Concrete: api.createType('StagingXcmV3MultiLocation', {
            parents: 0,
            interior: api.createType('StagingXcmV3Junctions', {
                X2: [
                    api.createType('StagingXcmV3Junction', {
                        PalletInstance: 50
                    }),
                    api.createType('StagingXcmV3Junction', {
                        GeneralIndex: 2000
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

function getUSDCMultiAssetX2(api, amount) {
    return api.createType('StagingXcmV3MultiAsset', {
        id: getUSDCAssetIdX2(api),
        fun: api.createType('StagingXcmV3MultiassetFungibility', {
            Fungible: api.createType('Compact<U128>', amount)
        })
    })
}


// TTT is used for foriegn token on Bridge hub test
function getTTTMultiAsset(api, amount) {
    return api.createType('StagingXcmV3MultiAsset', {
        id: getTTTAssetId(api),
        fun: api.createType('StagingXcmV3MultiassetFungibility', {
            Fungible: api.createType('Compact<U128>', amount)
        })
    })
}

function getTTTAssetId(api) {
    return api.createType('StagingXcmV3MultiassetAssetId', {
        Concrete: api.createType('StagingXcmV3MultiLocation', {
            parents: 1,
            interior: api.createType('StagingXcmV3Junctions', {
                X3: [
                    api.createType('StagingXcmV3Junction', {
                        Parachain: api.createType('Compact<U32>', 1013)
                    }),
                    api.createType('StagingXcmV3Junction', {
                        // 0x7379676d61 is general key of sygma
                        GeneralKey: {
                            length: 5,
                            data: '0x7379676d61000000000000000000000000000000000000000000000000000000'
                        }
                    }),
                    api.createType('StagingXcmV3Junction', {
                        // 0x545454 is general key of TTT
                        GeneralKey: {
                            length: 3,
                            data: '0x5454540000000000000000000000000000000000000000000000000000000000'
                        }
                    }),
                ]
            })
        })
    })
}

function getAssetDepositDest(api) {
    return api.createType('StagingXcmV3MultiLocation', {
        parents: 0,
        interior: api.createType('StagingXcmV3Junctions', {
            X4: [
                api.createType('StagingXcmV3Junction', {
                    GeneralKey: {
                        length: 5,
                        data: '0x7379676d61000000000000000000000000000000000000000000000000000000'
                    }
                }),
                api.createType('StagingXcmV3Junction', {
                    GeneralKey: {
                        length: 12,
                        data: '0x7379676d612d6272696467650000000000000000000000000000000000000000'
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

// The dest of teleport tokens from Asset hub
function getAssetHubTeleportDest(api) {
    return api.createType('StagingXcmVersionedMultiLocation', {
         V3: api.createType('StagingXcmV3MultiLocation', {
             parents: 1,
             interior: api.createType('StagingXcmV3Junctions', {
                 X1: api.createType('StagingXcmV3Junction', {
                     Parachain: api.createType('Compact<U32>', 1013)
                 }),
             })
         })
    })
}

// The Beneficiary of teleport tokens from Asset hub to Bridge hub
function getAssetHubTeleportBeneficiary(api, beneficiary) {
    return api.createType('StagingXcmVersionedMultiLocation', {
        V3: api.createType('StagingXcmV3MultiLocation', {
            parents: 0,
            interior: api.createType('StagingXcmV3Junctions', {
                X1: api.createType('StagingXcmV3Junction', {
                    AccountId32: {
                        network: api.createType('Option<StagingXcmV3JunctionNetworkId>', 'rococo'),
                        id: beneficiary,
                    }
                }),
            })
        })
    })
}

// The Beneficiary of teleport tokens from Asset hub to Sygma via Bridge hub
function getAssetHubTeleportBeneficiaryToSygma(api) {
    return api.createType('StagingXcmVersionedMultiLocation', {
        V3: api.createType('StagingXcmV3MultiLocation', getAssetDepositDest(api))
    })
}

// The asset of teleport tokens from Asset hub to Bridge hub
function getAssetHubTeleportAsset(api, asset) {
    return api.createType('StagingXcmVersionedMultiAssets', {
        V3: api.createType('StagingXcmV3MultiassetMultiAssets', [
            asset
        ])
    })
}

// The weight limit of teleport tokens from Asset hub
function getAssetHubTeleportWeightLimit(api) {
    return api.createType('StagingXcmV3WeightLimit', "Unlimited")
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

async function deposit(api, asset, dest, finalization, sudo) {
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

function getHRMPChannelDest(api) {
    return api.createType('StagingXcmVersionedMultiLocation', {
        V2: api.createType('StagingXcmV2MultiLocation', {
            parents: 1,
            interior: api.createType('StagingXcmV3Junctions', 'Here')
        })
    })
}

function getHRMPChannelMessage(api, encodedData, fromParaID) {
    return api.createType('StagingXcmVersionedXcm', {
        V2: api.createType('StagingXcmV2Xcm', [
            api.createType('StagingXcmV2Instruction', {
                WithdrawAsset: [
                    api.createType('StagingXcmV2MultiAsset', {
                        id: api.createType('StagingXcmV2MultiassetAssetId', {
                            Concrete: api.createType('StagingXcmV2MultiLocation', {
                                parents: 0,
                                interior: api.createType('StagingXcmV2MultilocationJunctions', 'Here')
                            }),
                        }),
                        fun: api.createType('StagingXcmV2MultiassetFungibility', {
                            Fungible: api.createType('Compact<U128>', 1000000000000)
                        })
                    })
                ]
            }),
            api.createType('StagingXcmV2Instruction', {
                BuyExecution: {
                    fees: api.createType('StagingXcmV2MultiAsset', {
                        id: api.createType('StagingXcmV2MultiassetAssetId', {
                            Concrete: api.createType('StagingXcmV2MultiLocation', {
                                parents: 0,
                                interior: api.createType('StagingXcmV2MultilocationJunctions', 'Here')
                            }),
                        }),
                        fun: api.createType('StagingXcmV2MultiassetFungibility', {
                            Fungible: api.createType('Compact<U128>', 1000000000000)
                        })
                    }),
                    weightLimit: api.createType('StagingXcmV2WeightLimit', "Unlimited")
                },
            }),
            api.createType('StagingXcmV2Instruction', {
                Transact: {
                    originType: api.createType('StagingXcmV2OriginKind', "Native"),
                    requireWeightAtMost: api.createType('Compact<U64>', 4000000000),
                    call: api.createType('StagingXcmDoubleEncoded', {
                        encoded: api.createType('Bytes', encodedData),
                    }),
                },
            }),
            api.createType('StagingXcmV2Instruction', {
                RefundSurplus: {},
            }),
            api.createType('StagingXcmV2Instruction', {
                DepositAsset: {
                    assets: api.createType('StagingXcmV2MultiassetMultiAssetFilter', {
                        Wild: api.createType('StagingXcmV2MultiassetWildMultiAsset', "All")
                    }),
                    maxAssets: api.createType('Compact<U32>', 1),
                    beneficiary: api.createType('StagingXcmV2MultiLocation', {
                        parents: 0,
                        interior: api.createType('StagingXcmV2MultilocationJunctions', {
                            X1: api.createType('StagingXcmV2Junction', {
                                Parachain: api.createType('Compact<U32>', fromParaID)
                            })
                        })
                    })
                }
            })
        ])
    })
}

async function hrmpChannelRequest(api, dest, message, fromParachainID, toParachainID, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting HRMP channel open request from ${fromParachainID} to ${toParachainID}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
            .sudo(api.tx.polkadotXcm.send(dest, message))
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

async function teleportTokenViaXCM(api, {dest, beneficiary, assets, feeAssetItem, weightLimit, fromParachainID, toParachainID}, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting Teleport Token call from ${fromParachainID} to ${toParachainID}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.polkadotXcm.limitedTeleportAssets(dest, beneficiary, assets, feeAssetItem, weightLimit)
            .signAndSend(sudo, {nonce: nonce, era: 0}, ({ events = [], status, isError }) => {
                console.log(`Current status is ${status}`);
                if (status.isInBlock) {
                    console.log(
                        `Transaction included at blockHash ${status.asInBlock}`
                    );
                    if (finalization) {
                        console.log('Waiting for finalization...');
                    } else {
                        unsub();
                        resolve();
                    }

                    console.log('Events:');
                    events.forEach(({ event: { data, method, section }, phase }) => {
                        console.log('\t', phase.toString(), `: ${section}.${method}`, data.toString());
                    });
                } else if (status.isFinalized) {
                    console.log(
                        `Transaction finalized at blockHash ${status.asFinalized}`
                    );
                    unsub();
                    resolve();
                } else if (isError) {
                    console.log(`Transaction Error`);
                    reject(`Transaction Error`);
                }
            });
    });
}

// Subscribe to system events via storage
async function subEvents (api, eventsList) {
    api.query.system.events((events) => {
        console.log(`\nReceived ${events.length} events:`);

        // Loop through the Vec<EventRecord>
        events.forEach((record) => {
            // Extract the phase, event and the event types
            const { event, phase } = record;
            const types = event.typeDef;

            // Show what we are busy with
            // console.log(`\t${event.section}:${event.method}:: (phase=${phase.toString()})`);
            // console.log(`\t\t${event.meta}`);
            console.log(`${event.section}:${event.method}`);
            if (event.section.startsWith("sygmaBridge") || event.section.startsWith("sygmaBridgeForwarder")) {
                eventsList.push(event.section);
            }

            // Loop through each of the parameters, displaying the type and data
            // event.data.forEach((data, index) => {
            //     console.log(`\t\t\t${types[index].type}: ${data.toString()}`);
            // });
        });
    });
}

async function queryBridgePauseStatus(api, domainID) {
    let result = await api.query.sygmaBridge.isPaused(domainID);
    return result.toJSON()
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

function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

module.exports = {
    relayChainProvider,
    assetHubProvider,
    bridgeHubProvider,
    getNativeAssetId,
    getNativeMultiAsset,
    getUSDCAssetId,
    getUSDCMultiAssetX2,
    getUSDCMultiAsset,
    getUSDCAssetIdX2,
    getAHNAssetId,
    getAHNMultiAsset,
    getTTTMultiAsset,
    getTTTAssetId,
    getAssetDepositDest,
    deposit,
    registerDomain,
    setMpcAddress,
    setFee,
    setFeeRate,
    setFeeHandler,
    executeProposal,
    transferBalance,
    createAsset,
    setAssetMetadata,
    mintAsset,
    subEvents,
    queryBridgePauseStatus,
    queryAssetBalance,
    queryBalance,
    queryMPCAddress,
    hrmpChannelRequest,
    getHRMPChannelMessage,
    getHRMPChannelDest,
    teleportTokenViaXCM,
    getAssetHubTeleportDest,
    getAssetHubTeleportBeneficiary,
    getAssetHubTeleportBeneficiaryToSygma,
    getAssetHubTeleportAsset,
    getAssetHubTeleportWeightLimit,
    delay,
    FeeReserveAccount,
    NativeTokenTransferReserveAccount,
    OtherTokenTransferReserveAccount,
    usdcAssetID,
    usdcMinBalance,
    usdcName,
    usdcSymbol,
    usdcDecimal,
    ahnAssetID,
    ahnMinBalance,
    ahnName,
    ahnSymbol,
    ahnDecimal,
    tttAssetID,
    tttMinBalance,
    tttName,
    tttSymbol,
    tttDecimal,
}
