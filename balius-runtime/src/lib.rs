use pallas::ledger::traverse::MultiEraBlock;
use serde_json::json;
use std::{collections::HashSet, path::Path};
use thiserror::Error;

mod wit {
    wasmtime::component::bindgen!({
        path:"../wit",
        async: true,
        tracing: true,
    });
}

mod adapter;
mod loader;
mod router;
mod store;

// implementations
pub mod ledgers;

pub use store::Store;

pub type WorkerId = String;

#[derive(Error, Debug)]
pub enum Error {
    #[error("wasm error {0}")]
    Wasm(wasmtime::Error),

    #[error("store error {0}")]
    Store(redb::Error),

    #[error("worker not found '{0}'")]
    WorkerNotFound(WorkerId),

    #[error("worker failed to handle event (code: '{0}', message: '{1}')")]
    Handle(u32, String),

    #[error("no target available to solve request")]
    NoTarget,

    #[error("more than one target available to solve request")]
    AmbiguousTarget,

    #[error("address in block failed to parse")]
    BadAddress(pallas::ledger::addresses::Error),

    #[error("ledger error: {0}")]
    Ledger(String),
}

impl From<wasmtime::Error> for Error {
    fn from(value: wasmtime::Error) -> Self {
        Self::Wasm(value)
    }
}

impl From<redb::Error> for Error {
    fn from(value: redb::Error) -> Self {
        Self::Store(value)
    }
}

impl From<redb::DatabaseError> for Error {
    fn from(value: redb::DatabaseError) -> Self {
        Self::Store(value.into())
    }
}

impl From<redb::TransactionError> for Error {
    fn from(value: redb::TransactionError) -> Self {
        Self::Store(value.into())
    }
}

impl From<redb::TableError> for Error {
    fn from(value: redb::TableError) -> Self {
        Self::Store(value.into())
    }
}

impl From<redb::StorageError> for Error {
    fn from(value: redb::StorageError) -> Self {
        Self::Store(value.into())
    }
}

impl From<pallas::ledger::addresses::Error> for Error {
    fn from(value: pallas::ledger::addresses::Error) -> Self {
        Self::BadAddress(value.into())
    }
}

pub type BlockSlot = u64;
pub type BlockHash = pallas::crypto::hash::Hash<32>;

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct ChainPoint(pub BlockSlot, pub BlockHash);

pub type LogSeq = u64;

#[derive(Clone)]
pub struct Runtime {
    loader: loader::Loader,
    router: router::Router,
    store: store::Store,
    ledger: ledgers::Ledger,
}

impl Runtime {
    pub fn new(store: store::Store, ledger: ledgers::Ledger) -> Result<Self, Error> {
        let router = router::Router::new();

        Ok(Self {
            loader: loader::Loader::new(router.clone())?,
            router,
            store,
            ledger,
        })
    }

    pub fn cursor(&self) -> Result<Option<LogSeq>, Error> {
        let cursor = self.store.lowest_cursor()?;

        Ok(cursor)
    }

    pub async fn register_worker(
        &mut self,
        id: &str,
        wasm_path: impl AsRef<Path>,
        config: serde_json::Value,
    ) -> Result<(), Error> {
        self.loader
            .register_worker(id, wasm_path, config, self.ledger.clone())
            .await?;

        Ok(())
    }

    async fn fire_and_forget(
        &self,
        event: &wit::Event,
        targets: HashSet<router::Target>,
    ) -> Result<(), Error> {
        for target in targets {
            let result = self
                .loader
                .dispatch_event(&target.worker, target.channel, event)
                .await;

            match result {
                Ok(wit::Response::Acknowledge) => {
                    tracing::debug!(worker = target.worker, "worker acknowledge");
                }
                Ok(_) => {
                    tracing::warn!(worker = target.worker, "worker returned unexpected data");
                }
                Err(Error::Handle(code, message)) => {
                    tracing::warn!(code, message);
                }
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    pub async fn apply_block(
        &self,
        block: &MultiEraBlock<'_>,
        wal_seq: LogSeq,
    ) -> Result<(), Error> {
        for tx in block.txs() {
            for utxo in tx.outputs() {
                let targets = self.router.find_utxo_targets(&utxo)?;
                let event = wit::Event::Utxo(utxo.encode());

                self.fire_and_forget(&event, targets).await?;
            }
        }

        Ok(())
    }

    pub fn undo_block(&self, block: &MultiEraBlock, wal_seq: LogSeq) -> Result<(), Error> {
        Ok(())
    }

    pub async fn handle_request(
        &self,
        worker: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let target = self.router.find_request_target(worker, method)?;

        let evt = wit::Event::Request(serde_json::to_vec(&params).unwrap());

        let reply = self
            .loader
            .dispatch_event(&target.worker, target.channel, &evt)
            .await?;

        let json = match reply {
            wit::Response::Acknowledge => json!({}),
            wit::Response::Json(x) => serde_json::from_slice(&x).unwrap(),
            wit::Response::Cbor(x) => json!({ "cbor": x }),
            wit::Response::PartialTx(x) => json!({ "tx": x }),
        };

        Ok(json)
    }
}