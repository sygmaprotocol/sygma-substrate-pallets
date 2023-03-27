// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use fixed::{types::extra::U16, FixedU128};
use frame_support::{pallet_prelude::*, traits::ContainsPair, PalletId};
use pallet_grandpa::AuthorityId as GrandpaId;
use polkadot_parachain::primitives::Sibling;
use primitive_types::U256;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, One, Verify,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	AccountId32, ApplyExtrinsicResult, MultiSignature, Perbill,
};
use sp_std::{borrow::Borrow, marker::PhantomData, prelude::*, result, vec::Vec};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use sygma_traits::{
	ChainID, DecimalConverter, DepositNonce, DomainID, ExtractDestinationData, ResourceId,
	VerifyingContractAddress,
};
use xcm::latest::{prelude::*, AssetId as XcmAssetId, MultiLocation};
use xcm_builder::{
	AccountId32Aliases, CurrencyAdapter, FungiblesAdapter, IsConcrete, NoChecking, ParentIsPreset,
	SiblingParachainConvertsVia,
};
use xcm_executor::traits::{Convert, Error as ExecutionError, MatchesFungibles};

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstU128, ConstU32, ConstU64, ConstU8, KeyOwnerProofSystem,
		Randomness, StorageInfo,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		IdentityFee, Weight,
	},
	StorageValue,
};
pub use frame_system::{Call as SystemCall, EnsureSigned};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{
	ConstFeeMultiplier, CurrencyAdapter as PaymentCurrencyAdapter, Multiplier,
};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
		}
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("sygma-substrate-pallet"),
	impl_name: create_runtime_str!("sygma-substrate-pallet"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 100,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::with_sensible_defaults(
			Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
			NORMAL_DISPATCH_RATIO,
		);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<32>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxAuthorities = ConstU32<32>;
	type MaxSetIdSessionEntries = ConstU64<0>;

	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: u128 = 500;

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = PaymentCurrencyAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	pub const UNIT: Balance = 1_000_000_000_000;
	pub const DOLLARS: Balance = UNIT::get();
	pub const CENTS: Balance = DOLLARS::get() / 100;
	pub const MILLICENTS: Balance = CENTS::get() / 1_000;
}

const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 15 * CENTS::get() + (bytes as Balance) * CENTS::get()
}

// Configure the sygma protocol.
parameter_types! {
	pub const AssetDeposit: Balance = 10 * UNIT::get(); // 10 UNITS deposit to create fungible asset class
	pub const AssetAccountDeposit: Balance = DOLLARS::get();
	pub const ApprovalDeposit: Balance = ExistentialDeposit::get();
	pub const AssetsStringLimit: u32 = 50;
	/// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = deposit(1, 68);
	pub const MetadataDepositPerByte: Balance = deposit(0, 1);
	pub const ExecutiveBody: BodyId = BodyId::Executive;
}

pub type AssetId = u32;
impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetId;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type RemoveItemsLimit = ConstU32<1000>;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	// Make sure put same value with `construct_runtime`
	pub const AccessSegregatorPalletIndex: u8 = 9;
	pub const FeeHandlerPalletIndex: u8 = 10;
	pub const BridgePalletIndex: u8 = 11;
	pub const FeeHandlerRouterPalletIndex: u8 = 12;
	// RegisteredExtrinsics here registers all valid (pallet index, extrinsic_name) paris
	// make sure to update this when adding new access control extrinsic
	pub RegisteredExtrinsics: Vec<(u8, Vec<u8>)> = [
		(AccessSegregatorPalletIndex::get(), b"grant_access".to_vec()),
		(FeeHandlerPalletIndex::get(), b"set_fee".to_vec()),
		(BridgePalletIndex::get(), b"set_mpc_address".to_vec()),
		(BridgePalletIndex::get(), b"pause_bridge".to_vec()),
		(BridgePalletIndex::get(), b"unpause_bridge".to_vec()),
		(BridgePalletIndex::get(), b"register_domain".to_vec()),
		(BridgePalletIndex::get(), b"unregister_domain".to_vec()),
		(BridgePalletIndex::get(), b"retry".to_vec()),
		(FeeHandlerRouterPalletIndex::get(), b"set_fee_handler".to_vec()),
	].to_vec();
}

