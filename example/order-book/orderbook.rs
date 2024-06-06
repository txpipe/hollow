use serde::{Deserialize, Serialize};

const ORDER_MINTING_POLICY: &str = "";
const CONTROL_TOKEN_NAME: &str = "";
const ORDER_BOOK: &str = "";
const ORDER_BOOK_NAMESPACE: &str = "";
const VALIDATOR_REF_UTXO: &str = "";

#[derive(Serialize, Deserialize, Debug)]
struct AssetClass {
    name: String,
    policy_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateOrderPayload {
    sender_address: String,
    sent: (AssetClass, u64),
    receive: (AssetClass, u64),
}

#[derive(Serialize, Deserialize, Debug)]
struct AddressDetails {
    address_type: AddressType,
    network: Network,
    address_bech32: String,
    address_hex: String,
    payment_credential: Option<Credential>,
    stake_credential: Option<Credential>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Pagination {
    cursor: Option<String>,
    limit: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ResolveOrderPayload {
    receiver_address: String,
    tx_out_ref: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CancelOrderPayload {
    sender_address: String,
    tx_out_ref: String,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
struct OrderDatum {
    sender_address: AddressType,
    receive_amount: u64,
    receive_assetclass: AssetClass
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
enum OrderRedeemer {
	Cancel,
	Resolve
}

#[on_request(topic = "create")]
fn create(order_data: CreateOrderPayload) -> Result<UnbalancedTx> {
    let (sent_asset, sent_qty) = order_data.sent;
    let (receive_asset, receive_qty) = order_data.receive;

    if (sent_qty > 0 && receive_qty > 0) {
        let datum = OrderDatum {
            sender_address: order_data.sender_address,
            receive_amount: receive_qty,
            receive_assetclass: receive_asset,
        };

        let tx = StagingTransaction::new()
            .output(
                Output::new(ORDER_BOOK, MIN_ADA)
                    .add_asset(sent_asset.policy_id, sent_asset.name, sent_qty)
                    .add_asset(ORDER_MINTING_POLICY, CONTROL_TOKEN_NAME, 1)
                    .set_inline_datum(datum.cbor()),
            )
            .mint_asset(ORDER_MINTING_POLICY, CONTROL_TOKEN_NAME, 1)
            .reference_input(MINTING_REF_UTXO);

        Ok(UnbalancedTx {
            unbalancedTx: tx.cbor_hex(),
        });
    } else {
        Err(InvalidPayloadWrongTokenQuantity);
    }
}

#[on_request(topic = "list")]
fn list(pagination: Pagination) -> Result<[Order]> {
    let kv_storage = use_extension::<Storage>(ORDER_BOOK_NAMESPACE);
    let order_refs = kv_storage.keys(Some(pagination.cursor), pagination.limit);
    orders.filter_map(|utxo_ref| kv_storage.get::<Order>(utxo_ref))
}

#[on_request(topic = "resolve")]
fn resolve(resolve_data: ResolveOrderPayload) {
    let payment_cred_hash = Address::payment_credential(resolve_data.receiver_address);
    build_tx_resolve(resolve_data.tx_out_ref, resolve_data.tx_out_ref)
}

fn build_tx_resolve(utxo_ref: OutputRef, payment_address: Address) -> UnbalancedTx {
    let (tx_hash, index)= utxo_ref;
    let utxo = UTxO::by_ref(utxo_ref);
    let datum = utxo.datum_as::<OrderDatum>();
    let (receive_asset, receive_qty) = order_data.receive;
    let sender_address = datum.sender_address;
    let control_token_unit = format!("{}{}", ORDER_MINTING_POLICY, CONTROL_TOKEN_NAME);

    let tx = StagingTransaction::new()
	.add_spend_redeemer(
	    Input {
		tx_hash: tx_hash,
		txo_index: index,
	    },
	    Resolve.cbor(),
	    None
	)
	.disclosed_signer(payment_address)
	.reference_input(VALIDATOR_REF_UTXO);
        .mint_asset(ORDER_MINTING_POLICY, CONTROL_TOKEN_NAME, -1)
	.output(
	    Output::new(sender_address, MIN_ADA)
		.add_asset(receive_asset.policy_id, receive_asset.name, datum.receiver_amount)
	)
        .add_metadata(
            674,
            String::from("ResolveOrder"),
        );

    UnbalancedTx { unbalancedTx: tx.cbor_hex() }
}

#[on_request(topic = "cancel")]
fn cancel(cancel_data: CancelOrderPayload) -> Result<UnbalancedTx> {
    let payment_cred_hash = Address::payment_credential(cancel_data.sender_address);
    let utxo_ref = cancel_data.tx_out_ref;

    let (tx_hash, index)= utxo_ref;
    let utxo = UTxO::by_ref(utxo_ref);

    let tx = StagingTransaction::new()
	.add_spend_redeemer(
	    Input {
		tx_hash: tx_hash,
		txo_index: index,
	    },
	    Cancel.cbor(),
	    None
	)
	.disclosed_signer(payment_address)
	.reference_input(VALIDATOR_REF_UTXO);
        .mint_asset(ORDER_MINTING_POLICY, CONTROL_TOKEN_NAME, -1)
        .add_metadata(
            674,
            String::from("CancelOrder"),
        );

    UnbalancedTx { unbalancedTx: tx.cbor_hex() }
}

enum OrderChange {
    New(Order),
    Cancel(Order),
    Resolve(Order),
    Other,
}

#[on_chain(address=ORDER_BOOK)]
fn on_order_book_change(tx: Tx) {
    let inputs = tx.inputs;
    let outputs = tx.outputs;
    let change = match_tx_purpose(tx);

    match change {
        OrderChange::New(order) => on_new_order(order),
        OrderChange::Cancel(order) => on_order_cancellation(order),
        OrderChange::Resolve(order) => on_order_resolution(order),
        _ => (),
    }
}

fn on_new_order(order: Order) {
    // create order in storage
    let kv_storage = use_extension::<Storage>(ORDER_BOOK_NAMESPACE);
    kv_storage.set(order.utxo_ref, order);
}

fn on_order_cancellation(order: Order) {
    // remove order from storage
    let kv_storage = use_extension::<Storage>(ORDER_BOOK_NAMESPACE);
    kv_storage.remove(order.utxo_ref);
}

fn on_order_resolution(order: Order) {
    // update order in storage
    let kv_storage = use_extension::<Storage>(ORDER_BOOK_NAMESPACE);
    kv_storage.set(order.utxo_ref, order);
}
