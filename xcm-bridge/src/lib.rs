// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

#[cfg(test)]
mod mock;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion};
	use sp_runtime::traits::Zero;
	use sp_std::sync::Arc;
	use sp_std::{prelude::*, vec};
	use xcm::v4::{prelude::*, Junctions::X1, Location, Weight as XCMWeight};
	use xcm_executor::traits::WeightBounds;

	use sygma_traits::{AssetReserveLocationParser, AssetTypeIdentifier, Bridge};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Weigher: WeightBounds<Self::RuntimeCall>;

		type XcmExecutor: ExecuteXcm<Self::RuntimeCall>;

		type AssetReservedChecker: AssetTypeIdentifier;

		type UniversalLocation: Get<InteriorLocation>;

		#[pallet::constant]
		type SelfLocation: Get<Location>;

		/// Minimum xcm execution fee paid on destination chain.
		type MinXcmFee: Get<Vec<(AssetId, u128)>>;
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum TransferKind {
		/// Transfer self reserve asset. assets reserved by the origin chain
		SelfReserveAsset,
		/// To reserve location. assets reserved by the dest chain
		ToReserve,
		/// To non-reserve location. assets not reserved by the dest chain
		ToNonReserve,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		XCMTransferSend { asset: Box<Asset>, origin: Box<Location>, dest: Box<Location> },
	}

	#[pallet::error]
	pub enum Error<T> {
		FailToWeightMessage,
		XcmExecutionFailed,
		InvalidDestination,
		UnknownTransferType,
		CannotReanchor,
		NoXcmMinFeeSet,
		AssetReservedLocationNotFound,
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	struct XcmObject<T: Config> {
		asset: Asset,
		fee: Asset,
		origin: Location,
		dest: Location,
		recipient: Location,
		weight: XCMWeight,
		_unused: PhantomData<T>,
	}

	pub trait XcmHandler<T: Config> {
		fn transfer_kind(&self, asset_reserved_location: Location) -> Option<TransferKind>;
		fn create_instructions(&self) -> Result<Xcm<T::RuntimeCall>, DispatchError>;
		fn execute_instructions(
			&self,
			xcm_instructions: &mut Xcm<T::RuntimeCall>,
		) -> DispatchResult;
	}

	impl<T: Config> XcmHandler<T> for XcmObject<T> {
		fn transfer_kind(&self, asset_reserved_location: Location) -> Option<TransferKind> {
			if T::AssetReservedChecker::is_native_asset(&self.asset.clone()) {
				Some(TransferKind::SelfReserveAsset)
			} else if asset_reserved_location == self.dest {
				Some(TransferKind::ToReserve)
			} else {
				Some(TransferKind::ToNonReserve)
			}
		}

		fn create_instructions(&self) -> Result<Xcm<T::RuntimeCall>, DispatchError> {
			let asset_reserved_location = Pallet::<T>::reserved_location(&self.asset.clone())
				.ok_or(Error::<T>::AssetReservedLocationNotFound)?;
			let kind = Self::transfer_kind(self, asset_reserved_location.clone())
				.ok_or(Error::<T>::UnknownTransferType)?;

			let mut assets = Assets::new();
			assets.push(self.asset.clone());

			let xcm_instructions = match kind {
				TransferKind::SelfReserveAsset => Pallet::<T>::transfer_self_reserve_asset(
					assets,
					self.fee.clone(),
					self.clone().dest,
					self.clone().recipient,
					WeightLimit::Limited(self.weight),
				)?,
				TransferKind::ToReserve => Pallet::<T>::transfer_to_reserve_asset(
					assets,
					self.clone().fee.clone(),
					self.clone().dest,
					self.clone().recipient,
					WeightLimit::Limited(self.weight),
				)?,
				TransferKind::ToNonReserve => Pallet::<T>::transfer_to_non_reserve_asset(
					assets,
					self.clone().fee.clone(),
					asset_reserved_location,
					self.clone().dest,
					self.clone().recipient,
					WeightLimit::Limited(self.weight),
				)?,
			};

			Ok(xcm_instructions)
		}

		fn execute_instructions(
			&self,
			xcm_instructions: &mut Xcm<T::RuntimeCall>,
		) -> DispatchResult {
			let message_weight = T::Weigher::weight(xcm_instructions)
				.map_err(|()| Error::<T>::FailToWeightMessage)?;

			let mut hash: XcmHash = xcm_instructions.using_encoded(sp_io::hashing::blake2_256);

			T::XcmExecutor::prepare_and_execute(
				self.clone().origin,
				xcm_instructions.clone(),
				&mut hash,
				self.weight,
				message_weight,
			)
			.ensure_complete()
			.map_err(|_| Error::<T>::XcmExecutionFailed)?;

			Ok(())
		}
	}

	impl<T: Config> AssetReserveLocationParser for Pallet<T> {
		fn reserved_location(asset: &Asset) -> Option<Location> {
			let location = match (&asset.id, &asset.fun) {
				(id, Fungible(_)) => Some(&id.0),
				_ => None,
			};

			location.and_then(|id| {
				match (id.parents, id.first_interior()) {
					// Sibling parachain
					(1, Some(Parachain(id))) => {
						Some(Location::new(1, X1(Arc::new([Parachain(*id)]))))
					},
					// Parent
					(1, _) => Some(Location::parent()),
					// Children parachain
					(0, Some(Parachain(id))) => {
						Some(Location::new(0, X1(Arc::new([Parachain(*id)]))))
					},
					// Local: (0, Here)
					(0, None) => Some(id.clone()),
					_ => None,
				}
			})
		}
	}

	pub struct BridgeImpl<T>(PhantomData<T>);

	impl<T: Config> Bridge for BridgeImpl<T> {
		fn transfer(
			sender: [u8; 32],
			asset: Asset,
			dest: Location,
			max_weight: Option<XCMWeight>,
		) -> DispatchResult {
			let origin_location: Location =
				Junction::AccountId32 { network: None, id: sender }.into();

			let (dest_location, recipient) =
				Pallet::<T>::extract_dest(&dest).ok_or(Error::<T>::InvalidDestination)?;

			ensure!(
				T::MinXcmFee::get()
					.iter()
					.position(|a| a.0 == asset.id)
					.map(|idx| { T::MinXcmFee::get()[idx].1 })
					.is_some(),
				Error::<T>::NoXcmMinFeeSet
			);
			let fee_per_asset = T::MinXcmFee::get()
				.iter()
				.position(|a| a.0 == asset.id)
				.map(|idx| T::MinXcmFee::get()[idx].1)
				.unwrap();
			let min_fee_to_dest: Asset = (asset.clone().id, fee_per_asset).into();

			let xcm = XcmObject::<T> {
				asset: asset.clone(),
				fee: min_fee_to_dest,
				origin: origin_location.clone(),
				dest: dest_location,
				recipient,
				weight: max_weight.unwrap_or(XCMWeight::from_parts(6_000_000_000u64, 2_000_000u64)),
				_unused: PhantomData,
			};

			let mut msg = xcm.create_instructions()?;
			xcm.execute_instructions(&mut msg)?;

			Pallet::<T>::deposit_event(Event::XCMTransferSend {
				asset: Box::new(asset),
				origin: Box::new(origin_location),
				dest: Box::new(dest),
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// extract the dest_location, recipient_location
		pub fn extract_dest(dest: &Location) -> Option<(Location, Location)> {
			match (dest.parents, dest.first_interior()) {
				(1, Some(Parachain(id))) => Some((
					Location::new(1, X1(Arc::new([Parachain(*id)]))),
					Location::new(0, dest.interior().clone().split_first().0),
				)),
				// parent: relay chain
				(1, _) => Some((Location::parent(), Location::new(0, dest.interior().clone()))),
				// local and children parachain have been filtered out in the TransactAsset
				_ => None,
			}
		}
		fn transfer_self_reserve_asset(
			assets: Assets,
			fee: Asset,
			dest: Location,
			recipient: Location,
			dest_weight_limit: WeightLimit,
		) -> Result<Xcm<T::RuntimeCall>, DispatchError> {
			Ok(Xcm(vec![TransferReserveAsset {
				assets: assets.clone(),
				dest: dest.clone(),
				xcm: Xcm(vec![
					Self::buy_execution(fee, &dest, dest_weight_limit)?,
					Self::deposit_asset(recipient, assets.len() as u32),
				]),
			}]))
		}

		fn transfer_to_reserve_asset(
			assets: Assets,
			fee: Asset,
			reserve: Location,
			recipient: Location,
			dest_weight_limit: WeightLimit,
		) -> Result<Xcm<T::RuntimeCall>, DispatchError> {
			Ok(Xcm(vec![
				WithdrawAsset(assets.clone()),
				InitiateReserveWithdraw {
					assets: All.into(),
					reserve: reserve.clone(),
					xcm: Xcm(vec![
						Self::buy_execution(fee, &reserve, dest_weight_limit)?,
						Self::deposit_asset(recipient, assets.len() as u32),
					]),
				},
			]))
		}

		fn transfer_to_non_reserve_asset(
			assets: Assets,
			fee: Asset,
			reserve: Location,
			dest: Location,
			recipient: Location,
			dest_weight_limit: WeightLimit,
		) -> Result<Xcm<T::RuntimeCall>, DispatchError> {
			let mut reanchored_dest = dest.clone();
			if reserve == Location::parent() {
				if let (1, [Parachain(id)]) = dest.unpack() {
					reanchored_dest = Parachain(*id).into();
				}
			}

			let max_assets = assets.len() as u32;

			Ok(Xcm(vec![
				WithdrawAsset(assets),
				InitiateReserveWithdraw {
					assets: All.into(),
					reserve: reserve.clone(),
					xcm: Xcm(vec![
						Self::buy_execution(Self::half(&fee), &reserve, dest_weight_limit.clone())?,
						DepositReserveAsset {
							assets: AllCounted(max_assets).into(),
							dest: reanchored_dest,
							xcm: Xcm(vec![
								Self::buy_execution(Self::half(&fee), &dest, dest_weight_limit)?,
								Self::deposit_asset(recipient, max_assets),
							]),
						},
					]),
				},
			]))
		}

		fn deposit_asset(recipient: Location, max_assets: u32) -> Instruction<()> {
			DepositAsset { assets: AllCounted(max_assets).into(), beneficiary: recipient }
		}

		fn buy_execution(
			asset: Asset,
			at: &Location,
			weight_limit: WeightLimit,
		) -> Result<Instruction<()>, DispatchError> {
			let ancestry = T::SelfLocation::get();

			let fees = asset
				.reanchored(at, &ancestry.interior)
				.map_err(|_| Error::<T>::CannotReanchor)?;

			Ok(BuyExecution { fees, weight_limit })
		}

		/// Returns amount if `asset` is fungible, or zero.
		fn fungible_amount(asset: &Asset) -> u128 {
			if let Fungible(amount) = &asset.fun {
				*amount
			} else {
				Zero::zero()
			}
		}

		fn half(asset: &Asset) -> Asset {
			let half_amount = Self::fungible_amount(&asset.clone())
				.checked_div(2)
				.expect("div 2 can't overflow; qed");
			Asset { fun: Fungible(half_amount), id: asset.clone().id }
		}
	}

	#[cfg(test)]
	mod test {
		use super::*;
		use crate::mock::para::{
			assert_events, Assets, NativeAssetId, PBALocation, Runtime, RuntimeEvent,
			RuntimeOrigin, UsdtAssetId, UsdtLocation,
		};
		use crate::mock::{
			ParaA, ParaAssets, ParaB, ParaBalances, ParaC, TestNet, ALICE, ASSET_OWNER, BOB,
			ENDOWED_BALANCE,
		};
		use crate::Event as SygmaXcmBridgeEvent;
		use frame_support::{
			assert_ok, traits::tokens::fungibles::metadata::Mutate as MetaMutate,
			traits::tokens::fungibles::Create as FungibleCerate,
		};
		use polkadot_parachain_primitives::primitives::Sibling;
		use sp_runtime::traits::AccountIdConversion;
		use sp_runtime::AccountId32;
		use sp_std::{boxed::Box, vec};
		use xcm::v4::Junctions::X2;
		use xcm_simulator::TestExt;

		fn init_logger() {
			let _ = env_logger::builder()
				// Include all events in tests
				.filter_level(log::LevelFilter::max())
				// Ensure events are captured by `cargo test`
				.is_test(true)
				// Ignore errors initializing the logger if tests race to configure it
				.try_init();
		}

		fn sibling_account(para_id: u32) -> AccountId32 {
			Sibling::from(para_id).into_account_truncating()
		}

		#[test]
		fn test_transfer_self_reserve_asset_to_parachain() {
			init_logger();

			TestNet::reset();

			// sending 10 tokens
			let amount = 10_000_000_000_000u128;
			let fee = 4u128;

			ParaB::execute_with(|| {
				// ParaB register the native asset of paraA
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(NativeAssetId::get(), ASSET_OWNER, true, 1,));

				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as MetaMutate<
					<Runtime as frame_system::Config>::AccountId,
				>>::set(
					NativeAssetId::get(),
					&ASSET_OWNER,
					b"ParaAAsset".to_vec(),
					b"PAA".to_vec(),
					12,
				));

				// make sure Bob on parachain B holds none of NativeAsset of paraA
				assert_eq!(ParaAssets::balance(NativeAssetId::get(), &BOB), 0u128);
			});

			// sending native asset from parachain A to parachain B
			ParaA::execute_with(|| {
				assert_eq!(ParaBalances::free_balance(&ALICE), ENDOWED_BALANCE);

				// transfer parachain A native asset from Alice to parachain B on Bob
				assert_ok!(BridgeImpl::<Runtime>::transfer(
					ALICE.into(),
					(AssetId(Location::new(0, Here)), Fungible(amount)).into(),
					Location::new(
						1,
						X2(Arc::new([
							Parachain(2u32),
							Junction::AccountId32 { network: None, id: BOB.into() }
						])),
					),
					None
				));

				// Alice should lost the amount of native asset of paraA
				assert_eq!(ParaBalances::free_balance(&ALICE), ENDOWED_BALANCE - amount);

				assert_events(vec![RuntimeEvent::SygmaXcmBridge(
					SygmaXcmBridgeEvent::XCMTransferSend {
						asset: Box::new((AssetId(Location::new(0, Here)), Fungible(amount)).into()),
						origin: Box::new(
							Junction::AccountId32 { network: None, id: ALICE.into() }.into(),
						),
						dest: Box::new(Location::new(
							1,
							X2(Arc::new([
								Parachain(2u32),
								Junction::AccountId32 { network: None, id: BOB.into() },
							])),
						)),
					},
				)]);

				// sibling_account of B on A should have amount of native asset as well
				assert_eq!(ParaBalances::free_balance(sibling_account(2)), amount);
			});

			ParaB::execute_with(|| {
				// Bob should get amount - fee of the native asset of paraA on paraB
				assert_eq!(ParaAssets::balance(NativeAssetId::get(), &BOB), amount - fee);
			});
		}

		#[test]
		fn test_transfer_to_reserve_to_parachain() {
			init_logger();

			TestNet::reset();

			// sending 10 tokens
			let amount = 10_000_000_000_000u128;
			let fee = 4u128;

			// register PBA on paraA
			ParaA::execute_with(|| {
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(NativeAssetId::get(), ASSET_OWNER, true, 1,));

				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as MetaMutate<
					<Runtime as frame_system::Config>::AccountId,
				>>::set(
					NativeAssetId::get(),
					&ASSET_OWNER,
					b"ParaBAsset".to_vec(),
					b"PBA".to_vec(),
					12,
				));
			});

			// transfer PBA from Alice on parachain B to Alice on parachain A
			ParaB::execute_with(|| {
				// Bob now has ENDOWED_BALANCE of PBB on parachain B
				assert_eq!(ParaBalances::free_balance(&BOB), ENDOWED_BALANCE);

				assert_ok!(BridgeImpl::<Runtime>::transfer(
					ALICE.into(),
					(AssetId(Location::new(0, Here)), Fungible(amount)).into(),
					Location::new(
						1,
						X2(Arc::new([
							Parachain(1u32),
							Junction::AccountId32 { network: None, id: ALICE.into() }
						])),
					),
					None
				));
				assert_eq!(ParaBalances::free_balance(&ALICE), ENDOWED_BALANCE - amount);
				assert_eq!(ParaBalances::free_balance(sibling_account(1)), amount);
			});

			// transfer PBA back to parachain B
			ParaA::execute_with(|| {
				assert_eq!(ParaAssets::balance(NativeAssetId::get(), &ALICE), amount - fee);

				// transfer PBA back to Bob on parachain B with (amount - fee)
				assert_ok!(BridgeImpl::<Runtime>::transfer(
					ALICE.into(),
					(PBALocation::get(), Fungible(amount - fee)).into(),
					Location::new(
						1,
						X2(Arc::new([
							Parachain(2u32),
							Junction::AccountId32 { network: None, id: BOB.into() }
						]))
					),
					None
				));

				// now Alice holds 0 of PBA
				assert_eq!(ParaAssets::balance(NativeAssetId::get(), &ALICE), 0u128);

				assert_events(vec![RuntimeEvent::SygmaXcmBridge(
					SygmaXcmBridgeEvent::XCMTransferSend {
						asset: Box::new(
							(AssetId(PBALocation::get()), Fungible(amount - fee)).into(),
						),
						origin: Box::new(
							Junction::AccountId32 { network: None, id: ALICE.into() }.into(),
						),
						dest: Box::new(Location::new(
							1,
							X2(Arc::new([
								Parachain(2u32),
								Junction::AccountId32 { network: None, id: BOB.into() },
							])),
						)),
					},
				)]);
			});

			ParaB::execute_with(|| {
				// Bob should get amount - fee * 2 bcs there are two times of xcm transfer
				assert_eq!(ParaBalances::free_balance(&BOB), ENDOWED_BALANCE + amount - fee * 2);
				assert_eq!(ParaBalances::free_balance(sibling_account(1)), amount - (amount - fee));
			});
		}

		#[test]
		fn test_transfer_to_non_reserve_to_parachain() {
			init_logger();

			TestNet::reset();

			// register token on Parachain C
			ParaC::execute_with(|| {
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdtAssetId::get(), ASSET_OWNER, true, 1,));
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as MetaMutate<
					<Runtime as frame_system::Config>::AccountId,
				>>::set(
					UsdtAssetId::get(),
					&ASSET_OWNER,
					b"USDT".to_vec(),
					b"USDT".to_vec(),
					12,
				));

				// mint USDT to ASSET_OWNER
				assert_ok!(Assets::mint(
					RuntimeOrigin::signed(ASSET_OWNER),
					UsdtAssetId::get(),
					ASSET_OWNER,
					ENDOWED_BALANCE,
				));

				// checking USDT balances
				assert_eq!(ParaAssets::balance(UsdtAssetId::get(), &ASSET_OWNER), ENDOWED_BALANCE);
				assert_eq!(ParaAssets::balance(UsdtAssetId::get(), &ALICE), 0u128);
				assert_eq!(ParaAssets::balance(UsdtAssetId::get(), &BOB), 0u128);

				// checking native asset balances
				assert_eq!(ParaBalances::free_balance(&ALICE), ENDOWED_BALANCE);
				assert_eq!(ParaBalances::free_balance(&BOB), ENDOWED_BALANCE);

				// make sure the sibling_account of parachain A has enough native asset
				// this is used in WithdrawAsset xcm instruction in InitiateReserveWithdraw
				assert_ok!(ParaBalances::transfer_keep_alive(
					RuntimeOrigin::signed(ASSET_OWNER),
					Sibling::from(1u32).into_account_truncating(),
					1_000_000_000_000_000_u128
				));
				assert_eq!(
					ParaBalances::free_balance(sibling_account(1)),
					1_000_000_000_000_000_u128
				);

				// sibling_account of B has 0 balance at this moment
				assert_eq!(ParaBalances::free_balance(sibling_account(2)), 0u128);
			});

			// register token on Parachain A
			ParaA::execute_with(|| {
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdtAssetId::get(), ASSET_OWNER, true, 1,));
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as MetaMutate<
					<Runtime as frame_system::Config>::AccountId,
				>>::set(
					UsdtAssetId::get(),
					&ASSET_OWNER,
					b"USDT".to_vec(),
					b"USDT".to_vec(),
					12,
				));
			});

			// transfer some USDT from C to Alice on A
			ParaC::execute_with(|| {
				assert_ok!(BridgeImpl::<Runtime>::transfer(
					ASSET_OWNER.into(),
					(AssetId(UsdtLocation::get()), Fungible(100_000_000u128)).into(),
					Location::new(
						1,
						X2(Arc::new([
							Parachain(1u32),
							Junction::AccountId32 { network: None, id: ALICE.into() }
						])),
					),
					None
				));
				assert_eq!(
					ParaAssets::balance(UsdtAssetId::get(), &ASSET_OWNER),
					ENDOWED_BALANCE - 100_000_000u128
				);
			});

			// Alice should get the USDT token - fee
			ParaA::execute_with(|| {
				assert_eq!(
					ParaAssets::balance(UsdtAssetId::get(), &ALICE),
					100_000_000u128 - 4u128
				);
			});

			// Parachain B register USDT token
			ParaB::execute_with(|| {
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdtAssetId::get(), ASSET_OWNER, true, 1,));
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as MetaMutate<
					<Runtime as frame_system::Config>::AccountId,
				>>::set(
					UsdtAssetId::get(),
					&ASSET_OWNER,
					b"USDT".to_vec(),
					b"USDT".to_vec(),
					12,
				));

				// Bob on parachain B has 0 USDT at this moment
				assert_eq!(ParaAssets::balance(UsdtAssetId::get(), &BOB), 0u128);
			});

			// send USDT token from parachainA to parachainB
			ParaA::execute_with(|| {
				// Alice transfer USDT token from parachain A to Bob on parachain B
				assert_ok!(BridgeImpl::<Runtime>::transfer(
					ALICE.into(),
					(AssetId(UsdtLocation::get()), Fungible(100_000_000u128 - 4u128)).into(),
					Location::new(
						1,
						X2(Arc::new([
							Parachain(2u32),
							Junction::AccountId32 { network: None, id: BOB.into() }
						])),
					),
					None
				));
				// Alice has 0 USDT now
				assert_eq!(ParaAssets::balance(UsdtAssetId::get(), &ALICE), 0u128);

				assert_events(vec![RuntimeEvent::SygmaXcmBridge(
					SygmaXcmBridgeEvent::XCMTransferSend {
						asset: Box::new(
							(AssetId(UsdtLocation::get()), Fungible(100_000_000u128 - 4u128))
								.into(),
						),
						origin: Box::new(
							Junction::AccountId32 { network: None, id: ALICE.into() }.into(),
						),
						dest: Box::new(Location::new(
							1,
							X2(Arc::new([
								Parachain(2u32),
								Junction::AccountId32 { network: None, id: BOB.into() },
							])),
						)),
					},
				)]);
			});

			ParaC::execute_with(|| {
				// on C, the sibling_account of parachain A will be withdrawn the same amount of Parachain C native assets
				assert_eq!(
					ParaBalances::free_balance(sibling_account(1)),
					1_000_000_000_000_000_u128 - (100_000_000u128 - 4u128)
				);

				// on C, the sibling_account of parachain B will be deposited the same amount of Parachain C native assets - xcm fee
				assert_eq!(
					ParaBalances::free_balance(sibling_account(2)),
					(100_000_000u128 - 4u128) - 4u128
				);
			});

			// Bob on Parachain B has USDT token now
			ParaB::execute_with(|| {
				// transferred amount from parachain is (100_000_000u128 - 4u128) minus the xcm fee twice on the reserved chain and the dest chain
				assert_eq!(
					ParaAssets::balance(UsdtAssetId::get(), &BOB),
					100_000_000u128 - 4u128 - 4u128 * 2
				);
			});
		}
	}
}
