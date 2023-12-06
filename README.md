
# Hollow

Hollow is an SDK for building Headless Cardano dApps.

## About dApps

:thinking: the term "dApp" is already quite ambiguous, and now we also have "headless dApps"? WTF is a headless dApp?

We should probably start from the begining.

Cardano uses the eUTxO model, an extended version of Bitcoin's UTxO model that introduces the concept of on-chain validators, custom code that can be attached to UTxOs to enforce rules on how it can be consumed.

Notice that we didn't use the word "smart contract", this is because "validators" are just functions that assert that a transaction is valid. They are the absolute minimum code[^1] needed to make sure all parties in a transaction are playing by the rules. They don't contain any business logic (no output, no side-effects, nothing).

As a mental exercise, lets take it to the extreme. You could build any Cardano dApp in the world using an on-chain validator that always returns "transaction is valid". It would be a very insecure app since everything is allowed, but a user wouldn't be able to tell the difference when executing a valid transaction[^2].

So where is all the business logic?

In Cardano dApps, the magic happens at the moment of **building the transactions**. Since transactions in the UTxO model are deterministic, they define the outcome of the interaction. They describe how things should look like after the transaction has been applied on-chain. This is quite different to what happens in blockchains that use the alterantive "account" model, where transactions describes the "action" that the blockchain node needs to execute.

Your job as a developer is to know how to build Cardano transactions that represent the interactions with your system. Your goal is to turn external events (a button in an UI, an API call, some condition on-chain, a cron-job, etc) into transactions that represent the desired change[^3].

## Snowflakes Apps

We now understand that the business logic of our app lives outside of the blockchain, but where exactly? is it in the browser of the user? in a backend in the cloud? as a mobile app?

The fact is, it can live anywhere.

If you're building a web app that is stateless and only requires some user-input and wallet data, you could probably build your business logic in javascript and bundle it with the rest of your frontend. Something similar applies for simple mobile apps.

If you're app requires you to query some state from a database and perform complex decision-making processes, you would probably include your business logic as part of a backend component and deploy it in the cloud.

If you're building a bot that needs to continuously monitor blockchain activity and react by submitting new transactions unattendedly (eg: like a DEX batcher), you could potentially run it in your laptop as a background process.

This flexibility is a double-edged sword. For sure, is nice for developers to choose the tool and environment that is better suited for their use-case. At the same time, the lack of a common interface and framework has some nasty side-effects:

- isolated apps: each app lives in isolation from the rest of the world, constrained by it's own interface, because there's no standard way of interacting with them.
- re-invent the wheel: each project ends up building their own middleware for interacting with the blockchain (reading on-chain data, submitting transactions, etc).

The end result is "snowflake" apps, each one is unique.
## Headless Apps

"Headless dApp" refers to the idea of decoupling the business logic from the external context by forcing all inputs and outputs through a well-defined interface. To run your app, you'll need a generic runtime that knows how to connect your business logic to the outside world[^4].

This strict separation of concerns provides several benefits:

- portability: you can run your app in different contexts depending on your needs. From the terminal, on your browser, on the cloud, etc. As long as you have a compatible runtime to host your app, you should be good.
- composability: by having a concrete, programatic interface to interact with the business logic of your app, you can "compose" complex apps that orchestrating other apps to fulfill a higher-level objective.
- multiple frontends: your app can have multiple frontends, maybe even developed by different teams. For example, a DEX could have different web frontends, the user could pick their favorite.
- less plumbing: the runtime component can be quite generic and reused by many dApps. There's no need to re-implement how to query on-chain data, how to build transactions, how to submit transaction, etc. You can focus just on your business logic knowing that plumbing is already taken care of.

The `Hollow` SDK is meant to provide the required artifacts to build headless Cardano dApps in a developer friendly way. It provides all of the plumbing out-of-the-box, you just need to relax and enjoy the ride.


[^1]: Keeping the on-chain code as small as possible it's very desirable trait. On-chain code is executed one time by each node of the blockchain. This is required as part of the consensus, but very redundant from the computational perspective.
[^2]: of course, one could inspect the script on-chain and tell the difference between a real validator and a mock one, but no difference whatsoever from the UX perspective.
[^3]: everything that lives outside of the blockchain is usually referred to as "off-chain". It's a little bit egocentric if you ask me, but serves its purpose. In our experience, there's usually much more off-chain code than on-chain.
[^4]: this concept is very similar to "hexagonal architecture" or "clean architecture". If this idea of strict separation of concerns resonates with you, I encourage you to take a look at those patterns too.

