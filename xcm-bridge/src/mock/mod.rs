// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg(test)]

use frame_support::sp_runtime::{AccountId32, BuildStorage};
use sp_io::TestExternalities;
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain, TestExt};

pub mod para;
pub mod relay;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);
pub const ENDOWED_BALANCE: u128 = 100_000_000_000_000_000_000;

pub type ParaBalances = pallet_balances::Pallet<para::Runtime>;
pub type ParaAssets = pallet_assets::Pallet<para::Runtime>;

decl_test_parachain! {
	pub struct ParaA {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(1),
	}
}

decl_test_parachain! {
	pub struct ParaB {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(2),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay::Runtime,
		RuntimeCall = relay::RuntimeCall,
		RuntimeEvent = relay::RuntimeEvent,
		XcmConfig = relay::XcmConfig,
		MessageQueue = relay::MessageQueue,
		System = relay::System,
		new_ext = relay_ext(),
	}
}

decl_test_network! {
	pub struct TestNet {
		relay_chain = Relay,
		parachains = vec![
			(1, ParaA),
			(2, ParaB),
		],
	}
}

pub fn para_ext(para_id: u32) -> TestExternalities {
	use para::{Runtime, System};

	let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	let parachain_info_config = pallet_parachain_info::GenesisConfig::<Runtime> {
		parachain_id: para_id.into(),
		phantom: Default::default(),
	};
	parachain_info_config.assimilate_storage(&mut t).unwrap();

	// set Alice and Bob with ENDOWED_BALANCE amount of native asset on every parachain
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, ENDOWED_BALANCE), (BOB, ENDOWED_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	// set Alice with ENDOWED_BALANCE amount of USDT asset on every parachain
	pallet_assets::GenesisConfig::<Runtime> {
		assets: vec![(1, ALICE, false, 1)],
		metadata: vec![(1, "USDT".into(), "USDT".into(), 6)],
		accounts: vec![(1, ALICE, ENDOWED_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn relay_ext() -> sp_io::TestExternalities {
	use relay::{Runtime, System};

	let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	// set Alice with ENDOWED_BALANCE amount of native asset on relay chain
	pallet_balances::GenesisConfig::<Runtime> { balances: vec![(ALICE, ENDOWED_BALANCE)] }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
