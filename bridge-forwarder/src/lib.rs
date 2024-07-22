// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

#[cfg(test)]
mod mock;
pub mod xcm_asset_transactor;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_support::traits::StorageVersion;
	use xcm::opaque::v4::{Asset, Location};
	use xcm::v4::Junction;

	use sygma_traits::{Bridge, TransactorForwarder};

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
		XCMTransferForward { asset: Asset, origin: Location, dest: Location },
		OtherWorldTransferForward { asset: Asset, origin: Location, dest: Location },
	}

	impl<T: Config> TransactorForwarder for Pallet<T> {
		fn xcm_transactor_forwarder(
			origin: [u8; 32],
			what: Asset,
			dest: Location,
		) -> DispatchResult {
			let cap_weight: Weight = Weight::from_all(u64::MAX);
			T::XCMBridge::transfer(origin, what.clone(), dest.clone(), Some(cap_weight))?;

			let origin_location: Location =
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
			what: Asset,
			dest: Location,
		) -> DispatchResult {
			T::SygmaBridge::transfer(origin, what.clone(), dest.clone(), None)?;

			let origin_location: Location =
				Junction::AccountId32 { network: None, id: origin }.into();

			Pallet::<T>::deposit_event(Event::OtherWorldTransferForward {
				asset: what,
				origin: origin_location,
				dest,
			});

			Ok(())
		}
	}

	#[cfg(test)]
	mod test {
		use codec::Encode;
		use frame_support::{assert_ok, traits::tokens::fungibles::Create as FungibleCerate};
		use hex_literal::hex;
		use sp_std::sync::Arc;
		use xcm::latest::AssetId;
		use xcm::prelude::{AccountId32, Fungible, GeneralKey, Here, Parachain};
		use xcm::v4::Junction::GeneralIndex;
		use xcm::v4::Junctions;
		use xcm::v4::{Asset, Location};
		use xcm::v4::{
			Junction,
			Junctions::{X1, X2, X3, X4},
			XcmContext,
		};
		use xcm_executor::traits::TransactAsset;

		use sygma_traits::{AssetTypeIdentifier, TransactorForwarder};

		use crate::mock::{
			assert_events, new_test_ext, slice_to_generalkey, Assets, Balances, CurrencyTransactor,
			ForwarderImplRuntime, FungiblesTransactor, NativeAssetTypeIdentifier, ParachainInfo,
			Runtime, RuntimeEvent, RuntimeOrigin, SygmaBridgeForwarder, UsdtAssetId, UsdtLocation,
			ALICE, ASSET_OWNER, BOB, ENDOWED_BALANCE,
		};
		use crate::{xcm_asset_transactor::XCMAssetTransactor, Event as SygmaBridgeForwarderEvent};

		#[test]
		fn test_xcm_transactor_forwarder() {
			new_test_ext().execute_with(|| {
				let asset: Asset = (AssetId(Location::new(0, Here)), Fungible(10u128)).into();
				let dest: Location =
					Location::new(1, Junctions::X2(Arc::new([Parachain(1), GeneralIndex(1u128)])));

				assert_ok!(SygmaBridgeForwarder::xcm_transactor_forwarder(
					ALICE.into(),
					asset.clone(),
					dest.clone()
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
				let asset: Asset = (AssetId(Location::new(0, Here)), Fungible(10u128)).into();
				let dest: Location =
					Location::new(1, Junctions::X2(Arc::new([Parachain(1), GeneralIndex(1u128)])));

				assert_ok!(SygmaBridgeForwarder::other_world_transactor_forwarder(
					ALICE.into(),
					asset.clone(),
					dest.clone()
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
				let asset_local_native: Asset =
					(AssetId(Location::new(0, Here)), Fungible(10u128)).into();
				assert!(NativeAssetTypeIdentifier::<ParachainInfo>::is_native_asset(
					&asset_local_native
				));

				// ParachainInfo is given parachain ID as 100
				let asset_foreign_native: Asset = (
					AssetId(Location::new(1, Junctions::X1(Arc::new([Parachain(100)])))),
					Fungible(10u128),
				)
					.into();
				assert!(NativeAssetTypeIdentifier::<ParachainInfo>::is_native_asset(
					&asset_foreign_native
				));

				let asset_foreign_asset: Asset =
					(AssetId(Location::new(0, X1(Arc::new([Parachain(100)])))), Fungible(10u128))
						.into();
				assert!(!NativeAssetTypeIdentifier::<ParachainInfo>::is_native_asset(
					&asset_foreign_asset
				));
			})
		}

		#[test]
		fn test_xcm_asset_transactor_local() {
			new_test_ext().execute_with(|| {
				let local_recipient: Location =
					Location::new(0, X1(Arc::new([AccountId32 { network: None, id: BOB.into() }])));

				// send native asset to local parachain
				let local_native_asset: Asset =
					(AssetId(Location::new(0, Here)), Fungible(10u128)).into();
				assert_ok!(XCMAssetTransactor::<
					CurrencyTransactor,
					FungiblesTransactor,
					NativeAssetTypeIdentifier<ParachainInfo>,
					ForwarderImplRuntime,
				>::deposit_asset(
					&local_native_asset,
					&local_recipient,
					Some(&XcmContext::with_message_id([0; 32]))
				));
				assert_eq!(Balances::free_balance(BOB), 10u128);

				// Register foreign asset (USDT) with asset id 1
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdtAssetId::get(), ASSET_OWNER, true, 1,));

				// send foreign asset to local parachain
				let local_foreign_asset: Asset =
					(AssetId(UsdtLocation::get()), Fungible(10u128)).into();
				assert_ok!(XCMAssetTransactor::<
					CurrencyTransactor,
					FungiblesTransactor,
					NativeAssetTypeIdentifier<ParachainInfo>,
					ForwarderImplRuntime,
				>::deposit_asset(
					&local_foreign_asset,
					&local_recipient,
					Some(&XcmContext::with_message_id([0; 32]))
				));
				assert_eq!(Assets::balance(UsdtAssetId::get(), &BOB), 10u128);
			})
		}

		#[test]
		fn test_xcm_asset_transactor_outer() {
			new_test_ext().execute_with(|| {
				let dest_domain_id = 1;
				let outer_recipient: Location = Location::new(
					1,
					X4(Arc::new([
						GeneralKey {
							length: 5,
							data: hex![
								"7379676d61000000000000000000000000000000000000000000000000000000"
							],
						},
						GeneralKey {
							length: 12,
							data: hex![
								"7379676d612d6272696467650000000000000000000000000000000000000000"
							],
						},
						GeneralIndex(dest_domain_id),
						slice_to_generalkey(b"ethereum recipient"),
					])),
				);

				// send native asset to the outer world
				let native_asset: Asset =
					(AssetId(Location::new(0, Here)), Fungible(10u128)).into();
				assert_ok!(XCMAssetTransactor::<
					CurrencyTransactor,
					FungiblesTransactor,
					NativeAssetTypeIdentifier<ParachainInfo>,
					ForwarderImplRuntime,
				>::deposit_asset(
					&native_asset,
					&outer_recipient,
					Some(&XcmContext::with_message_id([0; 32]))
				));
				// asset tmp holder for outer world transfer
				let tmp_account = sp_io::hashing::blake2_256(
					&Location::new(0, X1(Arc::new([GeneralKey { length: 8, data: [1u8; 32] }])))
						.encode(),
				);
				assert_eq!(
					Balances::free_balance(sp_runtime::AccountId32::from(tmp_account)),
					10u128
				);

				// Register foreign asset (USDT) with asset id 1
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdtAssetId::get(), ASSET_OWNER, true, 1,));

				// send foreign asset to the outer world
				let foreign_asset: Asset = (AssetId(UsdtLocation::get()), Fungible(10u128)).into();
				assert_ok!(XCMAssetTransactor::<
					CurrencyTransactor,
					FungiblesTransactor,
					NativeAssetTypeIdentifier<ParachainInfo>,
					ForwarderImplRuntime,
				>::deposit_asset(
					&foreign_asset,
					&outer_recipient,
					Some(&XcmContext::with_message_id([0; 32]))
				));
				// asset tmp holder for outer world transfer
				let tmp_account = sp_io::hashing::blake2_256(
					&Location::new(0, X1(Arc::new([GeneralKey { length: 8, data: [1u8; 32] }])))
						.encode(),
				);
				assert_eq!(
					Assets::balance(UsdtAssetId::get(), sp_runtime::AccountId32::from(tmp_account)),
					10u128
				);
			})
		}

		#[test]
		fn test_xcm_asset_transactor_substrate() {
			new_test_ext().execute_with(|| {
				let substrate_recipient: Location = Location::new(
					1,
					X2(Arc::new([Parachain(2005), slice_to_generalkey(b"substrate recipient")])),
				);

				// send native asset to the substrate world
				let native_asset: Asset =
					(AssetId(Location::new(0, Here)), Fungible(10u128)).into();
				assert_ok!(XCMAssetTransactor::<
					CurrencyTransactor,
					FungiblesTransactor,
					NativeAssetTypeIdentifier<ParachainInfo>,
					ForwarderImplRuntime,
				>::deposit_asset(
					&native_asset,
					&substrate_recipient,
					Some(&XcmContext::with_message_id([0; 32]))
				));
				// asset tmp holder for substrate world transfer
				let tmp_account = sp_io::hashing::blake2_256(
					&Location::new(0, X1(Arc::new([GeneralKey { length: 8, data: [2u8; 32] }])))
						.encode(),
				);
				assert_eq!(
					Balances::free_balance(sp_runtime::AccountId32::from(tmp_account)),
					10u128
				);

				// Register foreign asset (USDT) with asset id 1
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdtAssetId::get(), ASSET_OWNER, true, 1,));

				// send foreign asset to the outer world
				let foreign_asset: Asset = (AssetId(UsdtLocation::get()), Fungible(10u128)).into();
				assert_ok!(XCMAssetTransactor::<
					CurrencyTransactor,
					FungiblesTransactor,
					NativeAssetTypeIdentifier<ParachainInfo>,
					ForwarderImplRuntime,
				>::deposit_asset(
					&foreign_asset,
					&substrate_recipient,
					Some(&XcmContext::with_message_id([0; 32]))
				));
				// asset tmp holder for substrate world transfer
				let tmp_account = sp_io::hashing::blake2_256(
					&Location::new(0, X1(Arc::new([GeneralKey { length: 8, data: [2u8; 32] }])))
						.encode(),
				);
				assert_eq!(
					Assets::balance(UsdtAssetId::get(), sp_runtime::AccountId32::from(tmp_account)),
					10u128
				);
			})
		}

		#[test]
		fn test_xcm_asset_transactor_other_dest() {
			new_test_ext().execute_with(|| {
				let other_recipient: Location = Location::new(
					2,
					X3(Arc::new([
						Parachain(2005),
						slice_to_generalkey(b"substrate recipient"),
						AccountId32 { network: None, id: BOB.into() },
					])),
				);

				// send native asset to non-supported world
				let native_asset: Asset =
					(AssetId(Location::new(0, Here)), Fungible(10u128)).into();
				assert_ok!(XCMAssetTransactor::<
					CurrencyTransactor,
					FungiblesTransactor,
					NativeAssetTypeIdentifier<ParachainInfo>,
					ForwarderImplRuntime,
				>::deposit_asset(
					&native_asset,
					&other_recipient,
					Some(&XcmContext::with_message_id([0; 32]))
				));

				// asset tmp holder for substrate world and outer world transfer, should not receive any token
				let tmp_account_outer = sp_io::hashing::blake2_256(
					&Location::new(0, X1(Arc::new([GeneralKey { length: 8, data: [1u8; 32] }])))
						.encode(),
				);
				assert_eq!(
					Balances::free_balance(sp_runtime::AccountId32::from(tmp_account_outer)),
					0u128
				);
				let tmp_account_substrate = sp_io::hashing::blake2_256(
					&Location::new(0, X1(Arc::new([GeneralKey { length: 8, data: [2u8; 32] }])))
						.encode(),
				);
				assert_eq!(
					Balances::free_balance(sp_runtime::AccountId32::from(tmp_account_substrate)),
					10u128
				);
			})
		}

		#[test]
		fn test_xcm_asset_transactor_withdraw() {
			new_test_ext().execute_with(|| {
				let from_account: Location = Location::new(
					0,
					X1(Arc::new([AccountId32 { network: None, id: ALICE.into() }])),
				);

				// withdraw native asset from Alice
				let native_asset: Asset =
					(AssetId(Location::new(0, Here)), Fungible(10u128)).into();
				assert_ok!(XCMAssetTransactor::<
					CurrencyTransactor,
					FungiblesTransactor,
					NativeAssetTypeIdentifier<ParachainInfo>,
					ForwarderImplRuntime,
				>::withdraw_asset(&native_asset, &from_account, None));
				assert_eq!(Balances::free_balance(ALICE), ENDOWED_BALANCE - 10u128);

				// Register foreign asset (USDT) with asset id 1
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdtAssetId::get(), ASSET_OWNER, true, 1,));
				// Mint some USDT to ALICE for test
				assert_ok!(Assets::mint(
					RuntimeOrigin::signed(ASSET_OWNER),
					codec::Compact(1),
					ALICE,
					ENDOWED_BALANCE,
				));
				assert_eq!(Assets::balance(UsdtAssetId::get(), &ALICE), ENDOWED_BALANCE);

				// withdraw foreign asset from Alice
				let foreign_asset: Asset = (AssetId(UsdtLocation::get()), Fungible(10u128)).into();
				assert_ok!(XCMAssetTransactor::<
					CurrencyTransactor,
					FungiblesTransactor,
					NativeAssetTypeIdentifier<ParachainInfo>,
					ForwarderImplRuntime,
				>::withdraw_asset(&foreign_asset, &from_account, None));
				assert_eq!(Assets::balance(UsdtAssetId::get(), &ALICE), ENDOWED_BALANCE - 10u128);
			})
		}
	}
}
