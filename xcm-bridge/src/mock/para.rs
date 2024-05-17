// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

use sp_std::sync::Arc;
use std::marker::PhantomData;
use std::result;

use cumulus_primitives_core::{ParaId, Weight};

use crate as sygma_xcm_bridge;
use frame_support::pallet_prelude::Get;
use frame_support::traits::{ConstU16, ConstU64, Nothing};
use frame_support::{
	construct_runtime, parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU128, ConstU32, Everything},
};
use frame_system as system;
use frame_system::EnsureRoot;
use polkadot_parachain_primitives::primitives::Sibling;
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::traits::{IdentityLookup, Zero};
use sygma_traits::AssetTypeIdentifier;
use xcm::prelude::{
	Fungible, GlobalConsensus,
	Junctions::{X1, X2},
	Parachain, XcmError,
};
use xcm::v4::{
	Asset, AssetId as XcmAssetId, InteriorLocation, Location, NetworkId, Weight as XCMWeight,
	XcmContext,
};
#[allow(deprecated)]
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, CurrencyAdapter,
	FixedWeightBounds, FrameTransactionalProcessor, FungiblesAdapter, IsConcrete, NativeAsset,
	NoChecking, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative, SovereignSignedViaLocation,
	TakeWeightCredit,
};
use xcm_executor::{
	traits::{Error as ExecutionError, MatchesFungibles, WeightTrader, WithOriginFilter},
	AssetsInHolding, Config, XcmExecutor,
};

use super::ParachainXcmRouter;

construct_runtime!(
	pub struct Runtime {
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Config<T>, Storage, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},

		ParachainInfo: pallet_parachain_info::{Pallet, Storage, Config<T>},

		// XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},
		MsgQueue: orml_xcm_mock_message_queue,

		SygmaXcmBridge: sygma_xcm_bridge::{Pallet, Event<T>},
	}
);

type Block = frame_system::mocking::MockBlock<Runtime>;

pub(crate) type Balance = u128;

pub type AccountId = AccountId32;

pub type AssetId = u32;

impl frame_system::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<20>;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type RuntimeTask = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1; // 1 Unit deposit to create asset
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<1>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = u32;
	type AssetIdParameter = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = ExistentialDeposit;
	type AssetAccountDeposit = ConstU128<10>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	type RemoveItemsLimit = ConstU32<5>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl sygma_xcm_bridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type AssetReservedChecker = NativeAssetTypeIdentifier<ParachainInfo>;
	type UniversalLocation = UniversalLocation;
	type SelfLocation = SelfLocation;
	type MinXcmFee = MinXcmFee;
}

impl pallet_parachain_info::Config for Runtime {}

pub struct XcmConfig;
impl Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = (CurrencyTransactor, FungiblesTransactor);
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = AllTokensAreCreatedEqualToWeight;
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetClaims = ();
	type SubscriptionService = ();
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = ConstU32<64>;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = WithOriginFilter<Everything>;
	type SafeCallFilter = Everything;
	type Aliasers = ();
	type TransactionalProcessor = FrameTransactionalProcessor;
}

pub type XcmRouter = ParachainXcmRouter<ParachainInfo>;

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	AllowUnpaidExecutionFrom<Everything>,
);

pub type LocationToAccountId = (
	ParentIsPreset<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type XcmOriginToTransactDispatchOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
);

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Rococo;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UnitWeightCost: XCMWeight = 1u64.into();
	pub const MaxInstructions: u32 = 100;
}

parameter_types! {
	pub NativeLocation: Location = Location::here();
	pub NativeAssetId: AssetId = 0; // native asset ID is used for token registration on other parachain as foreign asset
	pub PAALocation: Location = Location::new(1, X1(Arc::new([Parachain(1u32)])));
	pub PBALocation: Location = Location::new(1, X1(Arc::new([Parachain(2u32)])));
	pub UsdtAssetId: AssetId = 1;
	pub UsdtLocation: Location = Location::new(
		1,
		X1(
			Arc::new([
				Parachain(2005)
			])
		),
	);
	// Parachain A and Parachain B native asset multilocation
	pub CheckingAccount: AccountId32 = AccountId32::new([102u8; 32]);
}

parameter_types! {
	pub SelfLocation: Location = Location::new(1, X1(Arc::new([Parachain(ParachainInfo::parachain_id().into())])));
	pub UniversalLocation: InteriorLocation = X2(Arc::new([GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into())]));

	// set 1 token as min fee
	pub MinXcmFee: Vec<(XcmAssetId, u128)> = vec![(NativeLocation::get().into(), 1_000_000_000_000u128), (PBALocation::get().into(), 1_000_000_000_000u128), (UsdtLocation::get().into(), 1_000_000u128)];
}

