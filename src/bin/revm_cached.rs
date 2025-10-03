use std::ops::Div;
use std::str::FromStr;
use std::sync::Arc;

use alloy::primitives::{Bytes, U256};
use alloy::providers::ProviderBuilder;

use denegnet::address::V3_POOL_3000_ADDR;
use denegnet::revm::{
    init_account, init_account_with_bytecode, init_cache_db, insert_mapping_storage_slot, revm_call,
};
use denegnet::{
    abi::{decode_quote_response, quote_calldata},
    address::{ME, USDC_ADDR, V3_QUOTER_ADDR, WETH_ADDR},
    constant::ONE_ETHER,
    helpers::volumes,
    setup_tracing,
};
use execution_time::ExecutionTime;
use revm::state::Bytecode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let eth_rpc_url = std::env::var("ETH_RPC_URL").unwrap().parse()?;
    let provider = ProviderBuilder::new().connect_http(eth_rpc_url);
    let provider = Arc::new(provider);

    let mut cache_db = init_cache_db(provider.clone());

    // ETH balances and nonces are not relevant to our simulation.
    // But REVM fetches them by default using basic_ref method of AlloyDB.
    // So here we preload/cache contract's bytecode and mock balance and nonce with zero values.
    // This approach reduces number of RPC calls.
    init_account(V3_QUOTER_ADDR, &mut cache_db, provider.clone()).await?;
    init_account(V3_POOL_3000_ADDR, &mut cache_db, provider.clone()).await?;

    // We don’t usually need the original ERC20 implementation.
    // In our case we can either use some generic ERC20 implementation or
    // preload/cache weth & usdc bytecode.

    let usdc_bytecode_hex = include_str!("../bytecode/usdc.hex").trim();
    let weth_bytecode_hex = include_str!("../bytecode/weth.hex").trim();

    let usdc_bytecode = Bytecode::new_raw(Bytes::from_str(usdc_bytecode_hex)?);
    let weth_bytecode = Bytecode::new_raw(Bytes::from_str(weth_bytecode_hex)?);

    init_account_with_bytecode(WETH_ADDR, weth_bytecode.clone(), &mut cache_db).await?;
    init_account_with_bytecode(USDC_ADDR, usdc_bytecode.clone(), &mut cache_db).await?;

    let mocked_balance = U256::MAX.div(U256::from(2));

    insert_mapping_storage_slot(
        WETH_ADDR,
        U256::ZERO,
        V3_POOL_3000_ADDR,
        mocked_balance,
        &mut cache_db,
    )?;
    insert_mapping_storage_slot(
        USDC_ADDR,
        U256::ZERO,
        V3_POOL_3000_ADDR,
        mocked_balance,
        &mut cache_db,
    )?;

    let pool_fee = 3000; // 0.03%
    let volumes = volumes(U256::ZERO, ONE_ETHER.div(U256::from(10)), 10);

    let execution_time = ExecutionTime::start();
    let calldata = quote_calldata(WETH_ADDR, USDC_ADDR, volumes[0], pool_fee);

    let response = revm_call(ME, V3_QUOTER_ADDR, calldata, &mut cache_db)?;
    let amount_out = decode_quote_response(response)?;

    print!("-> ");
    execution_time.print_elapsed_time();
    println!("{} WETH -> USDC {}", volumes[0], amount_out);

    let execution_time = ExecutionTime::start();
    for volume in volumes.into_iter() {
        let calldata = quote_calldata(WETH_ADDR, USDC_ADDR, volume, pool_fee);
        let response = revm_call(ME, V3_QUOTER_ADDR, calldata, &mut cache_db)?;
        let amount_out = decode_quote_response(response)?;
        println!("{} WETH -> USDC {}", volume, amount_out);
    }
    print!("-> ");
    execution_time.print_elapsed_time();

    Ok(())
}
