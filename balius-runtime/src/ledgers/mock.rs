use crate::wit::balius::app::ledger as wit;

#[derive(Clone)]
pub struct Ledger;

impl Ledger {
    pub async fn read_utxos(
        &mut self,
        _refs: Vec<wit::TxoRef>,
    ) -> Result<Vec<wit::Utxo>, wit::LedgerError> {
        let output = pallas::ledger::primitives::babbage::TransactionOutput::PostAlonzo(pallas::ledger::primitives::babbage::PostAlonzoTransactionOutput {
            address: pallas::ledger::addresses::Address::from_bech32("addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x").unwrap().to_vec().into(),
            value: pallas::ledger::primitives::babbage::Value::Coin(5_000_000),
            datum_option: None,
            script_ref: None,
        }.into());

        let cbor = pallas::codec::minicbor::to_vec(&output).unwrap();

        Ok(vec![wit::Utxo {
            ref_: wit::TxoRef {
                tx_hash: hex::decode(
                    "f7d3837715680f3a170e99cd202b726842d97f82c05af8fcd18053c64e33ec4f",
                )
                .unwrap(),
                tx_index: 0,
            },
            body: cbor,
        }])
    }

    pub async fn search_utxos(
        &mut self,
        _pattern: wit::UtxoPattern,
        _start: Option<String>,
        _max_items: u32,
    ) -> Result<wit::UtxoPage, wit::LedgerError> {
        todo!()
    }

    pub async fn read_params(&mut self) -> Result<wit::Json, wit::LedgerError> {
        let bytes = include_bytes!("./mock_pparams.json");
        Ok(bytes.into())
    }
}
