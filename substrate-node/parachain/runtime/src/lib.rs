// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 1024.
#![recursion_limit = "1024"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. This means the client is \
        built with `SKIP_WASM_BUILD` flag and it is only usable for \
        production chains. Please rebuild with the flag disabled.",
	)
}

mod weights;
pub mod xcm_config;

use cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;
use cumulus_primitives_core::ParaId;
use fixed::{types::extra::U16, FixedU128};
pub use frame_support::{
	construct_runtime,
	traits::{
		AsEnsureOriginWithArg, ConstU128, Currency, KeyOwnerProofSystem, OnUnbalanced, Randomness,
		StorageInfo,
	},
	weights::IdentityFee,
	StorageValue,
};
use frame_support::{
	dispatch::DispatchClass,
	genesis_builder_helper::{build_config, create_default_config},
	parameter_types,
	traits::{ConstBool, ConstU32, ConstU64, ConstU8, EitherOfDiverse, Everything},
	weights::{
		constants::WEIGHT_REF_TIME_PER_SECOND, ConstantMultiplier, Weight, WeightToFeeCoefficient,
		WeightToFeeCoefficients, WeightToFeePolynomial,
	},
	PalletId,
};
use frame_support::{pallet_prelude::*, traits::ContainsPair};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
pub use frame_system::{Call as SystemCall, EnsureSigned};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use primitive_types::U256;
use smallvec::smallvec;
use sp_api::impl_runtime_apis;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, Verify},
	transaction_validity::{TransactionSource, TransactionValidity},
	AccountId32, ApplyExtrinsicResult, MultiSignature,
};
pub use sp_runtime::{MultiAddress, Perbill, Permill};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::{marker::PhantomData, prelude::*, result, vec::Vec};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use sygma_bridge_forwarder::xcm_asset_transactor::XCMAssetTransactor;
use sygma_traits::{
	AssetTypeIdentifier, ChainID, DecimalConverter, DepositNonce, DomainID, ExtractDestinationData,
	ResourceId, VerifyingContractAddress,
};
use xcm::latest::{prelude::*, AssetId as XcmAssetId, MultiLocation};
use xcm_builder::{CurrencyAdapter, FungiblesAdapter, IsConcrete, NoChecking};
use xcm_config::{RelayLocation, XcmConfig, XcmOriginToTransactDispatchOrigin};
use xcm_executor::traits::{Error as ExecutionError, MatchesFungibles};

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// Polkadot imports
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};

use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};

// XCM Imports
use xcm::latest::prelude::BodyId;
use xcm_executor::XcmExecutor;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An index to a block.
pub type BlockNumber = u32;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

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

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - `[0, MAXIMUM_BLOCK_WEIGHT]`
///   - `[Balance::min, Balance::max]`
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// in Rococo, extrinsic base weight (smallest non-zero weight) is mapped to 1 MILLIUNIT:
		// in our template, we map to 1/10 of that, or 1/10 MILLIUNIT
		let p = MILLIUNIT / 10;
		let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	use sp_runtime::{
		generic,
		traits::{BlakeTwo256, Hash as HashT},
	};

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
	/// Opaque block hash type.
	pub type Hash = <BlakeTwo256 as HashT>::Output;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("template-parachain"),
	impl_name: create_runtime_str!("template-parachain"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 0,
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
pub const MILLISECS_PER_BLOCK: u64 = 12000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const MILLIUNIT: Balance = 1_000_000_000;
pub const MICROUNIT: Balance = 1_000_000;

/// The existential deposit. Set to 1/10 of the Connected Relay Chain.
pub const EXISTENTIAL_DEPOSIT: Balance = MILLIUNIT;

/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
/// used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
/// `Operational` extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

/// We allow for 0.5 of a second of compute with a 12 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
	cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included
/// into the relay chain.
const UNINCLUDED_SEGMENT_CAPACITY: u32 = 1;
/// How many parachain blocks are processed by the relay chain per parent. Limits the
/// number of blocks authored per slot.
const BLOCK_PROCESSING_VELOCITY: u32 = 1;
/// Relay chain slot duration, in milliseconds.
const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;

	// This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
	//  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
	// `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
	// the lazy contract deletion.
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u16 = 42;
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = Everything;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The block type.
	type Block = Block;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The action to take on a Runtime Upgrade
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = (CollatorSelection,);
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type FreezeIdentifier = ();
	type MaxHolds = ConstU32<0>;
	type MaxFreezes = ConstU32<0>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
	// Unit = the base number of indivisible units for balances
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
	/// Relay Chain `TransactionByteFee` / 10
	pub const TransactionByteFee: Balance = 10 * MICROUNIT;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = ConstU8<5>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = ();
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = pallet_parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpMessageHandler = DmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
	type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
		Runtime,
		RELAY_CHAIN_SLOT_DURATION_MILLIS,
		BLOCK_PROCESSING_VELOCITY,
		UNINCLUDED_SEGMENT_CAPACITY,
	>;
}

