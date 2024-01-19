// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

#[cfg(test)]
mod mock;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResult,
        pallet_prelude::*,
        traits::StorageVersion,
    };
    use sp_runtime::traits::Zero;
    use sp_std::{prelude::*, vec};
    use xcm::latest::{MultiLocation, prelude::*, Weight as XCMWeight};
    use xcm_executor::traits::WeightBounds;

    use sygma_traits::{AssetReserveLocationParser, Bridge};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type Weigher: WeightBounds<Self::RuntimeCall>;

        type XcmExecutor: ExecuteXcm<Self::RuntimeCall>;

        type UniversalLocation: Get<InteriorMultiLocation>;

        #[pallet::constant]
        type SelfLocation: Get<MultiLocation>;

        /// Minimum xcm execution fee paid on destination chain.
        type MinXcmFee: Get<Vec<(AssetId, u128)>>;
    }

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
        XCMTransferSend {
            asset: MultiAsset,
            origin: MultiLocation,
            dest: MultiLocation,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        FailToWeightMessage,
        XcmExecutionFailed,
        InvalidDestination,
        UnknownTransferType,
        CannotReanchor,
        NoXcmMiNFeeSet,
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode)]
    struct XcmObject<T: Config> {
        asset: MultiAsset,
        fee: MultiAsset,
        origin: MultiLocation,
        dest: MultiLocation,
        recipient: MultiLocation,
        weight: XCMWeight,
        _unused: PhantomData<T>,
    }

    pub trait XcmHandler<T: Config> {
        fn transfer_kind(&self) -> Option<TransferKind>;
        fn create_instructions(&self) -> Result<Xcm<T::RuntimeCall>, DispatchError>;
        fn execute_instructions(&self, xcm_instructions: &mut Xcm<T::RuntimeCall>) -> DispatchResult;
    }

    impl<T: Config> XcmHandler<T> for XcmObject<T> {
        fn transfer_kind(&self) -> Option<TransferKind> {
            let asset_location = Pallet::<T>::reserved_location(&self.asset.clone())?;
            if asset_location == T::SelfLocation::get() {
                Some(TransferKind::SelfReserveAsset)
            } else if asset_location == self.dest {
                Some(TransferKind::ToReserve)
            } else {
                Some(TransferKind::ToNonReserve)
            }
        }

        fn create_instructions(&self) -> Result<Xcm<T::RuntimeCall>, DispatchError> {
            let kind = Self::transfer_kind(self).ok_or(Error::<T>::UnknownTransferType)?;

            let mut assets = MultiAssets::new();
            assets.push(self.asset.clone());

            let xcm_instructions = match kind {
                TransferKind::SelfReserveAsset => Pallet::<T>::transfer_self_reserve_asset(assets, self.fee.clone(), self.dest, self.recipient, WeightLimit::Limited(self.weight))?,
                TransferKind::ToReserve => Pallet::<T>::transfer_to_reserve_asset(assets, self.fee.clone(), self.dest, self.recipient, WeightLimit::Limited(self.weight))?,
                TransferKind::ToNonReserve => Pallet::<T>::transfer_to_non_reserve_asset(
                    assets,
                    self.fee.clone(),
                    self.dest,
                    self.dest.clone(),
                    self.recipient,
                    WeightLimit::Limited(self.weight),
                )?,
            };

            Ok(xcm_instructions)
        }

        fn execute_instructions(&self, xcm_instructions: &mut Xcm<T::RuntimeCall>) -> DispatchResult {
            let message_weight = T::Weigher::weight(xcm_instructions).map_err(|()| Error::<T>::FailToWeightMessage)?;

            let hash = xcm_instructions.using_encoded(sp_io::hashing::blake2_256);

            T::XcmExecutor::execute_xcm_in_credit(
                self.origin.clone(),
                xcm_instructions.clone(),
                hash,
                message_weight,
                message_weight,
            ).ensure_complete().map_err(|_| Error::<T>::XcmExecutionFailed)?;

            Ok(())
        }
    }


    impl<T: Config> AssetReserveLocationParser for Pallet<T> {
        fn reserved_location(asset: &MultiAsset) -> Option<MultiLocation> {
            match (&asset.id, &asset.fun) {
                (Concrete(id), Fungible(_)) => Some(id.clone()),
                _ => None,
            }
        }
    }

    pub struct BridgeImpl<T>(PhantomData<T>);
    impl<T: Config> Bridge for BridgeImpl<T> {
        fn transfer(sender: [u8; 32],
                    asset: MultiAsset,
                    dest: MultiLocation) -> DispatchResult {
            let origin_location: MultiLocation = Junction::AccountId32 {
                network: None,
                id: sender,
            }.into();

            let (dest_location, recipient) =
                Pallet::<T>::extract_dest(&dest).ok_or(Error::<T>::InvalidDestination)?;

            ensure!(
				T::MinXcmFee::get().iter().position(|a| a.0 == asset.id).map(|idx| {
                T::MinXcmFee::get()[idx].1
            }).is_some(),
				Error::<T>::NoXcmMiNFeeSet
			);
            let fee_per_asset = T::MinXcmFee::get().iter().position(|a| a.0 == asset.id).map(|idx| {
                T::MinXcmFee::get()[idx].1
            }).unwrap();

            let fee_to_dest: MultiAsset = (asset.id, fee_per_asset).into();

            let xcm = XcmObject::<T> {
                asset: asset.clone(),
                fee: fee_to_dest,
                origin: origin_location.clone(),
                dest: dest_location,
                recipient,
                weight: XCMWeight::from_parts(6_000_000_000u64, 2_000_000u64),
                _unused: PhantomData,
            };

            let mut msg = xcm.create_instructions()?;
            xcm.execute_instructions(&mut msg)?;

            Pallet::<T>::deposit_event(Event::XCMTransferSend {
                asset,
                origin: origin_location,
                dest,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// extract the dest_location, recipient_location
        pub fn extract_dest(dest: &MultiLocation) -> Option<(MultiLocation, MultiLocation)> {
            match (dest.parents, dest.first_interior()) {
                (1, Some(Parachain(id))) => Some((
                    MultiLocation::new(1, X1(Parachain(*id))),
                    MultiLocation::new(0, dest.interior().clone().split_first().0),
                )),
                // parent: relay chain
                (1, _) => Some((
                    MultiLocation::parent(),
                    MultiLocation::new(0, dest.interior().clone()),
                )),
                // local and children parachain have been filtered out in the TransactAsset
                _ => None,
            }
        }
        fn transfer_self_reserve_asset(
            assets: MultiAssets,
            fee: MultiAsset,
            dest: MultiLocation,
            recipient: MultiLocation,
            dest_weight_limit: WeightLimit,
        ) -> Result<Xcm<T::RuntimeCall>, DispatchError> {
            Ok(Xcm(vec![TransferReserveAsset {
                assets: assets.clone(),
                dest,
                xcm: Xcm(vec![
                    Self::buy_execution(fee, &dest, dest_weight_limit)?,
                    Self::deposit_asset(recipient, assets.len() as u32),
                ]),
            }]))
        }

        fn transfer_to_reserve_asset(
            assets: MultiAssets,
            fee: MultiAsset,
            reserve: MultiLocation,
            recipient: MultiLocation,
            dest_weight_limit: WeightLimit,
        ) -> Result<Xcm<T::RuntimeCall>, DispatchError> {
            Ok(Xcm(vec![
                WithdrawAsset(assets.clone()),
                InitiateReserveWithdraw {
                    assets: All.into(),
                    reserve,
                    xcm: Xcm(vec![
                        Self::buy_execution(fee, &reserve, dest_weight_limit)?,
                        Self::deposit_asset(recipient, assets.len() as u32),
                    ]),
                },
            ]))
        }

        fn transfer_to_non_reserve_asset(
            assets: MultiAssets,
            fee: MultiAsset,
            reserve: MultiLocation,
            dest: MultiLocation,
            recipient: MultiLocation,
            dest_weight_limit: WeightLimit,
        ) -> Result<Xcm<T::RuntimeCall>, DispatchError> {
            let mut reanchored_dest = dest;
            if reserve == MultiLocation::parent() {
                if let MultiLocation {
                    parents: 1,
                    interior: X1(Parachain(id)),
                } = dest
                {
                    reanchored_dest = Parachain(id).into();
                }
            }

            let max_assets = assets.len() as u32;

            Ok(Xcm(vec![
                WithdrawAsset(assets),
                InitiateReserveWithdraw {
                    assets: All.into(),
                    reserve,
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

        fn deposit_asset(recipient: MultiLocation, max_assets: u32) -> Instruction<()> {
            DepositAsset {
                assets: AllCounted(max_assets).into(),
                beneficiary: recipient,
            }
        }

        fn buy_execution(
            asset: MultiAsset,
            at: &MultiLocation,
            weight_limit: WeightLimit,
        ) -> Result<Instruction<()>, DispatchError> {
            let ancestry = T::SelfLocation::get();

            let fees = asset.reanchored(at, ancestry.interior).map_err(|_| Error::<T>::CannotReanchor)?;

            Ok(BuyExecution { fees, weight_limit })
        }

        /// Returns amount if `asset` is fungible, or zero.
        fn fungible_amount(asset: &MultiAsset) -> u128 {
            if let Fungible(amount) = &asset.fun {
                *amount
            } else {
                Zero::zero()
            }
        }

        fn half(asset: &MultiAsset) -> MultiAsset {
            let half_amount = Self::fungible_amount(asset)
                .checked_div(2)
                .expect("div 2 can't overflow; qed");
            MultiAsset {
                fun: Fungible(half_amount),
                id: asset.id,
            }
        }
    }

    #[cfg(test)]
    mod test {
        use frame_support::assert_ok;
        use xcm_simulator::TestExt;

        use crate::Event as SygmaXcmBridgeEvent;
        use crate::mock::{
            ALICE, BOB, ENDOWED_BALANCE,
            ParaA, ParaAssets, ParaB, ParaBalances, TestNet,
        };
        use crate::mock::para::{assert_events, Runtime, RuntimeEvent, UsdtAssetId, UsdtLocation};

        use super::*;

        #[test]
        fn test_transfer_self_reserve_asset_to_parachain() {
            TestNet::reset();

            // sending native asset from parachain A to parachain B
            ParaA::execute_with(|| {
                assert_eq!(ParaBalances::free_balance(&ALICE), ENDOWED_BALANCE);

                // transfer parachain A native asset from Alice to parachain B on Bob
                assert_ok!(BridgeImpl::<Runtime>::transfer(ALICE.into(),
                              (Concrete(MultiLocation::new(0, Here)), Fungible(10_000_000_000_000u128)).into(),
                              MultiLocation::new(
                                  1,
                                  X2(
                                      Parachain(2u32.into()),
                                      Junction::AccountId32 {
                                          network: None,
                                          id: BOB.into(),
                                      },
                                  ),
                              )
                ));
                assert_eq!(ParaBalances::free_balance(&ALICE), ENDOWED_BALANCE - 10_000_000_000_000u128);

                assert_events(vec![RuntimeEvent::SygmaXcmBridge(SygmaXcmBridgeEvent::XCMTransferSend {
                    asset: (Concrete(MultiLocation::new(0, Here)), Fungible(10_000_000_000_000u128)).into(),
                    origin: Junction::AccountId32 {
                        network: None,
                        id: ALICE.into(),
                    }.into(),
                    dest: MultiLocation::new(
                        1,
                        X2(
                            Parachain(2u32.into()),
                            Junction::AccountId32 {
                                network: None,
                                id: BOB.into(),
                            },
                        ),
                    ),
                })]);
            });

            ParaB::execute_with(|| {
                assert_eq!(ParaAssets::balance(1u32.into(), &ALICE), ENDOWED_BALANCE);
                assert_eq!(ParaAssets::balance(1u32.into(), &BOB), 9_000_000_000_000u128);
            });
        }

        #[test]
        fn test_transfer_to_reserve_to_parachain() {
            TestNet::reset();

            let para_a_location = MultiLocation {
                parents: 1,
                interior: X1(Parachain(1)),
            };

            // Prepare step
            // sending parachainB native asset to parachainA
            ParaB::execute_with(|| {
                assert_ok!(BridgeImpl::<Runtime>::transfer(
					ALICE.into(),
					(Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into(),
					MultiLocation::new(
						1,
						X2(
							Parachain(1u32.into()),
							Junction::AccountId32 {
								network: None,
								id: BOB.into()
							}
						)
					)
				));

                assert_eq!(ParaBalances::free_balance(&ALICE), ENDOWED_BALANCE - 10);

                assert_events(vec![RuntimeEvent::SygmaXcmBridge(SygmaXcmBridgeEvent::XCMTransferSend {
                    asset: (Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into(),
                    origin: Junction::AccountId32 {
                        network: None,
                        id: ALICE.into(),
                    }.into(),
                    dest: MultiLocation::new(
                        1,
                        X2(
                            Parachain(1u32.into()),
                            Junction::AccountId32 {
                                network: None,
                                id: BOB.into(),
                            },
                        ),
                    ),
                })]);
            });
            // Bob on parachainA should have parachainB's native asset
            ParaA::execute_with(|| {
                assert_eq!(ParaAssets::balance(0u32.into(), &BOB), 10);
            });

            // sending parachainB's native asset from parachainA back to parachainB
            ParaA::execute_with(|| {
                assert_ok!(BridgeImpl::<Runtime>::transfer(
					BOB.into(),
					(Concrete(para_a_location.clone()), Fungible(5u128)).into(), // sending 5 tokens
					MultiLocation::new(
						1,
						X2(
							Parachain(2u32.into()),
							Junction::AccountId32 {
								network: None,
								id: ALICE.into()
							}
						)
					)
				));

                assert_eq!(ParaAssets::balance(0u32.into(), &BOB), 10 - 5);

                assert_events(vec![RuntimeEvent::SygmaXcmBridge(SygmaXcmBridgeEvent::XCMTransferSend {
                    asset: (Concrete(para_a_location.clone()), Fungible(5u128)).into(),
                    origin: Junction::AccountId32 {
                        network: None,
                        id: BOB.into(),
                    }.into(),
                    dest: MultiLocation::new(
                        1,
                        X2(
                            Parachain(2u32.into()),
                            Junction::AccountId32 {
                                network: None,
                                id: ALICE.into(),
                            },
                        ),
                    ),
                })]);
            });
            ParaA::execute_with(|| {
                assert_eq!(ParaBalances::free_balance(&ALICE), ENDOWED_BALANCE - 10 + 5);
            });
        }

        #[test]
        fn test_transfer_to_non_reserve_to_parachain() {
            TestNet::reset();

            // send USDT token from parachainA to parachainB
            ParaA::execute_with(|| {
                assert_ok!(BridgeImpl::<Runtime>::transfer(ALICE.into(),
                              (Concrete(UsdtLocation::get().into()), Fungible(10u128)).into(),
                              MultiLocation::new(
                                  1,
                                  X2(
                                      Parachain(2u32.into()),
                                      Junction::AccountId32 {
                                          network: None,
                                          id: BOB.into(),
                                      },
                                  ),
                              )
            ));
                assert_eq!(ParaAssets::balance(UsdtAssetId::get().into(), &ALICE), ENDOWED_BALANCE - 10);

                assert_events(vec![RuntimeEvent::SygmaXcmBridge(SygmaXcmBridgeEvent::XCMTransferSend {
                    asset: (Concrete(UsdtLocation::get().into()), Fungible(10u128)).into(),
                    origin: Junction::AccountId32 {
                        network: None,
                        id: ALICE.into(),
                    }.into(),
                    dest: MultiLocation::new(
                        1,
                        X2(
                            Parachain(2u32.into()),
                            Junction::AccountId32 {
                                network: None,
                                id: BOB.into(),
                            },
                        ),
                    ),
                })]);
            });

            ParaB::execute_with(|| {
                assert_eq!(ParaAssets::balance(UsdtAssetId::get().into(), &BOB), 10);
            });
        }
    }
}

