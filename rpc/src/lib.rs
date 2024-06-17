// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only
use std::{marker::PhantomData, sync::Arc};

use jsonrpsee::types::{ErrorObject, ErrorObjectOwned};
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sygma_runtime_api::SygmaBridgeApi;
use sygma_traits::{DepositNonce, DomainID};

/// Converts a runtime trap into a [`CallError`].
fn runtime_error_into_rpc_error(err: impl std::fmt::Debug) -> ErrorObjectOwned {
	ErrorObject::owned(500, "Runtime trapped", Some(format!("{:?}", err)))
}

pub struct SygmaBridgeStorage<Block: BlockT, C> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<Block: BlockT, C> SygmaBridgeStorage<Block, C> {
	/// Create new `SygmaBridgeStorage` instance with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

#[rpc(server, namespace = "sygma")]
pub trait SygmaBridgeRpc<BlockHash> {
	#[method(name = "isProposalExecuted")]
	fn is_proposal_executed(
		&self,
		nonce: DepositNonce,
		domain_id: DomainID,
		at: Option<BlockHash>,
	) -> RpcResult<bool>;
}

#[async_trait]
impl<Block, C> SygmaBridgeRpcServer<<Block as BlockT>::Hash> for SygmaBridgeStorage<Block, C>
where
	Block: BlockT,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: SygmaBridgeApi<Block>,
{
	fn is_proposal_executed(
		&self,
		nonce: DepositNonce,
		domain_id: DomainID,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<bool> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		api.is_proposal_executed(at, nonce, domain_id)
			.map_err(runtime_error_into_rpc_error)?;

		Ok(true)
	}
}