impl pallet_parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = ();
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = ();
	type PriceForSiblingDelivery = ();
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = CollatorSelection;
	// Essentially just Aura, but let's be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	#[cfg(feature = "experimental")]
	type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Self>;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const SessionLength: BlockNumber = 6 * HOURS;
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

/// We allow root and the StakingAdmin to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureXcm<IsVoiceOfBody<RelayLocation, StakingAdminBodyId>>,
>;

impl pallet_collator_selection::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type UpdateOrigin = CollatorSelectionUpdateOrigin;
	type PotId = PotId;
	type MaxCandidates = ConstU32<100>;
	type MinEligibleCollators = ConstU32<4>;
	type MaxInvulnerables = ConstU32<20>;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = Period;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = ();
}

parameter_types! {
	// Make sure put same value with `construct_runtime`
	pub const AccessSegregatorPalletIndex: u8 = 9;
	pub const BasicFeeHandlerPalletIndex: u8 = 10;
	pub const BridgePalletIndex: u8 = 11;
	pub const FeeHandlerRouterPalletIndex: u8 = 12;
	pub const PercentageFeeHandlerRouterPalletIndex: u8 = 13;
	// RegisteredExtrinsics here registers all valid (pallet index, extrinsic_name) paris
	// make sure to update this when adding new access control extrinsic
	pub RegisteredExtrinsics: Vec<(u8, Vec<u8>)> = [
		(AccessSegregatorPalletIndex::get(), b"grant_access".to_vec()),
		(BasicFeeHandlerPalletIndex::get(), b"set_fee".to_vec()),
		(BridgePalletIndex::get(), b"set_mpc_address".to_vec()),
		(BridgePalletIndex::get(), b"pause_bridge".to_vec()),
		(BridgePalletIndex::get(), b"unpause_bridge".to_vec()),
		(BridgePalletIndex::get(), b"register_domain".to_vec()),
		(BridgePalletIndex::get(), b"unregister_domain".to_vec()),
		(BridgePalletIndex::get(), b"retry".to_vec()),
		(BridgePalletIndex::get(), b"pause_all_bridges".to_vec()),
		(BridgePalletIndex::get(), b"unpause_all_bridges".to_vec()),
		(FeeHandlerRouterPalletIndex::get(), b"set_fee_handler".to_vec()),
		(PercentageFeeHandlerRouterPalletIndex::get(), b"set_fee_rate".to_vec()),
	].to_vec();
}

impl sygma_access_segregator::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgeCommitteeOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type PalletIndex = AccessSegregatorPalletIndex;
	type Extrinsics = RegisteredExtrinsics;
	type WeightInfo = sygma_access_segregator::weights::SygmaWeightInfo<Runtime>;
}

impl sygma_basic_feehandler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletIndex = BasicFeeHandlerPalletIndex;
	type WeightInfo = sygma_basic_feehandler::weights::SygmaWeightInfo<Runtime>;
}

impl sygma_percentage_feehandler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletIndex = PercentageFeeHandlerRouterPalletIndex;
	type WeightInfo = sygma_percentage_feehandler::weights::SygmaWeightInfo<Runtime>;
}

impl sygma_fee_handler_router::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BasicFeeHandler = SygmaBasicFeeHandler;
	type DynamicFeeHandler = ();
	type PercentageFeeHandler = SygmaPercentageFeeHandler;
	type PalletIndex = FeeHandlerRouterPalletIndex;
	type WeightInfo = sygma_fee_handler_router::weights::SygmaWeightInfo<Runtime>;
}

// This address is defined in the substrate E2E test of sygma-relayer
const DEST_VERIFYING_CONTRACT_ADDRESS: &str = "6CdE2Cd82a4F8B74693Ff5e194c19CA08c2d1c68";

