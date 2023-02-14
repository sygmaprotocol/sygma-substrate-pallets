// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg(test)]

use crate as sygma_bridge;
use alloc::string::ToString;
use core::str::FromStr;
use frame_support::{
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU32, PalletInfoAccess},
	PalletId,
};
use frame_system::{self as system, EnsureSigned};
use funty::Fundamental;
use polkadot_parachain::primitives::Sibling;
use sp_core::hash::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, Perbill,
};
use sp_std::{borrow::Borrow, marker::PhantomData, prelude::*, result};
use sygma_traits::{
	ChainID, DecimalConverter, DomainID, ExtractDestinationData, ResourceId,
	VerifyingContractAddress,
};
use xcm::latest::{prelude::*, AssetId as XcmAssetId, MultiLocation};
use xcm_builder::{
	AccountId32Aliases, CurrencyAdapter, FungiblesAdapter, IsConcrete, ParentIsPreset,
	SiblingParachainConvertsVia,
};
use xcm_executor::traits::{
	Convert, Error as ExecutionError, FilterAssetLocation, MatchesFungibles,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

pub(crate) type Balance = u128;

frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Config<T>, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		AccessSegregator: sygma_access_segregator::{Pallet, Call, Storage, Event<T>} = 4,
		SygmaBasicFeeHandler: sygma_basic_feehandler::{Pallet, Call, Storage, Event<T>} = 5,
		SygmaBridge: sygma_bridge::{Pallet, Call, Storage, Event<T>} = 6,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const MaxLocks: u32 = 100;
	pub const MinimumPeriod: u64 = 1;
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<2>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	pub const UNIT: Balance = 1_000_000_000_000;
	pub const DOLLARS: Balance = UNIT::get();
	pub const CENTS: Balance = DOLLARS::get() / 100;
	pub const MILLICENTS: Balance = CENTS::get() / 1_000;
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
}

parameter_types! {
	pub const AssetDeposit: Balance = 0;
	pub const AssetAccountDeposit: Balance =0;
	pub const ApprovalDeposit: Balance = ExistentialDeposit::get();
	pub const AssetsStringLimit: u32 = 50;
	/// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = 0;
	pub const MetadataDepositPerByte: Balance = 0;
	pub const ExecutiveBody: BodyId = BodyId::Executive;
}

pub type AssetId = u32;
impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetId;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId32>>;
	type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
}

impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	// Make sure put same value with `construct_runtime`
	pub const AccessSegregatorPalletIndex: u8 = 4;
	pub const FeeHandlerPalletIndex: u8 = 5;
	pub const BridgePalletIndex: u8 = 6;
	pub RegisteredExtrinsics: Vec<(u8, Vec<u8>)> = [
		(AccessSegregatorPalletIndex::get(), b"grant_access".to_vec()),
		(FeeHandlerPalletIndex::get(), b"set_fee".to_vec()),
		(BridgePalletIndex::get(), b"set_mpc_address".to_vec()),
		(BridgePalletIndex::get(), b"pause_bridge".to_vec()),
		(BridgePalletIndex::get(), b"unpause_bridge".to_vec()),
		(BridgePalletIndex::get(), b"register_domain".to_vec()),
		(BridgePalletIndex::get(), b"unregister_domain".to_vec()),
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

parameter_types! {
	pub TreasuryAccount: AccountId32 = AccountId32::new([100u8; 32]);
	pub EIP712ChainID: ChainID = primitive_types::U256([1u64; 4]);
	pub DestVerifyingContractAddress: VerifyingContractAddress = primitive_types::H160([1u8; 20]);
	pub BridgeAccount: AccountId32 = AccountId32::new([101u8; 32]);
	pub CheckingAccount: AccountId32 = AccountId32::new([102u8; 32]);
	pub RelayNetwork: NetworkId = NetworkId::Polkadot;
	pub AssetsPalletLocation: MultiLocation =
		PalletInstance(<Assets as PalletInfoAccess>::index() as u8).into();
	pub NativeLocation: MultiLocation = MultiLocation::here();
	pub UsdcAssetId: AssetId = 0;
	pub UsdcLocation: MultiLocation = MultiLocation::new(
		1,
		X3(
			Parachain(2004),
			GeneralKey(b"sygma".to_vec().try_into().expect("less than length limit; qed")),
			GeneralKey(b"usdc".to_vec().try_into().expect("less than length limit; qed")),
		),
	);
	pub AstrAssetId: AssetId = 1;
	pub AstrLocation: MultiLocation = MultiLocation::new(
		1,
		X3(
			Parachain(2005),
			GeneralKey(b"sygma".to_vec().try_into().expect("less than length limit; qed")),
			GeneralKey(b"astr".to_vec().try_into().expect("less than length limit; qed")),
		),
	);
	pub NativeResourceId: ResourceId = hex_literal::hex!("00e6dfb61a2fb903df487c401663825643bb825d41695e63df8af6162ab145a6");
	pub UsdcResourceId: ResourceId = hex_literal::hex!("00b14e071ddad0b12be5aca6dffc5f2584ea158d9b0ce73e1437115e97a32a3e");
	pub AstrResourceId: ResourceId = hex_literal::hex!("4e071db61a2fb903df487c401663825643ba158d9b0ce73e1437163825643bba");
	pub ResourcePairs: Vec<(XcmAssetId, ResourceId)> = vec![(NativeLocation::get().into(), NativeResourceId::get()), (UsdcLocation::get().into(), UsdcResourceId::get()), (AstrLocation::get().into(), AstrResourceId::get())];
	pub AssetDecimalPairs: Vec<(XcmAssetId, u8)> = vec![(NativeLocation::get().into(), 12u8), (UsdcLocation::get().into(), 18u8), (AstrLocation::get().into(), 24u8)];
	pub const SygmaBridgePalletId: PalletId = PalletId(*b"sygma/01");
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
		} else if &AstrLocation::get() == id.borrow() {
			Ok(AstrAssetId::get())
		} else {
			Err(())
		}
	}
	fn reverse_ref(what: impl Borrow<AssetId>) -> result::Result<MultiLocation, ()> {
		if *what.borrow() == UsdcAssetId::get() {
			Ok(UsdcLocation::get())
		} else if *what.borrow() == AstrAssetId::get() {
			Ok(AstrLocation::get())
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
				} else if id == &AstrLocation::get() {
					Ok((AstrAssetId::get(), *amount))
				} else {
					Err(ExecutionError::AssetNotFound)
				},
			_ => Err(ExecutionError::AssetNotFound),
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
	// We only want to allow teleports of known assets. We use non-zero issuance as an indication
	// that this asset is known.
	parachains_common::impls::NonZeroIssuance<AccountId32, Assets>,
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
			(Concrete(id), Fungible(_)) => Some(id.clone()),
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
						Some(MultiLocation::new(
							0,
							X1(GeneralKey(
								b"sygma".to_vec().try_into().expect("less than length limit; qed"),
							)),
						))
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

pub struct SygmaDecimalConverter;
impl DecimalConverter for SygmaDecimalConverter {
	fn convert_to(asset: &MultiAsset) -> Option<u128> {
		match (&asset.fun, &asset.id) {
			(Fungible(amount), _) => {
				for (asset_id, decimal) in AssetDecimalPairs::get().iter() {
					if *asset_id == asset.id {
						return if *decimal == 18 {
							Some(*amount)
						} else {
							let mut new_amount = amount.clone().to_string();
							if *decimal > 18 {
								let mut n = 0;
								while n < *decimal - 18 {
									new_amount.pop();
									n += 1;
								}
							} else {
								let mut n = 0;
								while n < 18 - *decimal {
									new_amount.push('0');
									n += 1;
								}
							};
							u128::from_str(&new_amount).ok()
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
				for (asset_id, decimal) in AssetDecimalPairs::get().iter() {
					if *asset_id == asset.id {
						return if *decimal == 18 {
							Some((asset.id.clone(), *amount).into())
						} else {
							let mut new_amount = amount.clone().to_string();
							if *decimal > 18 {
								let mut n = 0;
								while n < *decimal - 18 {
									new_amount.push('0');
									n += 1;
								}
							} else {
								let mut n = 0;
								while n < 18 - *decimal {
									new_amount.pop();
									n += 1;
								}
							};
							let f = u128::from_str(&new_amount).unwrap_or_default();
							if f == u128::default() {
								return None
							}
							Some((asset.id.clone(), f).into())
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
impl FilterAssetLocation for ReserveChecker {
	fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
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
			(0, Junctions::X2(GeneralKey(recipient), GeneralIndex(dest_domain_id))) =>
				Some((recipient.to_vec(), dest_domain_id.as_u8())),
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
	type FeeHandler = SygmaBasicFeeHandler;
	type AssetTransactor = AssetTransactors;
	type ResourcePairs = ResourcePairs;
	type IsReserve = ReserveChecker;
	type ExtractDestData = DestinationDataParser;
	type PalletId = SygmaBridgePalletId;
	type PalletIndex = BridgePalletIndex;
	type DecimalConverter = SygmaDecimalConverter;
	type AssetDecimalPairs = AssetDecimalPairs;
}

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const ASSET_OWNER: AccountId32 = AccountId32::new([1u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([2u8; 32]);
pub const ENDOWED_BALANCE: Balance = 1_000_000_000_000_000_000_000_000_000;
pub const DEST_DOMAIN_ID: DomainID = 1;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(ALICE, ENDOWED_BALANCE),
			(ASSET_OWNER, ENDOWED_BALANCE),
			(BOB, ENDOWED_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
#[allow(dead_code)]
pub fn assert_events(mut expected: Vec<RuntimeEvent>) {
	let mut actual: Vec<RuntimeEvent> =
		system::Pallet::<Runtime>::events().iter().map(|e| e.event.clone()).collect();

	expected.reverse();

	for evt in expected {
		let next = actual.pop().expect("event expected");
		assert_eq!(next, evt, "Events don't match");
	}
}
