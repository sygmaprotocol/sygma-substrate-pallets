// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

use sygma_traits::{DepositNonce, DomainID};

sp_api::decl_runtime_apis! {
	pub trait SumStorageApi {
		fn get_sum() -> u32;
		fn is_proposal_executed(nonce: DepositNonce, domain_id: DomainID) -> bool;
	}
}
