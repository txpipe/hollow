# Order book dApp

## What is an order book?
An order book is a list of token trades. Let’s see an example.
Alice wants to get `k1` tokens of `assetA` in exchange for `k2` tokens of `assetB`. For this end, Alice creates an order that holds those `k2` tokens of `assetB` and her intent to trade for `k1` tokens of `assetA`. Bob is looking through the available orders and resolves Alice’s order by depositing his `k1` tokens of `assetA` to Alice and he receives the `k2` tokens of `assetB` that Alice deposited. 

## Design overview
Each time a user starts an order, a new script UTxO is created. This UTxO contains, in its datum, information specific to that instance: payment details along with the sender addresses. To ensure the initial conditions of the orders, a special "Control Token" is minted at the start.

The script is the same for every order available. Anyone can see all the orders that were opened by other users and resolve them, or cancel their own. The "Control Token" Minting Policy remains constant for every order. When an order is canceled or resolved, the corresponding UTxO is spent, and funds go to the corresponding wallet addresses. The "Control Token" is then burned. 

## Endpoints Specification

We will assume that the users will interact with this application through HTTP. For this, each operation will be implemented in its own endpoint. We use HTTP to simplify the specification, but Hollow supports multiple connections through its generic connector interface.

We will have the following operations:
- Create an order
- Cancel an order
- Resolve an order
- List available orders

Here’s an overview of the input/output of each operation’s endpoint.

Create Order
```ts
URL: /create
Body: {
  sender_address: string,
  sent: [{name: string, policy_id: string }, number],
  receive: [{name: string, policy_id: string}, number],
}
Response:  Unbalanced/Partial Tx
```

Cancel Order
```ts
URL: /cancel:
Body: {
  sender_address: string,
  tx_out_ref: string,
}
Response:  Unbalanced/Partial Tx
```

Resolve Order
```ts
URL: /resolve
Body: {
  receiver_address: string,
  tx_out_ref: string,
}
Response: Unbalanced/Partial Tx
```
List Orders
```ts
URL: /list
Body: {
  cursor: string, limit: number
}
Response: Serialized orders
```

## Implementation with Hollow
In this section, we will review the key aspects of Hollow by implementing the previous specification. Because many aspects of the implementation are repetitive, and the goal is to be concise, we will only show some parts of the final code, but the complete implementation can be found [here](...).

