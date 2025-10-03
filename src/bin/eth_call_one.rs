use std::ops::Div;
use std::sync::Arc;

use alloy::network::TransactionBuilder;
use alloy::primitives::U256;
use alloy::providers::{Provider, ProviderBuilder};

use alloy::rpc::types::TransactionRequest;
use denegnet::abi::{decode_quote_response, quote_calldata};
use denegnet::address::{ME, USDC_ADDR, V3_QUOTER_ADDR, WETH_ADDR};
use denegnet::{constant::ONE_ETHER, setup_tracing};
use execution_time::ExecutionTime;

// Total: 2 network RPC calls

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let eth_rpc_url = std::env::var("ETH_RPC_URL").unwrap().parse()?;
    let provider = ProviderBuilder::new().connect_http(eth_rpc_url);
    let provider = Arc::new(provider);

    let execution_time = ExecutionTime::start();
    let current_gas_price = provider.get_gas_price().await?;
    let volume = ONE_ETHER.div(U256::from(10));
    let pool_fee = 3000; // 0.03%

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

    let call_response = provider.call(tx).await?;

    let amount_out = decode_quote_response(call_response)?;

    println!("{} WETH -> USDC {}", volume, amount_out);

    print!("-> ");
    execution_time.print_elapsed_time();

    Ok(())
}