impl sygma_access_segregator::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgeCommitteeOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type PalletIndex = AccessSegregatorPalletIndex;
	type Extrinsics = RegisteredExtrinsics;
}

impl sygma_basic_feehandler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgeCommitteeOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type PalletIndex = FeeHandlerPalletIndex;
}

impl sygma_fee_handler_router::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgeCommitteeOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type BasicFeeHandler = SygmaBasicFeeHandler;
	type DynamicFeeHandler = ();
	type PalletIndex = FeeHandlerRouterPalletIndex;
}

// This address is defined in the substrate E2E test of sygma-relayer
const DEST_VERIFYING_CONTRACT_ADDRESS: &str = "6CdE2Cd82a4F8B74693Ff5e194c19CA08c2d1c68";

parameter_types! {
	// TreasuryAccount is an substrate account and currently used for substrate -> EVM bridging fee collection
	// TreasuryAccount address: 5ELLU7ibt5ZrNEYRwohtaRBDBa3TzcWwwPELBPSWWd2mbgv3
	pub TreasuryAccount: AccountId32 = AccountId32::new([100u8; 32]);
	// BridgeAccount is an account for holding transferred asset collection
	// BridgeAccount address: 5EMepC39b7E2zfM9g6CkPp8KCAxGTh7D4w4T2tFjmjpd4tPw
	pub BridgeAccount: AccountId32 = AccountId32::new([101u8; 32]);
	// EIP712ChainID is the chainID that pallet is assigned with, used in EIP712 typed data domain
	pub EIP712ChainID: ChainID = U256::from(5);
	// DestVerifyingContractAddress is a H160 address that is used in proposal signature verification, specifically EIP712 typed data
	// When relayers signing, this address will be included in the EIP712Domain
	// As long as the relayer and pallet configured with the same address, EIP712Domain should be recognized properly.
	pub DestVerifyingContractAddress: VerifyingContractAddress = primitive_types::H160::from_slice(hex::decode(DEST_VERIFYING_CONTRACT_ADDRESS).ok().unwrap().as_slice());
	pub CheckingAccount: AccountId32 = AccountId32::new([102u8; 32]);
	pub RelayNetwork: NetworkId = NetworkId::Polkadot;
	pub AssetsPalletLocation: MultiLocation =
		PalletInstance(<Assets as PalletInfoAccess>::index() as u8).into();
	// NativeLocation is the representation of the current parachain's native asset location in substrate, it can be various on different parachains
	pub NativeLocation: MultiLocation = MultiLocation::here();
	// amount = 0 act as placeholder
	pub NativeAsset: MultiAsset = (Concrete(MultiLocation::here()), 0u128).into();
	// UsdcLocation is the representation of the USDC asset location in substrate
	// USDC is a foreign asset, and in our local testing env, it's being registered on Parachain 2004 with the following location
	pub UsdcLocation: MultiLocation = MultiLocation::new(
		1,
		X3(
			Parachain(2004),
			slice_to_generalkey(b"sygma"),
			slice_to_generalkey(b"usdc"),
		),
	);
	pub ERC20TSTLocation: MultiLocation = MultiLocation::new(
		1,
		X3(
			Parachain(2004),
			slice_to_generalkey(b"sygma"),
			slice_to_generalkey(b"erc20tst"),
		),
	);
	pub ERC20TSTD20Location: MultiLocation = MultiLocation::new(
		1,
		X3(
			Parachain(2004),
			slice_to_generalkey(b"sygma"),
			slice_to_generalkey(b"erc20tstd20"),
		),
	);
	// UsdcAssetId is the substrate assetID of USDC
	pub UsdcAssetId: AssetId = 2000;
	pub ERC20TSTAssetId: AssetId = 2001;
	pub ERC20TSTD20AssetId: AssetId = 2002;
	// NativeResourceId is the resourceID that mapping with the current parachain native asset
	pub NativeResourceId: ResourceId = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000001");
	// UsdcResourceId is the resourceID that mapping with the foreign asset USDC
	pub UsdcResourceId: ResourceId = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000300");
	pub ERC20TSTResourceId: ResourceId = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000000");
	pub ERC20TSTD20ResourceId: ResourceId = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000900");

	// ResourcePairs is where all supported assets and their associated resourceID are binding
	pub ResourcePairs: Vec<(XcmAssetId, ResourceId)> = vec![(NativeLocation::get().into(), NativeResourceId::get()), (UsdcLocation::get().into(), UsdcResourceId::get()), (ERC20TSTLocation::get().into(), ERC20TSTResourceId::get()), (ERC20TSTD20Location::get().into(), ERC20TSTD20ResourceId::get())];
	// SygmaBridgePalletId is the palletIDl
	// this is used as the replacement of handler address in the ProposalExecution event
	pub const SygmaBridgePalletId: PalletId = PalletId(*b"sygma/01");
	pub AssetDecimalPairs: Vec<(XcmAssetId, u8)> = vec![(NativeLocation::get().into(), 12u8), (UsdcLocation::get().into(), 12u8), (ERC20TSTLocation::get().into(), 18u8), (ERC20TSTD20Location::get().into(), 20u8)];
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId32>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId32>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId32>,
);

