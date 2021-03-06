use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use borsh::BorshDeserialize;

use near_primitives::hash::CryptoHash;
use near_primitives::transaction::ExecutionOutcomeWithIdAndProof;
use near_primitives::version::DbVersion;

use crate::db::{DBCol, RocksDB, VERSION_KEY};
use crate::Store;
use near_primitives::sharding::ShardChunk;

pub fn get_store_version(path: &str) -> DbVersion {
    RocksDB::get_version(path).expect("Failed to open the database")
}

pub fn set_store_version(store: &Store, db_version: u32) {
    let mut store_update = store.store_update();
    store_update.set(
        DBCol::ColDbVersion,
        VERSION_KEY,
        &serde_json::to_vec(&db_version).expect("Failed to serialize version"),
    );
    store_update.commit().expect("Failed to write version to database");
}

fn get_outcomes_by_block_hash(store: &Store, block_hash: &CryptoHash) -> HashSet<CryptoHash> {
    match store.get_ser(DBCol::ColOutcomesByBlockHash, block_hash.as_ref()) {
        Ok(Some(hash_set)) => hash_set,
        Ok(None) => HashSet::new(),
        Err(e) => panic!("Can't read DB, {:?}", e),
    }
}

pub fn fill_col_outcomes_by_hash(store: &Store) {
    let mut store_update = store.store_update();
    let outcomes: Vec<ExecutionOutcomeWithIdAndProof> = store
        .iter(DBCol::ColTransactionResult)
        .map(|key| {
            ExecutionOutcomeWithIdAndProof::try_from_slice(&key.1)
                .expect("BorshDeserialize should not fail")
        })
        .collect();
    let mut block_hash_to_outcomes: HashMap<CryptoHash, HashSet<CryptoHash>> = HashMap::new();
    for outcome in outcomes {
        match block_hash_to_outcomes.entry(outcome.block_hash) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().insert(outcome.id().clone());
            }
            Entry::Vacant(entry) => {
                let mut hash_set = get_outcomes_by_block_hash(store, &outcome.block_hash);
                hash_set.insert(outcome.id().clone());
                entry.insert(hash_set);
            }
        };
    }
    for (block_hash, hash_set) in block_hash_to_outcomes {
        store_update
            .set_ser(DBCol::ColOutcomesByBlockHash, block_hash.as_ref(), &hash_set)
            .expect("BorshSerialize should not fail");
    }
    store_update.commit().expect("Failed to migrate");
}

pub fn fill_col_transaction_refcount(store: &Store) {
    let mut store_update = store.store_update();
    let chunks: Vec<ShardChunk> = store
        .iter(DBCol::ColChunks)
        .map(|key| ShardChunk::try_from_slice(&key.1).expect("BorshDeserialize should not fail"))
        .collect();

    let mut tx_refcount: HashMap<CryptoHash, u64> = HashMap::new();
    for chunk in chunks {
        for tx in chunk.transactions {
            tx_refcount.entry(tx.get_hash()).and_modify(|x| *x += 1).or_insert(1);
        }
    }
    for (tx_hash, refcount) in tx_refcount {
        store_update
            .set_ser(DBCol::ColTransactionRefCount, tx_hash.as_ref(), &refcount)
            .expect("BorshSerialize should not fail");
    }
    store_update.commit().expect("Failed to migrate");
}
