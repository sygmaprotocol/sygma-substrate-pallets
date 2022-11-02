#![cfg(test)]

use crate as sygma_bridge;
use sygma_traits::{DomainID, ResourceId};

use frame_support::{
    parameter_types,
    traits::{ConstU32, PalletInfoAccess},
};
use frame_system::{self as system};
use polkadot_parachain::primitives::Sibling;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32, Perbill,
};
use xcm::latest::{prelude::*, AssetId as XcmAssetId, MultiLocation};
use xcm_builder::{
    AccountId32Aliases, AsPrefixedGeneralIndex, ConvertedConcreteAssetId, CurrencyAdapter,
    FungiblesAdapter, IsConcrete, ParentIsPreset, SiblingParachainConvertsVia,
};
use xcm_executor::traits::JustTry;

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
        SygmaBasicFeeHandler: sygma_basic_feehandler::{Pallet, Call, Storage, Event<T>},
        SygmaBridge: sygma_bridge::{Pallet, Call, Storage, Event<T>},
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
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type PalletInfo = PalletInfo;
    type BlockWeights = ();
    type BlockLength = ();
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

const fn deposit(items: u32, bytes: u32) -> Balance {
    items as Balance * 15 * CENTS::get() + (bytes as Balance) * 1 * CENTS::get()
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
    pub const AssetDeposit: Balance = 10 * UNIT::get(); // 10 UNITS deposit to create fungible asset class
    pub const AssetAccountDeposit: Balance = 1 * DOLLARS::get();
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
    type Currency = Balances;
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

impl sygma_basic_feehandler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BridgeCommitteeOrigin = frame_system::EnsureRoot<Self::AccountId>;
}

parameter_types! {
    pub DestDomainID: DomainID = 1;
    pub TreasuryAccount: AccountId32 = AccountId32::new([100u8; 32]);
    pub BridgeAccount: AccountId32 = AccountId32::new([101u8; 32]);
    pub CheckingAccount: AccountId32 = AccountId32::new([102u8; 32]);
    pub RelayNetwork: NetworkId = NetworkId::Polkadot;
    pub AssetsPalletLocation: MultiLocation =
        PalletInstance(<Assets as PalletInfoAccess>::index() as u8).into();
    pub PhaLocation: MultiLocation = MultiLocation::here();
    pub UsdcLocation: MultiLocation = MultiLocation::new(
        1,
        X3(
            Parachain(2004),
            GeneralKey(b"sygma".to_vec().try_into().expect("less than length limit; qed")),
            GeneralKey(b"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_vec().try_into().expect("less than length limit; qed")),
        ),
    );
    pub PhaResourceId: ResourceId = hex_literal::hex!("00e6dfb61a2fb903df487c401663825643bb825d41695e63df8af6162ab145a6").into();
    pub UsdcResourceId: ResourceId = hex_literal::hex!("00b14e071ddad0b12be5aca6dffc5f2584ea158d9b0ce73e1437115e97a32a3e").into();
    pub ResourcePairs: Vec<(XcmAssetId, ResourceId)> = vec![(PhaLocation::get().into(), PhaResourceId::get()), (UsdcLocation::get().into(), UsdcResourceId::get())];
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
    IsConcrete<PhaLocation>,
    // Convert an XCM MultiLocation into a local account id:
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
    ConvertedConcreteAssetId<
        AssetId,
        Balance,
        AsPrefixedGeneralIndex<AssetsPalletLocation, AssetId, JustTry>,
        JustTry,
    >,
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

impl sygma_bridge::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BridgeCommitteeOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type DestDomainID = DestDomainID;
    type TransferReserveAccount = BridgeAccount;
    type FeeReserveAccount = TreasuryAccount;
    type FeeHandler = sygma_basic_feehandler::BasicFeeHandlerImpl<Runtime>;
    type AssetTransactor = AssetTransactors;
    type ResourcePairs = ResourcePairs;
}

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const ENDOWED_BALANCE: Balance = 100_000_000;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
pub fn assert_events(mut expected: Vec<RuntimeEvent>) {
    let mut actual: Vec<RuntimeEvent> = system::Pallet::<Runtime>::events()
        .iter()
        .map(|e| e.event.clone())
        .collect();

    expected.reverse();

    for evt in expected {
        let next = actual.pop().expect("event expected");
        assert_eq!(next, evt.into(), "Events don't match");
    }
}
