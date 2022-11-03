#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

mod mock;

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
		use super::*;
		use crate::Pallet;
		use frame_support::assert_ok;
		use crate::AssetFees;
		use crate::mock::{TestNet, RuntimeOrigin as Origin, ParaA, para_assert_events, RuntimeEvent as Event};
		use crate::Event as BasicFeeHandlerEvent;

		#[test]
		fn set_get_fee() {
			TestNet::reset();

			ParaA::execute_with(|| {
				let asset_id = 10;
				let amount = 100;

				assert_ok!(Pallet::set_fee(Origin::root(), asset_id, amount));
				assert_eq!(<AssetFees<Runtime>>::get(asset_id), amount);

				para_assert_events(vec![
					Event::BasicFeeHandler(BasicFeeHandlerEvent::FeeSet { asset_id, amount })
				]);
			})
		}
	}
}
