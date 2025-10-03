use anyhow::anyhow;
use std::sync::Arc;

use alloy::{
    network::Ethereum,
    primitives::{Address, Bytes, U256},
    providers::{
        Identity, RootProvider,
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
    },
};
use revm::{
    Context, ExecuteEvm, MainBuilder, MainContext,
    context::result::{ExecutionResult, Output},
    database::{AlloyDB, CacheDB, WrapDatabaseAsync},
    primitives::TxKind,
};

pub type AlloyCacheDB = CacheDB<WrapDatabaseAsync<AlloyDB<Ethereum, RevmProvider>>>;

pub type RevmProvider = Arc<
    FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
>;

pub fn init_cache_db(provider: RevmProvider) -> AlloyCacheDB {
    CacheDB::new(WrapDatabaseAsync::new(AlloyDB::new(provider, Default::default())).unwrap())
}

pub fn revm_call(
    from: Address,
    to: Address,
    calldata: Bytes,
    cache_db: &mut AlloyCacheDB,
) -> anyhow::Result<Bytes> {
    let mut evm = Context::mainnet()
        .with_db(cache_db)
        .modify_tx_chained(|tx| {
            tx.caller = from;
            tx.kind = TxKind::Call(to);
            tx.data = calldata;
            tx.value = U256::ZERO;
        })
        .build_mainnet();

    let ref_tx = evm.replay()?;
    let result = ref_tx.result;

    let value = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => {
            return Err(anyhow!("execution failed: {result:?}"));
        }
    };

    Ok(value)
}