/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<NativeLocation>,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId32,
	// We don't track any teleports of `Balances`.
	(),
>;

/// A simple Asset converter that extract the bingding relationship between AssetId and
/// MultiLocation, And convert Asset transfer amount to Balance
pub struct SimpleForeignAssetConverter(PhantomData<()>);

impl Convert<MultiLocation, AssetId> for SimpleForeignAssetConverter {
	fn convert_ref(id: impl Borrow<MultiLocation>) -> result::Result<AssetId, ()> {
		if &UsdcLocation::get() == id.borrow() {
			Ok(UsdcAssetId::get())
		} else if &ERC20TSTLocation::get() == id.borrow() {
			Ok(ERC20TSTAssetId::get())
		} else if &ERC20TSTD20Location::get() == id.borrow() {
			Ok(ERC20TSTD20AssetId::get())
		} else {
			Err(())
		}
	}
	fn reverse_ref(what: impl Borrow<AssetId>) -> result::Result<MultiLocation, ()> {
		if *what.borrow() == UsdcAssetId::get() {
			Ok(UsdcLocation::get())
		} else if *what.borrow() == ERC20TSTAssetId::get() {
			Ok(ERC20TSTLocation::get())
		} else if *what.borrow() == ERC20TSTD20AssetId::get() {
			Ok(ERC20TSTD20Location::get())
		} else {
			Err(())
		}
	}
}

impl MatchesFungibles<AssetId, Balance> for SimpleForeignAssetConverter {
	fn matches_fungibles(a: &MultiAsset) -> result::Result<(AssetId, Balance), ExecutionError> {
		match (&a.fun, &a.id) {
			(Fungible(ref amount), Concrete(ref id)) =>
				if id == &UsdcLocation::get() {
					Ok((UsdcAssetId::get(), *amount))
				} else if id == &ERC20TSTLocation::get() {
					Ok((ERC20TSTAssetId::get(), *amount))
				} else if id == &ERC20TSTD20Location::get() {
					Ok((ERC20TSTD20AssetId::get(), *amount))
				} else {
					Err(ExecutionError::AssetNotHandled)
				},
			_ => Err(ExecutionError::AssetNotHandled),
		}
	}
}

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a fungible asset matching the given location or name:
	SimpleForeignAssetConverter,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId32,
	// Disable teleport.
	NoChecking,
	// The account to use for tracking teleports.
	CheckingAccount,
>;
/// Means for transacting assets on this chain.
pub type AssetTransactors = (CurrencyTransactor, FungiblesTransactor);

