#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

// #[cfg(test)]
// mod mock;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion};
	use frame_system::pallet_prelude::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// Mapping signature of extrinsic to account has access
	#[pallet::storage]
	#[pallet::getter(fn extrinsic_access)]
	pub type ExtrinsicAccess<T: Config> = StorageMap<_, Twox64Concat, [u8; 4], T::AccountId>;

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
		/// Extrinsic access grant to someone
		/// args: [sig, who]
		AccessGranted { sig: [u8; 4], who: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Function unimplemented
		Unimplemented,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Grants access to an account for a extrinsic.
		#[pallet::weight(195_000_000)]
		pub fn grant_access(
			origin: OriginFor<T>,
			sig: [u8; 4],
			who: T::AccountId,
		) -> DispatchResult {
			// Ensure bridge committee
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			// Apply access
			ExtrinsicAccess::<T>::insert(&sig, &who);

			// Emit AccessGranted event
			Self::deposit_event(Event::AccessGranted { sig, who });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn has_access(sig: [u8; 4], caller: T::AccountId) -> bool {
			// ExtrinsicAccess::<T>::get(&sig).is_some_and(|who| who == caller)
			false
		}
	}
}
