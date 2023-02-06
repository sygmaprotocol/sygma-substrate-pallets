// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only
use std::{marker::PhantomData, sync::Arc};

use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
};
use sp_api::{BlockId, BlockT, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sygma_runtime_api::SumStorageApi;
use sygma_traits::{DepositNonce, DomainID};

pub struct SumStorage<Block: BlockT, C> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<Block: BlockT, C> SumStorage<Block, C> {
	/// Create new `SumStorage` instance with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

#[rpc(server, namespace = "sygma_bridge_rpc")]
pub trait SumStorageRpc<BlockHash> {
	#[method(name = "getSum")]
	fn get_sum(&self, at: Option<BlockHash>) -> RpcResult<u32>;

	#[method(name = "isProposalExecuted")]
	fn is_proposal_executed(
		&self,
		nonce: DepositNonce,
		domain_id: DomainID,
		at: Option<BlockHash>
	) -> RpcResult<bool>;
}

#[async_trait]
impl<Block, C> SumStorageRpcServer<<Block as BlockT>::Hash> for SumStorage<Block, C>
where
	Block: BlockT,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: SumStorageApi<Block>,
{
	fn get_sum(&self, at: Option<<Block as BlockT>::Hash>) -> RpcResult<u32> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let runtime_api_result = api.get_sum(&at);
		runtime_api_result.map_err(|e| JsonRpseeError::Custom(format!("runtime error: {e:?}")))
	}

	fn is_proposal_executed(&self, nonce: DepositNonce, domain_id: DomainID, at: Option<<Block as BlockT>::Hash>) -> RpcResult<bool> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let runtime_api_result = api.is_proposal_executed(&at, nonce, domain_id);
		runtime_api_result.map_err(|e| JsonRpseeError::Custom(format!("runtime error: {e:?}")))
	}
}
