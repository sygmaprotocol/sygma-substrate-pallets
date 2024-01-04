// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_support::traits::StorageVersion;
    use xcm::latest::{MultiAsset, MultiLocation, Junction};

    use sygma_traits::{Bridge, TransactorForwarder};

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
}
