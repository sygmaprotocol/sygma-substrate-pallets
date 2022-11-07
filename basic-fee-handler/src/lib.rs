#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

#[cfg(test)]
mod mock;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion};
	use frame_system::pallet_prelude::*;
	use sygma_traits::FeeHandler;
	use xcm::latest::AssetId;

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
        use frame_support::{assert_err, assert_noop, assert_ok};
        use frame_support::sp_runtime::traits::BadOrigin;
        use xcm::latest::{MultiLocation, prelude::*};
        use basic_fee_handler::mock::{ALICE, assert_events,
                                      BasicFeeHandler, new_test_ext, RuntimeEvent as Event,
                                      RuntimeOrigin as Origin, Test};
        use crate as basic_fee_handler;
        use crate::AssetFees;
        use crate::Error as BasicFeeHandlerError;
        use crate::Event as BasicFeeHandlerEvent;

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

                // permission test: unauthorized account should not be able to set fee
                let unauthorized_account = Origin::from(Some(ALICE));
                assert_noop!(
                    BasicFeeHandler::set_fee(unauthorized_account, asset_id_a.clone(), amount_a),
                    BadOrigin
                );

                assert_events(vec![
                    Event::BasicFeeHandler(BasicFeeHandlerEvent::FeeSet { asset: asset_id_a.into(), amount: amount_a }),
                    Event::BasicFeeHandler(BasicFeeHandlerEvent::FeeSet { asset: asset_id_b.into(), amount: amount_b }),
                ]);
            })
        }
    }
}
