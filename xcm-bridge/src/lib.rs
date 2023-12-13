#![cfg_attr(not(feature = "std"), no_std)]

mod mock;

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use sygma_traits::{Bridge, AssetReserveLocationParser};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Current pallet index defined in runtime
        type PalletIndex: Get<u8>;

        type Weigher: WeightBounds<Self::RuntimeCall>;

        type XcmExecutor: ExecuteXcm<Self::RuntimeCall>;

        #[pallet::constant]
        type SelfLocation: Get<MultiLocation>;

        type MinXcmFee: GetByKey<MultiLocation, Option<u128>>;

    }

    enum TransferKind {
        /// Transfer self reserve asset.
        SelfReserveAsset,
        /// To reserve location.
        ToReserve,
        /// To non-reserve location.
        ToNonReserve,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        XCMTransferSend {},
    }

    #[pallet::error]
    pub enum Error<T> {
        FailToWeightMessage,
        XcmExecutionFailed,
        MinXcmFeeNotDefined,
        InvalidDestination,
    }

    struct Xcm<T: Config> {
        asset: MultiAsset,
        fee: MultiAsset,
        origin: MultiLocation,
        dest: MultiLocation,
        recipient: MultiLocation,
        weight: XCMWeight,
    }

    pub trait XcmHandler {
        fn transfer_kind(&self) -> Result<TransferKind, DispatchError>;
        fn create_instructions(&self) -> Result<Xcm<T::RuntimeCall>, DispatchError>;
        fn execute_instructions(&self, xcm_message: Xcm<T::RuntimeCall>) -> DispatchResult;
    }

    impl XcmHandler for Xcm {
        /// Get the transfer kind.
        fn transfer_kind(&self) -> Result<TransferKind, DispatchError> {
            let asset_location = Pallet::<T>::reserved_location(&self.asset).ok_or()?;
            if asset_location == T::SelfLocation {
                TransferKind::SelfReserveAsset
            } else if asset_location == self.dest {
                TransferKind::ToReserve
            } else {
                TransferKind::ToNonReserve
            }
        }
        fn create_instructions(&self) -> Result<Xcm<T::RuntimeCall>, DispatchError> {
            let kind = Self::transfer_kind(self)?;

            let mut xcm_instructions = match kind {
                SelfReserveAsset => Self::transfer_self_reserve_asset(self.assets, self.fee, self.dest, self.recipient, self.weight)?,
                ToReserve => Self::transfer_to_reserve_asset(self.assets, self.fee, self.dest, self.recipient, self.weight)?,
                ToNonReserve => Self::transfer_to_non_reserve_asset(
                    self.assets,
                    self.fee,
                    self.dest,
                    self.dest.clone(),
                    self.recipient,
                    self.weight,
                )?,
            };

            Ok(xcm_instructions)
        }

        fn execute_instructions(&self, xcm_instructions: Xcm<T::RuntimeCall>) -> DispatchResult {
            let message_weight = T::Weigher::weight(xcm_instructions).map_err(|()| Error::<T>::FailToWeightMessage)?;

            let hash = xcm_instructions.using_encoded(sp_io::hashing::blake2_256);

            T::XcmExecutor::execute_xcm_in_credit(
                self.origin.clone(),
                xcm_instructions.clone(),
                hash,
                message_weight,
                message_weight,
            ).ensure_complete().map_err(|_| Error::<T>::XcmExecutionFailed)?;

            oK(())
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

    #[pallet::call]
    impl<T: Config> Bridge for Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(195_000_000, 0))]
        #[transactional]
        fn transfer(sender: [u8; 32],
                    asset: MultiAsset,
                    dest: MultiLocation) -> DispatchResult {
            let origin_location: MultiLocation = Junction::AccountId32 {
                network: None,
                id: sender,
            }.into();

            let (dest_location, recipient) =
                Pallet::<T>::extract_dest(&dest).ok_or(Error::<T>::InvalidDestination)?;

            let min_xcm_fee = T::MinXcmFee::get(&dest).ok_or(Error::<T>::MinXcmFeeNotDefined)?;

            let xcm = Xcm::<T> {
                asset: asset.clone(),
                fee: min_xcm_fee,
                origin: origin_location.clone(),
                dest_location,
                recipient,
                weight: XCMWeight::from_parts(6_000_000_000u64, 2_000_000u64),
            };
            let mut msg = xcm.create_instructions()?;

            xcm.execute_instructions(msg)?;

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn extract_dest(dest: &MultiLocation) -> Option<(MultiLocation, MultiLocation)> {
            match (dest.parents, dest.first_interior()) {
                // parents must be 1 here because only parents as 1 can be forwarded to xcm bridge logic
                // parachains
                (1, Some(Parachain(id))) => Some((
                    MultiLocation::new(1, X1(Parachain(*id))),
                    MultiLocation::new(0, dest.interior().clone().split_first().0),
                )),
                (1, _) => Some((
                    MultiLocation::parent(),
                    MultiLocation::new(0, dest.interior().clone()),
                )),
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
                        Self::buy_execution(half(&fee), &reserve, dest_weight_limit.clone())?,
                        DepositReserveAsset {
                            assets: AllCounted(max_assets).into(),
                            dest: reanchored_dest,
                            xcm: Xcm(vec![
                                Self::buy_execution(half(&fee), &dest, dest_weight_limit)?,
                                Self::deposit_asset(recipient, max_assets),
                            ]),
                        },
                    ]),
                },
            ]))
        }
    }
}