pub struct ConcrateSygmaAsset;
impl ConcrateSygmaAsset {
	pub fn id(asset: &MultiAsset) -> Option<MultiLocation> {
		match (&asset.id, &asset.fun) {
			// So far our native asset is concrete
			(Concrete(id), Fungible(_)) => Some(*id),
			_ => None,
		}
	}

	pub fn origin(asset: &MultiAsset) -> Option<MultiLocation> {
		Self::id(asset).and_then(|id| {
			match (id.parents, id.first_interior()) {
				// Sibling parachain
				(1, Some(Parachain(id))) => {
					// Assume current parachain id is 2004, for production, you should always get
					// your it from parachain info
					if *id == 2004 {
						// The registered foreign assets actually reserved on EVM chains, so when
						// transfer back to EVM chains, they should be treated as non-reserve assets
						// relative to current chain.
						Some(MultiLocation::new(0, X1(slice_to_generalkey(b"sygma"))))
					} else {
						// Other parachain assets should be treat as reserve asset when transfered
						// to outside EVM chains
						Some(MultiLocation::here())
					}
				},
				// Parent assets should be treat as reserve asset when transfered to outside EVM
				// chains
				(1, _) => Some(MultiLocation::here()),
				// Children parachain
				(0, Some(Parachain(id))) => Some(MultiLocation::new(0, X1(Parachain(*id)))),
				// Local: (0, Here)
				(0, None) => Some(id),
				_ => None,
			}
		})
	}
}

pub struct SygmaDecimalConverter<DecimalPairs>(PhantomData<DecimalPairs>);
impl<DecimalPairs: Get<Vec<(XcmAssetId, u8)>>> DecimalConverter
	for SygmaDecimalConverter<DecimalPairs>
{
	fn convert_to(asset: &MultiAsset) -> Option<u128> {
		match (&asset.fun, &asset.id) {
			(Fungible(amount), _) => {
				for (asset_id, decimal) in DecimalPairs::get().iter() {
					if *asset_id == asset.id {
						return if *decimal == 18 {
							Some(*amount)
						} else {
							type U112F16 = FixedU128<U16>;
							if *decimal > 18 {
								let a =
									U112F16::from_num(10u128.saturating_pow(*decimal as u32 - 18));
								let b = U112F16::from_num(*amount).checked_div(a);
								let r: u128 = b.unwrap_or_else(|| U112F16::from_num(0)).to_num();
								if r == 0 {
									return None
								}
								Some(r)
							} else {
								// Max is 5192296858534827628530496329220095
								// if source asset decimal is 12, the max amount sending to sygma
								// relayer is 5192296858534827.628530496329
								if *amount > U112F16::MAX {
									return None
								}
								let a =
									U112F16::from_num(10u128.saturating_pow(18 - *decimal as u32));
								let b = U112F16::from_num(*amount).saturating_mul(a);
								Some(b.to_num())
							}
						}
					}
				}
				None
			},
			_ => None,
		}
	}

	fn convert_from(asset: &MultiAsset) -> Option<MultiAsset> {
		match (&asset.fun, &asset.id) {
			(Fungible(amount), _) => {
				for (asset_id, decimal) in DecimalPairs::get().iter() {
					if *asset_id == asset.id {
						return if *decimal == 18 {
							Some((asset.id, *amount).into())
						} else {
							type U112F16 = FixedU128<U16>;
							if *decimal > 18 {
								// Max is 5192296858534827628530496329220095
								// if dest asset decimal is 24, the max amount coming from sygma
								// relayer is 5192296858.534827628530496329
								if *amount > U112F16::MAX {
									return None
								}
								let a =
									U112F16::from_num(10u128.saturating_pow(*decimal as u32 - 18));
								let b = U112F16::from_num(*amount).saturating_mul(a);
								let r: u128 = b.to_num();
								Some((asset.id, r).into())
							} else {
								let a =
									U112F16::from_num(10u128.saturating_pow(18 - *decimal as u32));
								let b = U112F16::from_num(*amount).checked_div(a);
								let r: u128 = b.unwrap_or_else(|| U112F16::from_num(0)).to_num();
								if r == 0 {
									return None
								}
								Some((asset.id, r).into())
							}
						}
					}
				}
				None
			},
			_ => None,
		}
	}
}

