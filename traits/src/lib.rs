// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::dispatch::TypeInfo;
use primitive_types::{H160, U256};
use sp_std::vec::Vec;
use xcm::latest::{AssetId, MultiLocation};

pub type DomainID = u8;
pub type DepositNonce = u64;
pub type ResourceId = [u8; 32];
pub type ChainID = U256;
pub type VerifyingContractAddress = H160;

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub enum TransferType {
	FungibleTransfer,
	NonFungibleTransfer,
	GenericTransfer,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Encode, Decode, TypeInfo, Copy)]
pub struct MpcPubkey(pub [u8; 33]);

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Encode, Decode, TypeInfo, Copy, Default)]
pub struct MpcAddress(pub [u8; 20]);

impl Default for MpcPubkey {
	fn default() -> Self {
		MpcPubkey([0; 33])
	}
}

pub trait IsReserved {
	fn is_reserved(asset_id: &AssetId) -> bool;
}

pub trait ExtractRecipient {
	fn extract_recipient(dest: &MultiLocation) -> Option<Vec<u8>>;
}

pub trait FeeHandler: Sized {
	// Return fee represent by a specific asset
	fn get_fee(asset_id: &AssetId) -> Option<u128>;
}