pub struct SimpleForeignAssetConverter(PhantomData<()>);
impl MatchesFungibles<AssetId, Balance> for SimpleForeignAssetConverter {
	fn matches_fungibles(a: &Asset) -> result::Result<(AssetId, Balance), ExecutionError> {
		match (&a.fun, &a.id) {
			(Fungible(ref amount), XcmAssetId(ref id)) => {
				if id == &UsdtLocation::get() {
					Ok((UsdtAssetId::get(), *amount))
				} else if id == &PBALocation::get() || id == &PAALocation::get() {
					Ok((NativeAssetId::get(), *amount))
				} else {
					Err(ExecutionError::AssetNotHandled)
				}
			},
			_ => Err(ExecutionError::AssetNotHandled),
		}
	}
}

#[allow(deprecated)]
pub type CurrencyTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<NativeLocation>,
	// Convert an XCM Location into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId32,
	// We don't track any teleports of `Balances`.
	(),
>;

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a fungible asset matching the given location or name:
	SimpleForeignAssetConverter,
	// Convert an XCM Location into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId32,
	// Disable teleport.
	NoChecking,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

/// NativeAssetTypeIdentifier impl AssetTypeIdentifier for XCMAssetTransactor
/// This impl is only for local mock purpose, the integrated parachain might have their own version
pub struct NativeAssetTypeIdentifier<T>(PhantomData<T>);
impl<T: Get<ParaId>> AssetTypeIdentifier for NativeAssetTypeIdentifier<T> {
	/// check if the given Asset is a native asset
	fn is_native_asset(asset: &Asset) -> bool {
		// currently there are two multilocations are considered as native asset:
		// 1. integrated parachain native asset(Location::here())
		// 2. other parachain native asset(Location::new(1, X1(Parachain(T::get().into()))))
		let native_locations =
			[Location::here(), Location::new(1, X1(Arc::new([Parachain(T::get().into())])))];

		match (&asset.id, &asset.fun) {
			(XcmAssetId(ref id), Fungible(_)) => native_locations.contains(id),
			_ => false,
		}
	}
}

// /// Information about an XCMP channel.
// pub struct ChannelInfo {
// 	/// The maximum number of messages that can be pending in the channel at once.
// 	pub max_capacity: u32,
// 	/// The maximum total size of the messages that can be pending in the channel at once.
// 	pub max_total_size: u32,
// 	/// The maximum message size that could be put into the channel.
// 	pub max_message_size: u32,
// 	/// The current number of messages pending in the channel.
// 	/// Invariant: should be less or equal to `max_capacity`.s`.
// 	pub msg_count: u32,
// 	/// The total size in bytes of all message payloads in the channel.
// 	/// Invariant: should be less or equal to `max_total_size`.
// 	pub total_size: u32,
// }
//
// impl GetChannelInfo for ChannelInfo {
// 	fn get_channel_status(_id: ParaId) -> ChannelStatus {
// 		ChannelStatus::Ready(10, 10)
// 	}
// 	fn get_channel_info(id: ParaId) -> Option<ChannelInfo> {
// 		let channels = Self::relevant_messaging_state()?.egress_channels;
// 		let index = channels.binary_search_by_key(&id, |item| item.0).ok()?;
// 		let info = ChannelInfo {
// 			max_capacity: channels[index].1.max_capacity,
// 			max_total_size: channels[index].1.max_total_size,
// 			max_message_size: channels[index].1.max_message_size,
// 			msg_count: channels[index].1.msg_count,
// 			total_size: channels[index].1.total_size,
// 		};
// 		Some(info)
// 	}
// }

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl orml_xcm_mock_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub struct AllTokensAreCreatedEqualToWeight(Location);
impl WeightTrader for AllTokensAreCreatedEqualToWeight {
	fn new() -> Self {
		Self(Location::parent())
	}

	fn buy_weight(
		&mut self,
		weight: Weight,
		payment: AssetsInHolding,
		_context: &XcmContext,
	) -> Result<AssetsInHolding, XcmError> {
		let asset_id = payment.fungible.iter().next().expect("Payment must be something; qed").0;
		let required = Asset { id: asset_id.clone(), fun: Fungible(weight.ref_time() as u128) };

		let Asset { fun: _, id: XcmAssetId(ref id) } = &required;

		self.0 = id.clone();

		let unused = payment.checked_sub(required).map_err(|_| XcmError::TooExpensive)?;
		Ok(unused)
	}

	fn refund_weight(&mut self, weight: Weight, _context: &XcmContext) -> Option<Asset> {
		if weight.is_zero() {
			None
		} else {
			Some((self.0.clone(), weight.ref_time() as u128).into())
		}
	}
}

// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
pub fn assert_events(mut expected: Vec<RuntimeEvent>) {
	let mut actual: Vec<RuntimeEvent> =
		system::Pallet::<Runtime>::events().iter().map(|e| e.event.clone()).collect();

	expected.reverse();

	for evt in expected {
		let next = actual.pop().expect("event expected");
		assert_eq!(next, evt, "Events don't match");
	}
}
