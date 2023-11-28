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

        type Weigher: WeightBounds<Self::RuntimeCall>;

        type XcmExecutor: ExecuteXcm<Self::RuntimeCall>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        XCMTransferSend {},
    }

    #[pallet::error]
    pub enum Error<T> {
        FailToWeightMessage,
        XcmExecutionFailed,
    }

    struct Xcm<T: Config>{
        asset: MultiAsset,
        origin: MultiLocation,
        dest: MultiLocation,
        recipient: MultiLocation,
        weight: XCMWeight,
    }

    pub trait XcmHandler {
        fn create_message(&self) -> Result<Xcm<T::RuntimeCall>, DispatchError>;
        fn execute_message(&self, xcm_message: Xcm<T::RuntimeCall>) -> DispatchResult;
    }

    impl XcmHandler for Xcm {
        fn create_message(&self) {

        }

        fn execute_message(&self, xcm_message: Xcm<T::RuntimeCall>) {
            let message_weight = T::Weigher::weight(xcm_message).map_err(|()| Error::<T>::FailToWeightMessage)?;

            let hash = xcm_message.using_encoded(sp_io::hashing::blake2_256);

            T::XcmExecutor::execute_xcm_in_credit(
                self.origin.clone(),
                xcm_message.clone(),
                hash,
                message_weight,
                message_weight
            ).ensure_complete().map_err(|_| Error::<T>::XcmExecutionFailed)?;

            oK(())
        }
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
