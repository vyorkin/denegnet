use alloy::primitives::U256;

pub fn volumes(from: U256, to: U256, count: usize) -> Vec<U256> {
    let step = (to - from) / U256::from(count);
    (1..=count).map(|i| step * U256::from(i)).rev().collect()
}
