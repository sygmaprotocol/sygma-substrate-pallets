#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use sygma_traits::{Bridge};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        XCMTransferSend {},
    }

    #[pallet::call]
    impl<T: Config> Bridge for Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(195_000_000, 0))]
        #[transactional]
        fn transfer(sender: [u8; 32],
                    asset: MultiAsset,
                    dest: MultiLocation) -> DispatchResult {
            // TODO: create xcm message
            // TODO: execute xcm message
        }
    }
}