pub struct ReserveChecker;
impl ContainsPair<MultiAsset, MultiLocation> for ReserveChecker {
	fn contains(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		if let Some(ref id) = ConcrateSygmaAsset::origin(asset) {
			if id == origin {
				return true
			}
		}
		false
	}
}

// Project can have it's own implementation to adapt their own spec design.
pub struct DestinationDataParser;
impl ExtractDestinationData for DestinationDataParser {
	fn extract_dest(dest: &MultiLocation) -> Option<(Vec<u8>, DomainID)> {
		match (dest.parents, &dest.interior) {
			(
				0,
				Junctions::X2(
					GeneralKey { length: recipient_len, data: recipient },
					GeneralKey { length: _domain_len, data: dest_domain_id },
				),
			) => {
				let d = u8::default();
				let domain_id = dest_domain_id.as_slice().first().unwrap_or(&d);
				if *domain_id == d {
					return None
				}
				Some((recipient[..*recipient_len as usize].to_vec(), *domain_id))
			},
			_ => None,
		}
	}
}

impl sygma_bridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgeCommitteeOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type TransferReserveAccount = BridgeAccount;
	type FeeReserveAccount = TreasuryAccount;
	type EIP712ChainID = EIP712ChainID;
	type DestVerifyingContractAddress = DestVerifyingContractAddress;
	type FeeHandler = SygmaFeeHandlerRouter;
	type AssetTransactor = AssetTransactors;
	type ResourcePairs = ResourcePairs;
	type IsReserve = ReserveChecker;
	type ExtractDestData = DestinationDataParser;
	type PalletId = SygmaBridgePalletId;
	type PalletIndex = BridgePalletIndex;
	type DecimalConverter = SygmaDecimalConverter<AssetDecimalPairs>;
}

pub fn slice_to_generalkey(key: &[u8]) -> Junction {
	let len = key.len();
	assert!(len <= 32);
	GeneralKey {
		length: len as u8,
		data: {
			let mut data = [0u8; 32];
			data[..len].copy_from_slice(key);
			data
		},
	}
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime
	where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
		Timestamp: pallet_timestamp,
		Aura: pallet_aura,
		Grandpa: pallet_grandpa,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		Sudo: pallet_sudo,
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>} = 8,
		SygmaAccessSegregator: sygma_access_segregator::{Pallet, Call, Storage, Event<T>} = 9,
		SygmaBasicFeeHandler: sygma_basic_feehandler::{Pallet, Call, Storage, Event<T>} = 10,
		SygmaBridge: sygma_bridge::{Pallet, Call, Storage, Event<T>} = 11,
		SygmaFeeHandlerRouter: sygma_fee_handler_router::{Pallet, Call, Storage, Event<T>} = 12,
	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_template, TemplateModule]
	);
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sygma_runtime_api::SygmaBridgeApi<Block> for Runtime {
		fn is_proposal_executed(nonce: DepositNonce, domain_id: DomainID) -> bool {
			SygmaBridge::is_proposal_executed(nonce, domain_id)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			_authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, TrackedStorageKey};

			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::traits::WhitelistedStorageKeys;
	use sp_core::hexdisplay::HexDisplay;
	use std::collections::HashSet;

	#[test]
	fn check_whitelist() {
		let whitelist: HashSet<String> = AllPalletsWithSystem::whitelisted_storage_keys()
			.iter()
			.map(|e| HexDisplay::from(&e.key).to_string())
			.collect();

		// Block Number
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac")
		);
		// Total Issuance
		assert!(
			whitelist.contains("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80")
		);
		// Execution Phase
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a")
		);
		// Event Count
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850")
		);
		// System Events
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7")
		);
	}
}
