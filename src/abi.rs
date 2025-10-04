use std::cmp::min;

use alloy::{
    primitives::{Address, Bytes, U160, U256, aliases::U24},
    sol,
    sol_types::{SolCall, SolValue},
};

sol! {
    struct QuoteExactInputSingleParams {
        address tokenIn;
        address tokenOut;
        uint256 amountIn;
        uint24 fee;
        uint160 sqrtPriceLimitX96;
    }

    function quoteExactInputSingle(QuoteExactInputSingleParams memory params)
    public
    override
    returns (
        uint256 amountOut,
        uint160 sqrtPriceX96After,
        uint32 initializedTicksCrossed,
        uint256 gasEstimate
    );
}

pub fn decode_quote_response(response: Bytes) -> anyhow::Result<u128> {
    let (amount_out, _, _, _) = <(u128, u128, u32, u128)>::abi_decode(&response)?;
    Ok(amount_out)
}

/// Returns bytes encoded calldata for the `quoteExactInputSingle` quoting function.
pub fn quote_calldata(token_in: Address, token_out: Address, amount_in: U256, fee: u32) -> Bytes {
    let zero_for_one = token_in < token_out;

    // The `sqrt_price_limit_x96` represents price limit that we are willing to pay.
    // However, since our MEV statergy assumes that we will be the first to interact with
    // pool in the new block, we use MIN/MAX values (depending on which token we exchange).
    let sqrt_price_limit_x96: U160 = if zero_for_one {
        "4295128749".parse().unwrap() // min value
    } else {
        "1461446703485210103287273052203988822378723970341" // max value
            .parse()
            .unwrap()
    };

    let params = QuoteExactInputSingleParams {
        tokenIn: token_in,
        tokenOut: token_out,
        amountIn: amount_in,
        fee: U24::from(fee),
        sqrtPriceLimitX96: sqrt_price_limit_x96,
    };

    Bytes::from(quoteExactInputSingleCall { params }.abi_encode())
}

sol! {
    function getAmountOut(address pool, bool zeroForOne, uint256 amountIn) external;
}

pub fn get_amount_out_calldata(
    pool: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
) -> Bytes {
    Bytes::from(
        getAmountOutCall {
            pool,
            zeroForOne: token_in < token_out,
            amountIn: amount_in,
        }
        .abi_encode(),
    )
}

pub fn decode_get_amount_out_response(response: Bytes) -> anyhow::Result<u128> {
    let value = response.to_vec();
    let last_64_bytes = &value[value.len() - 64..];
    let (a, b) = <(i128, i128)>::abi_decode(last_64_bytes)?;
    let value_out = min(a, b);
    let value_out = -value_out;
    Ok(value_out as u128)
}
