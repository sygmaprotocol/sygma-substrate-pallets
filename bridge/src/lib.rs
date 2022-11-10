#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

pub use self::pallet::*;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{
		dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion, transactional,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_core::{hash::H256, U256};
	use sp_runtime::{traits::Clear, RuntimeDebug};
	use sp_std::{convert::From, vec, vec::Vec};
	use sygma_traits::{
		DepositNonce, DomainID, ExtractRecipient, FeeHandler, IsReserve, ResourceId,
	};
	use xcm::latest::{prelude::*, MultiLocation};
	use xcm_executor::traits::TransactAsset;

	#[allow(dead_code)]
	const LOG_TARGET: &str = "runtime::sygmabridge";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
	pub struct Proposal {
		origin_domain_id: DomainID,
		deposit_nonce: DepositNonce,
		resource_id: ResourceId,
		data: Vec<u8>,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Origin used to administer the pallet
		type BridgeCommitteeOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The identifier for this chain.
		/// This must be unique and must not collide with existing IDs within a set of bridged
		/// chains.
		#[pallet::constant]
		type DestDomainID: Get<DomainID>;

		/// Bridge transfer reserve account
		#[pallet::constant]
		type TransferReserveAccount: Get<Self::AccountId>;

		/// Fee reserve account
		#[pallet::constant]
		type FeeReserveAccount: Get<Self::AccountId>;

		/// Fee information getter
		type FeeHandler: FeeHandler;

		/// Implementation of withdraw and deposit an asset.
		type AssetTransactor: TransactAsset;

		/// AssetId and ResourceId pairs
		type ResourcePairs: Get<Vec<(AssetId, ResourceId)>>;

		/// Return if asset reserved on current chain
		type IsReserve: IsReserve;

		///  Extract recipient from given MultiLocation
		type ExtractRecipient: ExtractRecipient;
	}

	#[allow(dead_code)]
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// When initial bridge transfer send to dest domain
		/// args: [dest_domain_id, resource_id, deposit_nonce, sender, deposit_data,
		/// handler_reponse]
		Deposit {
			dest_domain_id: DomainID,
			resource_id: ResourceId,
			deposit_nonce: DepositNonce,
			sender: T::AccountId,
			deposit_data: Vec<u8>,
			handler_repoonse: Vec<u8>,
		},
		/// When user is going to retry a bridge transfer
		/// args: [tx_hash]
		Retry { hash: H256 },
		/// When bridge is paused
		/// args: [dest_domain_id]
		BridgePaused { dest_domain_id: DomainID },
		/// When bridge is unpaused
		/// args: [dest_domain_id]
		BridgeUnpaused { dest_domain_id: DomainID },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Protected operation, must be performed by relayer
		BadMpcSignature,
		/// Insufficient balance on sender account
		InsufficientBalance,
		/// Failed to extract EVM receipient address according to given recipient parser
		ExtractRecipientFailed,
		/// Asset transactor execution failed
		TransactFailed,
		/// The withdrawn amount can not cover the fee payment
		FeeTooExpensive,
		/// MPC key not set
		MissingMpcKey,
		/// MPC key can not be updated
		MpcKeyNotUpdatable,
		/// Bridge is paused
		BridgePaused,
		/// Bridge is unpaused
		BridgeUnpaused,
		/// Fee config option missing
		MissingFeeConfig,
		/// Asset not bound to a resource id
		AssetNotBound,
		/// Proposal has either failed or succeeded
		ProposalAlreadyComplete,
		/// Transactor operation failed
		TransactorFailed,
		/// Function unimplemented
		Unimplemented,
	}

	/// Deposit counter of dest domain
	#[pallet::storage]
	#[pallet::getter(fn dest_counts)]
	pub type DepositCounts<T> = StorageValue<_, DepositNonce, ValueQuery>;

	/// Bridge Pause indicator
	/// Bridge is unpaused initially, until pause
	/// After MPC key setup, bridge should be paused until ready to unpause
	#[pallet::storage]
	#[pallet::getter(fn is_paused)]
	pub type IsPaused<T> = StorageValue<_, bool, ValueQuery>;

	/// Pre-set MPC public key
	#[pallet::storage]
	#[pallet::getter(fn mpc_key)]
	pub type MpcKey<T> = StorageValue<_, [u8; 32], ValueQuery>;

	/// Mark whether a deposit nonce was used. Used to mark execution status of a proposal.
	#[pallet::storage]
	#[pallet::getter(fn mpc_keys)]
	pub type UsedNonces<T: Config> =
		StorageDoubleMap<_, Twox64Concat, DomainID, Twox64Concat, U256, U256>;

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
	{
		/// Pause bridge, this would lead to bridge transfer failure before it being unpaused.
		#[pallet::weight(195_000_000)]
		pub fn pause_bridge(origin: OriginFor<T>) -> DispatchResult {
			// Ensure bridge committee
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			// make sure MPC key is set up
			ensure!(!MpcKey::<T>::get().is_clear(), Error::<T>::MissingMpcKey);

			// Mark as paused
			IsPaused::<T>::set(true);

			// Emit BridgePause event
			Self::deposit_event(Event::BridgePaused { dest_domain_id: T::DestDomainID::get() });
			Ok(())
		}

		/// Unpause bridge.
		#[pallet::weight(195_000_000)]
		pub fn unpause_bridge(origin: OriginFor<T>) -> DispatchResult {
			// Ensure bridge committee
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			// make sure MPC key is set up
			ensure!(!MpcKey::<T>::get().is_clear(), Error::<T>::MissingMpcKey);

			// make sure the current status is paused
			ensure!(IsPaused::<T>::get(), Error::<T>::BridgeUnpaused);

			// Mark as unpaused
			IsPaused::<T>::set(false);

			// Emit BridgeUnpause event
			Self::deposit_event(Event::BridgeUnpaused { dest_domain_id: T::DestDomainID::get() });
			Ok(())
		}

		/// Mark an ECDSA public key as a MPC account.
		#[pallet::weight(195_000_000)]
		pub fn set_mpc_key(origin: OriginFor<T>, _key: [u8; 32]) -> DispatchResult {
			// Ensure bridge committee
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			// Cannot set MPC key is it's already set
			ensure!(MpcKey::<T>::get().is_clear(), Error::<T>::MpcKeyNotUpdatable);

			// Set MPC account public key
			MpcKey::<T>::set(_key);
			Ok(())
		}

		/// Initiates a transfer.
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn deposit(
			origin: OriginFor<T>,
			asset: MultiAsset,
			dest: MultiLocation,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(!MpcKey::<T>::get().is_clear(), Error::<T>::MissingMpcKey);
			ensure!(!IsPaused::<T>::get(), Error::<T>::BridgePaused);

			// Extract asset (MultiAsset) to get corresponding ResourceId and transfer amount
			let (resource_id, amount) =
				Self::extract_asset(&asset).ok_or(Error::<T>::AssetNotBound)?;
			// Extract dest (MultiLocation) to get corresponding Etheruem recipient address
			let recipient = T::ExtractRecipient::extract_recipient(&dest)
				.ok_or(Error::<T>::ExtractRecipientFailed)?;
			let fee = T::FeeHandler::get_fee(&asset.id).ok_or(Error::<T>::MissingFeeConfig)?;

			ensure!(amount > fee, Error::<T>::FeeTooExpensive);

			// Withdraw `amount` of asset from sender
			T::AssetTransactor::withdraw_asset(
				&asset,
				&Junction::AccountId32 { network: NetworkId::Any, id: sender.clone().into() }
					.into(),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

			// Deposit `fee` of asset to treasury account
			T::AssetTransactor::deposit_asset(
				&(asset.id.clone(), Fungible(fee)).into(),
				&Junction::AccountId32 {
					network: NetworkId::Any,
					id: T::FeeReserveAccount::get().into(),
				}
				.into(),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

			// Deposit `amount - fee` of asset to reserve account if asset is reserved in local
			// chain.
			if T::IsReserve::is_reserve(&asset.id) {
				T::AssetTransactor::deposit_asset(
					&(asset.id.clone(), Fungible(amount - fee)).into(),
					&Junction::AccountId32 {
						network: NetworkId::Any,
						id: T::TransferReserveAccount::get().into(),
					}
					.into(),
				)
				.map_err(|_| Error::<T>::TransactFailed)?;
			}

			// Bump deposit nonce
			let deposit_nonce = DepositCounts::<T>::get();
			DepositCounts::<T>::put(deposit_nonce + 1);

			// Emit Deposit event
			Self::deposit_event(Event::Deposit {
				dest_domain_id: T::DestDomainID::get(),
				resource_id,
				deposit_nonce,
				sender,
				deposit_data: Self::create_deposit_data(amount - fee, recipient),
				handler_repoonse: vec![],
			});

			Ok(())
		}

		/// This method is used to trigger the process for retrying failed deposits on the MPC side.
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn retry(_origin: OriginFor<T>, hash: H256) -> DispatchResult {
			// Emit retry event
			// For clippy happy
			Self::deposit_event(Event::<T>::Retry { hash });
			Err(Error::<T>::Unimplemented.into())
		}

		/// Executes a batch of deposit proposals (only if signature is signed by MPC).
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn execute_proposal(
			_origin: OriginFor<T>,
			_proposals: Vec<Proposal>,
			_signature: Vec<u8>,
		) -> DispatchResult {
			// Verify MPC signature

			// Parse proposal

			// Extract ResourceId from proposal data to get corresponding asset (MultiAsset)

			// Extract Receipt from proposal data to get corresponding location (MultiLocation)

			// Handle asset with Transactor

			// Update proposal status

			Err(Error::<T>::Unimplemented.into())
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
	{
		/// Verifies that proposal data is signed by MPC address.
		#[allow(dead_code)]
		fn verify(_proposals: Vec<Proposal>, _signature: Vec<u8>) -> bool {
			false
		}

		/// Extract asset id and transfer amount from `MultiAsset`, currently only fungible asset
		/// are supported.
		fn extract_asset(asset: &MultiAsset) -> Option<(ResourceId, u128)> {
			match (&asset.fun, &asset.id) {
				(Fungible(amount), _) => T::ResourcePairs::get()
					.iter()
					.position(|a| a.0 == asset.id)
					.map(|idx| (T::ResourcePairs::get()[idx].1, *amount)),
				_ => None,
			}
		}

		fn create_deposit_data(amount: u128, recipient: Vec<u8>) -> Vec<u8> {
			[
				&Self::hex_zero_padding_32(amount),
				&Self::hex_zero_padding_32(recipient.len() as u128),
				recipient.as_slice(),
			]
			.concat()
			.to_vec()
		}

		fn hex_zero_padding_32(i: u128) -> [u8; 32] {
			let mut result = [0u8; 32];
			U256::from(i).to_little_endian(&mut result);
			result
		}
	}

	#[cfg(test)]
	mod test {
		use crate as bridge;
		use crate::{Event as SygmaBridgeEvent, IsPaused, MpcKey};
		use bridge::mock::{
			assert_events, new_test_ext, Assets, Balances, BridgeAccount, DestDomainID,
			PhaLocation, PhaResourceId, Runtime, RuntimeEvent, RuntimeOrigin as Origin,
			SygmaBasicFeeHandler, SygmaBridge, TreasuryAccount, UsdcLocation, UsdcResourceId,
			ALICE, ASSET_OWNER, ENDOWED_BALANCE,
		};
		use frame_support::{
			assert_noop, assert_ok, traits::tokens::fungibles::Create as FungibleCerate,
		};
		use sp_runtime::{traits::BadOrigin, WeakBoundedVec};
		use sp_std::convert::TryFrom;
		use xcm::latest::prelude::*;

		#[test]
		fn set_mpc_key() {
			new_test_ext().execute_with(|| {
				let default_key: [u8; 32] = Default::default();
				let test_mpc_key_a: [u8; 32] = [1; 32];
				let test_mpc_key_b: [u8; 32] = [2; 32];

				assert_eq!(MpcKey::<Runtime>::get(), default_key);

				// set to test_key_a
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key_a));
				assert_eq!(MpcKey::<Runtime>::get(), test_mpc_key_a);

				// set to test_key_b: should be MpcKeyNotUpdatable error
				assert_noop!(
					SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key_b),
					bridge::Error::<Runtime>::MpcKeyNotUpdatable
				);

				// permission test: unauthorized account should not be able to set mpc key
				let unauthorized_account = Origin::from(Some(ALICE));
				assert_noop!(
					SygmaBridge::set_mpc_key(unauthorized_account, test_mpc_key_a),
					BadOrigin
				);
				assert_eq!(MpcKey::<Runtime>::get(), test_mpc_key_a);
			})
		}

		#[test]
		fn pause_bridge() {
			new_test_ext().execute_with(|| {
				let default_key: [u8; 32] = Default::default();
				let test_mpc_key_a: [u8; 32] = [1; 32];

				assert_eq!(MpcKey::<Runtime>::get(), default_key);

				// pause bridge when mpc key is not set, should be err
				assert_noop!(
					SygmaBridge::pause_bridge(Origin::root()),
					bridge::Error::<Runtime>::MissingMpcKey
				);

				// set mpc key to test_key_a
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key_a));
				assert_eq!(MpcKey::<Runtime>::get(), test_mpc_key_a);

				// pause bridge again, should be ok
				assert_ok!(SygmaBridge::pause_bridge(Origin::root()));
				assert!(IsPaused::<Runtime>::get());
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::BridgePaused {
					dest_domain_id: 1,
				})]);

				// pause bridge again after paused, should be ok
				assert_ok!(SygmaBridge::pause_bridge(Origin::root()));
				assert!(IsPaused::<Runtime>::get());
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::BridgePaused {
					dest_domain_id: 1,
				})]);

				// permission test: unauthorized account should not be able to pause bridge
				let unauthorized_account = Origin::from(Some(ALICE));
				assert_noop!(SygmaBridge::pause_bridge(unauthorized_account), BadOrigin);
				assert!(IsPaused::<Runtime>::get());
			})
		}

		#[test]
		fn unpause_bridge() {
			new_test_ext().execute_with(|| {
				let default_key: [u8; 32] = Default::default();
				let test_mpc_key_a: [u8; 32] = [1; 32];

				assert_eq!(MpcKey::<Runtime>::get(), default_key);

				// unpause bridge when mpc key is not set, should be error
				assert_noop!(
					SygmaBridge::unpause_bridge(Origin::root()),
					bridge::Error::<Runtime>::MissingMpcKey
				);

				// set mpc key to test_key_a and pause bridge
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key_a));
				assert_eq!(MpcKey::<Runtime>::get(), test_mpc_key_a);
				assert_ok!(SygmaBridge::pause_bridge(Origin::root()));
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::BridgePaused {
					dest_domain_id: 1,
				})]);

				// bridge should be paused here
				assert!(IsPaused::<Runtime>::get());

				// ready to unpause bridge, should be ok
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root()));
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::BridgeUnpaused {
					dest_domain_id: 1,
				})]);

				// try to unpause it again, should be error
				assert_noop!(
					SygmaBridge::unpause_bridge(Origin::root()),
					bridge::Error::<Runtime>::BridgeUnpaused
				);

				// permission test: unauthorized account should not be able to unpause a recognized
				// bridge
				let unauthorized_account = Origin::from(Some(ALICE));
				assert_noop!(SygmaBridge::unpause_bridge(unauthorized_account), BadOrigin);
				assert!(!IsPaused::<Runtime>::get());
			})
		}

		#[test]
		fn deposit_native_asset_should_work() {
			new_test_ext().execute_with(|| {
				let test_mpc_key: [u8; 32] = [1; 32];
				let fee = 100u128;
				let amount = 200u128;
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					PhaLocation::get().into(),
					fee
				));
				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					(Concrete(PhaLocation::get()), Fungible(amount)).into(),
					(
						0,
						X1(GeneralKey(
							WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
						))
					)
						.into(),
				));
				// Check balances
				assert_eq!(Balances::free_balance(ALICE), ENDOWED_BALANCE - amount);
				assert_eq!(Balances::free_balance(BridgeAccount::get()), amount - fee);
				assert_eq!(Balances::free_balance(TreasuryAccount::get()), fee);
				// Check event
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::Deposit {
					dest_domain_id: DestDomainID::get(),
					resource_id: PhaResourceId::get(),
					deposit_nonce: 0,
					sender: ALICE,
					deposit_data: SygmaBridge::create_deposit_data(
						amount - fee,
						b"ethereum recipient".to_vec(),
					),
					handler_repoonse: vec![],
				})]);
			})
		}

		#[test]
		fn deposit_foreign_asset_should_work() {
			new_test_ext().execute_with(|| {
				let test_mpc_key: [u8; 32] = [1; 32];
				let fee = 100u128;
				let amount = 200u128;
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					UsdcLocation::get().into(),
					fee
				));
				// Register foreign asset (USDC) with asset id 0
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(0, ASSET_OWNER, true, 1,));

				// Mint some USDC to ALICE for test
				assert_ok!(Assets::mint(Origin::signed(ASSET_OWNER), 0, ALICE, ENDOWED_BALANCE,));
				assert_eq!(Assets::balance(0u32, &ALICE), ENDOWED_BALANCE);

				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					(Concrete(UsdcLocation::get()), Fungible(amount)).into(),
					(
						0,
						X1(GeneralKey(
							WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
						))
					)
						.into(),
				));
				// Check balances
				assert_eq!(Assets::balance(0u32, &ALICE), ENDOWED_BALANCE - amount);
				assert_eq!(Assets::balance(0u32, &BridgeAccount::get()), 0);
				assert_eq!(Assets::balance(0u32, &TreasuryAccount::get()), fee);
				// Check event
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::Deposit {
					dest_domain_id: DestDomainID::get(),
					resource_id: UsdcResourceId::get(),
					deposit_nonce: 0,
					sender: ALICE,
					deposit_data: SygmaBridge::create_deposit_data(
						amount - fee,
						b"ethereum recipient".to_vec(),
					),
					handler_repoonse: vec![],
				})]);
			})
		}

		#[test]
		fn deposit_unbounded_asset_should_fail() {
			new_test_ext().execute_with(|| {})
		}

		#[test]
		fn deposit_to_unrecognized_dest_should_fail() {
			new_test_ext().execute_with(|| {})
		}

		#[test]
		fn deposit_without_fee_set_should_fail() {
			new_test_ext().execute_with(|| {})
		}

		#[test]
		fn deposit_less_than_fee_should_fail() {
			new_test_ext().execute_with(|| {})
		}

		#[test]
		fn deposit_when_bridge_paused_should_fail() {
			new_test_ext().execute_with(|| {})
		}

		#[test]
		fn deposit_without_mpc_set_should_fail() {
			new_test_ext().execute_with(|| {})
		}
	}
}
