use std::ops::Div;
use std::sync::Arc;

use alloy::primitives::U256;
use alloy::providers::ProviderBuilder;

use denegnet::revm::{init_cache_db, revm_call};
use denegnet::{
    abi::{decode_quote_response, quote_calldata},
    address::{ME, USDC_ADDR, V3_QUOTER_ADDR, WETH_ADDR},
    constant::ONE_ETHER,
    helpers::volumes,
    setup_tracing,
};
use execution_time::ExecutionTime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let eth_rpc_url = std::env::var("ETH_RPC_URL").unwrap().parse()?;
    let provider = ProviderBuilder::new().connect_http(eth_rpc_url);
    let provider = Arc::new(provider);

    let mut cache_db = init_cache_db(provider);

    let pool_fee = 3000; // 0.03%
    let volumes = volumes(U256::ZERO, ONE_ETHER.div(U256::from(10)), 10);

    let execution_time = ExecutionTime::start();
    let calldata = quote_calldata(WETH_ADDR, USDC_ADDR, volumes[0], pool_fee);
    let response = revm_call(ME, V3_QUOTER_ADDR, calldata, &mut cache_db)?;
    let amount_out = decode_quote_response(response)?;
    execution_time.print_elapsed_time();
    println!("{} WETH -> USDC {}", volumes[0], amount_out);

    let execution_time = ExecutionTime::start();
    for volume in volumes.into_iter() {
        let calldata = quote_calldata(WETH_ADDR, USDC_ADDR, volume, pool_fee);
        let response = revm_call(ME, V3_QUOTER_ADDR, calldata, &mut cache_db)?;
        let amount_out = decode_quote_response(response)?;
        println!("{} WETH -> USDC {}", volume, amount_out);
    }
    execution_time.print_elapsed_time();

    Ok(())
}
