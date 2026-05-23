use balius_sdk::{
    txbuilder::{
        Address, AssetName, FeeChangeReturn, MinUtxoLovelace, MintBuilder, OutputBuilder,
        ReferenceScript, TxBuilder, UtxoSource,
    },
    NewTx, WorkerResult,
};
use balius_sdk::{Config, FnHandler, Params, Worker};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

#[derive(Serialize, Deserialize, Clone)]
struct FaucetConfig {
    validator: ReferenceScript,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct ClaimRequest {
    token: AssetName,
    quantity: u64,
    #[serde_as(as = "DisplayFromStr")]
    recipient: Address,
    fuel: UtxoSource,
}

balius_sdk::define_asset_class!(FaucetAsset, b"abcabcababcabcababcabcababca");

fn claim(config: Config<FaucetConfig>, params: Params<ClaimRequest>) -> WorkerResult<NewTx> {
    let new_asset = FaucetAsset::value(params.token.clone(), params.quantity);

    let tx = TxBuilder::new()
        .with_reference_input(config.validator.clone())
        .with_input(params.fuel.clone())
        .with_mint(
            MintBuilder::new()
                .with_asset(new_asset.clone())
                .using_redeemer(()),
        )
        .with_output(
            OutputBuilder::new()
                .address(params.recipient.clone())
                .with_value(MinUtxoLovelace)
                .with_value(new_asset.clone()),
        )
        .with_output(FeeChangeReturn(params.fuel.clone()));

    Ok(NewTx(Box::new(tx)))
}

#[balius_sdk::main]
fn main() -> Worker {
    Worker::new().with_request_handler("claim", FnHandler::from(claim))
}

#[cfg(test)]
mod tests {
    use super::*;

    use balius_sdk::txbuilder::{primitives, Address, Hash, UtxoSet};

    use std::{collections::HashMap, str::FromStr as _};

    #[test]
    fn test_free_mint() {
        let output = primitives::TransactionOutput::PostAlonzo(primitives::PostAlonzoTransactionOutput {
            address: Address::from_bech32("addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x").unwrap().to_vec().into(),
            value: primitives::Value::Coin(5_000_000),
            datum_option: None,
            script_ref: None,
        }.into());

        let cbor = pallas_codec::minicbor::to_vec(&output).unwrap();

        let test_utxos: HashMap<_, _> = vec![(
            "f7d3837715680f3a170e99cd202b726842d97f82c05af8fcd18053c64e33ec4f#0"
                .parse()
                .unwrap(),
            cbor,
        )]
        .into_iter()
        .collect();

        let _ledger = UtxoSet::from(test_utxos);

        let config = FaucetConfig {
            validator: ReferenceScript {
                hash: Hash::from_str("ef7a1cebb2dc7de884ddf82f8fcbc91fe9750dcd8c12ec7643a99bbe").unwrap(),
                address: Address::from_bech32("addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x").unwrap(),
                ref_txo: primitives::TransactionInput {
                    transaction_id: Hash::from_str(
                        "f7d3837715680f3a170e99cd202b726842d97f82c05af8fcd18053c64e33ec4f",
                    )
                    .unwrap(),
                    index: 0,
                },
            },
        };

        let request = ClaimRequest {
            token: AssetName::new(b"TEST".to_vec().into()).unwrap(),
            quantity: 1,
            recipient: Address::from_bech32("addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x").unwrap(),
            fuel: UtxoSource::Refs(vec!["f7d3837715680f3a170e99cd202b726842d97f82c05af8fcd18053c64e33ec4f#0".parse().unwrap()]),
        };

        dbg!(serde_json::to_string(&request).unwrap());

        let _tx = claim(Config(config), Params(request)).unwrap();

        //let tx = balius_sdk::txbuilder::build(tx.0, ledger).unwrap();
    }
}
