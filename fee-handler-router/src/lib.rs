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
	use sygma_traits::{DomainID, FeeHandler};
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
		type FeeHandlers: Get<Vec<((DomainID, AssetId), Box<dyn FeeHandler>)>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {}

	#[derive(Clone)]
	pub struct FeeRouterImpl<T>(PhantomData<T>);
	impl<T: Config> FeeRouterImpl<T> {
		pub fn new() -> Self {
			Self(PhantomData)
		}
	}
	impl<T: Config> FeeHandler for FeeRouterImpl<T> {
		fn get_fee(&self, domain: DomainID, asset: &AssetId) -> Option<u128> {
			match T::FeeHandlers::get()
				.iter()
				.position(|e| e.0 == (domain, asset.clone()))
				.map(|idx| dyn_clone::clone_box(&*T::FeeHandlers::get()[idx].1))
			{
				Some(handler) => handler.get_fee(domain, asset),
				_ => None,
			}
		}
	}

	#[cfg(test)]
	mod test {
		use crate as fee_router;
		use fee_router::mock::{
			new_test_ext, BasicFeeHandlerForEthereum, BasicFeeHandlerForMoonbeam, EthereumDomainID,
			MoonbeamDomainID, PhaLocation, RuntimeOrigin as Origin, SygmaBasicFeeHandler, Test,
		};
		use frame_support::assert_ok;
		use sygma_traits::FeeHandler;

		#[test]
		fn fee_router_should_work() {
			new_test_ext().execute_with(|| {
				// set fee 100 with PHA for Ethereum
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					EthereumDomainID::get(),
					PhaLocation::get().into(),
					10000
				));

				// set fee 100 with PHA for Ethereum
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					MoonbeamDomainID::get(),
					PhaLocation::get().into(),
					100
				));

				assert_eq!(
					super::FeeRouterImpl::<Test>::new()
						.get_fee(EthereumDomainID::get(), &PhaLocation::get().into())
						.unwrap(),
					10000
				);
				// Or query from handler entity
				assert_eq!(
					BasicFeeHandlerForEthereum::get()
						.get_fee(EthereumDomainID::get(), &PhaLocation::get().into())
						.unwrap(),
					10000
				);

				assert_eq!(
					super::FeeRouterImpl::<Test>::new()
						.get_fee(MoonbeamDomainID::get(), &PhaLocation::get().into())
						.unwrap(),
					100
				);
				// Or query from handler entity
				assert_eq!(
					BasicFeeHandlerForMoonbeam::get()
						.get_fee(MoonbeamDomainID::get(), &PhaLocation::get().into())
						.unwrap(),
					100
				);
			})
		}
	}
}
