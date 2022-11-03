#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use codec::{Decode, Encode, EncodeLike};
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion};
    use frame_system::pallet_prelude::*;
    use sygma_traits::FeeHandler;
    use xcm::latest::{prelude::*, AssetId};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    /// Mapping fungible asset id to corresponding fee amount
    #[pallet::storage]
    #[pallet::getter(fn asset_fees)]
    pub type AssetFees<T: Config> = StorageMap<_, Twox64Concat, AssetId, u128>;

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Origin used to administer the pallet
        type BridgeCommitteeOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Fee set for a specific asset
        /// args: [asset, amount]
        FeeSet { asset: AssetId, amount: u128 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Function unimplemented
        Unimplemented,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set bridge fee for a specific asset
        #[pallet::weight(195_000_000)]
        pub fn set_fee(origin: OriginFor<T>, asset: AssetId, amount: u128) -> DispatchResult {
            // Ensure bridge committee
            T::BridgeCommitteeOrigin::ensure_origin(origin)?;

            // Update asset fee
            AssetFees::<T>::insert(&asset, amount);

            // Emit FeeSet event
            Self::deposit_event(Event::FeeSet { asset, amount });
            Ok(())
        }
    }

    pub struct BasicFeeHandlerImpl<T>(PhantomData<T>);

    impl<T: Config> FeeHandler for BasicFeeHandlerImpl<T> {
        fn new() -> Self {
            Self(PhantomData)
        }

        fn get_fee(&self, asset: AssetId) -> Option<u128> {
            AssetFees::<T>::get(asset)
        }
    }

    #[cfg(test)]
    mod test {
        use crate as basic_fee_handler;
        use crate::AssetFees;
        use frame_support::assert_ok;
        use basic_fee_handler::mock::{RuntimeOrigin as Origin, RuntimeEvent as Event, new_test_ext, assert_events, BasicFeeHandler, Test};
        use crate::Event as BasicFeeHandlerEvent;
        use xcm::latest::{prelude::*, MultiLocation};

        #[test]
        fn set_get_fee() {
            new_test_ext().execute_with(|| {
                let asset_id_a = Concrete(MultiLocation::new(1, Here));
                let amount_a = 100u128;

                let asset_id_b = Concrete(MultiLocation::new(2, Here));
                let amount_b = 101u128;

                // set fee 100 with assetId asset_id_a
                assert_ok!(BasicFeeHandler::set_fee(Origin::root(), asset_id_a.clone(), amount_a));
                assert_eq!(
                    AssetFees::<Test>::get(asset_id_a.clone()).unwrap(),
                    amount_a
                );

                // set fee 101 with assetId asset_id_b
                assert_ok!(BasicFeeHandler::set_fee(Origin::root(), asset_id_b.clone(), amount_b));
                assert_eq!(
                    AssetFees::<Test>::get(asset_id_b.clone()).unwrap(),
                    amount_b
                );

                // fee of asset_id_a should not be equal to amount_b
                assert_ne!(
                    AssetFees::<Test>::get(asset_id_a.clone()).unwrap(),
                    amount_b
                );

                // fee of asset_id_b should not be equal to amount_a
                assert_ne!(
                    AssetFees::<Test>::get(asset_id_b.clone()).unwrap(),
                    amount_a
                );

                assert_events(vec![
                    Event::BasicFeeHandler(BasicFeeHandlerEvent::FeeSet { asset: asset_id_a.into(), amount: amount_a }),
                    Event::BasicFeeHandler(BasicFeeHandlerEvent::FeeSet { asset: asset_id_b.into(), amount: amount_b })
                ]);
            })
        }
    }
}
