use core::marker::PhantomData;
use xcm::latest::{Junction, MultiAsset, MultiLocation, XcmContext};
use sygma_traits::{AssetTypeIdentifier, TransactorForwarder};
use xcm::prelude::*;
use xcm_executor::{Assets, traits::TransactAsset};

pub struct XCMAssetTransactor<CurrencyTransactor, FungiblesTransactor, AssetTypeChecker, Forwarder>(PhantomData<(CurrencyTransactor, FungiblesTransactor, AssetTypeChecker, Forwarder)>);

impl<CurrencyTransactor: TransactAsset, FungiblesTransactor: TransactAsset, AssetTypeChecker: AssetTypeIdentifier, Forwarder: TransactorForwarder> TransactAsset for XCMAssetTransactor<CurrencyTransactor, FungiblesTransactor, AssetTypeChecker, Forwarder> {
    // deposit_asset implements the TransactAsset deposit_asset method and contains the logic to classify
    // the asset recipient location:
    // 1. recipient is on the local parachain
    // 2. recipient is on the remote parachain
    // 3, recipient is on non-substrate chain(evm, cosmos, etc.)
    fn deposit_asset(what: &MultiAsset, who: &MultiLocation, context: &XcmContext) -> XcmResult {
        match (who.parents, who.interior) {
            // 1. recipient is the local parachain
            (0, None) => {
                // check if the asset is native or foreign, and call the corresponding deposit_asset()
                if AssetTypeChecker::is_native_asset(what) {
                    CurrencyTransactor::deposit_asset(what, who, context)?;
                } else {
                    FungiblesTransactor::deposit_asset(what, who, context)?
                }
            }
            // 2. recipient is remote parachain
            // trying to eliminate the forward logic here by adding the XCM handler pallet as one of the generic type for XCMAssetTransactor
            (1, Some(Parachain(_))) => {
                // xcm message must have a sender(origin), so a tmp account derived from pallet would be used
                let tmp_account = MultiLocation::new(0, X1(GeneralKey {length: 8, data: [2u8; 32]})).into_account();

                // check if the asset is native or foreign, and call the corresponding deposit_asset(), recipient will be the derived tmp account
                // xcm message execution
                if AssetTypeChecker::is_native_asset(what) {
                    CurrencyTransactor::deposit_asset(what, &Junction::AccountId32 { network: None, id: tmp_account }.into(), context)?;
                } else {
                    FungiblesTransactor::deposit_asset(what, &Junction::AccountId32 { network: None, id: tmp_account }.into(), context)?
                }

                // TODO: call the xcm handler pallet to construct the xcm message and execute it(to other remote parachain route)
                Forwarder::xcm_transactor_forwarder(tmp_account, what.into(), who.into())
            }
            // 3. recipient is on non-substrate chain(evm, cosmos, etc.), will forward to sygma bridge pallet
            // TODO: the junctions below is just an temporary example, will change it to proper sygma bridge standard, see the link below:
            // (https://www.notion.so/chainsafe/Sygma-as-an-Independent-pallet-c481f00ccff84ff49ce917c8b2feacda?pvs=4#6e51e6632e254b9b9a01444ef7297969)
            (0, &X3(Parachain(_), GeneralKey{length: 8, data: [1u8, 32]}, GeneralKey {length:8, data: [2u8, 32]})) => {
                // check if the asset is native or foreign, and deposit the asset to a tmp account first
                let tmp_account = MultiLocation::new(0, X1(GeneralKey {length: 8, data: [2u8; 32]})).into_account();
                if AssetTypeChecker::is_native_asset(what) {
                    CurrencyTransactor::deposit_asset(what, &Junction::AccountId32 { network: None, id: tmp_account }.into(), context)?;
                } else {
                    FungiblesTransactor::deposit_asset(what, &Junction::AccountId32 { network: None, id: tmp_account }.into(), context)?
                }

                // TODO: call deposit() extrisic in sygmaBrdige pallet. Sygma bridge pallet should also be in the PhantomData type
                Forwarder::other_world_transactor_forwarder(tmp_account, what.into(), who.into())
            }
            // Other destination multilocation not supported, return Err
            _ => {
                Err(XcmError::DestinationUnsupported)
            }
        }
        Ok(())
    }

    fn withdraw_asset(_what: &MultiAsset, _who: &MultiLocation, _maybe_context: Option<&XcmContext>) -> Result<Assets, XcmError> {
        // TODO:
        Ok(Assets::new())
    }
}
