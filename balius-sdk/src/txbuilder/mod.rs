pub(crate) mod asset_math;
pub mod plutus;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Builder is incomplete")]
    Incomplete,
    #[error("Conflicting requirement")]
    Conflicting,
    #[error("UTxO decoding failed")]
    UtxoDecode,
    #[error("UTxO set is empty")]
    EmptyUtxoSet,
    #[error("Transaction has no inputs")]
    MalformedScript,
    #[error("Could not decode address")]
    MalformedAddress,
    #[error("Could not decode datum bytes")]
    MalformedDatum,
    #[error("Invalid bytes length for datum hash")]
    MalformedDatumHash,
    #[error("Input/policy pointed to by redeemer not found in tx")]
    RedeemerTargetMissing,
    #[error("Invalid network ID")]
    InvalidNetworkId,
    #[error("Corrupted transaction bytes in built transaction")]
    CorruptedTxBytes,
    #[error("Public key for private key is malformed")]
    MalformedKey,
    #[error("Asset name must be 32 bytes or less")]
    AssetNameTooLong,
    #[error("Asset value must be less than 9223372036854775807")]
    AssetValueTooHigh,
    #[error("Total outputs of this transaction are greater than total inputs")]
    OutputsTooHigh,
    #[error("Invalid asset policy id hex")]
    MalformedAssetPolicyIdHex,
    #[error("Malformed TxoRef")]
    MalformedTxoRef,
    #[error("Ledger error: {0}")]
    LedgerError(String),
}

impl From<crate::wit::balius::app::ledger::LedgerError> for BuildError {
    fn from(err: crate::wit::balius::app::ledger::LedgerError) -> Self {
        BuildError::LedgerError(err.to_string())
    }
}

use std::sync::Arc;

pub use pallas_codec as codec;
pub use pallas_primitives::conway as primitives;

/// Protocol parameters surface the SDK consumes for tx-building.
///
/// The shape — wire-compatible with utxorpc-spec 0.17 pbjson, which is
/// what pre-BigInt worker decoders expect — is defined once in
/// `balius_core::proto::v0::cardano::PParams`. The runtime emits values
/// of this type via `read-params`; the SDK deserializes them here.
pub use balius_core::proto::v0::cardano::PParams;

pub trait Ledger {
    fn read_utxos(&self, refs: &[dsl::TxoRef]) -> Result<dsl::UtxoSet, BuildError>;
    fn search_utxos(&self, pattern: &dsl::UtxoPattern) -> Result<dsl::UtxoSet, BuildError>;
    fn read_params(&self) -> Result<PParams, BuildError>;
}

#[derive(Clone)]
pub struct BuildContext {
    pub network: primitives::NetworkId,
    pub pparams: PParams,
    pub total_input: primitives::Value,
    pub spent_output: primitives::Value,
    pub estimated_fee: u64,
    pub ledger: Arc<Box<dyn Ledger>>,

    pub tx_body: Option<primitives::TransactionBody<'static>>,
    pub parent_output: Option<primitives::TransactionOutput<'static>>,
}

impl BuildContext {
    pub fn with_parent_output(&self, output: primitives::TransactionOutput<'static>) -> Self {
        let mut ctx = self.clone();
        ctx.parent_output = Some(output);
        ctx
    }
}

mod build;
mod dsl;

pub use build::*;
pub use dsl::*;
