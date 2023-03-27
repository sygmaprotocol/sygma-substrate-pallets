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
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion};
	use frame_system::pallet_prelude::*;
	use sp_std::boxed::Box;
	use sygma_traits::{DomainID, FeeHandler};
	use xcm::latest::AssetId;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// Mapping fungible asset id to corresponding fee amount
	#[pallet::storage]
	#[pallet::getter(fn asset_fees)]
	pub type AssetFees<T: Config> = StorageMap<_, Twox64Concat, (DomainID, AssetId), u128>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + sygma_access_segregator::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Origin used to administer the pallet
		type BridgeCommitteeOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Current pallet index defined in runtime
		type PalletIndex: Get<u8>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Fee set for a specific asset
		/// args: [domain, asset, amount]
		FeeSet { domain: DomainID, asset: AssetId, amount: u128 },
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
		/// Set bridge fee for a specific asset
		#[pallet::call_index(0)]
		#[pallet::weight(195_000_000)]
		pub fn set_fee(
			origin: OriginFor<T>,
			domain: DomainID,
			asset: Box<AssetId>,
			amount: u128,
		) -> DispatchResult {
			let asset: AssetId = *asset;
			if <T as Config>::BridgeCommitteeOrigin::ensure_origin(origin.clone()).is_err() {
				// Ensure bridge committee or the account that has permisson to set fee
				let who = ensure_signed(origin)?;
				ensure!(
					<sygma_access_segregator::pallet::Pallet<T>>::has_access(
						<T as Config>::PalletIndex::get(),
						b"set_fee".to_vec(),
						who
					),
					Error::<T>::AccessDenied
				);
			}

			// Update asset fee
			AssetFees::<T>::insert((domain, &asset), amount);

			// Emit FeeSet event
			Self::deposit_event(Event::FeeSet { domain, asset, amount });
			Ok(())
		}
	}

	impl<T: Config> FeeHandler for Pallet<T> {
		fn get_fee(domain: DomainID, asset: &AssetId) -> Option<u128> {
			AssetFees::<T>::get((domain, asset))
		}
	}

	#[cfg(test)]
	mod test {
		use crate as basic_fee_handler;
		use crate::{AssetFees, Event as BasicFeeHandlerEvent};
		use basic_fee_handler::mock::{
			assert_events, new_test_ext, AccessSegregator, BasicFeeHandler, FeeHandlerPalletIndex,
			RuntimeEvent as Event, RuntimeOrigin as Origin, Test, ALICE,
		};
		use frame_support::{assert_noop, assert_ok};
		use sp_std::boxed::Box;
		use sygma_traits::DomainID;
		use xcm::latest::{prelude::*, MultiLocation};

		#[test]
		fn set_get_fee() {
			new_test_ext().execute_with(|| {
				let dest_domain_id: DomainID = 0;
				let another_dest_domain_id: DomainID = 1;
				let asset_id_a = Concrete(MultiLocation::new(1, Here));
				let amount_a = 100u128;

				let asset_id_b = Concrete(MultiLocation::new(2, Here));
				let amount_b = 101u128;

				// set fee 100 with assetId asset_id_a for one domain
				assert_ok!(BasicFeeHandler::set_fee(
					Origin::root(),
					dest_domain_id,
					Box::new(asset_id_a),
					amount_a
				));
				// set fee 200 with assetId asset_id_a for another domain
				assert_ok!(BasicFeeHandler::set_fee(
					Origin::root(),
					another_dest_domain_id,
					Box::new(asset_id_a),
					amount_a * 2
				));
				assert_eq!(AssetFees::<Test>::get((dest_domain_id, asset_id_a)).unwrap(), amount_a);
				assert_eq!(
					AssetFees::<Test>::get((another_dest_domain_id, asset_id_a)).unwrap(),
					amount_a * 2
				);

				// set fee 101 with assetId asset_id_b
				assert_ok!(BasicFeeHandler::set_fee(
					Origin::root(),
					dest_domain_id,
					Box::new(asset_id_b),
					amount_b
				));
				assert_eq!(AssetFees::<Test>::get((dest_domain_id, asset_id_b)).unwrap(), amount_b);

				// fee of asset_id_a should not be equal to amount_b
				assert_ne!(AssetFees::<Test>::get((dest_domain_id, asset_id_a)).unwrap(), amount_b);

				// fee of asset_id_b should not be equal to amount_a
				assert_ne!(AssetFees::<Test>::get((dest_domain_id, asset_id_b)).unwrap(), amount_a);

				// permission test: unauthorized account should not be able to set fee
				let unauthorized_account = Origin::from(Some(ALICE));
				assert_noop!(
					BasicFeeHandler::set_fee(
						unauthorized_account,
						dest_domain_id,
						Box::new(asset_id_a),
						amount_a
					),
					basic_fee_handler::Error::<Test>::AccessDenied
				);

				assert_events(vec![
					Event::BasicFeeHandler(BasicFeeHandlerEvent::FeeSet {
						domain: dest_domain_id,
						asset: asset_id_a,
						amount: amount_a,
					}),
					Event::BasicFeeHandler(BasicFeeHandlerEvent::FeeSet {
						domain: another_dest_domain_id,
						asset: asset_id_a,
						amount: amount_a * 2,
					}),
					Event::BasicFeeHandler(BasicFeeHandlerEvent::FeeSet {
						domain: dest_domain_id,
						asset: asset_id_b,
						amount: amount_b,
					}),
				]);
			})
		}

		#[test]
		fn access_control() {
			new_test_ext().execute_with(|| {
				let dest_domain_id: DomainID = 0;
				let asset_id = Concrete(MultiLocation::new(0, Here));

				assert_ok!(BasicFeeHandler::set_fee(
					Origin::root(),
					dest_domain_id,
					Box::new(asset_id),
					100
				),);
				assert_noop!(
					BasicFeeHandler::set_fee(
						Some(ALICE).into(),
						dest_domain_id,
						Box::new(asset_id),
						200
					),
					basic_fee_handler::Error::<Test>::AccessDenied
				);
				// (FeeHandlerPalletIndex:get(), b"set_fee") indicates extrinsic: `set_fee` of this
				// pallet
				assert!(!AccessSegregator::has_access(
					FeeHandlerPalletIndex::get(),
					b"set_fee".to_vec(),
					ALICE
				));
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					FeeHandlerPalletIndex::get(),
					b"set_fee".to_vec(),
					ALICE
				));
				assert!(AccessSegregator::has_access(
					FeeHandlerPalletIndex::get(),
					b"set_fee".to_vec(),
					ALICE
				));
				assert_ok!(BasicFeeHandler::set_fee(
					Some(ALICE).into(),
					dest_domain_id,
					Box::new(asset_id),
					200
				),);
				assert_eq!(AssetFees::<Test>::get((dest_domain_id, asset_id)).unwrap(), 200);
			})
		}
	}
}
