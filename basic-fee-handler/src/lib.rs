#![cfg_attr(not(feature = "std"), no_std)]

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
	#[pallet::generate_store(pub(super) trait Store)]
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
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
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

			Err(Error::<T>::Unimplemented.into())
		}
	}

	pub struct BasicFeeHandlerImpl<T>(PhantomData<T>);
	impl<T: Config> FeeHandler for BasicFeeHandlerImpl<T> {
		fn new() -> Self {
			Self(PhantomData)
		}

		fn get_fee(&self, asset: AssetId) -> Option<u128> {
			// TODO
			None
		}
	}

	#[cfg(test)]
	mod test {}
}
