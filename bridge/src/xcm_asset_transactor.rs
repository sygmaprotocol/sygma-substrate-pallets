use xcm::latest::{Junctions, MultiAsset, MultiLocation, XcmContext};
use xcm::prelude::{GeneralKey, X3};
use xcm::v3::Junction::Parachain;
use xcm_executor::{traits::TransactAsset};

pub struct XCMAssetTransactor;

impl TransactAsset for XCMAssetTransactor{
    // deposit_asset implements the TransactAsset deposit_asset method and contains the logic to classify
    // the asset recipient location:
    // 1. recipient is on the local parachain
    // 2. recipient is on the remote parachain
    // 3, recipient is on non-substrate chain(evm, cosmos, etc.)
    fn deposit_asset(what: &MultiAsset, who: &MultiLocation, context: &XcmContext){
        match (who.parents, who.interior) {
            // 1. recipient is the local parachain
            (0, None) => {
                // TODO: check if the asset is native or foreign, and call the corresponding deposit_asset()
            }
            // 2. recipient is remote parachain
            (1, Some(Parachain(_))) => {
                // TODO: call the xcm handler pallet to construct the xcm message (evm to remote parachain route)
                // xcm message must have a sender(origin), so a tmp account derived from pallet would be used
                // check if the asset is native or foreign, and call the corresponding deposit_asset(), recipient will be the derived tmp account
                // xcm message execution

                // trying to eliminate the forward logic here by adding the XCM handler pallet as one of the generic type for XCMAssetTransactor
            }
            // 3. recipient is on non-substrate chain(evm, cosmos, etc.), will forward to sygma bridge pallet
            // TODO: the junctions below is just an temporary example, will change it to proper sygma bridge standard, see the link below:
            // (https://www.notion.so/chainsafe/Sygma-as-an-Independent-pallet-c481f00ccff84ff49ce917c8b2feacda?pvs=4#6e51e6632e254b9b9a01444ef7297969)
            (0, &X3(Parachain(_), GeneralKey{length: 8, data: [1u8, 32]}, GeneralKey {length:8, data: [2u8, 32]})) => {
                // TODO: check if the asset is native or foreign, and deposit the asset to a tmp account first
                // TODO: call deposit() extrisic in sygmaBrdige pallet
            }
            // Other destination multilocation not supported, return Err
            _ => {
                Err("Destination not supported")
            }
        }
    }

    fn withdraw_asset(_what: &MultiAsset, _who: &MultiLocation, _maybe_context: Option<&XcmContext>,){
        // TODO:
    }
}