fn bridge_accounts_generator() -> BTreeMap<XcmAssetId, AccountId32> {
	let mut account_map: BTreeMap<XcmAssetId, AccountId32> = BTreeMap::new();
	account_map.insert(NativeLocation::get().into(), BridgeAccountNative::get());
	account_map.insert(UsdtLocation::get().into(), BridgeAccountOtherToken::get());
	account_map.insert(ERC20TSTLocation::get().into(), BridgeAccountOtherToken::get());
	account_map.insert(ERC20TSTD20Location::get().into(), BridgeAccountOtherToken::get());
	account_map
}

parameter_types! {
	// TreasuryAccount is an substrate account and currently used for substrate -> EVM bridging fee collection
	// TreasuryAccount address: 5ELLU7ibt5ZrNEYRwohtaRBDBa3TzcWwwPELBPSWWd2mbgv3
	pub TreasuryAccount: AccountId32 = AccountId32::new([100u8; 32]);
	// BridgeAccountNative: 5EYCAe5jLbHcAAMKvLFSXgCTbPrLgBJusvPwfKcaKzuf5X5e
	pub BridgeAccountNative: AccountId32 = SygmaBridgePalletId::get().into_account_truncating();
	// BridgeAccountOtherToken  5EYCAe5jLbHcAAMKvLFiGhk3htXY8jQncbLTDGJQnpnPMAVp
	pub BridgeAccountOtherToken: AccountId32 = SygmaBridgePalletId::get().into_sub_account_truncating(1u32);
	// BridgeAccounts is a list of accounts for holding transferred asset collection
	pub BridgeAccounts: BTreeMap<XcmAssetId, AccountId32> = bridge_accounts_generator();
	// EIP712ChainID is the chainID that pallet is assigned with, used in EIP712 typed data domain
	pub EIP712ChainID: ChainID = U256::from(5);
	// DestVerifyingContractAddress is a H160 address that is used in proposal signature verification, specifically EIP712 typed data
	// When relayers signing, this address will be included in the EIP712Domain
	// As long as the relayer and pallet configured with the same address, EIP712Domain should be recognized properly.
	pub DestVerifyingContractAddress: VerifyingContractAddress = primitive_types::H160::from_slice(hex::decode(DEST_VERIFYING_CONTRACT_ADDRESS).ok().unwrap().as_slice());
	pub CheckingAccount: AccountId32 = AccountId32::new([102u8; 32]);
	pub AssetsPalletLocation: MultiLocation =
		PalletInstance(<Assets as PalletInfoAccess>::index() as u8).into();
	// NativeLocation is the representation of the current parachain's native asset location in substrate, it can be various on different parachains
	pub NativeLocation: MultiLocation = MultiLocation::here();
	// UsdtLocation is the representation of the USDT asset location in substrate
	// USDT is a foreign asset, and in our local testing env, it's being registered on Parachain 2004 with the following location
	pub UsdtLocation: MultiLocation = MultiLocation::new(
		1,
		X3(
			Parachain(2005),
			slice_to_generalkey(b"sygma"),
			slice_to_generalkey(b"usdt"),
		),
	);
	pub ERC20TSTLocation: MultiLocation = MultiLocation::new(
		1,
		X3(
			Parachain(2005),
			slice_to_generalkey(b"sygma"),
			slice_to_generalkey(b"erc20tst"),
		),
	);
	pub ERC20TSTD20Location: MultiLocation = MultiLocation::new(
		1,
		X3(
			Parachain(2005),
			slice_to_generalkey(b"sygma"),
			slice_to_generalkey(b"erc20tstd20"),
		),
	);
	// UsdtAssetId is the substrate assetID of USDT
	pub UsdtAssetId: AssetId = 2000;
	pub ERC20TSTAssetId: AssetId = 2001;
	pub ERC20TSTD20AssetId: AssetId = 2002;
	// NativeResourceId is the resourceID that mapping with the current parachain native asset
	pub NativeResourceId: ResourceId = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000001");
	// UsdtResourceId is the resourceID that mapping with the foreign asset USDT
	pub UsdtResourceId: ResourceId = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000300");
	pub ERC20TSTResourceId: ResourceId = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000000");
	pub ERC20TSTD20ResourceId: ResourceId = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000900");

	// ResourcePairs is where all supported assets and their associated resourceID are binding
	pub ResourcePairs: Vec<(XcmAssetId, ResourceId)> = vec![(NativeLocation::get().into(), NativeResourceId::get()), (UsdtLocation::get().into(), UsdtResourceId::get()), (ERC20TSTLocation::get().into(), ERC20TSTResourceId::get()), (ERC20TSTD20Location::get().into(), ERC20TSTD20ResourceId::get())];
	// SygmaBridgePalletId is the palletIDl
	// this is used as the replacement of handler address in the ProposalExecution event
	pub const SygmaBridgePalletId: PalletId = PalletId(*b"sygma/01");
	pub AssetDecimalPairs: Vec<(XcmAssetId, u8)> = vec![(NativeLocation::get().into(), 12u8), (UsdtLocation::get().into(), 12u8), (ERC20TSTLocation::get().into(), 18u8), (ERC20TSTD20Location::get().into(), 20u8)];
}

