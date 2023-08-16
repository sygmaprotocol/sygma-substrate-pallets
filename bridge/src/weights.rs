
//! Autogenerated weights for `sygma_bridge`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-04-26, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/node-template
// benchmark
// pallet
// --chain
// dev
// --execution=wasm
// --wasm-execution=compiled
// --pallet
// sygma_bridge
// --extrinsic
// *
// --steps
// 50
// --repeat
// 20
// --output
// bridge_weight.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `sygma_bridge`.
pub struct SygmaWeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> super::WeightInfo for SygmaWeightInfo<T> {
	/// Storage: SygmaBridge DestDomainIds (r:1 w:0)
	/// Proof Skipped: SygmaBridge DestDomainIds (max_values: None, max_size: None, mode: Measured)
	/// Storage: SygmaBridge IsPaused (r:0 w:1)
	/// Proof Skipped: SygmaBridge IsPaused (max_values: None, max_size: None, mode: Measured)
	fn pause_bridge() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `109`
		//  Estimated: `3683`
		// Minimum execution time: 12_000_000 picoseconds.
		Weight::from_parts(12_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3683))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: SygmaBridge DestDomainIds (r:1 w:0)
	/// Proof Skipped: SygmaBridge DestDomainIds (max_values: None, max_size: None, mode: Measured)
	/// Storage: SygmaBridge IsPaused (r:1 w:1)
	/// Proof Skipped: SygmaBridge IsPaused (max_values: None, max_size: None, mode: Measured)
	fn unpause_bridge() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `143`
		//  Estimated: `7216`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(14_000_000, 0)
			.saturating_add(Weight::from_parts(0, 7216))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: SygmaBridge MpcAddr (r:1 w:1)
	/// Proof Skipped: SygmaBridge MpcAddr (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: SygmaBridge IsPaused (r:1 w:0)
	/// Proof Skipped: SygmaBridge IsPaused (max_values: None, max_size: None, mode: Measured)
	fn set_mpc_address() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `42`
		//  Estimated: `5034`
		// Minimum execution time: 8_000_000 picoseconds.
		Weight::from_parts(9_000_000, 0)
			.saturating_add(Weight::from_parts(0, 5034))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: SygmaBridge DestDomainIds (r:0 w:1)
	/// Proof Skipped: SygmaBridge DestDomainIds (max_values: None, max_size: None, mode: Measured)
	/// Storage: SygmaBridge DestChainIds (r:0 w:1)
	/// Proof Skipped: SygmaBridge DestChainIds (max_values: None, max_size: None, mode: Measured)
	fn register_domain() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 10_000_000 picoseconds.
		Weight::from_parts(10_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: SygmaBridge DestDomainIds (r:1 w:1)
	/// Proof Skipped: SygmaBridge DestDomainIds (max_values: None, max_size: None, mode: Measured)
	/// Storage: SygmaBridge DestChainIds (r:1 w:1)
	/// Proof Skipped: SygmaBridge DestChainIds (max_values: None, max_size: None, mode: Measured)
	fn unregister_domain() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `137`
		//  Estimated: `7204`
		// Minimum execution time: 16_000_000 picoseconds.
		Weight::from_parts(17_000_000, 0)
			.saturating_add(Weight::from_parts(0, 7204))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: SygmaBridge MpcAddr (r:1 w:0)
	/// Proof Skipped: SygmaBridge MpcAddr (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: SygmaBridge IsPaused (r:1 w:0)
	/// Proof Skipped: SygmaBridge IsPaused (max_values: None, max_size: None, mode: Measured)
	/// Storage: SygmaBridge DestDomainIds (r:1 w:0)
	/// Proof Skipped: SygmaBridge DestDomainIds (max_values: None, max_size: None, mode: Measured)
	/// Storage: SygmaFeeHandlerRouter HandlerType (r:1 w:0)
	/// Proof Skipped: SygmaFeeHandlerRouter HandlerType (max_values: None, max_size: None, mode: Measured)
	/// Storage: SygmaBasicFeeHandler AssetFees (r:1 w:0)
	/// Proof Skipped: SygmaBasicFeeHandler AssetFees (max_values: None, max_size: None, mode: Measured)
	/// Storage: System Account (r:2 w:2)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: SygmaBridge DepositCounts (r:1 w:1)
	/// Proof Skipped: SygmaBridge DepositCounts (max_values: None, max_size: None, mode: Measured)
	fn deposit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `312`
		//  Estimated: `26878`
		// Minimum execution time: 89_000_000 picoseconds.
		Weight::from_parts(91_000_000, 0)
			.saturating_add(Weight::from_parts(0, 26878))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: SygmaBridge MpcAddr (r:1 w:0)
	/// Proof Skipped: SygmaBridge MpcAddr (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: SygmaBridge IsPaused (r:1 w:0)
	/// Proof Skipped: SygmaBridge IsPaused (max_values: None, max_size: None, mode: Measured)
	/// Storage: SygmaBridge DestDomainIds (r:1 w:0)
	/// Proof Skipped: SygmaBridge DestDomainIds (max_values: None, max_size: None, mode: Measured)
	fn retry() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `177`
		//  Estimated: `8946`
		// Minimum execution time: 16_000_000 picoseconds.
		Weight::from_parts(17_000_000, 0)
			.saturating_add(Weight::from_parts(0, 8946))
			.saturating_add(T::DbWeight::get().reads(3))
	}
	/// Storage: SygmaBridge MpcAddr (r:1 w:0)
	/// Proof Skipped: SygmaBridge MpcAddr (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: SygmaBridge IsPaused (r:1 w:0)
	/// Proof Skipped: SygmaBridge IsPaused (max_values: None, max_size: None, mode: Measured)
	/// Storage: SygmaBridge DestDomainIds (r:1 w:0)
	/// Proof Skipped: SygmaBridge DestDomainIds (max_values: None, max_size: None, mode: Measured)
	/// Storage: SygmaBridge UsedNonces (r:1 w:1)
	/// Proof Skipped: SygmaBridge UsedNonces (max_values: None, max_size: None, mode: Measured)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// The range of component `n` is `[1, 1000]`.
	fn execute_proposal(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `280`
		//  Estimated: `16593`
		// Minimum execution time: 123_000_000 picoseconds.
		Weight::from_parts(151_050_908, 0)
			.saturating_add(Weight::from_parts(0, 16593))
			// Standard Error: 18_882
			.saturating_add(Weight::from_parts(10_748_102, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(2))
	}

	/// Storage: SygmaBridge DestDomainIds (r:4 w:0)
	/// Proof: SygmaBridge DestDomainIds (max_values: None, max_size: Some(10), added: 2485, mode: MaxEncodedLen)
	/// Storage: SygmaBridge IsPaused (r:4 w:3)
	/// Proof: SygmaBridge IsPaused (max_values: None, max_size: Some(10), added: 2485, mode: MaxEncodedLen)
	fn pause_all_bridges() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `302`
		//  Estimated: `10930`
		// Minimum execution time: 76_000_000 picoseconds.
		Weight::from_parts(80_000_000, 0)
			.saturating_add(Weight::from_parts(0, 10930))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(3))
	}

	/// Storage: SygmaBridge MpcAddr (r:1 w:0)
	/// Proof: SygmaBridge MpcAddr (max_values: Some(1), max_size: Some(20), added: 515, mode: MaxEncodedLen)
	/// Storage: SygmaBridge DestDomainIds (r:4 w:0)
	/// Proof: SygmaBridge DestDomainIds (max_values: None, max_size: Some(10), added: 2485, mode: MaxEncodedLen)
	/// Storage: SygmaBridge IsPaused (r:4 w:3)
	/// Proof: SygmaBridge IsPaused (max_values: None, max_size: Some(10), added: 2485, mode: MaxEncodedLen)
	fn unpause_all_bridges() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `342`
		//  Estimated: `10930`
		// Minimum execution time: 81_000_000 picoseconds.
		Weight::from_parts(84_000_000, 0)
			.saturating_add(Weight::from_parts(0, 10930))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(3))
	}
}
