require('dotenv').config();

const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
const {cryptoWaitReady} = require('@polkadot/util-crypto');
const {
    setBalance,
    setFeeHandler,
    setMpcAddress,
    registerDomain,
    setFee,
    getNativeAssetId,
    createAsset,
    setAssetMetadata,
    mintAsset,
    getUSDCAssetId,
    queryBridgePauseStatus
} = require("./util");

const BN = require('bn.js');
const bn1e12 = new BN(10).pow(new BN(12));

const feeHandlerType = {
    BasicFeeHandler: "BasicFeeHandler",
    DynamicFeeHandler: "DynamicFeeHandler"
}

const supportedDestDomains = [
    {
        domainID: 1,
        chainID: 1
    },
    {
        domainID: 2,
        chainID: 2
    }
]

// those account are configured in the substrate-node runtime, and are only applicable for sygma pallet standalone node,
// other parachain might have different runtime config so those account address need to be adjusted accordingly
const FeeReserveAccountAddress = "5ELLU7ibt5ZrNEYRwohtaRBDBa3TzcWwwPELBPSWWd2mbgv3";
const TransferReserveAccount = "5EMepC39b7E2zfM9g6CkPp8KCAxGTh7D4w4T2tFjmjpd4tPw";

async function main() {
    const sygmaPalletProvider = new WsProvider(process.env.PALLETWSENDPOINT || 'ws://127.0.0.1:9944');
    const api = await ApiPromise.create({
        provider: sygmaPalletProvider,
    });

    await cryptoWaitReady();
    const keyring = new Keyring({type: 'sr25519'});
    const sudo = keyring.addFromUri('//Alice');
    const basicFeeAmount = bn1e12.mul(new BN(1)); // 1 * 10 ** 12
    const mpcAddr = process.env.MPCADDR || '0x1c5541A79AcC662ab2D2647F3B141a3B7Cdb2Ae4';

    // set up MPC address
    await setMpcAddress(api, mpcAddr, true, sudo);

    // register dest domains
    for (const domain of supportedDestDomains) {
        await registerDomain(api, domain.domainID, domain.chainID, true, sudo);
    }

    // set fee for native asset for domains
    for (const domain of supportedDestDomains) {
        await setFeeHandler(api, domain.domainID, getNativeAssetId(api), feeHandlerType.BasicFeeHandler, true, sudo)
        await setFee(api, domain.domainID, getNativeAssetId(api), basicFeeAmount, true, sudo);
    }

    // create USDC test asset (foreign asset)
    // UsdcAssetId: AssetId defined in runtime.rs
    const usdcAssetID = 2000;
    const usdcAdmin = sudo.address;
    const usdcMinBalance = 100;
    const usdcName = "USDC test asset";
    const usdcSymbol = "USDC";
    const usdcDecimal = 12;
    await createAsset(api, usdcAssetID, usdcAdmin, usdcMinBalance, true, sudo);
    await setAssetMetadata(api, usdcAssetID, usdcName, usdcSymbol, usdcDecimal, true, sudo);
    await mintAsset(api, usdcAssetID, usdcAdmin, 100000000000000, true, sudo); // mint 100 USDC to Alice

    // set fee for USDC for domains
    for (const domain of supportedDestDomains) {
        await setFeeHandler(api, domain.domainID, getUSDCAssetId(api), feeHandlerType.BasicFeeHandler, true, sudo)
        await setFee(api, domain.domainID, getUSDCAssetId(api), basicFeeAmount, true, sudo);
    }

    // transfer some native asset to FeeReserveAccount and TransferReserveAccount as Existential Deposit(aka ED)
    await setBalance(api, FeeReserveAccountAddress, bn1e12.mul(new BN(10000)), true, sudo); // set balance to 10000 native asset
    await setBalance(api, TransferReserveAccount, bn1e12.mul(new BN(10000)), true, sudo); // set balance to 10000 native asset

    // bridge should be unpaused by the end of the setup
    for (const domain of supportedDestDomains) {
        if (!await queryBridgePauseStatus(api, domain.domainID)) console.log(`DestDomainID: ${domain.domainID} is ready✅`);
    }

    console.log('🚀 Sygma substrate pallet setup is done! 🚀');

    // It is unnecessary to set up access segregator here since ALICE will be the sudo account and all methods with access control logic are already setup in this script.
    // so that on Relayer, E2E test only cases about public extrinsic such as deposit, executionProposal, retry .etc
}

main().catch(console.error).finally(() => process.exit());
