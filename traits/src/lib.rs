#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::vec::Vec;
use xcm::latest::{AssetId, MultiLocation};

pub type DomainID = u8;
pub type DepositNonce = u64;
pub type ResourceId = [u8; 32];

pub trait IsReserve {
	fn is_reserve(asset_id: &AssetId) -> bool;
}

pub trait ExtractRecipient {
	fn extract_recipient(dest: &MultiLocation) -> Option<Vec<u8>>;
}

pub trait FeeHandler: Sized {
	// Return fee represent by a specific asset
	fn get_fee(asset_id: &AssetId) -> Option<u128>;
}
