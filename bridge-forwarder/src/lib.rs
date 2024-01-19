// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

pub mod xcm_asset_transactor;
#[cfg(test)]
mod mock;

#[frame_support::pallet]
pub mod pallet {
    use cumulus_primitives_core::ParaId;
    use frame_support::pallet_prelude::*;
    use frame_support::traits::StorageVersion;
    use xcm::latest::{MultiAsset, MultiLocation, Junction};
    use xcm::prelude::{Concrete, Fungible, Parachain, X1};

    use sygma_traits::{AssetTypeIdentifier, Bridge, TransactorForwarder};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type SygmaBridge: Bridge;
        type XCMBridge: Bridge;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        XCMTransferForward {
            asset: MultiAsset,
            origin: MultiLocation,
            dest: MultiLocation,
        },
        OtherWorldTransferForward {
            asset: MultiAsset,
            origin: MultiLocation,
            dest: MultiLocation,
        },
    }

    impl<T: Config> TransactorForwarder for Pallet<T> {
        fn xcm_transactor_forwarder(origin: [u8; 32], what: MultiAsset, dest: MultiLocation) -> DispatchResult {
            T::XCMBridge::transfer(origin, what.clone(), dest.clone())?;

            let origin_location: MultiLocation = Junction::AccountId32 {
                network: None,
                id: origin,
            }.into();

            Pallet::<T>::deposit_event(Event::XCMTransferForward {
                asset: what,
                origin: origin_location,
                dest,
            });

            Ok(())
        }

        fn other_world_transactor_forwarder(origin: [u8; 32], what: MultiAsset, dest: MultiLocation) -> DispatchResult {
            T::SygmaBridge::transfer(origin, what.clone(), dest.clone())?;

            let origin_location: MultiLocation = Junction::AccountId32 {
                network: None,
                id: origin,
            }.into();

            Pallet::<T>::deposit_event(Event::OtherWorldTransferForward {
                asset: what,
                origin: origin_location,
                dest,
            });

            Ok(())
        }
    }

    pub struct NativeAssetTypeIdentifier<T>(PhantomData<T>);
    impl<T: Get<ParaId>> AssetTypeIdentifier for NativeAssetTypeIdentifier<T> {
        /// check if the given MultiAsset is a native asset
        fn is_native_asset(asset: &MultiAsset) -> bool {
            // currently there are two multilocations are considered as native asset:
            // 1. integrated parachain native asset(MultiLocation::here())
            // 2. other parachain native asset(MultiLocation::new(1, X1(Parachain(T::get().into()))))
            let native_locations = [
                MultiLocation::here(),
                MultiLocation::new(1, X1(Parachain(T::get().into()))),
            ];

            match (&asset.id, &asset.fun) {
                (Concrete(ref id), Fungible(_)) => {
                    native_locations.contains(id)
                }
                _ => false,
            }
        }
    }

    #[cfg(test)]
    mod test {
        use frame_support::assert_ok;
        use xcm::latest::Junction;
        use xcm::prelude::{Concrete, Fungible, Parachain, X2, Here, X1};
        use xcm::v3::Junction::GeneralIndex;
        use xcm::v3::{MultiAsset, MultiLocation};
        use sygma_traits::{AssetTypeIdentifier, TransactorForwarder};
        use crate::mock::{new_test_ext, SygmaBridgeForwarder, ALICE, assert_events, RuntimeEvent, ParachainInfo};
        use crate::{Event as SygmaBridgeForwarderEvent, NativeAssetTypeIdentifier};
        #[test]
        fn test_xcm_transactor_forwarder() {
            new_test_ext().execute_with(|| {

                let asset: MultiAsset = (Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into();
                let dest: MultiLocation = MultiLocation::new(1, X2(Parachain(1), GeneralIndex(1u128)));

                assert_ok!(SygmaBridgeForwarder::xcm_transactor_forwarder(ALICE.into(), asset.clone(), dest.clone()));

                assert_events(vec![RuntimeEvent::SygmaBridgeForwarder(SygmaBridgeForwarderEvent::XCMTransferForward {
                    asset,
                    origin: Junction::AccountId32 {
                        network: None,
                        id: ALICE.into(),
                    }.into(),
                    dest,
                })]);
            })
        }

        #[test]
        fn test_other_world_transactor_forwarder() {
            new_test_ext().execute_with(|| {

                let asset: MultiAsset = (Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into();
                let dest: MultiLocation = MultiLocation::new(1, X2(Parachain(1), GeneralIndex(1u128)));

                assert_ok!(SygmaBridgeForwarder::other_world_transactor_forwarder(ALICE.into(), asset.clone(), dest.clone()));

                assert_events(vec![RuntimeEvent::SygmaBridgeForwarder(SygmaBridgeForwarderEvent::OtherWorldTransferForward {
                    asset,
                    origin: Junction::AccountId32 {
                        network: None,
                        id: ALICE.into(),
                    }.into(),
                    dest,
                })]);
            })
        }

        #[test]
        fn test_asset_type_identifier_native_asset() {
            new_test_ext().execute_with(|| {

                let asset_local_native: MultiAsset = (Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into();
                assert_eq!(NativeAssetTypeIdentifier::<ParachainInfo>::is_native_asset(&asset_local_native), true);

                // ParachainInfo is given parachain ID as 100
                let asset_foreign_native: MultiAsset = (Concrete(MultiLocation::new(1, X1(Parachain(100)))), Fungible(10u128)).into();
                assert_eq!(NativeAssetTypeIdentifier::<ParachainInfo>::is_native_asset(&asset_foreign_native), true);

                let asset_foreign_asset: MultiAsset = (Concrete(MultiLocation::new(0, X1(Parachain(100)))), Fungible(10u128)).into();
                assert_eq!(NativeAssetTypeIdentifier::<ParachainInfo>::is_native_asset(&asset_foreign_asset), false);
            })
        }
    }
}
