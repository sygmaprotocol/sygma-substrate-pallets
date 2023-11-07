#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use frame_support::transactional;
    use frame_support::traits::StorageVersion;
    use sygma_traits::{TransactorForwarder, Bridge};
    use xcm::latest::{prelude::*, MultiAsset, MultiLocation};

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
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        XCMTransferForward {},
        OtherWorldTransferForward {},
    }

    #[pallet::call]
    impl<T: Config> TransactorForwarder for Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(195_000_000, 0))]
        #[transactional]
        fn xcm_transactor_forwarder(origin: OriginFor<T>, what: MultiAsset, who: MultiLocation) -> DispatchResult {
            T::XCMBridge::transfer(origin.into(), what, who)?;
        }

        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(195_000_000, 0))]
        #[transactional]
        fn other_world_transactor_forwarder(origin: OriginFor<T>, what: MultiAsset, who: MultiLocation) -> DispatchResult {
            T::SygmaBridge::transfer(origin.into(), what, who)?
        }
    }
}
