// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

#[cfg(test)]
mod mock;
pub mod xcm_asset_transactor;

#[frame_support::pallet]
pub mod pallet {
	use cumulus_primitives_core::ParaId;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::StorageVersion;
	use xcm::latest::{Junction, MultiAsset, MultiLocation};
	use xcm::prelude::{Concrete, Fungible, Parachain, X1};

	use sygma_traits::{AssetTypeIdentifier, Bridge, TransactorForwarder};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type SygmaBridge: Bridge;
		type XCMBridge: Bridge;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		XCMTransferForward { asset: MultiAsset, origin: MultiLocation, dest: MultiLocation },
		OtherWorldTransferForward { asset: MultiAsset, origin: MultiLocation, dest: MultiLocation },
	}

	impl<T: Config> TransactorForwarder for Pallet<T> {
		fn xcm_transactor_forwarder(
			origin: [u8; 32],
			what: MultiAsset,
			dest: MultiLocation,
		) -> DispatchResult {
			T::XCMBridge::transfer(origin, what.clone(), dest)?;

			let origin_location: MultiLocation =
				Junction::AccountId32 { network: None, id: origin }.into();

			Pallet::<T>::deposit_event(Event::XCMTransferForward {
				asset: what,
				origin: origin_location,
				dest,
			});

			Ok(())
		}

		fn other_world_transactor_forwarder(
			origin: [u8; 32],
			what: MultiAsset,
			dest: MultiLocation,
		) -> DispatchResult {
			T::SygmaBridge::transfer(origin, what.clone(), dest)?;

			let origin_location: MultiLocation =
				Junction::AccountId32 { network: None, id: origin }.into();

			Pallet::<T>::deposit_event(Event::OtherWorldTransferForward {
				asset: what,
				origin: origin_location,
				dest,
			});

			Ok(())
		}
	}

	pub struct NativeAssetTypeIdentifier<T>(PhantomData<T>);

	impl<T: Get<ParaId>> AssetTypeIdentifier for NativeAssetTypeIdentifier<T> {
		/// check if the given MultiAsset is a native asset
		fn is_native_asset(asset: &MultiAsset) -> bool {
			// currently there are two multilocations are considered as native asset:
			// 1. integrated parachain native asset(MultiLocation::here())
			// 2. other parachain native asset(MultiLocation::new(1, X1(Parachain(T::get().into()))))
			let native_locations =
				[MultiLocation::here(), MultiLocation::new(1, X1(Parachain(T::get().into())))];

			match (&asset.id, &asset.fun) {
				(Concrete(ref id), Fungible(_)) => native_locations.contains(id),
				_ => false,
			}
		}
	}

	#[cfg(test)]
	mod test {
		use frame_support::assert_ok;
		use xcm::latest::{Junction, XcmContext};
		use xcm::prelude::{AccountId32, Concrete, Fungible, Here, Parachain, X1, X2};
		use xcm::v3::Junction::GeneralIndex;
		use xcm::v3::{MultiAsset, MultiLocation};
		use xcm_executor::traits::TransactAsset;

		use sygma_traits::{AssetTypeIdentifier, TransactorForwarder};

		use crate::mock::{
			assert_events, new_test_ext, Assets, Balances, CurrencyTransactor,
			ForwarderImplRuntime, FungiblesTransactor, ParachainInfo, RuntimeEvent,
			SygmaBridgeForwarder, UsdtAssetId, UsdtLocation, ALICE, BOB,
		};
		use crate::{
			xcm_asset_transactor::XCMAssetTransactor, Event as SygmaBridgeForwarderEvent,
			NativeAssetTypeIdentifier,
		};

		#[test]
		fn test_xcm_transactor_forwarder() {
			new_test_ext().execute_with(|| {
				let asset: MultiAsset =
					(Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into();
				let dest: MultiLocation =
					MultiLocation::new(1, X2(Parachain(1), GeneralIndex(1u128)));

				assert_ok!(SygmaBridgeForwarder::xcm_transactor_forwarder(
					ALICE.into(),
					asset.clone(),
					dest
				));

				assert_events(vec![RuntimeEvent::SygmaBridgeForwarder(
					SygmaBridgeForwarderEvent::XCMTransferForward {
						asset,
						origin: Junction::AccountId32 { network: None, id: ALICE.into() }.into(),
						dest,
					},
				)]);
			})
		}

		#[test]
		fn test_other_world_transactor_forwarder() {
			new_test_ext().execute_with(|| {
				let asset: MultiAsset =
					(Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into();
				let dest: MultiLocation =
					MultiLocation::new(1, X2(Parachain(1), GeneralIndex(1u128)));

				assert_ok!(SygmaBridgeForwarder::other_world_transactor_forwarder(
					ALICE.into(),
					asset.clone(),
					dest
				));

				assert_events(vec![RuntimeEvent::SygmaBridgeForwarder(
					SygmaBridgeForwarderEvent::OtherWorldTransferForward {
						asset,
						origin: Junction::AccountId32 { network: None, id: ALICE.into() }.into(),
						dest,
					},
				)]);
			})
		}

		#[test]
		fn test_asset_type_identifier_native_asset() {
			new_test_ext().execute_with(|| {
				let asset_local_native: MultiAsset =
					(Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into();
				assert!(NativeAssetTypeIdentifier::<ParachainInfo>::is_native_asset(
					&asset_local_native
				));

				// ParachainInfo is given parachain ID as 100
				let asset_foreign_native: MultiAsset =
					(Concrete(MultiLocation::new(1, X1(Parachain(100)))), Fungible(10u128)).into();
				assert!(NativeAssetTypeIdentifier::<ParachainInfo>::is_native_asset(
					&asset_foreign_native
				));

				let asset_foreign_asset: MultiAsset =
					(Concrete(MultiLocation::new(0, X1(Parachain(100)))), Fungible(10u128)).into();
				assert!(!NativeAssetTypeIdentifier::<ParachainInfo>::is_native_asset(
					&asset_foreign_asset
				));
			})
		}

		#[test]
		fn test_xcm_asset_transactor_local() {
			new_test_ext().execute_with(|| {
				let local_recipient: MultiLocation =
					MultiLocation::new(0, X1(AccountId32 { network: None, id: BOB.into() }));

				// send native asset to local parachain
				let local_native_asset: MultiAsset =
					(Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into();
				assert_ok!(XCMAssetTransactor::<
					CurrencyTransactor,
					FungiblesTransactor,
					NativeAssetTypeIdentifier<ParachainInfo>,
					ForwarderImplRuntime,
				>::deposit_asset(
					&local_native_asset,
					&local_recipient,
					&XcmContext::with_message_id([0; 32])
				));
				assert_eq!(Balances::free_balance(BOB), 10u128);

				// send foreign asset to local parachain
				let local_foreign_asset: MultiAsset =
					(Concrete(UsdtLocation::get()), Fungible(10u128)).into();
				assert_ok!(XCMAssetTransactor::<
					CurrencyTransactor,
					FungiblesTransactor,
					NativeAssetTypeIdentifier<ParachainInfo>,
					ForwarderImplRuntime,
				>::deposit_asset(
					&local_foreign_asset,
					&local_recipient,
					&XcmContext::with_message_id([0; 32])
				));
				assert_eq!(Assets::balance(UsdtAssetId::get(), &BOB), 10u128);
			})
		}
	}
}