/// A simple Asset converter that extract the bingding relationship between AssetId and
/// MultiLocation, And convert Asset transfer amount to Balance
pub struct SimpleForeignAssetConverter(PhantomData<()>);

impl MatchesFungibles<AssetId, Balance> for SimpleForeignAssetConverter {
	fn matches_fungibles(a: &MultiAsset) -> result::Result<(AssetId, Balance), ExecutionError> {
		match (&a.fun, &a.id) {
			(Fungible(ref amount), Concrete(ref id)) => {
				if id == &UsdtLocation::get() {
					Ok((UsdtAssetId::get(), *amount))
				} else if id == &ERC20TSTLocation::get() {
					Ok((ERC20TSTAssetId::get(), *amount))
				} else if id == &ERC20TSTD20Location::get() {
					Ok((ERC20TSTD20AssetId::get(), *amount))
				} else {
					Err(ExecutionError::AssetNotHandled)
				}
			},
			_ => Err(ExecutionError::AssetNotHandled),
		}
	}
}

/// Means for transacting assets on this chain.
pub type CurrencyTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<RelayLocation>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
	xcm_config::LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a fungible asset matching the given location or name:
	SimpleForeignAssetConverter,
	// Convert an XCM MultiLocation into a local account id:
	xcm_config::LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId32,
	// Disable teleport.
	NoChecking,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

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
									return None;
								}
								Some(r)
							} else {
								// Max is 5192296858534827628530496329220095
								// if source asset decimal is 12, the max amount sending to sygma
								// relayer is 5192296858534827.628530496329
								if *amount > U112F16::MAX {
									return None;
								}
								let a =
									U112F16::from_num(10u128.saturating_pow(18 - *decimal as u32));
								let b = U112F16::from_num(*amount).saturating_mul(a);
								Some(b.to_num())
							}
						};
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
									return None;
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
									return None;
								}
								Some((asset.id, r).into())
							}
						};
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
				return true;
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
				Junctions::X3(
					GeneralKey { length: path_len, data: sygma_path },
					GeneralIndex(dest_domain_id),
					GeneralKey { length: recipient_len, data: recipient },
				),
			) => {
				if sygma_path[..*path_len as usize] == [0x73, 0x79, 0x67, 0x6d, 0x61] {
					return TryInto::<DomainID>::try_into(*dest_domain_id).ok().map(|domain_id| {
						(recipient[..*recipient_len as usize].to_vec(), domain_id)
					});
				}
				None
			},
			_ => None,
		}
	}
}

impl sygma_bridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type TransferReserveAccounts = BridgeAccounts;
	type FeeReserveAccount = TreasuryAccount;
	type EIP712ChainID = EIP712ChainID;
	type DestVerifyingContractAddress = DestVerifyingContractAddress;
	type FeeHandler = SygmaFeeHandlerRouter;
	type AssetTransactor = XCMAssetTransactor<
		CurrencyTransactor,
		FungiblesTransactor,
		NativeAssetTypeIdentifier<ParachainInfo>,
		SygmaBridgeForwarder,
	>;
	type ResourcePairs = ResourcePairs;
	type IsReserve = ReserveChecker;
	type ExtractDestData = DestinationDataParser;
	type PalletId = SygmaBridgePalletId;
	type PalletIndex = BridgePalletIndex;
	type DecimalConverter = SygmaDecimalConverter<AssetDecimalPairs>;
	type WeightInfo = sygma_bridge::weights::SygmaWeightInfo<Runtime>;
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

