#![cfg_attr(not(feature = "std"), no_std)]

use xcm::latest::{prelude::*, AssetId};

pub type DomainID = u8;
pub type DepositNonce = u64;
pub type ResourceId = [u8; 32];

pub trait FeeHandler: Sized {
	/// Create a new trader instance.
	fn new() -> Self;

	// Return fee represent by a specific asset
	fn get_fee(&self, asset_id: AssetId) -> Option<u128>;
}

#[impl_trait_for_tuples::impl_for_tuples(30)]
impl FeeHandler for Tuple {
	fn new() -> Self {
		for_tuples!( ( #( Tuple::new() ),* ) )
	}

	fn get_fee(&self, asset_id: AssetId) -> Option<u128> {
		// TODO
		None
	}
}
