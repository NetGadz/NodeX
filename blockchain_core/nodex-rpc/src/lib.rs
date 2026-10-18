#![forbid(unsafe_code)]

use jsonrpsee::proc_macros::rpc;
use nodex_types::Address;

#[rpc(Server, Client)]
pub mod NdxRpc {
    #[method(name = "ndx_chainId")]
    fn chain_id(&self) -> Result<u32, jsonrpsee::core::Error>;

    #[method(name = "ndx_blockNumber")]
    fn block_number(&self) -> Result<u64, jsonrpsee::core::Error>;

    #[method(name = "ndx_getBalance")]
    fn get_balance(&self, address: Address) -> Result<u128, jsonrpsee::core::Error>;
}

pub struct NdxRpcServerImpl;

impl NdxRpcServerServer for NdxRpcServerImpl {
    fn chain_id(&self) -> Result<u32, jsonrpsee::core::Error> {
        Ok(1)
    }

    fn block_number(&self) -> Result<u64, jsonrpsee::core::Error> {
        Ok(0)
    }

    fn get_balance(&self, _address: Address) -> Result<u128, jsonrpsee::core::Error> {
        Ok(0)
    }
}
