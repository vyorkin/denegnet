use anyhow::anyhow;
use std::sync::Arc;

use alloy::{
    network::Ethereum,
    primitives::{Address, Bytes, U256},
    providers::{
        Identity, Provider, RootProvider,
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
    },
    sol_types::SolValue,
};
use revm::{
    Context, ExecuteEvm, MainBuilder, MainContext,
    context::result::{ExecutionResult, Output},
    database::{AlloyDB, CacheDB, WrapDatabaseAsync},
    primitives::{TxKind, keccak256},
    state::{AccountInfo, Bytecode},
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

const CACHE_DIR: &str = ".evm_cache";

// Warning: always make sure to compare the
// results of your simulations with standard eth_call.

// This approach reduces number of RPC calls by:
// 1. Caching contract's bytecode.
// 2. Mocking account's balance and nonce with zero values.
pub async fn init_account(
    address: Address,
    cache_db: &mut AlloyCacheDB,
    provider: RevmProvider,
) -> anyhow::Result<()> {
    let cache_key = format!("bytecode-{:?}", address);

    let bytecode = match cacache::read(CACHE_DIR, cache_key.clone()).await {
        Ok(bytes) => {
            let bytes = Bytes::from(bytes);
            Bytecode::new_raw(bytes)
        }
        Err(_) => {
            let bytes = provider.get_code_at(address).await?;
            let bytecode = Bytecode::new_raw(bytes.clone());
            cacache::write(CACHE_DIR, cache_key, bytes.to_vec()).await?;
            bytecode
        }
    };

    init_account_with_bytecode(address, bytecode, cache_db).await
}

pub async fn init_account_with_bytecode(
    address: Address,
    bytecode: Bytecode,
    cache_db: &mut AlloyCacheDB,
) -> anyhow::Result<()> {
    let code_hash = bytecode.hash_slow();
    let account_info = AccountInfo {
        balance: U256::ZERO,
        nonce: 0u64,
        code: Some(bytecode),
        code_hash,
    };

    cache_db.insert_account_info(address, account_info);

    Ok(())
}

pub fn insert_mapping_storage_slot(
    contract: Address,
    slot: U256,
    slot_address: Address,
    value: U256,
    cache_db: &mut AlloyCacheDB,
) -> anyhow::Result<()> {
    let hashed_balance_slot = keccak256((slot_address, slot).abi_encode());

    cache_db.insert_account_storage(contract, hashed_balance_slot.into(), value)?;
    Ok(())
}