This dApp will need to address two types of events: Those triggered by a user action and those by the blockchain. [Hollow supports four types of events](https://github.com/txpipe/hollow/blob/main/adr/003_available_event_types.md). The relevant ones for this example will be the [`Request`](https://github.com/txpipe/hollow/blob/main/adr/003_available_event_types.md#request-event) event and the [`Chain`](https://github.com/txpipe/hollow/blob/main/adr/003_available_event_types.md#chain-event) event.
Before entering into the details of these two events, we can quickly mention that the remaining events are [`PubSub`](https://github.com/txpipe/hollow/blob/main/adr/003_available_event_types.md#pubsub-event) and [`Timer`](https://github.com/txpipe/hollow/blob/main/adr/003_available_event_types.md#timer-event) events. The `PubSub` event can be thought of as similar to the `Request` event but without expecting any response, and the `Timer` event, as its name suggests, helps us to setup a recurring action to be performed by some function.

### Functions triggered by users
Before starting this section, a quick disclaimer is that when we mention “users”, we clearly refer to any piece of software able to perform HTTP requests; it could be a frontend, middleware, etc. Also, the interaction is achieved through the Hollow Runtime, which publishes messages on different topics. When a message is published on a topic, the Hollow Runtime executes the corresponding function according to the specified topic.

Implementing a new request event is done by associating an `on_request` attribute together with a `topic`, with a function. The function’s signature will have some reasonable restrictions: It must have only one argument, which will be the request's payload, and the resulting type must be encapsulated inside the `Result` type. Besides that, every involved type must be serializable.

A necessary data structure that is worth introducing is UnbalancedTx. This kind of transaction will contain minimal information to perform the corresponding action, in this particular case, be it the creation, cancellation, or resolution of an order. To be clear, this transaction will not have any information about inputs for paying fees, for instance. From there, the “unbalanced” nickname.
The structure will encapsulate a string representing a CBOR encoded transaction.

```rust
struct UnbalancedTx { unbalancedTx: string }
```

To fulfill the specification, we need to implement the request events `create`, `list`, `resolve`, and `cancel`. Except for the listing event that will return a list of orders, the remaining three will return an unbalanced transaction ready to be balanced, signed, and submitted if successful.

It’s very important to note the serialization of each payload type: [`CreateOrderPayload`](...), [`Pagination`](...), [`ResolveOrderPayload`](...), and [`CancelOrderPayload`](...). Coincide with the specification.

- `Create`, it’s subscribed to the topic create in the runtime, and it receives a payload of type CreateOrderPayload, as we already mentioned, it needs to be serializable. The resulting transaction creates an order.
```rust
#[on_request(topic = "create")]
fn create(order_data: CreateOrderPayload) -> Result<UnbalancedTx> {
    todo!()
}
```
- `list`, it’s subscribed to the topic list in the runtime, and it receives a payload of type Pagination. This function will return unresolved orders to the user.
```rust
#[on_request(topic = "list")]
fn list(page: Pagination) -> Result<[Order]> {
    todo!()
}
```
- `resolve`, it’s subscribed to the topic resolve in the runtime, and it receives a payload of type ResolveOrderPayload. The resulting transaction will resolve an order.
```rust
#[on_request(topic = "resolve")]
fn resolve(resolve_data: ResolveOrderPayload)->Result<UnbalancedTx> {
    todo!()
}
```
- `cancel`, it’s subscribed to the topic cancel in the runtime, and it receives a payload of type CancelOrderPayload. The resulting transaction will cancel an order.
```rust
#[on_request(topic = "cancel")]
fn cancel(request: CancelOrderPayload) -> Result<UnbalancedTx> {
    todo!()
}
```

Up to this point, we could try our events in the Hollow runtime by simply completing each function with “dummy” values: It could be an empty string for transactions and an empty list for the listing. By doing this, we could test that everything is starting to fit together.

It’s interesting to differentiate the event subscription from the dApp's business logic. Thus, the [Transaction Building](#transaction-building) section will address the proper completion of these functions.

### Functions triggered by events in the blockchain. 
These functions are subscribed to specific events in the blockchain. When an event occurs, the associated function(s) gets the data from it (this could be a transaction, UTxO, etc).

on_order_book_change This function will be triggered when a UTxO is produced to (or consumed from) the address ORDER_BOOK_ADDRESS. By specifying the type Transaction, Hollow fills the parameter tx with the fully resolved Cardano transaction that triggered the event. The function will be in charge of syncing the database with the on-chain order book.
```rust
#[on_chain(address = ORDER_BOOK)]
fn on_order_book_change(tx: Transaction)
{ … }
```

### Transaction building
Building a transaction includes many parts (or phases), which we can roughly identify as follows:
- Building a transaction containing only the relevant information (inputs, outputs, scrips, datums) related to the dApp's business logic. We say this transaction is unbalanced, meaning it's most probably (at least) missing some inputs for paying fees and, therefore, some outputs with the remaining ADAs after fees.
- Balance the transaction. As we previously mentioned, in this phase, we need to include all the inputs required to pay fees and all the tokens paid to the transaction's outputs that weren’t already in the inputs.
- Sign and submit. Lastly, we need to include all the required signatures and submit the transaction to the Blockchain.

The quick description of these phases doesn’t intend to be thorough but will help us fix some terminology for what follows. Hollow provides support for all these phases, particularly for the transaction building through the [pallas](https://github.com/txpipe/pallas/tree/main/pallas-txbuilder) library.

We will focus on the [`create`](...) function, which is associated with the corresponding `on_request` topic. The remaining functions can be found in the complete example. This function is in charge of building a transaction that must create a UTxO that will represent the order, locking the number of tokens the user is offering plus a minted (by the transaction) control token, and including correct datum information.

The diagram of the unbalanced transaction we want to build is:
<p align="center">
  <img src="https://github.com/filabs-dev/hollow-examples/assets/73836359/335c27a6-9351-4d7b-ab0f-1bbc44eabcda" />
</p>

```rust
#[on_request(topic = "create")]
fn create(order_data: CreateOrderPayload) -> Result<UnbalancedTx> {

    let (sent_asset, sent_qty) = order_data.sent;
    let (receive_asset, receive_qty) = order_data.receive;

    if (sent_qty > 0 && receive_qty > 0) {

	let datum = OrderDatum{
	    sender_address: order_data.sender_address,
	    receive_amount: receive_qty,
	    receive_assetclass: receive_asset
	};

	let tx = StagingTransaction::new()
	    .output(
		Output::new(ORDER_BOOK, MIN_ADA)
		    .add_asset(sent_asset.policy_id, sent_asset.name, sent_qty)
		    .add_asset(ORDER_MINTING_POLICY, CONTROL_TOKEN_NAME, 1)
		    .set_inline_datum(datum.cbor())
    	    )
	    .mint_asset(ORDER_MINTING_POLICY, CONTROL_TOKEN_NAME, 1)
	    .reference_input(MINTING_REF_UTXO);

	Ok(UnbalancedTx { unbalancedTx: tx.cbor_hex() });
    } else {
	Err(InvalidPayloadWrongTokenQuantity);
    }
}
```

The `create` function it has two possible outcomes, it’s successful and it builds an unbalanced transaction. Or fails because of some incorrect payload information. Let’s focus on the transaction building, we start by creating an “empty” transaction `StagingTransaction::new()`. Then, following the diagram we must include a new output and mint the control token. Besides that, we include as reference input with a reference script of the minting policy.

### Database management
Hollow provides a way to interact with an instance of key-value storage (with namespaces).
```rust
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

fn list(pagination: Pagination) -> Result<[Order]> {
    let kv_storage = use_extension::<Storage>(ORDER_BOOK_NAMESPACE);
    let order_refs = kv_storage.keys(Some(pagination.cursor), pagination.limit);
    orders.filter_map(|utxo_ref| kv_storage.get::<Order>(utxo_ref))
}
```
With `use_extension::<Storage>("example1")` we get access to the key-value storage indexed in the `"example1"` namespace. The access in the `kv_storage` which holds methods that we can use to interact with the storage. Those methods are:

- `kv_storage.set(k, v)` Saves under the key `k` the serialized representation of `v`.
- `kv_storage.get<T>(k)` Deserializes into the (deserializable) type `T` the value saved under the key `k` in the namespace of the `kv_storage`.
- `kv_storage.keys(maybe_cursor, limit)` Returns at most the amount `limit` of keys in the storage added after the key `maybe_key` (`maybe_key` is of type `Option<String>`). If the `maybe_key` is `None`, then the keys are returned from the beginning.

### Buyer Bot with Hollow

Let's suppose you are really interested in exchanging a couple of particular tokens. Of course, setting up a minimum price that you are willing to pay. Up to this point, we saw that we can use `Request` events to build transactions and `Chain` events to keep updated the book order database. But, we could use this last kind of event to build a transaction to buy some particular order that is in our interest. We already mentioned that Hollow supports the balance, signing, and submission of a transaction thus we can put everything together in the `buyer_bot` function.

```rust
#[on_chain(mints=ORDER_MINTING_POLICY)]
fn buyer_bot(tx: Tx) {
    let mut orders = vec![];
    for (output_index, output) in tx.outputs.iter().enumerate() {
	let order_value = output
	    .non_ada_assets
	    .iter()
	    .filter(|x| is_control_token(x) || is_token_to_buy(x))
	    .collect();

	if (is_valid_order_value(order_value)) {
	    let order_datum = output.datum_as::<OrderDatum>();
	    if (order_datum.receive_assetclass == TOKEN_TO_SELL && is_valid_price(order_value, order_datum.amount)) {
		orders.add(output_index);
	    }
	}
    }
    if (!orders.is_empty()) {
	// At the moment we only consume the first order.
        let order_index = orders.into_iter().nth(0);
	let order_to_resolve = OutputRef::new(tx.hash(), order_index);
        let unbalancedTx = build_tx_resolve(order_to_resolve, , wallet::address());

        // "crane::wallet" maybe?
        // Probably the balance function could have an Option argument, to specify some
        // balancing algorithm (https://hackage.haskell.org/package/cardano-coin-selection)
	let balancedTx = wallet::balance(unbalancedTx);
	let signedTx = wallet::sign(balancedTx);
	crane::submit(signedTx);
    }
}

```