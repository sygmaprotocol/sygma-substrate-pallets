// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

#[cfg(test)]
mod mock;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, traits::StorageVersion};
	use sygma_traits::FeeHandler;
	use xcm::latest::AssetId;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

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

		/// Fee handlers
		type FeeHandlers: Get<Vec<(AssetId, Box<dyn FeeHandler>)>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {}

	#[derive(Clone)]
	pub struct FeeHandlerRouterImpl<T>(PhantomData<T>);

	impl<T: Config> FeeHandler for FeeHandlerRouterImpl<T> {
		fn get_fee(&self, asset: &AssetId) -> Option<u128> {
			match T::FeeHandlers::get()
				.iter()
				.position(|e| e.0 == *asset)
				.map(|idx| dyn_clone::clone_box(&*T::FeeHandlers::get()[idx].1))
			{
				Some(handler) => handler.get_fee(asset),
				_ => None,
			}
		}
	}

	#[cfg(test)]
	mod test {}
}
