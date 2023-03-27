// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate arrayref;

pub use self::pallet::*;

mod eip712;
mod encode;

#[cfg(test)]
mod mock;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use crate::encode::{abi::encode_packed, SolidityDataType};
	use codec::{Decode, Encode};
	use ethabi::{encode as abi_encode, token::Token};
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{ContainsPair, StorageVersion},
		transactional, PalletId,
	};

	use frame_system::pallet_prelude::*;
	use primitive_types::U256;
	use scale_info::TypeInfo;
	use sp_io::{crypto::secp256k1_ecdsa_recover, hashing::keccak_256};
	use sp_runtime::{
		traits::{AccountIdConversion, Clear},
		RuntimeDebug,
	};
	use sp_std::{boxed::Box, convert::From, vec, vec::Vec};
	use xcm::latest::{prelude::*, MultiLocation};
	use xcm_executor::traits::TransactAsset;

	use crate::eip712;
	use sygma_traits::{
		ChainID, DecimalConverter, DepositNonce, DomainID, ExtractDestinationData, FeeHandler,
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

		/// Return true if asset reserved on current chain
		type IsReserve: ContainsPair<MultiAsset, MultiLocation>;

		/// Extract dest data from given MultiLocation
		type ExtractDestData: ExtractDestinationData;

		/// Config ID for the current pallet instance
		type PalletId: Get<PalletId>;

		/// Current pallet index defined in runtime
		type PalletIndex: Get<u8>;

		/// Asset decimal converter
		type DecimalConverter: DecimalConverter;
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
		/// args: [deposit_on_block_height, dest_domain_id, sender]
		Retry { deposit_on_block_height: u128, dest_domain_id: DomainID, sender: T::AccountId },
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
		/// Proposal list empty
		EmptyProposalList,
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
		/// Failed on the decimal converter
		DecimalConversionFail,
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
		#[pallet::call_index(0)]
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
			ensure!(DestDomainIds::<T>::get(dest_domain_id), Error::<T>::DestDomainNotSupported);

			// Mark as paused
			IsPaused::<T>::insert(dest_domain_id, true);

			// Emit BridgePause event
			Self::deposit_event(Event::BridgePaused { dest_domain_id });
			Ok(())
		}

		/// Unpause bridge.
		#[pallet::weight(195_000_000)]
		#[pallet::call_index(1)]
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
		#[pallet::call_index(2)]
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

			// unpause bridge
			Self::unpause_all_domains();

			Ok(())
		}

		/// Mark the give dest domainID with chainID to be enabled
		#[pallet::weight(195_000_000)]
		#[pallet::call_index(3)]
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
		#[pallet::call_index(4)]
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
		#[pallet::call_index(5)]
		pub fn deposit(
			origin: OriginFor<T>,
			asset: Box<MultiAsset>,
			dest: Box<MultiLocation>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(!MpcAddr::<T>::get().is_clear(), Error::<T>::MissingMpcAddress);

			// Extract dest (MultiLocation) to get corresponding dest domainID and Ethereum
			// recipient address
			let (recipient, dest_domain_id) =
				T::ExtractDestData::extract_dest(&dest).ok_or(Error::<T>::ExtractDestDataFailed)?;

			ensure!(!IsPaused::<T>::get(dest_domain_id), Error::<T>::BridgePaused);

			ensure!(DestDomainIds::<T>::get(dest_domain_id), Error::<T>::DestDomainNotSupported);

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
				&Junction::AccountId32 { network: None, id: sender.clone().into() }.into(),
				None,
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

			// Deposit `fee` of asset to treasury account
			T::AssetTransactor::deposit_asset(
				&(asset.id, Fungible(fee)).into(),
				&Junction::AccountId32 { network: None, id: T::FeeReserveAccount::get().into() }
					.into(),
				// Put empty message hash here because we are not sending XCM message
				&XcmContext::with_message_hash([0; 32]),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

			let bridge_amount = amount - fee;

			// Deposit `bridge_amount` of asset to reserve account if asset is reserved in local
			// chain.
			if T::IsReserve::contains(&asset, &MultiLocation::here()) {
				T::AssetTransactor::deposit_asset(
					&(asset.id, Fungible(bridge_amount)).into(),
					&Junction::AccountId32 {
						network: None,
						id: T::TransferReserveAccount::get().into(),
					}
					.into(),
					// Put empty message hash here because we are not sending XCM message
					&XcmContext::with_message_hash([0; 32]),
				)
				.map_err(|_| Error::<T>::TransactFailed)?;
			}

			// Bump deposit nonce
			let deposit_nonce = DepositCounts::<T>::get(dest_domain_id);
			DepositCounts::<T>::insert(dest_domain_id, deposit_nonce + 1);

			// convert the asset decimal
			let decimal_converted_amount =
				T::DecimalConverter::convert_to(&(asset.id, bridge_amount).into())
					.ok_or(Error::<T>::DecimalConversionFail)?;

			// Emit Deposit event
			Self::deposit_event(Event::Deposit {
				dest_domain_id,
				resource_id,
				deposit_nonce,
				sender,
				transfer_type,
				deposit_data: Self::create_deposit_data(decimal_converted_amount, recipient),
				handler_response: vec![],
			});

			Ok(())
		}

		/// This method is used to trigger the process for retrying failed deposits on the MPC side.
		#[pallet::weight(195_000_000)]
		#[transactional]
		#[pallet::call_index(6)]
		pub fn retry(
			origin: OriginFor<T>,
			deposit_on_block_height: u128,
			dest_domain_id: DomainID,
		) -> DispatchResult {
			let mut sender: T::AccountId = [0u8; 32].into();
			if <T as Config>::BridgeCommitteeOrigin::ensure_origin(origin.clone()).is_err() {
				// Ensure bridge committee or the account that has permission to register the dest
				// domain
				let who = ensure_signed(origin)?;
				ensure!(
					<sygma_access_segregator::pallet::Pallet<T>>::has_access(
						<T as Config>::PalletIndex::get(),
						b"retry".to_vec(),
						who.clone()
					),
					Error::<T>::AccessDenied
				);
				sender = who;
			}

			ensure!(!MpcAddr::<T>::get().is_clear(), Error::<T>::MissingMpcAddress);
			ensure!(!IsPaused::<T>::get(dest_domain_id), Error::<T>::BridgePaused);
			ensure!(DestDomainIds::<T>::get(dest_domain_id), Error::<T>::DestDomainNotSupported);

			// Emit retry event
			Self::deposit_event(Event::<T>::Retry {
				deposit_on_block_height,
				dest_domain_id,
				sender,
			});
			Ok(())
		}

		/// Executes a batch of deposit proposals (only if signature is signed by MPC).
		#[pallet::weight(195_000_000)]
		#[transactional]
		#[pallet::call_index(7)]
		pub fn execute_proposal(
			_origin: OriginFor<T>,
			proposals: Vec<Proposal>,
			signature: Vec<u8>,
		) -> DispatchResult {
			// Check MPC address and bridge status
			ensure!(!MpcAddr::<T>::get().is_clear(), Error::<T>::MissingMpcAddress);

			ensure!(!proposals.is_empty(), Error::<T>::EmptyProposalList);

			// parse proposals and construct signing message to meet EIP712 typed data
			let final_message = Self::construct_ecdsa_signing_proposals_data(&proposals);

			// Verify MPC signature
			ensure!(
				Self::verify_by_mpc_address(final_message, signature),
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
		/// Verifies that EIP712 typed proposal data is signed by MPC address
		#[allow(dead_code)]
		fn verify_by_mpc_address(signing_message: [u8; 32], signature: Vec<u8>) -> bool {
			let sig = match signature.try_into() {
				Ok(_sig) => _sig,
				Err(error) => return false,
			};

			// recover the signing address
			if let Ok(pubkey) =
				// recover the uncompressed pubkey
				secp256k1_ecdsa_recover(&sig, &signing_message)
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
			let proposals_typehash = keccak_256(
				"Proposals(Proposal[] proposals)Proposal(uint8 originDomainID,uint64 depositNonce,bytes32 resourceID,bytes data)"
					.as_bytes(),
			);
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
			let bytes = encode_packed(final_keccak_data_input);
			let hashed_keccak_data = keccak_256(bytes.as_slice());

			let struct_hash = keccak_256(&abi_encode(&[
				Token::FixedBytes(proposals_typehash.to_vec()),
				Token::FixedBytes(hashed_keccak_data.to_vec()),
			]));

			// domain separator
			let default_eip712_domain = eip712::EIP712Domain::default();
			let eip712_domain = eip712::EIP712Domain {
				name: b"Bridge".to_vec(),
				version: b"3.1.0".to_vec(),
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
			let bytes = encode_packed(typed_data_hash_input);
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
		/// Only fungible transfer is supported so far.
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
				.map(|idx| T::ResourcePairs::get()[idx].0)
		}

		fn hex_zero_padding_32(i: u128) -> [u8; 32] {
			let mut result = [0u8; 32];
			U256::from(i).to_big_endian(&mut result);
			result
		}

		/// Return true if deposit nonce has been used
		pub fn is_proposal_executed(nonce: DepositNonce, domain_id: DomainID) -> bool {
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
			// Check if dest domain bridge is paused
			ensure!(!IsPaused::<T>::get(proposal.origin_domain_id), Error::<T>::BridgePaused);
			// Check if domain is supported
			ensure!(
				DestDomainIds::<T>::get(proposal.origin_domain_id),
				Error::<T>::DestDomainNotSupported
			);
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

			// convert the asset decimal
			let decimal_converted_asset =
				T::DecimalConverter::convert_from(&(asset_id, amount).into())
					.ok_or(Error::<T>::DecimalConversionFail)?;

			// Withdraw `decimal_converted_asset` of asset from reserve account
			if T::IsReserve::contains(&decimal_converted_asset, &MultiLocation::here()) {
				T::AssetTransactor::withdraw_asset(
					&decimal_converted_asset,
					&Junction::AccountId32 {
						network: None,
						id: T::TransferReserveAccount::get().into(),
					}
					.into(),
					None,
				)
				.map_err(|_| Error::<T>::TransactFailed)?;
			}

			// Deposit `decimal_converted_asset` of asset to dest location
			T::AssetTransactor::deposit_asset(
				&decimal_converted_asset,
				&location,
				// Put empty message hash here because we are not sending XCM message
				&XcmContext::with_message_hash([0; 32]),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

			Ok(())
		}

		/// unpause all registered domains in the storage
		fn unpause_all_domains() {
			for bridge_pair in IsPaused::<T>::iter() {
				IsPaused::<T>::insert(bridge_pair.0, false);
			}
		}
	}

	#[cfg(test)]
	mod test {
		use crate as bridge;
		use crate::{
			mock::{AstrAssetId, AstrLocation, AstrResourceId},
			DestChainIds, DestDomainIds, Error, Event as SygmaBridgeEvent, IsPaused, MpcAddr,
			Proposal,
		};
		use bridge::mock::{
			assert_events, new_test_ext, slice_to_generalkey, AccessSegregator, Assets, Balances,
			BridgeAccount, BridgePalletIndex, NativeLocation, NativeResourceId, Runtime,
			RuntimeEvent, RuntimeOrigin as Origin, SygmaBasicFeeHandler, SygmaBridge,
			TreasuryAccount, UsdcAssetId, UsdcLocation, UsdcResourceId, ALICE, ASSET_OWNER, BOB,
			DEST_DOMAIN_ID, ENDOWED_BALANCE,
		};
		use codec::{self, Encode};
		use frame_support::{
			assert_noop, assert_ok, crypto::ecdsa::ECDSAExt,
			traits::tokens::fungibles::Create as FungibleCerate,
		};
		use primitive_types::U256;
		use sp_core::{ecdsa, Pair};
		use sp_std::{boxed::Box, vec};
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
				assert_eq!(MpcAddr::<Runtime>::get(), default_addr);

				// register domain
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				// pause bridge, should be ok
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
				assert_eq!(MpcAddr::<Runtime>::get(), default_addr);

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

				let final_message = SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals);

				// should be false
				assert!(!SygmaBridge::verify_by_mpc_address(final_message, signature.encode()));
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

				let final_message = SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals);

				// verify non matched signature against proposal list, should be false
				assert!(!SygmaBridge::verify_by_mpc_address(final_message, signature.encode()));
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
				let signature = pair.sign_prehashed(&final_message);

				// verify signature, should be false because the signing address != mpc address
				assert!(!SygmaBridge::verify_by_mpc_address(final_message, signature.encode()));
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
				// `pari.sign` will hash the final message into blake2_256 then sign it, so use
				// sign_prehashed here
				let signature = pair.sign_prehashed(&final_message);

				// verify signature, should be true
				assert!(SygmaBridge::verify_by_mpc_address(final_message, signature.encode()));
			})
		}

		#[test]
		fn deposit_native_asset_should_work() {
			new_test_ext().execute_with(|| {
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				let fee = 1_000_000_000_000u128; // 1 with 12 decimals
				let amount = 200_000_000_000_000u128; // 200 with 12 decimals
				let final_amount_in_deposit_event = 199_000_000_000_000_000_000; // 200 - 1 then adjust to 18 decimals

				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					Box::new(NativeLocation::get().into()),
					fee
				));
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));

				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					Box::new((Concrete(NativeLocation::get()), Fungible(amount)).into()),
					Box::new(MultiLocation {
						parents: 0,
						interior: X2(
							slice_to_generalkey(b"ethereum recipient"),
							slice_to_generalkey(&[1]),
						)
					}),
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
						final_amount_in_deposit_event,
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

				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					Box::new(UsdcLocation::get().into()),
					fee
				));
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));

				// Register foreign asset (USDC) with asset id 0
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdcAssetId::get(), ASSET_OWNER, true, 1,));

				// Mint some USDC to ALICE for test
				assert_ok!(Assets::mint(
					Origin::signed(ASSET_OWNER),
					codec::Compact(0),
					ALICE,
					ENDOWED_BALANCE,
				));
				assert_eq!(Assets::balance(UsdcAssetId::get(), &ALICE), ENDOWED_BALANCE);

				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					Box::new((Concrete(UsdcLocation::get()), Fungible(amount)).into()),
					Box::new(MultiLocation {
						parents: 0,
						interior: X2(
							slice_to_generalkey(b"ethereum recipient"),
							slice_to_generalkey(&[1]),
						)
					}),
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
					Box::new(unbounded_asset_location.into()),
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
						Box::new((Concrete(unbounded_asset_location), Fungible(amount)).into()),
						Box::new(MultiLocation {
							parents: 0,
							interior: X2(
								slice_to_generalkey(b"ethereum recipient"),
								slice_to_generalkey(&[1]),
							)
						}),
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
					X2(GeneralIndex(0), slice_to_generalkey(b"ethereum recipient")),
				);
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				let fee = 100u128;
				let amount = 200u128;

				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					Box::new(NativeLocation::get().into()),
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
						Box::new((Concrete(NativeLocation::get()), Fungible(amount)).into()),
						Box::new(invalid_dest),
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
						Box::new((Concrete(NativeLocation::get()), Fungible(amount)).into()),
						Box::new(MultiLocation {
							parents: 0,
							interior: X2(
								slice_to_generalkey(b"ethereum recipient"),
								slice_to_generalkey(&[1]),
							)
						}),
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
					Box::new(NativeLocation::get().into()),
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
						Box::new((Concrete(NativeLocation::get()), Fungible(amount)).into()),
						Box::new(MultiLocation {
							parents: 0,
							interior: X2(
								slice_to_generalkey(b"ethereum recipient"),
								slice_to_generalkey(&[1]),
							)
						}),
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

				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					Box::new(NativeLocation::get().into()),
					fee
				));
				// register domain
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));
				// set mpc address will also unpause all bridges
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));

				// Pause bridge again
				assert_ok!(SygmaBridge::pause_bridge(Origin::root(), DEST_DOMAIN_ID));
				// Should failed
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						Box::new((Concrete(NativeLocation::get()), Fungible(amount)).into()),
						Box::new(MultiLocation {
							parents: 0,
							interior: X2(
								slice_to_generalkey(b"ethereum recipient"),
								slice_to_generalkey(&[1]),
							)
						}),
					),
					bridge::Error::<Runtime>::BridgePaused
				);
				// Unpause bridge
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root(), DEST_DOMAIN_ID));
				// Should success
				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					Box::new((Concrete(NativeLocation::get()), Fungible(amount)).into()),
					Box::new(MultiLocation {
						parents: 0,
						interior: X2(
							slice_to_generalkey(b"ethereum recipient"),
							slice_to_generalkey(&[1]),
						)
					}),
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
					Box::new(NativeLocation::get().into()),
					fee
				));
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						Box::new((Concrete(NativeLocation::get()), Fungible(amount)).into()),
						Box::new(MultiLocation {
							parents: 0,
							interior: X2(
								slice_to_generalkey(b"ethereum recipient"),
								slice_to_generalkey(&[1]),
							)
						}),
					),
					bridge::Error::<Runtime>::MissingMpcAddress
				);
			})
		}

		#[test]
		fn retry_bridge() {
			new_test_ext().execute_with(|| {
				// should be access denied SINCE Alice does not have permission to retry
				assert_noop!(
					SygmaBridge::retry(Origin::signed(ALICE), 1234567u128, DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::AccessDenied
				);

				// Grant ALICE the access of `retry`
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"retry".to_vec(),
					ALICE
				));

				// mpc address is missing, should fail
				assert_noop!(
					SygmaBridge::retry(Origin::signed(ALICE), 1234567u128, DEST_DOMAIN_ID),
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

				// pause bridge after set mpc address and retry, should fail
				assert_ok!(SygmaBridge::pause_bridge(Origin::root(), DEST_DOMAIN_ID));
				assert_noop!(
					SygmaBridge::retry(Origin::signed(ALICE), 1234567u128, DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::BridgePaused
				);

				// unpause bridge
				assert_ok!(SygmaBridge::unpause_bridge(Origin::root(), DEST_DOMAIN_ID));
				assert!(!IsPaused::<Runtime>::get(DEST_DOMAIN_ID));

				// retry again, should work
				assert_ok!(SygmaBridge::retry(Origin::signed(ALICE), 1234567u128, DEST_DOMAIN_ID));
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::Retry {
					deposit_on_block_height: 1234567u128,
					dest_domain_id: DEST_DOMAIN_ID,
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
				let fee = 1_000_000_000_000u128;
				let amount = 200_000_000_000_000u128;
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					Box::new(NativeLocation::get().into()),
					fee
				));
				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					Box::new((Concrete(NativeLocation::get()), Fungible(amount)).into()),
					Box::new(MultiLocation {
						parents: 0,
						interior: X2(
							slice_to_generalkey(b"ethereum recipient"),
							slice_to_generalkey(&[1]),
						)
					}),
				));

				// Register foreign asset (USDC) with asset id 0
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdcAssetId::get(), ASSET_OWNER, true, 1,));

				// Generate proposals
				// amount is in 18 decimal 0.000200000000000000, will be convert to 12 decimal
				// 0.000200000000
				let valid_native_transfer_proposal = Proposal {
					origin_domain_id: DEST_DOMAIN_ID,
					deposit_nonce: 1,
					resource_id: NativeResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: None, id: BOB.into() }))
							.encode(),
					),
				};
				// amount is in 18 decimal 0.000200000000000000, will be convert to 18 decimal
				// 0.000200000000000000
				let valid_usdc_transfer_proposal = Proposal {
					origin_domain_id: DEST_DOMAIN_ID,
					deposit_nonce: 2,
					resource_id: UsdcResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: None, id: BOB.into() }))
							.encode(),
					),
				};
				let invalid_depositnonce_proposal = Proposal {
					origin_domain_id: DEST_DOMAIN_ID,
					deposit_nonce: 2,
					resource_id: NativeResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: None, id: BOB.into() }))
							.encode(),
					),
				};
				let invalid_domainid_proposal = Proposal {
					origin_domain_id: 2,
					deposit_nonce: 3,
					resource_id: NativeResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: None, id: BOB.into() }))
							.encode(),
					),
				};
				let invalid_resourceid_proposal = Proposal {
					origin_domain_id: DEST_DOMAIN_ID,
					deposit_nonce: 3,
					resource_id: [2u8; 32],
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: None, id: BOB.into() }))
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

				let final_message = SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals);
				let proposals_with_valid_signature = pair.sign_prehashed(&final_message);
				let proposals_with_bad_signature = evil_pair.sign_prehashed(&final_message);

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
					final_message,
					proposals_with_valid_signature.encode()
				));
				assert_ok!(SygmaBridge::execute_proposal(
					Origin::signed(ALICE),
					proposals,
					proposals_with_valid_signature.encode(),
				));
				// proposal amount is in 18 decimal 0.000200000000000000, will be convert to 12
				// decimal 0.000200000000(200000000) because native asset is defined in 12 decimal
				assert_eq!(Balances::free_balance(&BOB), ENDOWED_BALANCE + 200000000);
				// usdc is defined in 18 decimal so that converted amount is the same as in proposal
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
				// root register domainID 1 with chainID 0, should be ok
				assert_ok!(SygmaBridge::register_domain(Origin::root(), 1u8, U256::from(0)));

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

		#[test]
		fn deposit_with_decimal_converter() {
			new_test_ext().execute_with(|| {
				let test_mpc_addr: MpcAddress = MpcAddress([1u8; 20]);
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));

				// native asset with 12 decimal
				let fee_native_asset = 1_000_000_000_000u128; // 1.0 native asset
				let amount_native_asset = 123_456_789_123_456u128; // 123.456_789_123_456
				let adjusted_amount_native_asset = 122_456_789_123_456_000_000u128; // amount_native_asset - fee_native_asset then adjust it to 18 decimals

				// usdc asset with 18 decimal
				let fee_usdc_asset = 1_000_000_000_000_000_000u128; // 1.0 usdc asset
				let amount_usdc_asset = 123_456_789_123_456_789_123u128; // 123.456_789_123_456_789_123
				let adjusted_amount_usdc_asset = 122_456_789_123_456_789_123u128; // amount_usdc_asset - fee_usdc_asset then adjust it to 18 decimals

				// astr asset with 24 decimal
				let fee_astr_asset = 1_000_000_000_000_000_000_000_000u128; // 1.0 astr asset
				let amount_astr_asset = 123_456_789_123_456_789_123_456_789u128; // 123.456_789_123_456_789_123_456_789
				let adjusted_amount_astr_asset = 122_456_789_123_456_789_123u128; // amount_astr_asset - fee_astr_asset then adjust it to 18 decimals

				// set fees
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					Box::new(NativeLocation::get().into()),
					fee_native_asset
				));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					Box::new(UsdcLocation::get().into()),
					fee_usdc_asset
				));
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					Box::new(AstrLocation::get().into()),
					fee_astr_asset
				));

				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));

				// deposit native asset which has 12 decimal
				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					Box::new(
						(Concrete(NativeLocation::get()), Fungible(amount_native_asset)).into()
					),
					Box::new(MultiLocation {
						parents: 0,
						interior: X2(
							slice_to_generalkey(b"ethereum recipient"),
							slice_to_generalkey(&[1]),
						)
					}),
				));
				// Check balances
				assert_eq!(Balances::free_balance(ALICE), ENDOWED_BALANCE - amount_native_asset);
				// native asset should be reserved so that BridgeAccount should hold it
				assert_eq!(
					Balances::free_balance(BridgeAccount::get()),
					amount_native_asset - fee_native_asset
				);
				// TreasuryAccount is collecting the bridging fee
				assert_eq!(Balances::free_balance(TreasuryAccount::get()), fee_native_asset);
				// Check event
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::Deposit {
					dest_domain_id: DEST_DOMAIN_ID,
					resource_id: NativeResourceId::get(),
					deposit_nonce: 0,
					sender: ALICE,
					transfer_type: TransferType::FungibleTransfer,
					deposit_data: SygmaBridge::create_deposit_data(
						adjusted_amount_native_asset,
						b"ethereum recipient".to_vec(),
					),
					handler_response: vec![],
				})]);

				// deposit usdc asset which has 18 decimal
				// Register foreign asset (usdc) with asset id 0
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdcAssetId::get(), ASSET_OWNER, true, 1,));

				// Mint some usdc to ALICE for test
				assert_ok!(Assets::mint(
					Origin::signed(ASSET_OWNER),
					codec::Compact(0),
					ALICE,
					ENDOWED_BALANCE,
				)); // make sure Alice owns enough funds here
				assert_eq!(Assets::balance(UsdcAssetId::get(), &ALICE), ENDOWED_BALANCE);

				// deposit
				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					Box::new((Concrete(UsdcLocation::get()), Fungible(amount_usdc_asset)).into()),
					Box::new(MultiLocation {
						parents: 0,
						interior: X2(
							slice_to_generalkey(b"ethereum recipient"),
							slice_to_generalkey(&[1]),
						)
					}),
				));
				// Check balances
				assert_eq!(
					Assets::balance(UsdcAssetId::get(), &ALICE),
					ENDOWED_BALANCE - amount_usdc_asset
				);
				// usdc asset should not be reserved so that BridgeAccount should not hold it
				assert_eq!(Assets::balance(UsdcAssetId::get(), &BridgeAccount::get()), 0);
				// TreasuryAccount is collecting the bridging fee
				assert_eq!(
					Assets::balance(UsdcAssetId::get(), TreasuryAccount::get()),
					fee_usdc_asset
				);

				// Check event
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::Deposit {
					dest_domain_id: DEST_DOMAIN_ID,
					resource_id: UsdcResourceId::get(),
					deposit_nonce: 1,
					sender: ALICE,
					transfer_type: TransferType::FungibleTransfer,
					deposit_data: SygmaBridge::create_deposit_data(
						adjusted_amount_usdc_asset,
						b"ethereum recipient".to_vec(),
					),
					handler_response: vec![],
				})]);

				// deposit astr asset which has 24 decimal
				// Register foreign asset (astr) with asset id 1
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(AstrAssetId::get(), ASSET_OWNER, true, 1,));

				// Mint some astr to ALICE for test
				assert_ok!(Assets::mint(
					Origin::signed(ASSET_OWNER),
					codec::Compact(1),
					ALICE,
					ENDOWED_BALANCE,
				)); // make sure Alice owns enough funds here
				assert_eq!(Assets::balance(AstrAssetId::get(), &ALICE), ENDOWED_BALANCE);

				// deposit
				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					Box::new((Concrete(AstrLocation::get()), Fungible(amount_astr_asset)).into()),
					Box::new(MultiLocation {
						parents: 0,
						interior: X2(
							slice_to_generalkey(b"ethereum recipient"),
							slice_to_generalkey(&[1]),
						)
					}),
				));
				// Check balances
				assert_eq!(
					Assets::balance(AstrAssetId::get(), &ALICE),
					ENDOWED_BALANCE - amount_astr_asset
				);
				// astr asset should be reserved so that BridgeAccount should hold it(Astr is not
				// defined in ConcrateSygmaAsset)
				assert_eq!(
					Assets::balance(AstrAssetId::get(), &BridgeAccount::get()),
					amount_astr_asset - fee_astr_asset
				);
				// TreasuryAccount is collecting the bridging fee
				assert_eq!(
					Assets::balance(AstrAssetId::get(), TreasuryAccount::get()),
					fee_astr_asset
				);

				// Check event
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::Deposit {
					dest_domain_id: DEST_DOMAIN_ID,
					resource_id: AstrResourceId::get(),
					deposit_nonce: 2,
					sender: ALICE,
					transfer_type: TransferType::FungibleTransfer,
					deposit_data: SygmaBridge::create_deposit_data(
						adjusted_amount_astr_asset,
						b"ethereum recipient".to_vec(),
					),
					handler_response: vec![],
				})]);

				// deposit astr asset which has 24 decimal, extreme small amount edge case
				let amount_astr_asset_extreme_small_amount = 100_000; // 0.000000000000000000100000 astr
				let fee_astr_asset_extreme_small_amount = 1; // 0.000000000000000000000001 astr
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					Box::new(AstrLocation::get().into()),
					fee_astr_asset_extreme_small_amount
				));
				// after decimal conversion from 24 to 18, the final amount will be 0 so that
				// decimal conversion will raise error deposit should not work
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						Box::new(
							(
								Concrete(AstrLocation::get()),
								Fungible(amount_astr_asset_extreme_small_amount)
							)
								.into()
						),
						Box::new(MultiLocation {
							parents: 0,
							interior: X2(
								slice_to_generalkey(b"ethereum recipient"),
								slice_to_generalkey(&[1]),
							)
						}),
					),
					bridge::Error::<Runtime>::DecimalConversionFail
				);
			})
		}

		#[test]
		fn proposal_execution_with_decimal_converter() {
			new_test_ext().execute_with(|| {
				// generate mpc keypair
				let (pair, _): (ecdsa::Pair, _) = Pair::generate();
				let test_mpc_addr: MpcAddress = MpcAddress(pair.public().to_eth_address().unwrap());
				// set mpc address
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				// register domain
				assert_ok!(SygmaBridge::register_domain(
					Origin::root(),
					DEST_DOMAIN_ID,
					U256::from(1)
				));
				let fee = 1_000_000_000_000u128; // 1 token in 12 decimals
				let init_deposit = 10_000_000_000_000u128; // 12 token in 12 decimal
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					Box::new(NativeLocation::get().into()),
					fee
				));
				// deposit in advance to make sure the native asset has enough funds in
				// TransferReserveAccount by doing this, Alice will deposit (half of her native
				// asset - fee) into TransferReserveAccount
				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					Box::new(
						(Concrete(NativeLocation::get()), Fungible(ENDOWED_BALANCE / 2)).into()
					),
					Box::new(MultiLocation {
						parents: 0,
						interior: X2(
							slice_to_generalkey(b"ethereum recipient"),
							slice_to_generalkey(&[1]),
						)
					}),
				));
				// BridgeAccount should have half of alice native asset - fee
				assert_eq!(Balances::free_balance(BridgeAccount::get()), ENDOWED_BALANCE / 2 - fee);
				// TreasuryAccount is collecting the bridging fee
				assert_eq!(Balances::free_balance(TreasuryAccount::get()), fee);

				let bridge_amount = 100_000_000_000_000_000_000; // 100 native with 18 decimals

				// proposal for bridging native asset to alice(native asset is 12 decimal)
				let p_native = Proposal {
					origin_domain_id: 1,
					resource_id: NativeResourceId::get(),
					deposit_nonce: 1,
					data: SygmaBridge::create_deposit_data(
						bridge_amount,
						MultiLocation::new(0, X1(AccountId32 { network: None, id: ALICE.into() }))
							.encode(),
					),
				};
				let proposals = vec![p_native];
				let final_message = SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals);
				let signature = pair.sign_prehashed(&final_message);

				// check Alice balance of native asset before executing, should have half of the
				// init native asset
				assert_eq!(Balances::free_balance(ALICE), ENDOWED_BALANCE / 2);
				assert_ok!(SygmaBridge::execute_proposal(
					Origin::signed(ALICE),
					proposals,
					signature.encode()
				));
				// check Alice balance of native asset after executing, should have half of the init
				// native asset + 100_000_000_000_000(12 decimal)
				assert_eq!(
					Balances::free_balance(ALICE),
					ENDOWED_BALANCE / 2 + 100_000_000_000_000
				);

				// proposal for bridging usdc asset to alice(usdc asset is 18 decimal)
				// Register foreign asset (usdc) with asset id 0
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(UsdcAssetId::get(), ASSET_OWNER, true, 1,));

				let p_usdc = Proposal {
					origin_domain_id: 1,
					deposit_nonce: 2,
					resource_id: UsdcResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						bridge_amount,
						MultiLocation::new(0, X1(AccountId32 { network: None, id: ALICE.into() }))
							.encode(),
					),
				};
				let proposals_usdc = vec![p_usdc];
				let final_message_usdc =
					SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals_usdc);
				let signature_usdc = pair.sign_prehashed(&final_message_usdc);

				// alice does not have any usdc at this moment
				assert_eq!(Assets::balance(UsdcAssetId::get(), &ALICE), 0);
				assert_ok!(SygmaBridge::execute_proposal(
					Origin::signed(ALICE),
					proposals_usdc,
					signature_usdc.encode()
				));
				// alice should have 100 usdc at this moment (100 usdc with 18 decimals)
				assert_eq!(
					Assets::balance(UsdcAssetId::get(), &ALICE),
					100_000_000_000_000_000_000
				);

				// proposal for bridging astr asset to alice(astr asset is 24 decimal)
				// Register foreign asset (astr) with asset id 1
				assert_ok!(<pallet_assets::pallet::Pallet<Runtime> as FungibleCerate<
					<Runtime as frame_system::Config>::AccountId,
				>>::create(AstrAssetId::get(), ASSET_OWNER, true, 1,));
				// Mint some astr to BridgeAccount for test because astr is reserved asset for
				// testing
				assert_ok!(Assets::mint(
					Origin::signed(ASSET_OWNER),
					codec::Compact(1),
					BridgeAccount::get(),
					ENDOWED_BALANCE
				));
				assert_eq!(
					Assets::balance(AstrAssetId::get(), BridgeAccount::get()),
					ENDOWED_BALANCE
				);

				let p_astr = Proposal {
					origin_domain_id: 1,
					deposit_nonce: 3,
					resource_id: AstrResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						bridge_amount,
						MultiLocation::new(0, X1(AccountId32 { network: None, id: ALICE.into() }))
							.encode(),
					),
				};
				let proposals_astr = vec![p_astr];
				let final_message_astr =
					SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals_astr);
				let signature_astr = pair.sign_prehashed(&final_message_astr);

				// alice does not have any astr at this moment
				assert_eq!(Assets::balance(AstrAssetId::get(), &ALICE), 0);
				assert_ok!(SygmaBridge::execute_proposal(
					Origin::signed(ALICE),
					proposals_astr,
					signature_astr.encode()
				));
				// alice should have 100 astr at this moment (100 astr with 24 decimals)
				assert_eq!(
					Assets::balance(AstrAssetId::get(), &ALICE),
					100_000_000_000_000_000_000_000_000
				);

				// extreme small amount edge case
				let extreme_small_bridge_amount = 100_000; // 0.000000000000100000 native asset with 18 decimals
										   // proposal for bridging native asset to alice(native asset is 12 decimal)
				let p_native_extreme = Proposal {
					origin_domain_id: 1,
					resource_id: NativeResourceId::get(),
					deposit_nonce: 4,
					data: SygmaBridge::create_deposit_data(
						extreme_small_bridge_amount,
						MultiLocation::new(0, X1(AccountId32 { network: None, id: ALICE.into() }))
							.encode(),
					),
				};
				let proposals_extreme = vec![p_native_extreme];
				let final_message_extreme =
					SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals_extreme);
				let signature_extreme = pair.sign_prehashed(&final_message_extreme);

				// execute_proposal extrinsic should work but it will actually failed at decimal
				// conversion step because 0.000000000000100000 in 18 decimal converts to 12 decimal
				// would be 0.000000000000 which is 0
				assert_ok!(SygmaBridge::execute_proposal(
					Origin::signed(ALICE),
					proposals_extreme,
					signature_extreme.encode()
				));
				// should emit FailedHandlerExecution event
				assert_events(vec![RuntimeEvent::SygmaBridge(
					SygmaBridgeEvent::FailedHandlerExecution {
						error: vec![
							68, 101, 99, 105, 109, 97, 108, 67, 111, 110, 118, 101, 114, 115, 105,
							111, 110, 70, 97, 105, 108,
						],
						origin_domain_id: 1,
						deposit_nonce: 4,
					},
				)]);
			})
		}

		#[test]
		fn unpause_all_domains_test() {
			new_test_ext().execute_with(|| {
				// Grant ALICE the access of `register_domain`
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"register_domain".to_vec(),
					ALICE
				));
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"pause_bridge".to_vec(),
					ALICE
				));
				// alice register some domains
				assert_ok!(SygmaBridge::register_domain(
					Origin::from(Some(ALICE)),
					1u8,
					U256::from(1)
				));
				assert_ok!(SygmaBridge::register_domain(
					Origin::from(Some(ALICE)),
					2u8,
					U256::from(2)
				));
				assert_ok!(SygmaBridge::register_domain(
					Origin::from(Some(ALICE)),
					3u8,
					U256::from(3)
				));

				// pause all
				assert_ok!(SygmaBridge::pause_bridge(Some(ALICE).into(), 1));
				assert_ok!(SygmaBridge::pause_bridge(Some(ALICE).into(), 2));
				assert_ok!(SygmaBridge::pause_bridge(Some(ALICE).into(), 3));

				// double check if they are all paused
				assert!(SygmaBridge::is_paused(1));
				assert!(SygmaBridge::is_paused(2));
				assert!(SygmaBridge::is_paused(3));

				SygmaBridge::unpause_all_domains();

				// all domains should be unpaused now
				assert!(!SygmaBridge::is_paused(1));
				assert!(!SygmaBridge::is_paused(2));
				assert!(!SygmaBridge::is_paused(3));
			})
		}

		#[test]
		fn setup_order_test() {
			new_test_ext().execute_with(|| {
				// Make sure mpc address is not set
				let default_addr: MpcAddress = MpcAddress::default();
				assert_eq!(MpcAddr::<Runtime>::get(), default_addr);

				// Grant ALICE the access admin extrinsics
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"register_domain".to_vec(),
					ALICE
				));
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"unregister_domain".to_vec(),
					ALICE
				));
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"pause_bridge".to_vec(),
					ALICE
				));
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"unpause_bridge".to_vec(),
					ALICE
				));
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					BridgePalletIndex::get(),
					b"retry".to_vec(),
					ALICE
				));

				// alice setup bridges without mpc address setup
				assert_ok!(SygmaBridge::register_domain(
					Origin::from(Some(ALICE)),
					DEST_DOMAIN_ID,
					U256::from(1)
				));
				assert_ok!(SygmaBridge::unregister_domain(
					Origin::from(Some(ALICE)),
					DEST_DOMAIN_ID,
					U256::from(1)
				));
				// register it back
				assert_ok!(SygmaBridge::register_domain(
					Origin::from(Some(ALICE)),
					DEST_DOMAIN_ID,
					U256::from(1)
				));
				assert_ok!(SygmaBridge::pause_bridge(Origin::from(Some(ALICE)), 1u8));
				assert_ok!(SygmaBridge::unpause_bridge(Origin::from(Some(ALICE)), 1u8));
				// pause domain 2 again to see if mpc address setup will unpause it
				assert_ok!(SygmaBridge::pause_bridge(Origin::from(Some(ALICE)), 1u8));

				// double check if it's paused
				assert!(SygmaBridge::is_paused(1));

				// retry should not work here, should raise MissingMpcAddress
				assert_noop!(
					SygmaBridge::retry(Origin::signed(ALICE), 1234567u128, DEST_DOMAIN_ID),
					bridge::Error::<Runtime>::MissingMpcAddress
				);
				// deposit should not work, should raise MissingMpcAddress
				assert_noop!(
					SygmaBridge::deposit(
						Origin::signed(ALICE),
						Box::new((Concrete(AstrLocation::get()), Fungible(100)).into()),
						Box::new(MultiLocation {
							parents: 0,
							interior: X2(
								slice_to_generalkey(b"ethereum recipient"),
								slice_to_generalkey(&[1]),
							)
						}),
					),
					bridge::Error::<Runtime>::MissingMpcAddress
				);
				// proposal execution should not work either, should raise MissingMpcAddress
				assert_noop!(
					SygmaBridge::execute_proposal(Origin::signed(ALICE), vec![], vec![]),
					bridge::Error::<Runtime>::MissingMpcAddress,
				);

				// set mpc address to generated keypair's address
				let (pair, _): (ecdsa::Pair, _) = Pair::generate();
				let test_mpc_addr: MpcAddress = MpcAddress(pair.public().to_eth_address().unwrap());
				assert_ok!(SygmaBridge::set_mpc_address(Origin::root(), test_mpc_addr));
				assert_eq!(MpcAddr::<Runtime>::get(), test_mpc_addr);

				// double check if it's unpause now
				assert!(!SygmaBridge::is_paused(1));

				// retry again, should work
				assert_ok!(SygmaBridge::retry(Origin::signed(ALICE), 1234567u128, DEST_DOMAIN_ID));
				assert_events(vec![RuntimeEvent::SygmaBridge(SygmaBridgeEvent::Retry {
					deposit_on_block_height: 1234567u128,
					dest_domain_id: DEST_DOMAIN_ID,
					sender: ALICE,
				})]);

				// deposit should work now
				let fee = 1_000_000_000_000u128;
				let amount = 200_000_000_000_000u128;
				assert_ok!(SygmaBasicFeeHandler::set_fee(
					Origin::root(),
					DEST_DOMAIN_ID,
					Box::new(NativeLocation::get().into()),
					fee
				));
				assert_ok!(SygmaBridge::deposit(
					Origin::signed(ALICE),
					Box::new((Concrete(NativeLocation::get()), Fungible(amount)).into()),
					Box::new(MultiLocation {
						parents: 0,
						interior: X2(
							slice_to_generalkey(b"ethereum recipient"),
							slice_to_generalkey(&[1]),
						)
					}),
				));
				// Check balances
				assert_eq!(Balances::free_balance(ALICE), ENDOWED_BALANCE - amount);
				assert_eq!(Balances::free_balance(BridgeAccount::get()), amount - fee);
				assert_eq!(Balances::free_balance(TreasuryAccount::get()), fee);

				// proposal execution should work
				let valid_native_transfer_proposal = Proposal {
					origin_domain_id: DEST_DOMAIN_ID,
					deposit_nonce: 1,
					resource_id: NativeResourceId::get(),
					data: SygmaBridge::create_deposit_data(
						amount,
						MultiLocation::new(0, X1(AccountId32 { network: None, id: BOB.into() }))
							.encode(),
					),
				};
				let proposals = vec![valid_native_transfer_proposal];
				let final_message = SygmaBridge::construct_ecdsa_signing_proposals_data(&proposals);
				let proposals_with_valid_signature = pair.sign_prehashed(&final_message);
				assert_ok!(SygmaBridge::execute_proposal(
					Origin::signed(ALICE),
					proposals,
					proposals_with_valid_signature.encode(),
				));
				// check native asset balance
				// proposal amount is in 18 decimal 0.000200000000000000, will be convert to 12
				// decimal 0.000200000000(200000000) because native asset is defined in 12 decimal
				assert_eq!(Balances::free_balance(&BOB), ENDOWED_BALANCE + 200000000);
			})
		}
	}
}