## What does it look like?

The introduction sounds good and very fancy but you're not convinced until you see some code, right?

The following snippet shows how to create an HTTP endpoint from using the framework:

```rust
#[on_http_get(path = "/hello")]
fn say_hello() -> Result<JsonValue> {
    let output = json!({ "message": "hello world!" });
    Ok(output)
}
```

The following snippet shows how react to an UTxO being locked in a script address relevant for your dApp:

```rust

#[derive(Datum)]
struct MyDatum {
    foo: PlutusInt,
    bar: PlutusInt,
}

#[on_inbound_utxo(to=SCRIPT_ADDRESS)]
fn on_asset_sold(utxo: UTxO) -> Result<()> {
    // do something interesting with the new UTxO

    let datum = utxo.datum_as::<MyDatum>();
    println!(datum.foo);

    Ok(())
}
```

The following snippet shows how react to an UTxO being unlocked in a script address relevant for your dApp:

```rust
#[on_outbound_utxo(from=SCRIPT_ADDRESS)]
fn on_asset_sold(utxo: UTxO) -> Result<()> {
    // do something interesting with the UTxO data

    println!(utxo.address);
    println!(utxo.coin);

    Ok(())
}
```


## Putting it all together

The following code shows what the off-chain code looks like for basic NFT marketplace that uses the `Hollow` SDK.

The idea is simple: the marketplace address can lock NFT and release them only if the correct price for the NFT is paid. To accomplish that, our off-chain component is going to do the following:

- for each time our marketplace address receives an UTxO representing an buyable asset, we store a reference in database table.
- for each time our marketplace address releases an UTxO representing a buyable asset, we remove it from our database table.
- if someone wants to query the list available assets, we lookup available items in the db and provide them as a json values.
- if someone want to buy one of the available assets, we create a partial transaction and return it for the user to balance & sign.

Here's the code:

::warning:: this is WIP, specific function names and structures will most likely suffer some changes. The essence of the framework and the semantic meaning of the artifacts will remain.

```rust

const MARKETPLACE: &str = "addr1xxx";

#[derive(Datum)]
struct AssetDatum {
    price: PlutusInt,
}

#[chain_event]
#[match_inbound_utxo(to_address=MARKETPLACE)]
fn on_new_asset(utxo: UTxO) -> Result<()> {
    let db = use_extension::<Database>();

    let datum = utxo.datum_as::<AssetDatum>();

    if datum.is_none() {
        bail!("unexpected utxo");
    }

    for asset in utxo.assets() {
        db.execute(
            "INSERT INTO (policy, asset, price) VALUES ({}, {}, {})",
            asset.policy_id,
            asset.asset_name,
            datum.price,
        );
    }

    Ok(())
}

#[chain_event]
#[match_outbound_utxo(from_address=MARKETPLACE)]
fn on_asset_sold(utxo: UTxO) -> Result<()> {
    let db = use_extension::<Database>();

    for asset in utxo.assets() {
        db.execute(
            "DELETE FROM assets WHERE policy={} and asset={}",
            asset.policy_id,
            asset.asset_name,
        );
    }

    Ok(())
}

#[extrinsic_event]
#[match_http_route(path = "/assets", method = "GET")]
fn query_assets() -> Result<JsonValue> {
    let db = use_extension::<Database>();

    let assets = db.query_all("SELECT * from assets;").to_json();

    Ok(assets)
}

#[derive(Serialize, Deserialize)]
struct Order {
    policy_id: String,
    asset_name: String,
    buyer: Address,
}

#[extrinsic_event]
#[match_http_route(path = "/orders", method = "POST")]
fn create_order(params: Order) -> Result<PartialTx> {
    let db = use_extension::<Database>();

    let asset = db.query_first(
        "SELECT * from assets WHERE policy={}, name={};",
        params.policy_id,
        params.asset_name,
    );

    if asset.is_none() {
        bail!("we don't have that asset");
    }

    let market = party_from_address(MARKETPLACE);
    let buyer = party_from_address(params.buyer);

    let tx = TxBuilder::new()
        .transfer_asset(
            market, // from party
            buyer,  // to party
            params.policy_id,
            params.asset_name,
            TransferQuantity::All,
        )
        .output_ada(
            market,      // to party
            asset.price, // lovelace amount
        )
        .build();

    Ok(tx)
}
```


