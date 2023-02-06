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
}
