// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

use core::marker::PhantomData;

use codec::Encode;
use xcm::latest::{Junction, MultiAsset, MultiLocation, XcmContext};
use xcm::prelude::*;
use xcm_executor::{Assets, traits::TransactAsset};
use hex_literal::hex;
use sygma_traits::{AssetTypeIdentifier, TransactorForwarder};

pub struct XCMAssetTransactor<CurrencyTransactor, FungiblesTransactor, AssetTypeChecker, Forwarder>(PhantomData<(CurrencyTransactor, FungiblesTransactor, AssetTypeChecker, Forwarder)>);
impl<CurrencyTransactor: TransactAsset, FungiblesTransactor: TransactAsset, AssetTypeChecker: AssetTypeIdentifier, Forwarder: TransactorForwarder> TransactAsset for XCMAssetTransactor<CurrencyTransactor, FungiblesTransactor, AssetTypeChecker, Forwarder> {
    // deposit_asset implements the TransactAsset deposit_asset method and contains the logic to classify
    // the asset recipient location:
    // 1. recipient is on the local parachain
    // 2. recipient is on non-substrate chain(evm, cosmos, etc.)
    // 3. recipient is on the remote parachain
    fn deposit_asset(what: &MultiAsset, who: &MultiLocation, context: &XcmContext) -> XcmResult {
        match (who.parents, who.first_interior()) {
            // 1. recipient is on the local parachain
            (0, Some(Parachain(_))) => {
                // check if the asset is native, and call the corresponding deposit_asset()
                if AssetTypeChecker::is_native_asset(what) {
                    CurrencyTransactor::deposit_asset(what, who, context)?;
                } else {
                    FungiblesTransactor::deposit_asset(what, who, context)?
                }
            }
            // recipient is on the remote chain
            (1, Some(Parachain(_))) => {
                // 2. recipient is on non-substrate chain(evm, cosmos, etc.), will forward to sygma bridge pallet
                match who.interior {
                    // sygma: 7379676d61000000000000000000000000000000000000000000000000000000
                    // sygma-bridge: 7379676d612d6272696467650000000000000000000000000000000000000000
                    X5(Parachain(1000), GeneralKey { length: 5, data: hex!["7379676d61000000000000000000000000000000000000000000000000000000"]},  GeneralKey { length: 12, data: hex!["7379676d612d6272696467650000000000000000000000000000000000000000"]}, GeneralIndex(..), GeneralKey { .. }) => {
                        // check if the asset is native or foreign, and deposit the asset to a tmp account first
                        let tmp_account = sp_io::hashing::blake2_256(&MultiLocation::new(0, X1(GeneralKey { length: 8, data: [2u8; 32] })).encode());
                        if AssetTypeChecker::is_native_asset(what) {
                            CurrencyTransactor::deposit_asset(&what.clone(), &Junction::AccountId32 { network: None, id: tmp_account }.into(), context)?;
                        } else {
                            FungiblesTransactor::deposit_asset(&what.clone(), &Junction::AccountId32 { network: None, id: tmp_account }.into(), context)?
                        }

                        Forwarder::other_world_transactor_forwarder(tmp_account, what.clone(), *who).map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
                    }
                    _ => {
                        // 3. recipient is on remote parachain
                        // xcm message must have a sender(origin), so a tmp account derived from pallet would be necessary here
                        let tmp_account = sp_io::hashing::blake2_256(&MultiLocation::new(0, X1(GeneralKey { length: 8, data: [2u8; 32] })).encode());

                        // check if the asset is native or foreign, and call the corresponding deposit_asset(), recipient will be the derived tmp account
                        // xcm message execution
                        if AssetTypeChecker::is_native_asset(what) {
                            CurrencyTransactor::deposit_asset(&what.clone(), &Junction::AccountId32 { network: None, id: tmp_account }.into(), context)?;
                        } else {
                            FungiblesTransactor::deposit_asset(&what.clone(), &Junction::AccountId32 { network: None, id: tmp_account }.into(), context)?
                        }

                        Forwarder::xcm_transactor_forwarder(tmp_account, what.clone(), *who).map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
                    }
                }
            }
            // Other destination multiLocation not supported, return Err
            _ => {
                return Err(XcmError::DestinationUnsupported);
            }
        }

        Ok(())
    }

    fn withdraw_asset(what: &MultiAsset, who: &MultiLocation, maybe_context: Option<&XcmContext>) -> Result<Assets, XcmError> {
        let assets = if AssetTypeChecker::is_native_asset(what) {
            CurrencyTransactor::withdraw_asset(what, who, maybe_context)?
        } else {
            FungiblesTransactor::withdraw_asset(what, who, maybe_context)?
        };

        Ok(assets)
    }
}
