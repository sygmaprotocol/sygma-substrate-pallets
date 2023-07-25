// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion};
	use frame_system::pallet_prelude::*;
	use sp_std::boxed::Box;
	use sygma_traits::{DomainID, FeeHandler};
	use xcm::latest::{AssetId, Fungibility::Fungible, MultiAsset};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// Mapping fungible asset id and domain id to fee percentage
	#[pallet::storage]
	pub type AssetFeeRate<T: Config> = StorageMap<_, Twox64Concat, (DomainID, AssetId), u8>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + sygma_access_segregator::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Current pallet index defined in runtime
		type PalletIndex: Get<u8>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Fee set rate for a specific asset and domain
		/// args: [domain, asset, rate_basis_point]
		FeeRateSet { domain: DomainID, asset: AssetId, rate_basis_point: u8 },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Function unimplemented
		Unimplemented,
		/// Account has not gained access permission
		AccessDenied,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set bridge fee rate for a specific asset and domain. Note the fee rate is in Basis Point
		/// representation
		#[pallet::call_index(0)]
		#[pallet::weight(195_000_000)]
		pub fn set_fee_rate(
			origin: OriginFor<T>,
			domain: DomainID,
			asset: Box<AssetId>,
			fee_rate_basis_point: u8,
		) -> DispatchResult {
			let asset: AssetId = *asset;
			ensure!(
				<sygma_access_segregator::pallet::Pallet<T>>::has_access(
					<T as Config>::PalletIndex::get(),
					b"set_fee_rate".to_vec(),
					origin
				),
				Error::<T>::AccessDenied
			);

			// Update asset fee rate
			AssetFeeRate::<T>::insert((domain, &asset), fee_rate_basis_point);

			// Emit FeeRateSet event
			Self::deposit_event(Event::FeeRateSet {
				domain,
				asset,
				rate_basis_point: fee_rate_basis_point,
			});
			Ok(())
		}
	}

	impl<T: Config> FeeHandler for Pallet<T> {
		fn get_fee(domain: DomainID, asset: MultiAsset) -> Option<u128> {
			match (asset.fun, asset.id) {
				(Fungible(amount), _) => {
					let fee_rate_basis_point =
						AssetFeeRate::<T>::get((domain, asset.id)).unwrap_or_default();
					Some(amount.saturating_mul(fee_rate_basis_point as u128).saturating_div(10000))
				},
				_ => None,
			}
		}
	}
}
