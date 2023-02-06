// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate arrayref;

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
		crypto::secp256k1_ecdsa_recover,
		hashing::{blake2_256, keccak_256},
	};
	use sp_runtime::{
		traits::{AccountIdConversion, Clear},
		RuntimeDebug,
	};
	use sp_std::{convert::From, vec, vec::Vec};
	use xcm::latest::{prelude::*, MultiLocation};
	use xcm_executor::traits::TransactAsset;

	use crate::eip712;
	use sygma_traits::{
		ChainID, DepositNonce, DomainID, ExtractDestinationData, FeeHandler, IsReserved,
		MpcAddress, ResourceId, TransferType, VerifyingContractAddress,
	};

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

		/// Bridge transfer reserve account
		#[pallet::constant]
		type TransferReserveAccount: Get<Self::AccountId>;

		/// EIP712 Verifying contract address
		/// This is used in EIP712 typed data domain
		#[pallet::constant]
		type DestVerifyingContractAddress: Get<VerifyingContractAddress>;

		/// Pallet ChainID
		/// This is used in EIP712 typed data domain
		#[pallet::constant]
		type EIP712ChainID: Get<ChainID>;

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

		/// Extract dest data from given MultiLocation
		type ExtractDestData: ExtractDestinationData;

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
		/// args: [dest_domain_id, resource_id, deposit_nonce, sender, transfer_type,
		/// deposit_data, handler_response, ]
		Deposit {
			dest_domain_id: DomainID,
			resource_id: ResourceId,
			deposit_nonce: DepositNonce,
			sender: T::AccountId,
			transfer_type: TransferType,
			deposit_data: Vec<u8>,
			handler_response: Vec<u8>,
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
		/// When registering a new dest domainID with its corresponding chainID
		RegisterDestDomain { sender: T::AccountId, domain_id: DomainID, chain_id: ChainID },
		/// When unregistering a dest domainID with its corresponding chainID
		UnregisterDestDomain { sender: T::AccountId, domain_id: DomainID, chain_id: ChainID },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Account has not gained access permission
		AccessDenied,
		/// Protected operation, must be performed by relayer
		BadMpcSignature,
		/// Insufficient balance on sender account
		InsufficientBalance,
		/// Asset transactor execution failed
		TransactFailed,
		/// The withdrawn amount can not cover the fee payment
		FeeTooExpensive,
		/// MPC address not set
		MissingMpcAddress,
		/// MPC address can not be updated
		MpcAddrNotUpdatable,
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
		/// Deposit data not correct
		InvalidDepositData,
		/// Dest domain not supported
		DestDomainNotSupported,
		/// Dest chain id not match
		DestChainIDNotMatch,
		/// Failed to extract destination data
		ExtractDestDataFailed,
		/// Function unimplemented
		Unimplemented,
	}

	/// Deposit counter of dest domain
	#[pallet::storage]
	#[pallet::getter(fn deposit_counts)]
	pub type DepositCounts<T> = StorageMap<_, Twox64Concat, DomainID, DepositNonce, ValueQuery>;

	/// Bridge Pause indicator
	/// Bridge is unpaused initially, until pause
	/// After mpc address setup, bridge should be paused until ready to unpause
	#[pallet::storage]
	#[pallet::getter(fn is_paused)]
	pub type IsPaused<T> = StorageMap<_, Twox64Concat, DomainID, bool, ValueQuery>;

	/// Pre-set MPC address
	#[pallet::storage]
	#[pallet::getter(fn mpc_addr)]
	pub type MpcAddr<T> = StorageValue<_, MpcAddress, ValueQuery>;

	/// Mark whether a deposit nonce was used. Used to mark execution status of a proposal.
	#[pallet::storage]
	#[pallet::getter(fn used_nonces)]
	pub type UsedNonces<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		DomainID,
		Twox64Concat,
		DepositNonce,
		DepositNonce,
		ValueQuery,
	>;

	/// Mark supported dest domainID
	#[pallet::storage]
	#[pallet::getter(fn dest_domain_ids)]
	pub type DestDomainIds<T: Config> = StorageMap<_, Twox64Concat, DomainID, bool, ValueQuery>;

	/// Mark the pairs for supported dest domainID with its corresponding chainID
	/// The chainID is not directly used in pallet, this map is designed more about rechecking the
	/// domainID
	#[pallet::storage]
	#[pallet::getter(fn dest_chain_ids)]
	pub type DestChainIds<T: Config> = StorageMap<_, Twox64Concat, DomainID, ChainID>;

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
	{
		/// Pause bridge, this would lead to bridge transfer failure before it being unpaused.
		#[pallet::weight(195_000_000)]
		pub fn pause_bridge(origin: OriginFor<T>, dest_domain_id: DomainID) -> DispatchResult {
			if <T as Config>::BridgeCommitteeOrigin::ensure_origin(origin.clone()).is_err() {
				// Ensure bridge committee or the account that has permission to pause bridge
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
			// make sure MPC address is set up
			ensure!(!MpcAddr::<T>::get().is_clear(), Error::<T>::MissingMpcAddress);

			ensure!(DestDomainIds::<T>::get(dest_domain_id), Error::<T>::DestDomainNotSupported);

			// Mark as paused
			IsPaused::<T>::insert(dest_domain_id, true);

			// Emit BridgePause event
			Self::deposit_event(Event::BridgePaused { dest_domain_id });
			Ok(())
		}

		/// Unpause bridge.
		#[pallet::weight(195_000_000)]
		pub fn unpause_bridge(origin: OriginFor<T>, dest_domain_id: DomainID) -> DispatchResult {
			if <T as Config>::BridgeCommitteeOrigin::ensure_origin(origin.clone()).is_err() {
				// Ensure bridge committee or the account that has permission to unpause bridge
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
			// make sure MPC address is set up
			ensure!(!MpcAddr::<T>::get().is_clear(), Error::<T>::MissingMpcAddress);

			ensure!(DestDomainIds::<T>::get(dest_domain_id), Error::<T>::DestDomainNotSupported);

			// make sure the current status is paused
			ensure!(IsPaused::<T>::get(dest_domain_id), Error::<T>::BridgeUnpaused);

			// Mark as unpaused
			IsPaused::<T>::insert(dest_domain_id, false);

			// Emit BridgeUnpause event
			Self::deposit_event(Event::BridgeUnpaused { dest_domain_id });
			Ok(())
		}

		/// Mark an ECDSA address as a MPC account.
		#[pallet::weight(195_000_000)]
		pub fn set_mpc_address(origin: OriginFor<T>, addr: MpcAddress) -> DispatchResult {
			if <T as Config>::BridgeCommitteeOrigin::ensure_origin(origin.clone()).is_err() {
				// Ensure bridge committee or the account that has permission to set mpc address
				let who = ensure_signed(origin)?;
				ensure!(
					<sygma_access_segregator::pallet::Pallet<T>>::has_access(
						<T as Config>::PalletIndex::get(),
						b"set_mpc_address".to_vec(),
						who
					),
					Error::<T>::AccessDenied
				);
			}
			// Cannot set MPC address as it's already set
			ensure!(MpcAddr::<T>::get().is_clear(), Error::<T>::MpcAddrNotUpdatable);

			// Set MPC account address
			MpcAddr::<T>::set(addr);
			Ok(())
		}

		/// Mark the give dest domainID with chainID to be enabled
		#[pallet::weight(195_000_000)]
		pub fn register_domain(
			origin: OriginFor<T>,
			dest_domain_id: DomainID,
			dest_chain_id: ChainID,
		) -> DispatchResult {
			let mut sender: T::AccountId = [0u8; 32].into();
			if <T as Config>::BridgeCommitteeOrigin::ensure_origin(origin.clone()).is_err() {
				// Ensure bridge committee or the account that has permission to register the dest
				// domain
				let who = ensure_signed(origin)?;
				ensure!(
					<sygma_access_segregator::pallet::Pallet<T>>::has_access(
						<T as Config>::PalletIndex::get(),
						b"register_domain".to_vec(),
						who.clone()
					),
					Error::<T>::AccessDenied
				);
				sender = who;
			}
			// make sure MPC address is set up
			ensure!(!MpcAddr::<T>::get().is_clear(), Error::<T>::MissingMpcAddress);

			DestDomainIds::<T>::insert(dest_domain_id, true);
			DestChainIds::<T>::insert(dest_domain_id, dest_chain_id);

			// Emit register dest domain event
			Self::deposit_event(Event::RegisterDestDomain {
				sender,
				domain_id: dest_domain_id,
				chain_id: dest_chain_id,
			});
			Ok(())
		}

		/// Mark the give dest domainID with chainID to be disabled
		#[pallet::weight(195_000_000)]
		pub fn unregister_domain(
			origin: OriginFor<T>,
			dest_domain_id: DomainID,
			dest_chain_id: ChainID,
		) -> DispatchResult {
			let mut sender: T::AccountId = [0u8; 32].into();
			if <T as Config>::BridgeCommitteeOrigin::ensure_origin(origin.clone()).is_err() {
				// Ensure bridge committee or the account that has permission to unregister the dest
				// domain
				let who = ensure_signed(origin)?;
				ensure!(
					<sygma_access_segregator::pallet::Pallet<T>>::has_access(
						<T as Config>::PalletIndex::get(),
						b"unregister_domain".to_vec(),
						who.clone()
					),
					Error::<T>::AccessDenied
				);
				sender = who;
			}
			// make sure MPC address is set up
			ensure!(!MpcAddr::<T>::get().is_clear(), Error::<T>::MissingMpcAddress);

			ensure!(
				DestDomainIds::<T>::get(dest_domain_id) &&
					DestChainIds::<T>::get(dest_domain_id).is_some(),
				Error::<T>::DestDomainNotSupported
			);

			let co_chain_id = DestChainIds::<T>::get(dest_domain_id).unwrap();
			ensure!(co_chain_id == dest_chain_id, Error::<T>::DestChainIDNotMatch);

			DestDomainIds::<T>::remove(dest_domain_id);
			DestChainIds::<T>::remove(dest_domain_id);

			// Emit unregister dest domain event
			Self::deposit_event(Event::UnregisterDestDomain {
				sender,
				domain_id: dest_domain_id,
				chain_id: dest_chain_id,
			});
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

			ensure!(!MpcAddr::<T>::get().is_clear(), Error::<T>::MissingMpcAddress);

			// Extract dest (MultiLocation) to get corresponding dest domainID and Ethereum
			// recipient address
			let (recipient, dest_domain_id) =
				T::ExtractDestData::extract_dest(&dest).ok_or(Error::<T>::ExtractDestDataFailed)?;

			ensure!(DestDomainIds::<T>::get(dest_domain_id), Error::<T>::DestDomainNotSupported);

			ensure!(!IsPaused::<T>::get(dest_domain_id), Error::<T>::BridgePaused);

			// Extract asset (MultiAsset) to get corresponding ResourceId, transfer amount and the
			// transfer type
			let (resource_id, amount, transfer_type) =
				Self::extract_asset(&asset).ok_or(Error::<T>::AssetNotBound)?;
			// Return error if no fee handler set
			let fee = T::FeeHandler::get_fee(dest_domain_id, &asset.id)
				.ok_or(Error::<T>::MissingFeeConfig)?;

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
			let deposit_nonce = DepositCounts::<T>::get(dest_domain_id);
			DepositCounts::<T>::insert(dest_domain_id, deposit_nonce + 1);

			// Emit Deposit event
			Self::deposit_event(Event::Deposit {
				dest_domain_id,
				resource_id,
				deposit_nonce,
				sender,
				transfer_type,
				deposit_data: Self::create_deposit_data(amount - fee, recipient),
				handler_response: vec![],
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
			dest_domain_id: DomainID,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(!MpcAddr::<T>::get().is_clear(), Error::<T>::MissingMpcAddress);
			ensure!(DestDomainIds::<T>::get(dest_domain_id), Error::<T>::DestDomainNotSupported);
			ensure!(!IsPaused::<T>::get(dest_domain_id), Error::<T>::BridgePaused);

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
			// Check MPC address and bridge status
			ensure!(!MpcAddr::<T>::get().is_clear(), Error::<T>::MissingMpcAddress);

			// Verify MPC signature
			ensure!(
				Self::verify_by_mpc_address(&proposals, signature),
				Error::<T>::BadMpcSignature
			);

			// Execute proposals one by one.
			// Note if one proposal failed to execute, we emit `FailedHandlerExecution` rather
			// than revert whole transaction
			for proposal in proposals.iter() {
				Self::execute_proposal_internal(proposal).map_or_else(
					|e| {
						let err_msg: &'static str = e.into();
						// Any error during proposal list execution will emit FailedHandlerExecution
						Self::deposit_event(Event::FailedHandlerExecution {
							error: err_msg.as_bytes().to_vec(),
							origin_domain_id: proposal.origin_domain_id,
							deposit_nonce: proposal.deposit_nonce,
						});
					},
					|_| {
						// Update proposal status
						Self::set_proposal_executed(
							proposal.deposit_nonce,
							proposal.origin_domain_id,
						);

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
		pub fn get_sum() -> u32 {
			4u32
		}

		/// Verifies that proposal data is signed by MPC address.
		#[allow(dead_code)]
		fn verify_by_mpc_address(proposals: &Vec<Proposal>, signature: Vec<u8>) -> bool {
			let sig = match signature.try_into() {
				Ok(_sig) => _sig,
				Err(error) => return false,
			};

			if proposals.is_empty() {
				return false
			}

			// parse proposals and construct signing message
			let final_message = Self::construct_ecdsa_signing_proposals_data(proposals);

			// recover the signing address
			if let Ok(pubkey) =
				// recover the uncompressed pubkey
				secp256k1_ecdsa_recover(&sig, &blake2_256(&final_message))
			{
				let address = Self::public_key_to_address(&pubkey);

				address == MpcAddr::<T>::get().0
			} else {
				false
			}
		}

		/// convert the ECDSA 64-byte uncompressed pubkey to H160 address
		pub fn public_key_to_address(public_key: &[u8]) -> [u8; 20] {
			let hash = keccak_256(public_key);
			let final_hash = array_ref![&hash, 12, 20];
			*final_hash
		}

		/// Parse proposals and construct the original signing message
		pub fn construct_ecdsa_signing_proposals_data(proposals: &Vec<Proposal>) -> [u8; 32] {
			let proposal_typehash = keccak_256(
				"Proposal(uint8 originDomainID,uint64 depositNonce,bytes32 resourceID,bytes data)"
					.as_bytes(),
			);

			if proposals.is_empty() {
				return [0u8; 32]
			}

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
				chain_id: T::EIP712ChainID::get(),
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
			let amount: u128 = U256::from_big_endian(&data[0..32])
				.try_into()
				.expect("Amount convert failed. qed.");
			let recipient_len: usize = U256::from_big_endian(&data[32..64])
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
			U256::from(i).to_big_endian(&mut result);
			result
		}

		/// Return true if deposit nonce has been used
		fn is_proposal_executed(nonce: DepositNonce, domain_id: DomainID) -> bool {
			(UsedNonces::<T>::get(domain_id, nonce / 256) & (1 << (nonce % 256))) != 0
		}

		/// Set bit mask for specific nonce as used
		fn set_proposal_executed(nonce: DepositNonce, domain_id: DomainID) {
			let mut current_nonces = UsedNonces::<T>::get(domain_id, nonce / 256);
			current_nonces |= 1 << (nonce % 256);
			UsedNonces::<T>::insert(domain_id, nonce / 256, current_nonces);
		}

		/// Execute a single proposal
		fn execute_proposal_internal(proposal: &Proposal) -> DispatchResult {
			// Check if domain is supported
			ensure!(
				DestDomainIds::<T>::get(proposal.origin_domain_id),
				Error::<T>::DestDomainNotSupported
			);
			// Check if dest domain bridge is paused
			ensure!(!IsPaused::<T>::get(proposal.origin_domain_id), Error::<T>::BridgePaused);
			// Check if proposal has executed
			ensure!(
				!Self::is_proposal_executed(proposal.deposit_nonce, proposal.origin_domain_id),
				Error::<T>::ProposalAlreadyComplete
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
		use crate::{
			DestChainIds, DestDomainIds, Error, Event as SygmaBridgeEvent, IsPaused, MpcAddr,
			Proposal,
		};
		use alloc::vec;
		use bridge::mock::{
			assert_events, new_test_ext, AccessSegregator, Assets, Balances, BridgeAccount,
			BridgePalletIndex, NativeLocation, NativeResourceId, Runtime, RuntimeEvent,
			RuntimeOrigin as Origin, SygmaBasicFeeHandler, SygmaBridge, TreasuryAccount,
			UsdcAssetId, UsdcLocation, UsdcResourceId, ALICE, ASSET_OWNER, BOB, DEST_DOMAIN_ID,
			ENDOWED_BALANCE,
		};
		use codec::Encode;
		use frame_support::{
			assert_noop, assert_ok, crypto::ecdsa::ECDSAExt,
			traits::tokens::fungibles::Create as FungibleCerate,
		};
		use primitive_types::U256;
		use sp_core::{ecdsa, Pair};
		use sp_runtime::WeakBoundedVec;
		use sp_std::convert::TryFrom;
		use sygma_traits::{MpcAddress, TransferType};
		use xcm::latest::prelude::*;

		#[test]
		fn set_mpc_address() {
			new_test_ext().execute_with(|| {
				let default_addr: MpcAddress = MpcAddress::default();
				let test_mpc_addr_a: MpcAddress = MpcAddress([1u8; 20]);
				let test_mpc_addr_b: MpcAddress = MpcAddress([2u8; 20]);

				assert_eq!(MpcAddr::<Runtime>::get(), default_addr);

				// set to test_mpc_addr_a
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr_a));
				assert_eq!(MpcAddr::<Runtime>::get(), test_mpc_addr_a);

				// set to test_mpc_addr_b: should be MpcAddrNotUpdatable error
				assert_noop!(
					SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr_b),
					bridge::Error::<Runtime>::MpcAddrNotUpdatable
				);

				// permission test: unauthorized account should not be able to set mpc address
				let unauthorized_account = Origin::from(Some(ALICE));
				assert_noop!(
					SygmaBridge::set_mpc_address(unauthorized_account, test_mpc_addr_a),
					bridge::Error::<Runtime>::AccessDenied
				);
				assert_eq!(MpcAddr::<Runtime>::get(), test_mpc_addr_a);
			})
		}

		#[test]
		fn pause_bridge() {
			new_test_ext().execute_with(|| {
				let default_addr = MpcAddress::default();
				let test_mpc_addr_a: MpcAddress = MpcAddress([1u8; 20]);

				assert_eq!(MpcAddr::<Runtime>::get(), default_addr);

				// pause bridge when mpc address is not set, should be err
				assert_noop!(
					SygmaBridge::pause_bridge(Origin::root(), DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::MissingMpcAddress
				);

				// set mpc address to test_key_a
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr_a));
				assert_eq!(MpcAddr::<Runtime>::get(), test_mpc_addr_a);
				// register domain
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				// pause bridge again, should be ok
				assert_ok!(SygmaBridge::pause_bridge(Origin::root(), DEST_DOMAIN_ID));
				assert!(IsPaused::<Runtime>::get(DEST_DOMAIN_ID));
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::BridgePaused {
					dest_domain_id: DEST_DOMAIN_ID,
				})]);

				// pause bridge again after paused, should be ok
				assert_ok!(SygmaBridge::pause_bridge(Origin::root(), DEST_DOMAIN_ID));
				assert!(IsPaused::<Runtime>::get(DEST_DOMAIN_ID));
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::BridgePaused {
					dest_domain_id: DEST_DOMAIN_ID,
				})]);

				// permission test: unauthorized account should not be able to pause bridge
				let unauthorized_account = Origin::from(Some(ALICE));
				assert_noop!(
					SygmaBridge::pause_bridge(unauthorized_account, DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::AccessDenied
				);
				assert!(IsPaused::<Runtime>::get(DEST_DOMAIN_ID));
			})
		}

		#[test]
		fn unpause_bridge() {
			new_test_ext().execute_with(|| {
				let default_addr: MpcAddress = MpcAddress::default();
				let test_mpc_addr_a: MpcAddress = MpcAddress([1u8; 20]);

				assert_eq!(MpcAddr::<Runtime>::get(), default_addr);

				// unpause bridge when mpc address is not set, should be error
				assert_noop!(
					SygmaBridge::unpause_bridge(Origin::root(), DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::MissingMpcAddress
				);

				// set mpc address to test_key_a and pause bridge
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr_a));
				assert_eq!(MpcAddr::<Runtime>::get(), test_mpc_addr_a);
				// register domain
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				assert_ok!(SygmaBridge::pause_bridge(Origin::root(), DEST_DOMAIN_ID));
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::BridgePaused {
					dest_domain_id: DEST_DOMAIN_ID,
				})]);

				// bridge should be paused here
				assert!(IsPaused::<Runtime>::get(DEST_DOMAIN_ID));

				// ready to unpause bridge, should be ok
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root(), DEST_DOMAIN_ID));
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::BridgeUnpaused {
					dest_domain_id: DEST_DOMAIN_ID,
				})]);

				// try to unpause it again, should be error
				assert_noop!(
					SygmaBridge::unpause_bridge(Origin::root(), DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::BridgeUnpaused
				);

				// permission test: unauthorized account should not be able to unpause a recognized
				// bridge
				let unauthorized_account = Origin::from(Some(ALICE));
				assert_noop!(
					SygmaBridge::unpause_bridge(unauthorized_account, DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::AccessDenied
				);
				assert!(!IsPaused::<Runtime>::get(DEST_DOMAIN_ID));
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
				assert!(!SygmaBridge::verify_by_mpc_address(&proposals, signature.encode()));
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
				assert!(!SygmaBridge::verify_by_mpc_address(&proposals, signature.encode()));
			})
		}

		#[test]
		fn verify_mpc_signature_valid_message_unmatched_mpc() {
			new_test_ext().execute_with(|| {
				// generate the signing keypair
				let (pair, _): (ecdsa::Pair, _) = Pair::generate();

				// set mpc address to another random key
				let test_mpc_addr: MpcAddress = MpcAddress([7u8; 20]);
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_eq!(MpcAddr::<Runtime>::get(), test_mpc_addr);

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

				// verify signature, should be false because the signing address != mpc address
				assert!(!SygmaBridge::verify_by_mpc_address(&proposals, signature.encode()));
			})
		}

		#[test]
		fn verify_mpc_signature_valid_message_valid_signature() {
			new_test_ext().execute_with(|| {
				// generate mpc keypair
				let (pair, _): (ecdsa::Pair, _) = Pair::generate();
				let test_mpc_addr: MpcAddress = MpcAddress(pair.public().to_eth_address().unwrap());

				// set mpc address to generated keypair's address
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_eq!(MpcAddr::<Runtime>::get(), test_mpc_addr);

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
				assert!(SygmaBridge::verify_by_mpc_address(&proposals, signature.encode()));
			})
		}

		#[test]
		fn deposit_native_asset_should_work() {
			new_test_ext().execute_with(|| {
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				let fee = 100u128;
				let amount = 200u128;

				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					NativeLocation::get().into(),
					fee
				));
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					(Concrete(NativeLocation::get()), Fungible(amount)).into(),
					(
						0,
						X2(
							GeneralKey(
								WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
							),
							GeneralIndex(1)
						)
					)
						.into(),
				));
				// Check balances
				assert_eq!(Balances::free_balance(ALICE), ENDOWED_BALANCE - amount);
				assert_eq!(Balances::free_balance(BridgeAccount::get()), amount - fee);
				assert_eq!(Balances::free_balance(TreasuryAccount::get()), fee);
				// Check event
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::Deposit {
					dest_domain_id: DEST_DOMAIN_ID,
					resource_id: NativeResourceId::get(),
					deposit_nonce: 0,
					sender: ALICE,
					transfer_type: TransferType::FungibleTransfer,
					deposit_data: SygmaBridge::create_deposit_data(
						amount - fee,
						b"ethereum recipient".to_vec(),
					),
					handler_response: vec![],
				})]);
			})
		}

		#[test]
		fn hex_zero_padding_32_test() {
			new_test_ext().execute_with(|| {
				assert_eq!(
					SygmaBridge::hex_zero_padding_32(100).to_vec(),
					vec![
						0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
						0, 0, 0, 0, 0, 0, 100
					]
				);
				let recipient = String::from("0x95ECF5ae000e0fe0e0dE63aDE9b7D82a372038b4");
				assert_eq!(
					SygmaBridge::hex_zero_padding_32(recipient.len() as u128).to_vec(),
					vec![
						0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
						0, 0, 0, 0, 0, 0, 42
					]
				);
			})
		}

		#[test]
		fn create_deposit_data_test() {
			new_test_ext().execute_with(|| {
				let recipient = b"0x95ECF5ae000e0fe0e0dE63aDE9b7D82a372038b4".to_vec();
				let data = SygmaBridge::create_deposit_data(100, recipient);
				// 32 + 32 + 42
				assert_eq!(data.len(), 106);
				assert_eq!(
					data.to_vec(),
					vec![
						0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
						0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
						0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 48, 120, 57, 53, 69, 67, 70,
						53, 97, 101, 48, 48, 48, 101, 48, 102, 101, 48, 101, 48, 100, 69, 54, 51,
						97, 68, 69, 57, 98, 55, 68, 56, 50, 97, 51, 55, 50, 48, 51, 56, 98, 52
					]
				);
			})
		}

		#[test]
		fn deposit_foreign_asset_should_work() {
			new_test_ext().execute_with(|| {
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				let fee = 100u128;
				let amount = 200u128;

				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					UsdcLocation::get().into(),
					fee
				));
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
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
						X2(
							GeneralKey(
								WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
							),
							GeneralIndex(1)
						)
					)
						.into(),
				));
				// Check balances
				assert_eq!(Assets::balance(UsdcAssetId::get(), &ALICE), ENDOWED_BALANCE - amount);
				assert_eq!(Assets::balance(UsdcAssetId::get(), &BridgeAccount::get()), 0);
				assert_eq!(Assets::balance(UsdcAssetId::get(), &TreasuryAccount::get()), fee);
				// Check event
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::Deposit {
					dest_domain_id: DEST_DOMAIN_ID,
					resource_id: UsdcResourceId::get(),
					deposit_nonce: 0,
					sender: ALICE,
					transfer_type: TransferType::FungibleTransfer,
					deposit_data: SygmaBridge::create_deposit_data(
						amount - fee,
						b"ethereum recipient".to_vec(),
					),
					handler_response: vec![],
				})]);
			})
		}

		#[test]
		fn deposit_unbounded_asset_should_fail() {
			new_test_ext().execute_with(|| {
				let unbounded_asset_location = MultiLocation::new(1, X1(GeneralIndex(123)));
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				let fee = 100u128;
				let amount = 200u128;

				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					unbounded_asset_location.clone().into(),
					fee
				));
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(unbounded_asset_location), Fungible(amount)).into(),
						(
							0,
							X2(
								GeneralKey(
									WeakBoundedVec::try_from(b"ethereum recipient".to_vec())
										.unwrap()
								),
								GeneralIndex(1)
							)
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
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				let fee = 100u128;
				let amount = 200u128;

				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					NativeLocation::get().into(),
					fee
				));
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(NativeLocation::get()), Fungible(amount)).into(),
						invalid_dest,
					),
					bridge::Error::<Runtime>::ExtractDestDataFailed
				);
			})
		}

		#[test]
		fn deposit_without_fee_set_should_fail() {
			new_test_ext().execute_with(|| {
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				let amount = 200u128;
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(NativeLocation::get()), Fungible(amount)).into(),
						(
							0,
							X2(
								GeneralKey(
									WeakBoundedVec::try_from(b"ethereum recipient".to_vec())
										.unwrap()
								),
								GeneralIndex(1)
							)
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
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				let fee = 200u128;
				let amount = 100u128;

				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					NativeLocation::get().into(),
					fee
				));
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(NativeLocation::get()), Fungible(amount)).into(),
						(
							0,
							X2(
								GeneralKey(
									WeakBoundedVec::try_from(b"ethereum recipient".to_vec())
										.unwrap()
								),
								GeneralIndex(1)
							)
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
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				let fee = 100u128;
				let amount = 200u128;

				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					NativeLocation::get().into(),
					fee
				));
				// register domain
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				// Pause bridge
				assert_ok!(SygmaBridge::pause_bridge(Origin::root(), DEST_DOMAIN_ID));
				// Should failed
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(NativeLocation::get()), Fungible(amount)).into(),
						(
							0,
							X2(
								GeneralKey(
									WeakBoundedVec::try_from(b"ethereum recipient".to_vec())
										.unwrap()
								),
								GeneralIndex(1)
							)
						)
							.into(),
					),
					bridge::Error::<Runtime>::BridgePaused
				);
				// Unpause bridge
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root(), DEST_DOMAIN_ID));
				// Should success
				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					(Concrete(NativeLocation::get()), Fungible(amount)).into(),
					(
						0,
						X2(
							GeneralKey(
								WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
							),
							GeneralIndex(1)
						)
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
					DEST_DOMAIN_ID,
					NativeLocation::get().into(),
					fee
				));
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						(Concrete(NativeLocation::get()), Fungible(amount)).into(),
						(
							0,
							X2(
								GeneralKey(
									WeakBoundedVec::try_from(b"ethereum recipient".to_vec())
										.unwrap()
								),
								GeneralIndex(1)
							)
						)
							.into(),
					),
					bridge::Error::<Runtime>::MissingMpcAddress
				);
			})
		}

		#[test]
		fn retry_bridge() {
			new_test_ext().execute_with(|| {
				// mpc address is missing, should fail
				assert_noop!(
					SygmaBridge::retry(
						Origin::signed(ALICE),
						1234567u128,
						1234u128,
						DEST_DOMAIN_ID
					),
					bridge::Error::<Runtime>::MissingMpcAddress
				);

				// set mpc address
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				// pause bridge and retry, should fail
				assert_ok!(SygmaBridge::pause_bridge(Origin::root(), DEST_DOMAIN_ID));
				assert_noop!(
					SygmaBridge::retry(
						Origin::signed(ALICE),
						1234567u128,
						1234u128,
						DEST_DOMAIN_ID
					),
					bridge::Error::<Runtime>::BridgePaused
				);

				// unpause bridge
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root(), DEST_DOMAIN_ID));
				assert!(!IsPaused::<Runtime>::get(DEST_DOMAIN_ID));

				// retry again, should work
				assert_ok!(SygmaBridge::retry(
					Origin::signed(ALICE),
					1234567u128,
					1234u128,
					DEST_DOMAIN_ID
				));
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
				// mpc address is missing, should fail
				assert_noop!(
					SygmaBridge::execute_proposal(Origin::signed(ALICE), vec![], vec![]),
					bridge::Error::<Runtime>::MissingMpcAddress,
				);
				// set mpc address to generated keypair's address
				let (pair, _): (ecdsa::Pair, _) = Pair::generate();
				let test_mpc_addr: MpcAddress = MpcAddress(pair.public().to_eth_address().unwrap());
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_eq!(MpcAddr::<Runtime>::get(), test_mpc_addr);
				// register domain
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				// Generate an evil key
				let (evil_pair, _): (ecdsa::Pair, _) = Pair::generate();

				// Deposit some native asset in advance
				let fee = 100u128;
				let amount = 200u128;
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					NativeLocation::get().into(),
					fee
				));
				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					(Concrete(NativeLocation::get()), Fungible(2 * amount)).into(),
					(
						0,
						X2(
							GeneralKey(
								WeakBoundedVec::try_from(b"ethereum recipient".to_vec()).unwrap()
							),
							GeneralIndex(1)
						)
					)
						.into(),
				));

				// Register foreign asset (USDC) with asset id 0
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdcAssetId::get(), ASSET_OWNER, true, 1,));

				// Generate proposals
				let valid_native_transfer_proposal = Proposal {
					origin_domain_id: DEST_DOMAIN_ID,
					deposit_nonce: 1,
					resource_id: NativeResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: Any, id: BOB.into() }))
							.encode(),
					),
				};
				let valid_usdc_transfer_proposal = Proposal {
					origin_domain_id: DEST_DOMAIN_ID,
					deposit_nonce: 2,
					resource_id: UsdcResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: Any, id: BOB.into() }))
							.encode(),
					),
				};
				let invalid_depositnonce_proposal = Proposal {
					origin_domain_id: DEST_DOMAIN_ID,
					deposit_nonce: 2,
					resource_id: NativeResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: Any, id: BOB.into() }))
							.encode(),
					),
				};
				let invalid_domainid_proposal = Proposal {
					origin_domain_id: 2,
					deposit_nonce: 3,
					resource_id: NativeResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: Any, id: BOB.into() }))
							.encode(),
					),
				};
				let invalid_resourceid_proposal = Proposal {
					origin_domain_id: DEST_DOMAIN_ID,
					deposit_nonce: 3,
					resource_id: [2u8; 32],
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: Any, id: BOB.into() }))
							.encode(),
					),
				};
				let invalid_recipient_proposal = Proposal {
					origin_domain_id: DEST_DOMAIN_ID,
					deposit_nonce: 3,
					resource_id: NativeResourceId::get(),
					data: SygmaBridge::create_deposit_data(amount, b"invalid recipient".to_vec()),
				};

				let proposals = vec![
					valid_native_transfer_proposal,
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

				// Should failed if dest domain 1 bridge paused
				assert_ok!(SygmaBridge::pause_bridge(Origin::root(), DEST_DOMAIN_ID));
				assert!(IsPaused::<Runtime>::get(DEST_DOMAIN_ID));
				assert_ok!(SygmaBridge::execute_proposal(
					Origin::signed(ALICE),
					proposals.clone(),
					proposals_with_valid_signature.encode()
				));
				// should emit FailedHandlerExecution event
				assert_events(vec![RuntimeEvent::SygmaBridge(
					SygmaBridgeEvent::FailedHandlerExecution {
						error: vec![66, 114, 105, 100, 103, 101, 80, 97, 117, 115, 101, 100],
						origin_domain_id: 1,
						deposit_nonce: 3,
					},
				)]);
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root(), DEST_DOMAIN_ID));

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
				assert!(SygmaBridge::verify_by_mpc_address(
					&proposals,
					proposals_with_valid_signature.encode()
				));
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
				assert!(!SygmaBridge::is_paused(DEST_DOMAIN_ID));

				// set mpc address
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				// register domain
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				// pause bridge
				assert_ok!(SygmaBridge::pause_bridge(Origin::root(), DEST_DOMAIN_ID));
				assert!(SygmaBridge::is_paused(DEST_DOMAIN_ID));

				// unpause bridge
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root(), DEST_DOMAIN_ID));
				assert!(!SygmaBridge::is_paused(DEST_DOMAIN_ID));
			})
		}

		#[test]
		fn access_control() {
			new_test_ext().execute_with(|| {
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);

				assert_noop!(
					SygmaBridge::set_mpc_address(Some(ALICE).into(), test_mpc_addr),
					bridge::Error::<Runtime>::AccessDenied
				);

				assert_noop!(
					SygmaBridge::pause_bridge(Some(BOB).into(), DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::AccessDenied
				);
				assert_noop!(
					SygmaBridge::unpause_bridge(Some(BOB).into(), DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::AccessDenied
				);

				// Grant ALICE the access of `set_mpc_address`
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"set_mpc_address".to_vec(),
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

				// BOB set mpc address should still failed
				assert_noop!(
					SygmaBridge::set_mpc_address(Some(BOB).into(), test_mpc_addr),
					bridge::Error::<Runtime>::AccessDenied
				);
				// ALICE set mpc address should work
				assert_ok!(SygmaBridge::set_mpc_address(Some(ALICE).into(), test_mpc_addr));
				// register domain
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				// ALICE pause&unpause bridge should still failed
				assert_noop!(
					SygmaBridge::pause_bridge(Some(ALICE).into(), DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::AccessDenied
				);
				assert_noop!(
					SygmaBridge::unpause_bridge(Some(ALICE).into(), DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::AccessDenied
				);
				// BOB pause&unpause bridge should work
				assert_ok!(SygmaBridge::pause_bridge(Some(BOB).into(), DEST_DOMAIN_ID));
				assert_ok!(SygmaBridge::unpause_bridge(Some(BOB).into(), DEST_DOMAIN_ID));
			})
		}

		#[test]
		fn multi_domain_test() {
			new_test_ext().execute_with(|| {
				// root register domainID 1 with chainID 0, should raise error MissingMpcAddress
				assert_noop!(
					SygmaBridge::register_domain(Origin::root(), 1u8, U256::from(0)),
					Error::<Runtime>::MissingMpcAddress
				);

				// set mpc address
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));

				// alice register domainID 1 with chainID 1, should raise error AccessDenied
				assert_noop!(
					SygmaBridge::register_domain(Origin::from(Some(ALICE)), 1u8, U256::from(1)),
					Error::<Runtime>::AccessDenied
				);
				// Grant ALICE the access of `register_domain`
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"register_domain".to_vec(),
					ALICE
				));
				// alice register domainID 1 with chainID 1, should be ok
				assert_ok!(SygmaBridge::register_domain(
					Origin::from(Some(ALICE)),
					1u8,
					U256::from(1)
				));
				// should emit RegisterDestDomain event
				assert_events(vec![RuntimeEvent::SygmaBridge(
					SygmaBridgeEvent::RegisterDestDomain {
						sender: ALICE,
						domain_id: 1,
						chain_id: U256::from(1),
					},
				)]);
				// storage check
				assert!(DestDomainIds::<Runtime>::get(1u8));
				assert_eq!(DestChainIds::<Runtime>::get(1u8).unwrap(), U256::from(1));

				// alice unregister domainID 1 with chainID 0, should raise error AccessDenied
				assert_noop!(
					SygmaBridge::unregister_domain(Origin::from(Some(ALICE)), 1u8, U256::from(0)),
					Error::<Runtime>::AccessDenied
				);
				// Grant ALICE the access of `unregister_domain`
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"unregister_domain".to_vec(),
					ALICE
				));
				// alice unregister domainID 1 with chainID 2, should raise error
				// DestChainIDNotMatch
				assert_noop!(
					SygmaBridge::unregister_domain(Origin::from(Some(ALICE)), 1u8, U256::from(2)),
					Error::<Runtime>::DestChainIDNotMatch
				);
				// alice unregister domainID 2 with chainID 2, should raise error
				// DestDomainNotSupported
				assert_noop!(
					SygmaBridge::unregister_domain(Origin::from(Some(ALICE)), 2u8, U256::from(2)),
					Error::<Runtime>::DestDomainNotSupported
				);
				// alice unregister domainID 1 with chainID 1, should success
				assert_ok!(SygmaBridge::unregister_domain(
					Origin::from(Some(ALICE)),
					1u8,
					U256::from(1)
				));
				// should emit UnregisterDestDomain event
				assert_events(vec![RuntimeEvent::SygmaBridge(
					SygmaBridgeEvent::UnregisterDestDomain {
						sender: ALICE,
						domain_id: 1,
						chain_id: U256::from(1),
					},
				)]);

				// storage check
				// DomainID 1 should not support anymore
				assert!(!DestDomainIds::<Runtime>::get(1u8));
				// corresponding chainID should be None since kv not exist anymore
				assert!(DestChainIds::<Runtime>::get(1u8).is_none());
			})
		}
	}
}
