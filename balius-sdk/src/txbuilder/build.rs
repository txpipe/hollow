use pallas_codec::utils::KeepRaw;
use std::collections::HashMap;
use std::sync::Arc;

use super::{
    asset_math, primitives, BuildContext, BuildError, Ledger, PParams, TxExpr, TxoRef, UtxoPattern,
    UtxoSet,
};

impl BuildContext {
    pub fn mint_redeemer_index(&self, policy: primitives::ScriptHash) -> Result<u32, BuildError> {
        if let Some(tx_body) = &self.tx_body {
            let mut out: Vec<_> = tx_body
                .mint
                .iter()
                .flat_map(|x| x.iter())
                .map(|(p, _)| *p)
                .collect();

            out.sort();
            out.dedup();

            if let Some(index) = out.iter().position(|p| *p == policy) {
                return Ok(index as u32);
            }
        }

        Err(BuildError::RedeemerTargetMissing)
    }

    pub fn eval_ex_units(
        &self,
        _script: primitives::ScriptHash,
        _data: &primitives::PlutusData,
    ) -> primitives::ExUnits {
        // TODO
        primitives::ExUnits { mem: 8, steps: 8 }
    }
}

pub(crate) struct ExtLedgerFacade;

impl crate::txbuilder::Ledger for ExtLedgerFacade {
    fn read_utxos(&self, refs: &[TxoRef]) -> Result<UtxoSet, BuildError> {
        let refs: Vec<_> = refs.iter().cloned().map(Into::into).collect();
        let x = crate::wit::balius::app::ledger::read_utxos(&refs)?;

        let x: Vec<_> = x
            .into_iter()
            .map(|x| (TxoRef::from(x.ref_), x.body.to_vec()))
            .collect();

        Ok(UtxoSet::from_iter(x))
    }

    fn search_utxos(&self, pattern: &UtxoPattern) -> Result<UtxoSet, BuildError> {
        let pattern = pattern.clone().into();
        let mut utxos = HashMap::new();
        let max_items = 32;
        let mut utxo_page = Some(crate::wit::balius::app::ledger::search_utxos(
            &pattern, None, max_items,
        )?);
        while let Some(page) = utxo_page.take() {
            for utxo in page.utxos {
                utxos.insert(utxo.ref_.into(), utxo.body);
            }
            if let Some(next) = page.next_token {
                utxo_page = Some(crate::wit::balius::app::ledger::search_utxos(
                    &pattern,
                    Some(&next),
                    max_items,
                )?);
            }
        }
        Ok(utxos.into())
    }

    fn read_params(&self) -> Result<PParams, BuildError> {
        let bytes = crate::wit::balius::app::ledger::read_params()?;

        serde_json::from_slice(&bytes)
            .map_err(|_| BuildError::LedgerError("failed to parse params json".to_string()))
    }
}

pub fn build<T, L>(mut tx: T, ledger: L) -> Result<primitives::Tx<'static>, BuildError>
where
    T: TxExpr,
    L: Ledger + 'static,
{
    let mut ctx = BuildContext {
        network: primitives::NetworkId::Testnet,
        pparams: ledger.read_params()?,
        total_input: primitives::Value::Coin(0),
        spent_output: primitives::Value::Coin(0),
        estimated_fee: 0,
        ledger: Arc::new(Box::new(ledger)),
        tx_body: None,
        parent_output: None,
    };

    // Build the raw transaction, so we have the info needed to estimate fees and
    // compute change.
    let body = tx.eval_body(&ctx)?;

    let input_refs: Vec<_> = body
        .inputs
        .iter()
        .map(|i| TxoRef {
            hash: i.transaction_id,
            index: i.index,
        })
        .collect();
    let utxos = ctx.ledger.read_utxos(&input_refs)?;
    ctx.total_input =
        asset_math::aggregate_values(utxos.txos().map(|txo| txo.value().into_conway()));
    if let Some(mint) = &body.mint {
        ctx.total_input = asset_math::add_mint(&ctx.total_input, mint)?;
    }
    ctx.spent_output = asset_math::aggregate_values(body.outputs.iter().map(output_into_conway));
    // TODO: estimate the fee
    ctx.estimated_fee = 2_000_000;

    // Now that we know the inputs/outputs/fee, build the "final" (unsigned)tx
    let body = tx.eval_body(&ctx)?;
    ctx.tx_body = Some(body);
    for _ in 0..3 {
        let body = tx.eval_body(&ctx)?;
        ctx.tx_body = Some(body);
    }

    let wit = tx.eval_witness_set(&ctx).unwrap();

    let tx = primitives::Tx {
        transaction_body: KeepRaw::from(ctx.tx_body.take().unwrap()),
        transaction_witness_set: KeepRaw::from(wit),
        auxiliary_data: pallas_codec::utils::Nullable::Null,
        success: true,
    };

    Ok(tx)
}

fn output_into_conway(output: &primitives::TransactionOutput<'_>) -> primitives::Value {
    use pallas_primitives::{alonzo, conway};
    match output {
        primitives::TransactionOutput::Legacy(o) => match &o.amount {
            alonzo::Value::Coin(c) => primitives::Value::Coin(*c),
            alonzo::Value::Multiasset(c, assets) => {
                let assets: conway::Multiasset<conway::PositiveCoin> = assets
                    .iter()
                    .filter_map(|(k, v)| {
                        let v: Vec<(conway::Bytes, conway::PositiveCoin)> = v
                            .iter()
                            .filter_map(|(k, v)| Some((k.clone(), (*v).try_into().ok()?)))
                            .collect();
                        if v.is_empty() {
                            None
                        } else {
                            Some((*k, v.into_iter().collect()))
                        }
                    })
                    .collect();
                if !assets.is_empty() {
                    primitives::Value::Multiasset(*c, assets)
                } else {
                    primitives::Value::Coin(*c)
                }
            }
        },
        primitives::TransactionOutput::PostAlonzo(o) => o.value.clone(),
    }
}
