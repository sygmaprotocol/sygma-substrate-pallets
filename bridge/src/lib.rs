// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use self::pallet::*;

mod eip712;
#[cfg(test)]
mod mock;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use alloc::string::String;

	use codec::{Decode, Encode};
	use eth_encode_packed::{abi::encode_packed, SolidityDataType};
	use ethabi::{encode as abi_encode, token::Token};
	use frame_support::{
		dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion, transactional,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use primitive_types::U256;
	use scale_info::TypeInfo;
	use sp_io::{
		crypto::secp256k1_ecdsa_recover_compressed,
		hashing::{blake2_256, keccak_256},
	};
	use sp_runtime::{
		traits::{AccountIdConversion, Clear},
		RuntimeDebug,
	};
	use sp_std::{convert::From, vec, vec::Vec};
	use xcm::latest::{prelude::*, MultiLocation};
	use xcm_executor::traits::TransactAsset;

	use sygma_traits::{
		ChainID, DepositNonce, DomainID, ExtractRecipient, FeeHandler, IsReserved, MpcPubkey,
		ResourceId, TransferType, VerifyingContractAddress,
	};

	use crate::eip712;

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
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + sygma_access_segregator::Config {
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

		/// Pallet ChainID
		/// This is used in EIP712 typed data domain
		#[pallet::constant]
		type DestChainID: Get<ChainID>;

		/// EIP712 Verifying contract address
		/// This is used in EIP712 typed data domain
		#[pallet::constant]
		type DestVerifyingContractAddress: Get<VerifyingContractAddress>;

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
		type ReserveChecker: IsReserved;

		/// Extract recipient from given MultiLocation
		type ExtractRecipient: ExtractRecipient;

		/// Config ID for the current pallet instance
		type PalletId: Get<PalletId>;

		/// Current pallet index defined in runtime
		type PalletIndex: Get<u8>;
	}

	#[allow(dead_code)]
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// When initial bridge transfer send to dest domain
		/// args: [dest_domain_id, resource_id, deposit_nonce, sender, deposit_data,
		/// handler_response, transfer_type]
		Deposit {
			dest_domain_id: DomainID,
			resource_id: ResourceId,
			deposit_nonce: DepositNonce,
			sender: T::AccountId,
			deposit_data: Vec<u8>,
			handler_response: Vec<u8>,
			transfer_type: TransferType,
		},
		/// When proposal was executed successfully
		ProposalExecution {
			origin_domain_id: DomainID,
			deposit_nonce: DepositNonce,
			data_hash: [u8; 32],
		},
		/// When proposal was faild to execute
		FailedHandlerExecution {
			error: Vec<u8>,
			origin_domain_id: DomainID,
			deposit_nonce: DepositNonce,
		},
		/// When user is going to retry a bridge transfer
		/// args: [deposit_on_block_height, deposit_extrinsic_index, sender]
		Retry { deposit_on_block_height: u128, deposit_extrinsic_index: u128, sender: T::AccountId },
		/// When bridge is paused
		/// args: [dest_domain_id]
		BridgePaused { dest_domain_id: DomainID },
		/// When bridge is unpaused
		/// args: [dest_domain_id]
		BridgeUnpaused { dest_domain_id: DomainID },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Account has not gained access permission
		AccessDenied,
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
		/// Origin domain id mismatch
		InvalidOriginDomainId,
		/// Deposit data not correct
		InvalidDepositData,
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
	pub type MpcKey<T> = StorageValue<_, MpcPubkey, ValueQuery>;

	/// Mark whether a deposit nonce was used. Used to mark execution status of a proposal.
	#[pallet::storage]
	#[pallet::getter(fn used_nonces)]
	pub type UsedNonces<T: Config> =
		StorageMap<_, Twox64Concat, DepositNonce, DepositNonce, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
	{
		/// Pause bridge, this would lead to bridge transfer failure before it being unpaused.
		#[pallet::weight(195_000_000)]
		pub fn pause_bridge(origin: OriginFor<T>) -> DispatchResult {
			if <T as Config>::BridgeCommitteeOrigin::ensure_origin(origin.clone()).is_err() {
				// Ensure bridge committee or the account that has permisson to pause bridge
				let who = ensure_signed(origin)?;
				ensure!(
					<sygma_access_segregator::pallet::Pallet<T>>::has_access(
						<T as Config>::PalletIndex::get(),
						b"pause_bridge".to_vec(),
						who
					),
					Error::<T>::AccessDenied
				);
			}
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
			if <T as Config>::BridgeCommitteeOrigin::ensure_origin(origin.clone()).is_err() {
				// Ensure bridge committee or the account that has permisson to unpause bridge
				let who = ensure_signed(origin)?;
				ensure!(
					<sygma_access_segregator::pallet::Pallet<T>>::has_access(
						<T as Config>::PalletIndex::get(),
						b"unpause_bridge".to_vec(),
						who
					),
					Error::<T>::AccessDenied
				);
			}
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
		pub fn set_mpc_key(origin: OriginFor<T>, _key: MpcPubkey) -> DispatchResult {
			if <T as Config>::BridgeCommitteeOrigin::ensure_origin(origin.clone()).is_err() {
				// Ensure bridge committee or the account that has permisson to set mpc key
				let who = ensure_signed(origin)?;
				ensure!(
					<sygma_access_segregator::pallet::Pallet<T>>::has_access(
						<T as Config>::PalletIndex::get(),
						b"set_mpc_key".to_vec(),
						who
					),
					Error::<T>::AccessDenied
				);
			}
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

			// Extract asset (MultiAsset) to get corresponding ResourceId, transfer amount and the
			// transfer type
			let (resource_id, amount, transfer_type) =
				Self::extract_asset(&asset).ok_or(Error::<T>::AssetNotBound)?;
			// Extract dest (MultiLocation) to get corresponding Ethereum recipient address
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
			if T::ReserveChecker::is_reserved(&asset.id) {
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
				handler_response: vec![],
				transfer_type,
			});

			Ok(())
		}

		/// This method is used to trigger the process for retrying failed deposits on the MPC side.
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn retry(
			origin: OriginFor<T>,
			deposit_on_block_height: u128,
			deposit_extrinsic_index: u128,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(!MpcKey::<T>::get().is_clear(), Error::<T>::MissingMpcKey);
			ensure!(!IsPaused::<T>::get(), Error::<T>::BridgePaused);

			// Emit retry event
			Self::deposit_event(Event::<T>::Retry {
				deposit_on_block_height,
				deposit_extrinsic_index,
				sender,
			});
			Ok(())
		}

		/// Executes a batch of deposit proposals (only if signature is signed by MPC).
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn execute_proposal(
			_origin: OriginFor<T>,
			proposals: Vec<Proposal>,
			signature: Vec<u8>,
		) -> DispatchResult {
			// Check MPC key and bridge status
			ensure!(!MpcKey::<T>::get().is_clear(), Error::<T>::MissingMpcKey);
			ensure!(!IsPaused::<T>::get(), Error::<T>::BridgePaused);
			// Verify MPC signature
			ensure!(Self::verify(&proposals, signature), Error::<T>::BadMpcSignature);

			// Execute proposals one by on.
			// Note if one proposal failed to execute, we emit `FailedHandlerExecution` rather
			// than revert whole transaction
			for proposal in proposals.iter() {
				Self::execute_proposal_internal(proposal).map_or_else(
					|e| {
						let err_msg: &'static str = e.into();
						// Emit FailedHandlerExecution
						Self::deposit_event(Event::FailedHandlerExecution {
							error: err_msg.as_bytes().to_vec(),
							origin_domain_id: proposal.origin_domain_id,
							deposit_nonce: proposal.deposit_nonce,
						});
					},
					|_| {
						// Update proposal status
						Self::set_proposal_executed(proposal.deposit_nonce);

						// Emit ProposalExecution
						Self::deposit_event(Event::ProposalExecution {
							origin_domain_id: proposal.origin_domain_id,
							deposit_nonce: proposal.deposit_nonce,
							data_hash: keccak_256(
								&[
									proposal.data.clone(),
									T::PalletId::get().into_account_truncating(),
								]
								.concat(),
							),
						});
					},
				);
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
	{
		/// Verifies that proposal data is signed by MPC address.
		#[allow(dead_code)]
		fn verify(proposals: &Vec<Proposal>, signature: Vec<u8>) -> bool {
			let sig = match signature.try_into() {
				Ok(_sig) => _sig,
				Err(error) => return false,
			};

			// parse proposals and construct signing message
			let final_message = Self::construct_ecdsa_signing_proposals_data(proposals);

			// recover the signing pubkey
			if let Ok(pubkey) =
				secp256k1_ecdsa_recover_compressed(&sig, &blake2_256(&final_message))
			{
				pubkey == MpcKey::<T>::get().0
			} else {
				false
			}
		}

		/// Parse proposals and construct the original signing message
		pub fn construct_ecdsa_signing_proposals_data(proposals: &Vec<Proposal>) -> [u8; 32] {
			let proposal_typehash = keccak_256(
				"Proposal(uint8 originDomainID,uint64 depositNonce,bytes32 resourceID,bytes data)"
					.as_bytes(),
			);

			let mut keccak_data = Vec::new();
			for prop in proposals {
				let proposal_domain_id_token = Token::Uint(prop.origin_domain_id.into());
				let proposal_deposit_nonce_token = Token::Uint(prop.deposit_nonce.into());
				let proposal_resource_id_token = Token::FixedBytes(prop.resource_id.to_vec());
				let proposal_data_token = Token::FixedBytes(keccak_256(&prop.data).to_vec());

				keccak_data.push(keccak_256(&abi_encode(&[
					Token::FixedBytes(proposal_typehash.to_vec()),
					proposal_domain_id_token,
					proposal_deposit_nonce_token,
					proposal_resource_id_token,
					proposal_data_token,
				])));
			}

			// flatten the keccak_data into vec<u8>
			let mut final_keccak_data = Vec::new();
			for data in keccak_data {
				for d in data {
					final_keccak_data.push(d)
				}
			}

			let final_keccak_data_input = &vec![SolidityDataType::Bytes(&final_keccak_data)];
			let (bytes, _) = encode_packed(final_keccak_data_input);
			let hashed_keccak_data = keccak_256(bytes.as_slice());

			let struct_hash = keccak_256(&abi_encode(&[
				Token::FixedBytes(proposal_typehash.to_vec()),
				Token::FixedBytes(hashed_keccak_data.to_vec()),
			]));

			// domain separator
			let default_eip712_domain = eip712::EIP712Domain::default();
			let eip712_domain = eip712::EIP712Domain {
				name: String::from("Bridge"),
				version: String::from("3.1.0"),
				chain_id: T::DestChainID::get(),
				verifying_contract: T::DestVerifyingContractAddress::get(),
				salt: default_eip712_domain.salt,
			};
			let domain_separator = eip712_domain.separator();

			let typed_data_hash_input = &vec![
				SolidityDataType::String("\x19\x01"),
				SolidityDataType::Bytes(&domain_separator),
				SolidityDataType::Bytes(&struct_hash),
			];
			let (bytes, _) = encode_packed(typed_data_hash_input);
			keccak_256(bytes.as_slice())
		}

		/// Extract asset id and transfer amount from `MultiAsset`, currently only fungible asset
		/// are supported.
		fn extract_asset(asset: &MultiAsset) -> Option<(ResourceId, u128, TransferType)> {
			match (&asset.fun, &asset.id) {
				(Fungible(amount), _) =>
					T::ResourcePairs::get().iter().position(|a| a.0 == asset.id).map(|idx| {
						(T::ResourcePairs::get()[idx].1, *amount, TransferType::FungibleTransfer)
					}),
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

		/// Extract transfer amount and recipient location from deposit data.
		/// For fungible transfer, data passed into the function should be constructed as follows:
		/// amount                    uint256     bytes  0 - 32
		/// recipient data length     uint256     bytes  32 - 64
		/// recipient data            bytes       bytes  64 - END
		///
		/// Only fungible transfer is supportted so far.
		fn extract_deposit_data(data: &Vec<u8>) -> Option<(u128, MultiLocation)> {
			if data.len() < 64 {
				return None
			}
			let amount: u128 = U256::from_little_endian(&data[0..32])
				.try_into()
				.expect("Amount convert failed. qed.");
			let recipient_len: usize = U256::from_little_endian(&data[32..64])
				.try_into()
				.expect("Length convert failed. qed.");
			if data.len() != (64 + recipient_len) {
				return None
			}
			let recipient = data[64..data.len()].to_vec();
			if let Ok(location) = <MultiLocation>::decode(&mut recipient.as_slice()) {
				Some((amount, location))
			} else {
				None
			}
		}

		fn rid_to_assetid(rid: &ResourceId) -> Option<AssetId> {
			T::ResourcePairs::get()
				.iter()
				.position(|a| &a.1 == rid)
				.map(|idx| T::ResourcePairs::get()[idx].0.clone())
		}

		fn hex_zero_padding_32(i: u128) -> [u8; 32] {
			let mut result = [0u8; 32];
			U256::from(i).to_little_endian(&mut result);
			result
		}

		/// Return true if deposit nonce has been used
		fn is_proposal_executed(nonce: DepositNonce) -> bool {
			(UsedNonces::<T>::get(nonce / 256) & (1 << (nonce % 256))) != 0
		}

		/// Set bit mask for specific nonce as used
		fn set_proposal_executed(nonce: DepositNonce) {
			let mut current_nonces = UsedNonces::<T>::get(nonce / 256);
			current_nonces |= 1 << (nonce % 256);
			UsedNonces::<T>::insert(nonce / 256, current_nonces);
		}

		/// Execute a single proposal
		fn execute_proposal_internal(proposal: &Proposal) -> DispatchResult {
			// Check if proposal has executed
			ensure!(
				!Self::is_proposal_executed(proposal.deposit_nonce),
				Error::<T>::ProposalAlreadyComplete
			);
			// Check if the dest domain id is correct
			ensure!(
				proposal.origin_domain_id == T::DestDomainID::get(),
				Error::<T>::InvalidOriginDomainId
			);
			// Extract ResourceId from proposal data to get corresponding asset (MultiAsset)
			let asset_id =
				Self::rid_to_assetid(&proposal.resource_id).ok_or(Error::<T>::AssetNotBound)?;
			// Extract Receipt from proposal data to get corresponding location (MultiLocation)
			let (amount, location) =
				Self::extract_deposit_data(&proposal.data).ok_or(Error::<T>::InvalidDepositData)?;
			let asset = (asset_id.clone(), amount).into();

			// Withdraw `amount` of asset from reserve account
			if T::ReserveChecker::is_reserved(&asset_id) {
				T::AssetTransactor::withdraw_asset(
					&asset,
					&Junction::AccountId32 {
						network: NetworkId::Any,
						id: T::TransferReserveAccount::get().into(),
					}
					.into(),
				)
				.map_err(|_| Error::<T>::TransactFailed)?;
			}

			// Deposit `amount` of asset to dest location
			T::AssetTransactor::deposit_asset(&asset, &location)
				.map_err(|_| Error::<T>::TransactFailed)?;

			Ok(())
		}
	}

	#[cfg(test)]
	mod test {
		use crate as bridge;
		use crate::{Event as SygmaBridgeEvent, IsPaused, MpcKey, Proposal};
		use bridge::mock::{
			assert_events, new_test_ext, AccessSegregator, Assets, Balances, BridgeAccount,
			BridgePalletIndex, DestDomainID, PhaLocation, PhaResourceId, Runtime, RuntimeEvent,
			RuntimeOrigin as Origin, SygmaBasicFeeHandler, SygmaBridge, TreasuryAccount,
			UsdcAssetId, UsdcLocation, UsdcResourceId, ALICE, ASSET_OWNER, BOB, ENDOWED_BALANCE,
		};
		use codec::Encode;
		use frame_support::{
			assert_noop, assert_ok, traits::tokens::fungibles::Create as FungibleCerate,
		};
		use sp_core::{ecdsa, Pair};
		use sp_runtime::WeakBoundedVec;
		use sp_std::convert::TryFrom;
		use sygma_traits::{MpcPubkey, TransferType};
		use xcm::latest::prelude::*;

		#[test]
		fn set_mpc_key() {
			new_test_ext().execute_with(|| {
				let default_key: MpcPubkey = MpcPubkey::default();
				let test_mpc_key_a: MpcPubkey = MpcPubkey([1u8; 33]);
				let test_mpc_key_b: MpcPubkey = MpcPubkey([2u8; 33]);

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
					bridge::Error::<Runtime>::AccessDenied
				);
				assert_eq!(MpcKey::<Runtime>::get(), test_mpc_key_a);
			})
		}

		#[test]
		fn pause_bridge() {
			new_test_ext().execute_with(|| {
				let default_key: MpcPubkey = MpcPubkey::default();
				let test_mpc_key_a: MpcPubkey = MpcPubkey([1u8; 33]);

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
				assert_noop!(
					SygmaBridge::pause_bridge(unauthorized_account),
					bridge::Error::<Runtime>::AccessDenied
				);
				assert!(IsPaused::<Runtime>::get());
			})
		}

		#[test]
		fn unpause_bridge() {
			new_test_ext().execute_with(|| {
				let default_key: MpcPubkey = MpcPubkey::default();
				let test_mpc_key_a: MpcPubkey = MpcPubkey([1u8; 33]);

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
				assert_noop!(
					SygmaBridge::unpause_bridge(unauthorized_account),
					bridge::Error::<Runtime>::AccessDenied
				);
				assert!(!IsPaused::<Runtime>::get());
			})
		}

		#[test]
		fn verify_mpc_signature_invalid_signature() {
			new_test_ext().execute_with(|| {
				let signature = vec![1u8];

				// dummy proposals
				let p1 = Proposal {
					origin_domain_id: 1,
					deposit_nonce: 1,
					resource_id: [1u8; 32],
					data: vec![1u8],
				};
				let p2 = Proposal {
					origin_domain_id: 2,
					deposit_nonce: 2,
					resource_id: [2u8; 32],
					data: vec![2u8],
				};
				let proposals = vec![p1, p2];

				// should be false
				assert!(!SygmaBridge::verify(&proposals, signature.encode()));
			})
		}

		#[test]
		fn verify_mpc_signature_invalid_message() {
			new_test_ext().execute_with(|| {
				// generate mpc keypair
				let (pair, _): (ecdsa::Pair, _) = Pair::generate();
				let public = pair.public();
				let message = b"Something important";
				let signature = pair.sign(&message[..]);

				// make sure generated keypair, message and signature are all good
				assert!(ecdsa::Pair::verify(&signature, &message[..], &public));
				assert!(!ecdsa::Pair::verify(&signature, b"Something else", &public));

				// dummy proposals
				let p1 = Proposal {
					origin_domain_id: 1,
					deposit_nonce: 1,
					resource_id: [1u8; 32],
					data: vec![1u8],
				};
				let p2 = Proposal {
					origin_domain_id: 2,
					deposit_nonce: 2,
					resource_id: [2u8; 32],
					data: vec![2u8],
				};
				let proposals = vec![p1, p2];

				// verify non matched signature against proposal list, should be false
				assert!(!SygmaBridge::verify(&proposals, signature.encode()));
			})
		}

		#[test]
		fn verify_mpc_signature_valid_message_unmatched_mpc() {
			new_test_ext().execute_with(|| {
				// generate the signing keypair
				let (pair, _): (ecdsa::Pair, _) = Pair::generate();

				// set mpc key to another random key
				let test_mpc_key: MpcPubkey = MpcPubkey([7u8; 33]);
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));
				assert_eq!(MpcKey::<Runtime>::get(), test_mpc_key);

				// dummy proposals
				let p1 = Proposal {
					origin_domain_id: 1,
					deposit_nonce: 1,
					resource_id: [1u8; 32],
					data: vec![1u8],
				};
				let p2 = Proposal {
					origin_domain_id: 2,
					deposit_nonce: 2,
					resource_id: [2u8; 32],
					data: vec![2u8],
				};
				let proposals = vec![p1, p2];

				let final_message = SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals);

				// sign final message using generated prikey
				let signature = pair.sign(&final_message[..]);

				// verify signature, should be false because the signing key != mpc key
				assert!(!SygmaBridge::verify(&proposals, signature.encode()));
			})
		}

		#[test]
		fn verify_mpc_signature_valid_message_valid_signature() {
			new_test_ext().execute_with(|| {
				// generate mpc keypair
				let (pair, _): (ecdsa::Pair, _) = Pair::generate();
				let test_mpc_key: MpcPubkey = MpcPubkey(pair.public().0);

				// set mpc key to generated keypair's pubkey
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));
				assert_eq!(MpcKey::<Runtime>::get(), test_mpc_key);

				// dummy proposals
				let p1 = Proposal {
					origin_domain_id: 1,
					deposit_nonce: 1,
					resource_id: [1u8; 32],
					data: vec![1u8],
				};
				let p2 = Proposal {
					origin_domain_id: 2,
					deposit_nonce: 2,
					resource_id: [2u8; 32],
					data: vec![2u8],
				};
				let proposals = vec![p1, p2];

				let final_message = SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals);

				// sign final message using generated mpc prikey
				let signature = pair.sign(&final_message[..]);

				// verify signature, should be true
				assert!(SygmaBridge::verify(&proposals, signature.encode()));
			})
		}

		#[test]
		fn deposit_native_asset_should_work() {
			new_test_ext().execute_with(|| {
				let test_mpc_key: MpcPubkey = MpcPubkey([1u8; 33]);
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
					handler_response: vec![],
					transfer_type: TransferType::FungibleTransfer,
				})]);
			})
		}

		#[test]
		fn deposit_foreign_asset_should_work() {
			new_test_ext().execute_with(|| {
				let test_mpc_key: MpcPubkey = MpcPubkey([1u8; 33]);
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
				>>::create(UsdcAssetId::get(), ASSET_OWNER, true, 1,));

				// Mint some USDC to ALICE for test
				assert_ok!(Assets::mint(Origin::signed(ASSET_OWNER), 0, ALICE, ENDOWED_BALANCE,));
				assert_eq!(Assets::balance(UsdcAssetId::get(), &ALICE), ENDOWED_BALANCE);

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
				assert_eq!(Assets::balance(UsdcAssetId::get(), &ALICE), ENDOWED_BALANCE - amount);
				assert_eq!(Assets::balance(UsdcAssetId::get(), &BridgeAccount::get()), 0);
				assert_eq!(Assets::balance(UsdcAssetId::get(), &TreasuryAccount::get()), fee);
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
					handler_response: vec![],
					transfer_type: TransferType::FungibleTransfer,
				})]);
			})
		}

		#[test]
		fn deposit_unbounded_asset_should_fail() {
			new_test_ext().execute_with(|| {
				let unbounded_asset_location = MultiLocation::new(1, X1(GeneralIndex(123)));
				let test_mpc_key: MpcPubkey = MpcPubkey([1u8; 33]);
				let fee = 100u128;
				let amount = 200u128;
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					unbounded_asset_location.clone().into(),
					fee
				));
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(unbounded_asset_location), Fungible(amount)).into(),
						(
							0,
							X1(GeneralKey(
								WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
							))
						)
							.into(),
					),
					bridge::Error::<Runtime>::AssetNotBound
				);
			})
		}

		#[test]
		fn deposit_to_unrecognized_dest_should_fail() {
			new_test_ext().execute_with(|| {
				let invalid_dest = MultiLocation::new(
					0,
					X2(
						GeneralIndex(0),
						GeneralKey(
							WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap(),
						),
					),
				);
				let test_mpc_key: MpcPubkey = MpcPubkey([1u8; 33]);
				let fee = 100u128;
				let amount = 200u128;
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					PhaLocation::get().into(),
					fee
				));
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(PhaLocation::get()), Fungible(amount)).into(),
						invalid_dest,
					),
					bridge::Error::<Runtime>::ExtractRecipientFailed
				);
			})
		}

		#[test]
		fn deposit_without_fee_set_should_fail() {
			new_test_ext().execute_with(|| {
				let test_mpc_key: MpcPubkey = MpcPubkey([1u8; 33]);
				let amount = 200u128;
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(PhaLocation::get()), Fungible(amount)).into(),
						(
							0,
							X1(GeneralKey(
								WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
							))
						)
							.into(),
					),
					bridge::Error::<Runtime>::MissingFeeConfig
				);
			})
		}

		#[test]
		fn deposit_less_than_fee_should_fail() {
			new_test_ext().execute_with(|| {
				let test_mpc_key: MpcPubkey = MpcPubkey([1u8; 33]);
				let fee = 200u128;
				let amount = 100u128;
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					PhaLocation::get().into(),
					fee
				));
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(PhaLocation::get()), Fungible(amount)).into(),
						(
							0,
							X1(GeneralKey(
								WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
							))
						)
							.into(),
					),
					bridge::Error::<Runtime>::FeeTooExpensive
				);
			})
		}

		#[test]
		fn deposit_when_bridge_paused_should_fail() {
			new_test_ext().execute_with(|| {
				let test_mpc_key: MpcPubkey = MpcPubkey([1u8; 33]);
				let fee = 100u128;
				let amount = 200u128;
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					PhaLocation::get().into(),
					fee
				));
				// Pause bridge
				assert_ok!(SygmaBridge::pause_bridge(Origin::root()));
				// Should failed
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(PhaLocation::get()), Fungible(amount)).into(),
						(
							0,
							X1(GeneralKey(
								WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
							))
						)
							.into(),
					),
					bridge::Error::<Runtime>::BridgePaused
				);
				// Unpause bridge
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root()));
				// Should success
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
			})
		}

		#[test]
		fn deposit_without_mpc_set_should_fail() {
			new_test_ext().execute_with(|| {
				let fee = 200u128;
				let amount = 100u128;
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					PhaLocation::get().into(),
					fee
				));
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(PhaLocation::get()), Fungible(amount)).into(),
						(
							0,
							X1(GeneralKey(
								WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
							))
						)
							.into(),
					),
					bridge::Error::<Runtime>::MissingMpcKey
				);
			})
		}

		#[test]
		fn retry_bridge() {
			new_test_ext().execute_with(|| {
				// mpc key is missing, should fail
				assert_noop!(
					SygmaBridge::retry(Origin::signed(ALICE), 1234567u128, 1234u128),
					bridge::Error::<Runtime>::MissingMpcKey
				);

				// set mpc key
				let test_mpc_key: MpcPubkey = MpcPubkey([1u8; 33]);
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));

				// pause bridge and retry, should fail
				assert_ok!(SygmaBridge::pause_bridge(Origin::root()));
				assert_noop!(
					SygmaBridge::retry(Origin::signed(ALICE), 1234567u128, 1234u128),
					bridge::Error::<Runtime>::BridgePaused
				);

				// unpause bridge
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root()));
				assert!(!IsPaused::<Runtime>::get());

				// retry again, should work
				assert_ok!(SygmaBridge::retry(Origin::signed(ALICE), 1234567u128, 1234u128));
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::Retry {
					deposit_on_block_height: 1234567u128,
					deposit_extrinsic_index: 1234u128,
					sender: ALICE,
				})]);
			})
		}

		#[test]
		fn proposal_execution_should_work() {
			new_test_ext().execute_with(|| {
				// Mpc key is missing, should fail
				assert_noop!(
					SygmaBridge::execute_proposal(Origin::signed(ALICE), vec![], vec![]),
					bridge::Error::<Runtime>::MissingMpcKey,
				);
				// Set mpc key to generated keypair's pubkey
				let (pair, _): (ecdsa::Pair, _) = Pair::generate();
				let test_mpc_key: MpcPubkey = MpcPubkey(pair.public().0);
				// Generate an evil key
				let (evil_pair, _): (ecdsa::Pair, _) = Pair::generate();
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));
				assert_eq!(MpcKey::<Runtime>::get(), test_mpc_key);

				// Should failed if bridge paused
				assert_ok!(SygmaBridge::pause_bridge(Origin::root()));
				assert_noop!(
					SygmaBridge::execute_proposal(Origin::signed(ALICE), vec![], vec![]),
					bridge::Error::<Runtime>::BridgePaused,
				);
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root()));

				// Deposit some PHA in advance
				let fee = 100u128;
				let amount = 200u128;
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					PhaLocation::get().into(),
					fee
				));
				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					(Concrete(PhaLocation::get()), Fungible(2 * amount)).into(),
					(
						0,
						X1(GeneralKey(
							WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
						))
					)
						.into(),
				));

				// Register foreign asset (USDC) with asset id 0
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdcAssetId::get(), ASSET_OWNER, true, 1,));

				// Generate proposals
				let valid_pha_transfer_proposal = Proposal {
					origin_domain_id: DestDomainID::get(),
					deposit_nonce: 1,
					resource_id: PhaResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: Any, id: BOB.into() }))
							.encode(),
					),
				};
				let valid_usdc_transfer_proposal = Proposal {
					origin_domain_id: DestDomainID::get(),
					deposit_nonce: 2,
					resource_id: UsdcResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: Any, id: BOB.into() }))
							.encode(),
					),
				};
				let invalid_depositnonce_proposal = Proposal {
					origin_domain_id: DestDomainID::get(),
					deposit_nonce: 2,
					resource_id: PhaResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: Any, id: BOB.into() }))
							.encode(),
					),
				};
				let invalid_domainid_proposal = Proposal {
					origin_domain_id: 2,
					deposit_nonce: 3,
					resource_id: PhaResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: Any, id: BOB.into() }))
							.encode(),
					),
				};
				let invalid_resourceid_proposal = Proposal {
					origin_domain_id: DestDomainID::get(),
					deposit_nonce: 3,
					resource_id: [2u8; 32],
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: Any, id: BOB.into() }))
							.encode(),
					),
				};
				let invalid_recipient_proposal = Proposal {
					origin_domain_id: DestDomainID::get(),
					deposit_nonce: 3,
					resource_id: PhaResourceId::get(),
					data: SygmaBridge::create_deposit_data(amount, b"invalid recipient".to_vec()),
				};

				let proposals = vec![
					valid_pha_transfer_proposal,
					valid_usdc_transfer_proposal,
					invalid_depositnonce_proposal,
					invalid_domainid_proposal,
					invalid_resourceid_proposal,
					invalid_recipient_proposal,
				];

				let proposals_with_valid_signature =
					pair.sign(&SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals));
				let proposals_with_bad_signature = evil_pair
					.sign(&SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals));

				assert_noop!(
					SygmaBridge::execute_proposal(
						Origin::signed(ALICE),
						proposals.clone(),
						proposals_with_bad_signature.encode(),
					),
					bridge::Error::<Runtime>::BadMpcSignature,
				);
				assert_eq!(Balances::free_balance(&BOB), ENDOWED_BALANCE);
				assert_eq!(Assets::balance(UsdcAssetId::get(), &BOB), 0);
				assert!(SygmaBridge::verify(&proposals, proposals_with_valid_signature.encode()));
				assert_ok!(SygmaBridge::execute_proposal(
					Origin::signed(ALICE),
					proposals,
					proposals_with_valid_signature.encode(),
				));
				assert_eq!(Balances::free_balance(&BOB), ENDOWED_BALANCE + amount);
				assert_eq!(Assets::balance(UsdcAssetId::get(), &BOB), amount);
			})
		}

		#[test]
		fn get_bridge_pause_status() {
			new_test_ext().execute_with(|| {
				assert!(!SygmaBridge::is_paused());

				// set mpc key
				let test_mpc_key: MpcPubkey = MpcPubkey([1u8; 33]);
				assert_ok!(SygmaBridge::set_mpc_key(Origin::root(), test_mpc_key));

				// pause bridge
				assert_ok!(SygmaBridge::pause_bridge(Origin::root()));
				assert!(SygmaBridge::is_paused());

				// unpause bridge
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root()));
				assert!(!SygmaBridge::is_paused());
			})
		}

		#[test]
		fn access_control() {
			new_test_ext().execute_with(|| {
				let test_mpc_key: MpcPubkey = MpcPubkey([1u8; 33]);

				assert_noop!(
					SygmaBridge::set_mpc_key(Some(ALICE).into(), test_mpc_key),
					bridge::Error::<Runtime>::AccessDenied
				);
				assert_noop!(
					SygmaBridge::pause_bridge(Some(BOB).into()),
					bridge::Error::<Runtime>::AccessDenied
				);
				assert_noop!(
					SygmaBridge::unpause_bridge(Some(BOB).into()),
					bridge::Error::<Runtime>::AccessDenied
				);

				// Grant ALICE the access of `set_mpc_key`
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"set_mpc_key".to_vec(),
					ALICE
				));
				// Grant BOB the access of `pause_bridge` and `unpause_bridge`
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"pause_bridge".to_vec(),
					BOB
				));
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"unpause_bridge".to_vec(),
					BOB
				));

				// BOB set mpc key should still failed
				assert_noop!(
					SygmaBridge::set_mpc_key(Some(BOB).into(), test_mpc_key),
					bridge::Error::<Runtime>::AccessDenied
				);
				// ALICE set mpc key should work
				assert_ok!(SygmaBridge::set_mpc_key(Some(ALICE).into(), test_mpc_key));

				// ALICE pause&unpause bridge should still failed
				assert_noop!(
					SygmaBridge::pause_bridge(Some(ALICE).into()),
					bridge::Error::<Runtime>::AccessDenied
				);
				assert_noop!(
					SygmaBridge::unpause_bridge(Some(ALICE).into()),
					bridge::Error::<Runtime>::AccessDenied
				);
				// BOB pause&unpause bridge should work
				assert_ok!(SygmaBridge::pause_bridge(Some(BOB).into()));
				assert_ok!(SygmaBridge::unpause_bridge(Some(BOB).into()));
			})
		}
	}
}
