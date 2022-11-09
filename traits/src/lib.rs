#![cfg_attr(not(feature = "std"), no_std)]

use xcm::latest::AssetId;

pub type DomainID = u8;
pub type DepositNonce = u64;
pub type ResourceId = [u8; 32];

pub trait FeeHandler: Sized {
	// Return fee represent by a specific asset
	fn get_fee(asset_id: &AssetId) -> Option<u128>;
}
