// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

pub use self::pallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub use weights::*;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use xcm::latest::MultiAsset;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// Mapping fungible asset id and domain id to fee percentage
	#[pallet::storage]
	pub type AssetFeeRate<T: Config> = StorageMap<_, Twox64Concat, (DomainID, AssetId), u8>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	pub trait WeightInfo {
		fn set_fee_rate() -> Weight;
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + sygma_access_segregator::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Current pallet index defined in runtime
		type PalletIndex: Get<u8>;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
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
		/// Set bridge fee rate for a specific asset and domain. Note the fee rate is in Basis Point representation
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::set_fee_rate())]
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
			AssetFeeRate::<T>::insert((domain, &asset), rate_basis_point);

			// Emit FeeRateSet event
			Self::deposit_event(Event::FeeRateSet { domain, asset, rate_basis_point: fee_rate_basis_point });
			Ok(())
		}
	}

	impl<T: Config> FeeHandler for Pallet<T> {
		fn get_fee(domain: DomainID, asset: &MultiAsset) -> Option<u8> {
			let fee_rate_basis_point = AssetFeeRate::<T>::get((domain, asset.id));
			asset.fun * fee_rate_basis_point / 1e4
		}
	}
}