/// NativeAssetTypeIdentifier impl AssetTypeIdentifier for XCMAssetTransactor
/// This impl is only for local mock purpose, the integrated parachain might have their own version
pub struct NativeAssetTypeIdentifier<T>(PhantomData<T>);
impl<T: Get<ParaId>> AssetTypeIdentifier for NativeAssetTypeIdentifier<T> {
	/// check if the given MultiAsset is a native asset
	fn is_native_asset(asset: &MultiAsset) -> bool {
		// currently there are two multilocations are considered as native asset:
		// 1. integrated parachain native asset(MultiLocation::here())
		// 2. other parachain native asset(MultiLocation::new(1, X1(Parachain(T::get().into()))))
		let native_locations =
			[MultiLocation::here(), MultiLocation::new(1, X1(Parachain(T::get().into())))];

		match (&asset.id, &asset.fun) {
			(Concrete(ref id), Fungible(_)) => native_locations.contains(id),
			_ => false,
		}
	}
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime {
		// System support stuff.
		System: frame_system = 0,
		ParachainSystem: cumulus_pallet_parachain_system = 1,
		Timestamp: pallet_timestamp = 2,
		ParachainInfo: pallet_parachain_info = 3,

		// Monetary stuff.
		Balances: pallet_balances = 10,
		TransactionPayment: pallet_transaction_payment = 11,
		Assets: pallet_assets = 12,

		// Governance
		Sudo: pallet_sudo = 15,

		// Collator support. The order of these 5 are important and shall not change.
		Authorship: pallet_authorship = 20,
		CollatorSelection: pallet_collator_selection = 21,
		Session: pallet_session = 22,
		Aura: pallet_aura = 23,
		AuraExt: cumulus_pallet_aura_ext = 24,

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue = 30,
		PolkadotXcm: pallet_xcm = 31,
		CumulusXcm: cumulus_pallet_xcm = 32,
		DmpQueue: cumulus_pallet_dmp_queue = 33,

		SygmaAccessSegregator: sygma_access_segregator::{Pallet, Call, Storage, Event<T>} = 40, // 9
		SygmaBasicFeeHandler: sygma_basic_feehandler::{Pallet, Call, Storage, Event<T>} = 41, // 10,
		SygmaBridge: sygma_bridge::{Pallet, Call, Storage, Event<T>} = 42, // 11
		SygmaFeeHandlerRouter: sygma_fee_handler_router::{Pallet, Call, Storage, Event<T>} = 43, // 12
		SygmaPercentageFeeHandler: sygma_percentage_feehandler::{Pallet, Call, Storage, Event<T>} = 44, // 13
		SygmaXcmBridge: sygma_xcm_bridge::{Pallet, Event<T>} = 45,
		SygmaBridgeForwarder: sygma_bridge_forwarder::{Pallet, Event<T>} = 46,
	}
);

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_timestamp, Timestamp]
		[pallet_sudo, Sudo]
		[pallet_collator_selection, CollatorSelection]
		[cumulus_pallet_xcmp_queue, XcmpQueue]
		[frame_benchmarking, BaselineBench::<Runtime>]
		[sygma_bridge, SygmaBridge::<Runtime>]
		[sygma_access_segregator, SygmaAccessSegregator::<Runtime>]
		[sygma_basic_feehandler, SygmaBasicFeeHandler::<Runtime>]
		[sygma_percentage_feehandler, SygmaPercentageFeeHandler::<Runtime>]
		[sygma_fee_handler_router, SygmaFeeHandlerRouter::<Runtime>]
	);
}

impl_runtime_apis! {
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
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

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
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

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect,
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use cumulus_pallet_session_benchmarking::Pallet as SessionBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{BenchmarkError, Benchmarking, BenchmarkBatch};

			use frame_system_benchmarking::Pallet as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {
				fn setup_set_code_requirements(code: &sp_std::vec::Vec<u8>) -> Result<(), BenchmarkError> {
					ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
					Ok(())
				}

				fn verify_set_code() {
					System::assert_last_event(cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into());
				}
			}

			use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
			impl cumulus_pallet_session_benchmarking::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn create_default_config() -> Vec<u8> {
			create_default_config::<RuntimeGenesisConfig>()
		}

		fn build_config(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_config::<RuntimeGenesisConfig>(config)
		}
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
