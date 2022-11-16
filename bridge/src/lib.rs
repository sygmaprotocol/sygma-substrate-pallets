#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
mod mock;

pub use self::pallet::*;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use alloc::string::String;
	use codec::{Decode, Encode};
	use eth_encode_packed::{abi, SolidityDataType};
	use ethers::types::{transaction::eip712, H160, U256 as ethers_u256};
	use ethers_core::abi::{encode, Token};
	use frame_support::{
		dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion, transactional,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_core::{hash::H256, U256};
	use sp_io::{
		crypto::secp256k1_ecdsa_recover_compressed,
		hashing::{blake2_256, keccak_256},
	};
	use sp_runtime::{traits::Clear, RuntimeDebug};
	use sp_std::{convert::From, vec, vec::Vec};
	use sygma_traits::{DepositNonce, DomainID, FeeHandler, MpcPubkey, ResourceId};
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
	}

	#[allow(dead_code)]
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// When initial bridge transfer send to dest domain
		/// args: [dest_domain_id, resource_id, deposit_nonce, sender, deposit_data,
		/// handler_reponse]
		Deposit(DomainID, ResourceId, DepositNonce, T::AccountId, Vec<u8>, Vec<u8>),
		/// When user is going to retry a bridge transfer
		/// args: [tx_hash]
		Retry(H256),
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
	pub type MpcKey<T> = StorageValue<_, MpcPubkey, ValueQuery>;

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
		pub fn set_mpc_key(origin: OriginFor<T>, _key: MpcPubkey) -> DispatchResult {
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
			_origin: OriginFor<T>,
			_asset: MultiAsset,
			_dest: MultiLocation,
		) -> DispatchResult {
			// Asset transactor

			// Extract asset (MultiAsset) to get corresponding ResourceId

			// Extract dest (MultiLocation) to get corresponding DomainId and Etheruem address

			// Handle asset with Transactor, potential examples:
			// T::Transactor::withdraw_asset(fee + amount, sender_location);
			// T::Transactor::deposit_asset(fee, T::FeeReserveAccount::get().into());
			// T::Transactor::deposit_asset(amount, T::TransferReserveAccount::get().into());

			// Bump deposit nonce

			// Emit Deposit event

			Err(Error::<T>::Unimplemented.into())
		}

		/// This method is used to trigger the process for retrying failed deposits on the MPC side.
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn retry(_origin: OriginFor<T>, hash: H256) -> DispatchResult {
			// Emit retry event
			// For clippy happy
			Self::deposit_event(Event::<T>::Retry(hash));
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
			let _sig_result = &_signature.try_into();
			let _sig = match _sig_result {
				Ok(sig) => sig,
				Err(error) => return false,
			};

			// parse proposals and construct signing message
			let final_message = Pallet::<T>::construct_ecdsa_signing_proposals_data(&_proposals);

			// recover the signing pubkey
			if let Ok(_pubkey) =
				secp256k1_ecdsa_recover_compressed(_sig, &blake2_256(&final_message))
			{
				_pubkey == MpcKey::<T>::get().0
			} else {
				false
			}
		}

		/// Parse proposals and construct the original signing message
		pub fn construct_ecdsa_signing_proposals_data(proposals: &Vec<Proposal>) -> [u8; 32] {
			let _proposal_typehash = keccak_256(
				"Proposal(uint8 originDomainID,uint64 depositNonce,bytes32 resourceID,bytes data)"
					.as_bytes(),
			);

			let mut keccak_data = Vec::new();
			for prop in proposals {
				let _proposal_domain_id_token = Token::Uint(prop.origin_domain_id.into());
				let _proposal_deposit_nonce_token = Token::Uint(prop.deposit_nonce.into());
				let _proposal_resource_id_token = Token::FixedBytes(prop.resource_id.to_vec());
				let _proposal_data_token = Token::FixedBytes(keccak_256(&prop.data).to_vec());

				keccak_data.push(keccak_256(&encode(&[
					Token::FixedBytes(_proposal_typehash.to_vec()),
					_proposal_domain_id_token,
					_proposal_deposit_nonce_token,
					_proposal_resource_id_token,
					_proposal_data_token,
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
			let (_bytes, _) = abi::encode_packed(final_keccak_data_input);
			let hashed_keccak_data = keccak_256(_bytes.as_slice());

			let _struct_hash = keccak_256(&encode(&[
				Token::FixedBytes(_proposal_typehash.to_vec()),
				Token::FixedBytes(hashed_keccak_data.to_vec()),
			]));

			// domain separator
			let default_eip712_domain = eip712::EIP712Domain::default();
			let eip712_domain = eip712::EIP712Domain {
				name: String::from("Bridge"),
				version: String::from("3.1.0"),
				chain_id: ethers_u256([1u64; 4]),    // todo: how to get chain_id?
				verifying_contract: H160([1u8; 20]), // todo: how to get contract address?
				salt: default_eip712_domain.salt,
			};
			let _domain_separator = eip712_domain.separator();

			let typed_data_hash_input = &vec![
				SolidityDataType::String("\x19\x01"),
				SolidityDataType::Bytes(&_domain_separator),
				SolidityDataType::Bytes(&_struct_hash),
			];
			let (_bytes, _) = abi::encode_packed(typed_data_hash_input);

			keccak_256(_bytes.as_slice())
		}
	}

	#[cfg(test)]
	mod test {
		use crate as bridge;
		use crate::{Event as SygmaBridgeEvent, IsPaused, MpcKey, Proposal};
		use bridge::mock::{
			assert_events, new_test_ext, Runtime, RuntimeEvent, RuntimeOrigin as Origin,
			SygmaBridge, ALICE,
		};
		use codec::Encode;
		use frame_support::{assert_noop, assert_ok, sp_runtime::traits::BadOrigin};
		use sp_core::{ecdsa, Pair};
		use sygma_traits::MpcPubkey;

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
					BadOrigin
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
				assert_noop!(SygmaBridge::pause_bridge(unauthorized_account), BadOrigin);
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
				assert_noop!(SygmaBridge::unpause_bridge(unauthorized_account), BadOrigin);
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
				assert!(!SygmaBridge::verify(proposals, signature.encode()));
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
				assert!(!SygmaBridge::verify(proposals, signature.encode()));
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
				assert!(!SygmaBridge::verify(proposals, signature.encode()));
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
				assert!(SygmaBridge::verify(proposals, signature.encode()));
			})
		}
	}
}
