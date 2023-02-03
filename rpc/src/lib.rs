// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only
use std::{sync::Arc};

use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError},
    proc_macros::rpc,
};
use sp_api::{BlockId, BlockT, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sygma_runtime_api::SumStorageApi;

#[rpc(server, namespace = "sygma-rpc")]
pub trait SumStorageRpc<BlockHash> {
    #[method(name = "sumStorage_getSum")]
    fn get_sum(
        &self,
        at: Option<BlockHash>
    ) -> Result<u32, JsonRpseeError>;
}

/// A struct that implements the `SumStorageApi`.
pub struct SumStorage<C, M> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<M>,
}

impl<C, M> SumStorage<C, M> {
    /// Create new `SumStorage` instance with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client, _marker: Default::default() }
    }
}

#[async_trait]
impl<C, Block> SumStorageRpc<<Block as BlockT>::Hash>
for SumStorage<C, Block>
    where
        Block: BlockT,
        C: Send + Sync + 'static,
        C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
        C: HeaderBackend<Block>,
        C::Api: SumStorageApi<Block>,
{
    fn get_sum(
        &self,
        at: Option<<Block as BlockT>::Hash>
    ) -> Result<u32, JsonRpseeError> {

        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let runtime_api_result = api.get_sum(&at);
        runtime_api_result.map_err(|e| JsonRpseeError::Custom(format!("runtime error: {:?}", e)))
    }
}
