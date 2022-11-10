#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::dispatch::TypeInfo;
use xcm::latest::AssetId;

pub type DomainID = u8;
pub type DepositNonce = u64;
pub type ResourceId = [u8; 32];

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Encode, Decode, TypeInfo, Copy)]
pub struct MpcPubkey(pub [u8; 33]);

impl Default for MpcPubkey {
	fn default() -> Self {
		MpcPubkey([0; 33])
	}
}

pub trait FeeHandler: Sized {
	// Return fee represent by a specific asset
	fn get_fee(asset_id: &AssetId) -> Option<u128>;
}
