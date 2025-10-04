use std::{ops::Div, str::FromStr, sync::Arc};

use alloy::{
    primitives::{Bytes, U256},
    providers::ProviderBuilder,
};

use denegnet::{
    abi::{decode_get_amount_out_response, get_amount_out_calldata},
    address::{CUSTOM_QUOTER_ADDR, ME, USDC_ADDR, V3_POOL_500_ADDR, V3_POOL_3000_ADDR, WETH_ADDR},
    constant::ONE_ETHER,
    helpers::volumes,
    revm::{
        init_account, init_account_with_bytecode, init_cache_db, insert_mapping_storage_slot,
        revm_revert,
    },
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

    init_account(ME, &mut cache_db, provider.clone()).await?;
    init_account(V3_POOL_3000_ADDR, &mut cache_db, provider.clone()).await?;
    init_account(V3_POOL_500_ADDR, &mut cache_db, provider.clone()).await?;

    // cast code c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2 | pbcopy
    // cast code a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48 | pbcopy

    let usdc_bytecode_hex = include_str!("../bytecode/usdc.hex").trim();
    let weth_bytecode_hex = include_str!("../bytecode/weth.hex").trim();

    let usdc_bytecode = Bytecode::new_raw(Bytes::from_str(usdc_bytecode_hex)?);
    let weth_bytecode = Bytecode::new_raw(Bytes::from_str(weth_bytecode_hex)?);

    init_account_with_bytecode(WETH_ADDR, weth_bytecode.clone(), &mut cache_db)?;
    init_account_with_bytecode(USDC_ADDR, usdc_bytecode.clone(), &mut cache_db)?;

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
        V3_POOL_500_ADDR,
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
    insert_mapping_storage_slot(
        WETH_ADDR,
        U256::ZERO,
        V3_POOL_500_ADDR,
        mocked_balance,
        &mut cache_db,
    )?;

    let uni_v3_custom_quoter_bytecode_hex = include_str!("../bytecode/uni_v3_quoter.hex").trim();
    let uni_v3_custom_quoter_bytecode =
        Bytecode::new_raw(Bytes::from_str(uni_v3_custom_quoter_bytecode_hex)?);

    init_account_with_bytecode(
        CUSTOM_QUOTER_ADDR,
        uni_v3_custom_quoter_bytecode,
        &mut cache_db,
    )?;

    let volumes = volumes(U256::ZERO, ONE_ETHER.div(U256::from(10)), 100);

    let execution_time = ExecutionTime::start();
    for volume in volumes.into_iter() {
        let calldata = get_amount_out_calldata(V3_POOL_500_ADDR, WETH_ADDR, USDC_ADDR, volume);
        let response = revm_revert(ME, CUSTOM_QUOTER_ADDR, calldata, &mut cache_db)?;
        let usdc_amount_out = decode_get_amount_out_response(response)?;

        let calldata = get_amount_out_calldata(
            V3_POOL_3000_ADDR,
            USDC_ADDR,
            WETH_ADDR,
            U256::from(usdc_amount_out),
        );
        let response = revm_revert(ME, CUSTOM_QUOTER_ADDR, calldata, &mut cache_db)?;
        let weth_amount_out = decode_get_amount_out_response(response)?;

        println!("{volume} WETH -> {usdc_amount_out} USDC -> {weth_amount_out} WETH");

        let weth_amount_out = U256::from(weth_amount_out);
        if weth_amount_out > volume {
            let profit = weth_amount_out - volume;
            println!("Profit: {profit} WETH");
        } else {
            println!("Sosi huy.");
        }
    }
    print!("-> ");
    execution_time.print_elapsed_time();

    Ok(())
}
