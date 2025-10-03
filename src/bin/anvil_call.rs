use std::ops::Div;
use std::sync::Arc;

use alloy::network::TransactionBuilder;
use alloy::node_bindings::Anvil;
use alloy::primitives::U256;
use alloy::providers::{Provider, ProviderBuilder};

use alloy::rpc::types::TransactionRequest;
use alloy::transports::http::reqwest::Url;
use denegnet::{
    abi::{decode_quote_response, quote_calldata},
    address::{ME, USDC_ADDR, V3_QUOTER_ADDR, WETH_ADDR},
    constant::ONE_ETHER,
    helpers::volumes,
    setup_tracing,
};
use execution_time::ExecutionTime;

// Total: 10 local (app <-> anvil) + 3 network RPC requests
//        These 10 local requests were targetting our local Anvil process,
//        i.e., not spamming the full node.
//
// * Anvil works by implicitly fetching necessary data on demand. I
// * Using your own Full Node is ~100 times faster than using a third-party node.

// But we don't need to talk to local Anvil instance because
// we can run REVM which Anvil (and RETH) uses under the hood.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let eth_rpc_url: Url = std::env::var("ETH_RPC_URL").unwrap().parse()?;
    let provider = ProviderBuilder::new().connect_http(eth_rpc_url.clone());
    let provider = Arc::new(provider);

    let current_gas_price = provider.get_gas_price().await?;
    let fork_block = provider.get_block_number().await?;

    let anvil = Anvil::new()
        .fork(eth_rpc_url)
        .fork_block_number(fork_block)
        .block_time(1u64)
        .spawn();

    let anvil_provider = ProviderBuilder::new().connect_http(anvil.endpoint_url());
    let anvil_provider = Arc::new(anvil_provider);

    let pool_fee = 3000; // 0.03%
    let volumes = volumes(U256::ZERO, ONE_ETHER.div(U256::from(10)), 10);

    let execution_time = ExecutionTime::start();
    let calldata = quote_calldata(WETH_ADDR, USDC_ADDR, volumes[0], pool_fee);

    let tx = TransactionRequest::default()
        .from(ME)
        .to(V3_QUOTER_ADDR)
        .with_input(calldata)
        .nonce(0)
        .gas_limit(1000000)
        .max_fee_per_gas(current_gas_price)
        .max_priority_fee_per_gas(0)
        .build_unsigned()?
        .into();

    let response = anvil_provider.call(tx).await?;
    let amount_out = decode_quote_response(response)?;

    print!("-> ");
    execution_time.print_elapsed_time();
    println!("{} WETH -> USDC {}", volumes[0], amount_out);

    let execution_time = ExecutionTime::start();
    for volume in volumes.into_iter() {
        let calldata = quote_calldata(WETH_ADDR, USDC_ADDR, volume, pool_fee);

        let tx = TransactionRequest::default()
            .from(ME)
            .to(V3_QUOTER_ADDR)
            .with_input(calldata)
            .nonce(0)
            .gas_limit(1000000)
            .max_fee_per_gas(current_gas_price)
            .max_priority_fee_per_gas(0)
            .build_unsigned()?
            .into();

        let response = anvil_provider.call(tx).await?;
        let amount_out = decode_quote_response(response)?;
        println!("{} WETH -> USDC {}", volume, amount_out);
    }
    print!("-> ");
    execution_time.print_elapsed_time();

    // Drop it earlier to free the resources
    drop(anvil);

    Ok(())
}
