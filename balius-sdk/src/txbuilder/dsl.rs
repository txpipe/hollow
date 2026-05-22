use pallas_primitives::{conway, MaybeIndefArray};
use pallas_traverse::MultiEraOutput;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::{HashMap, HashSet};

use super::*;

pub type Hash<const N: usize> = pallas_crypto::hash::Hash<N>;
pub type Address = pallas_addresses::Address;
pub type Value = pallas_primitives::conway::Value;
pub type Bytes = pallas_codec::utils::Bytes;
pub type KeyValuePairs<K, V> = pallas_codec::utils::KeyValuePairs<K, V>;
pub type NonEmptyKeyValuePairs<K, V> = pallas_codec::utils::NonEmptyKeyValuePairs<K, V>;
pub type NonEmptySet<T> = pallas_codec::utils::NonEmptySet<T>;
pub type KeepRaw<'b, T> = pallas_codec::utils::KeepRaw<'b, T>;

pub type Cbor = Vec<u8>;

#[derive(Debug, Clone, Default)]
pub struct UtxoSet(HashMap<TxoRef, Cbor>);

impl UtxoSet {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&TxoRef, MultiEraOutput<'_>)> {
        self.0.iter().map(|(k, v)| {
            (
                k,
                MultiEraOutput::decode(pallas_traverse::Era::Conway, v).unwrap(),
            )
        })
    }

    pub fn refs(&self) -> impl Iterator<Item = &TxoRef> {
        self.0.keys()
    }

    pub fn txos(&self) -> impl Iterator<Item = MultiEraOutput<'_>> {
        self.0
            .values()
            .map(|v| MultiEraOutput::decode(pallas_traverse::Era::Conway, v).unwrap())
    }
}

impl FromIterator<(TxoRef, Cbor)> for UtxoSet {
    fn from_iter<T: IntoIterator<Item = (TxoRef, Cbor)>>(iter: T) -> Self {
        Self(HashMap::from_iter(iter))
    }
}

impl From<HashMap<TxoRef, Cbor>> for UtxoSet {
    fn from(value: HashMap<TxoRef, Cbor>) -> Self {
        UtxoSet(value)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct UtxoPattern {
    pub address: Option<AddressPattern>,
    pub asset: Option<AssetPattern>,
}

impl From<UtxoPattern> for crate::wit::balius::app::ledger::UtxoPattern {
    fn from(value: UtxoPattern) -> Self {
        Self {
            address: value.address.map(Into::into),
            asset: value.asset.map(Into::into),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AddressPattern {
    pub exact_address: Vec<u8>,
}

impl From<AddressPattern> for crate::wit::balius::app::ledger::AddressPattern {
    fn from(value: AddressPattern) -> Self {
        Self {
            exact_address: value.exact_address,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AssetPattern {
    pub policy: Vec<u8>,
    pub name: Option<Vec<u8>>,
}

impl From<AssetPattern> for crate::wit::balius::app::ledger::AssetPattern {
    fn from(value: AssetPattern) -> Self {
        Self {
            policy: value.policy,
            name: value.name,
        }
    }
}

pub trait InputExpr: 'static + Send + Sync {
    fn eval(&self, ctx: &BuildContext) -> Result<Vec<conway::TransactionInput>, BuildError>;
}

#[derive(Clone, Serialize, Deserialize)]
pub enum UtxoSource {
    Refs(Vec<TxoRef>),
    Search(UtxoPattern),
}

impl UtxoSource {
    pub fn resolve(&self, ctx: &BuildContext) -> Result<UtxoSet, BuildError> {
        match self {
            Self::Refs(refs) => ctx.ledger.read_utxos(refs),
            Self::Search(utxo_pattern) => ctx.ledger.search_utxos(utxo_pattern),
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ReferenceScript {
    pub ref_txo: conway::TransactionInput,
    pub hash: Hash<28>,
    #[serde_as(as = "DisplayFromStr")]
    pub address: Address,
}

impl InputExpr for ReferenceScript {
    fn eval(&self, _: &dsl::BuildContext) -> Result<Vec<conway::TransactionInput>, BuildError> {
        Ok(vec![self.ref_txo.clone()])
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct AssetPolicyId(Hash<28>);

impl AssetPolicyId {
    pub fn new(hash: Hash<28>) -> Self {
        Self(hash)
    }
}

impl From<Hash<28>> for AssetPolicyId {
    fn from(value: Hash<28>) -> Self {
        Self(value)
    }
}

impl From<AssetPolicyId> for Hash<28> {
    fn from(value: AssetPolicyId) -> Self {
        value.0
    }
}

impl TryFrom<&str> for AssetPolicyId {
    type Error = BuildError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let hash = <Hash<28> as std::str::FromStr>::from_str(value)
            .map_err(|_| BuildError::MalformedAssetPolicyIdHex)?;
        Ok(AssetPolicyId(hash))
    }
}

impl std::ops::Deref for AssetPolicyId {
    type Target = Hash<28>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for AssetPolicyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct AssetName(Bytes);

impl AssetName {
    pub fn new(name: Bytes) -> Result<Self, BuildError> {
        if name.len() > 32 {
            panic!("Asset name too long");
        }

        Ok(Self(name))
    }
}

impl TryFrom<Vec<u8>> for AssetName {
    type Error = BuildError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::new(value.into())
    }
}

impl TryFrom<&str> for AssetName {
    type Error = BuildError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value.as_bytes().to_vec().into())
    }
}

impl From<AssetName> for Bytes {
    fn from(value: AssetName) -> Self {
        value.0
    }
}

impl std::ops::Deref for AssetName {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Hash, Clone)]
pub struct TxoRef {
    pub hash: Hash<32>,
    pub index: u64,
}

impl std::str::FromStr for TxoRef {
    type Err = BuildError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (hash, index) = s.split_once("#").ok_or(BuildError::MalformedTxoRef)?;
        let hash = Hash::from_str(hash).map_err(|_| BuildError::MalformedTxoRef)?;
        let index = index.parse().map_err(|_| BuildError::MalformedTxoRef)?;
        Ok(TxoRef::new(hash, index))
    }
}

impl From<crate::wit::balius::app::ledger::TxoRef> for TxoRef {
    fn from(value: crate::wit::balius::app::ledger::TxoRef) -> Self {
        Self::new(Hash::from(value.tx_hash.as_slice()), value.tx_index as u64)
    }
}

impl From<TxoRef> for crate::wit::balius::app::ledger::TxoRef {
    fn from(value: TxoRef) -> crate::wit::balius::app::ledger::TxoRef {
        crate::wit::balius::app::ledger::TxoRef {
            tx_hash: value.hash.to_vec(),
            tx_index: value.index as u32,
        }
    }
}

impl TxoRef {
    pub fn new(hash: Hash<32>, index: u64) -> Self {
        Self { hash, index }
    }
}

impl dsl::InputExpr for TxoRef {
    fn eval(&self, _: &BuildContext) -> Result<Vec<conway::TransactionInput>, BuildError> {
        Ok(vec![self.into()])
    }
}

impl From<&TxoRef> for conway::TransactionInput {
    fn from(value: &TxoRef) -> Self {
        conway::TransactionInput {
            transaction_id: value.hash,
            index: value.index,
        }
    }
}

impl InputExpr for UtxoSource {
    fn eval(&self, ctx: &BuildContext) -> Result<Vec<conway::TransactionInput>, BuildError> {
        let out = self.resolve(ctx)?.refs().map(|i| i.into()).collect();

        Ok(out)
    }
}

pub trait ValueExpr: 'static + Send + Sync {
    fn eval(&self, ctx: &BuildContext) -> Result<conway::Value, BuildError>;

    fn eval_as_mint(&self, ctx: &BuildContext) -> Result<conway::Mint, BuildError> {
        let value = self.eval(ctx)?;

        match value {
            conway::Value::Multiasset(_, assets) => asset_math::multiasset_coin_to_mint(assets),
            conway::Value::Coin(_) => Err(BuildError::Conflicting),
        }
    }

    fn eval_as_burn(&self, ctx: &BuildContext) -> Result<conway::Mint, BuildError> {
        let value = self.eval(ctx)?;

        match value {
            conway::Value::Multiasset(_, assets) => asset_math::multiasset_coin_to_burn(assets),
            conway::Value::Coin(_) => Err(BuildError::Conflicting),
        }
    }
}

impl ValueExpr for u64 {
    fn eval(&self, _ctx: &BuildContext) -> Result<conway::Value, BuildError> {
        Ok(conway::Value::Coin(*self))
    }
}

impl<F> ValueExpr for F
where
    F: Fn(&BuildContext) -> Result<conway::Value, BuildError> + 'static + Send + Sync,
{
    fn eval(&self, ctx: &BuildContext) -> Result<conway::Value, BuildError> {
        self(ctx)
    }
}

impl<T: ValueExpr> ValueExpr for Option<T> {
    fn eval(&self, ctx: &BuildContext) -> Result<conway::Value, BuildError> {
        match self {
            Some(v) => v.eval(ctx),
            None => Err(BuildError::Incomplete),
        }
    }
}

// calculate min utxo lovelace according to spec
// https://cips.cardano.org/cip/CIP-55
pub struct MinUtxoLovelace;

impl ValueExpr for MinUtxoLovelace {
    fn eval(&self, ctx: &BuildContext) -> Result<conway::Value, BuildError> {
        let parent = match &ctx.parent_output {
            Some(x) => x,
            None => return Ok(conway::Value::Coin(0)),
        };

        let serialized = pallas_codec::minicbor::to_vec(parent).unwrap();
        let coins_per_utxo_byte = ctx.pparams.coins_per_utxo_byte;
        let min_lovelace = (160u64 + serialized.len() as u64) * coins_per_utxo_byte;
        let current_value = match parent {
            conway::TransactionOutput::PostAlonzo(x) => &x.value,
            _ => unimplemented!(),
        };

        let current_lovelace = asset_math::value_coin(current_value);

        if current_lovelace >= min_lovelace {
            return Ok(current_value.clone());
        }

        let optimized = asset_math::value_saturating_add_coin(
            current_value.clone(),
            (min_lovelace - current_lovelace) as i64,
        );

        Ok(optimized)
    }
}

impl ValueExpr for Box<dyn ValueExpr> {
    fn eval(&self, ctx: &BuildContext) -> Result<conway::Value, BuildError> {
        (**self).eval(ctx)
    }
}

impl<T: ValueExpr> ValueExpr for Vec<T> {
    fn eval(&self, ctx: &BuildContext) -> Result<conway::Value, BuildError> {
        let values = self
            .iter()
            .map(|v| v.eval(ctx))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(asset_math::aggregate_values(values))
    }
}

pub trait AddressExpr: 'static + Send + Sync {
    fn eval(&self, ctx: &BuildContext) -> Result<Address, BuildError>;
}

impl AddressExpr for &'static str {
    fn eval(&self, _ctx: &BuildContext) -> Result<Address, BuildError> {
        Address::from_bech32(self).map_err(|_| BuildError::MalformedAddress)
    }
}

impl AddressExpr for String {
    fn eval(&self, _ctx: &BuildContext) -> Result<Address, BuildError> {
        Address::from_bech32(self).map_err(|_| BuildError::MalformedAddress)
    }
}

impl AddressExpr for Address {
    fn eval(&self, _ctx: &BuildContext) -> Result<Address, BuildError> {
        Ok(self.clone())
    }
}

impl AddressExpr for Box<dyn AddressExpr> {
    fn eval(&self, ctx: &BuildContext) -> Result<Address, BuildError> {
        (**self).eval(ctx)
    }
}

impl<T: AddressExpr> AddressExpr for Option<T> {
    fn eval(&self, ctx: &BuildContext) -> Result<Address, BuildError> {
        match self {
            Some(v) => v.eval(ctx),
            None => Err(BuildError::Incomplete),
        }
    }
}

impl<F> AddressExpr for F
where
    F: Fn(&BuildContext) -> Result<Address, BuildError> + 'static + Send + Sync,
{
    fn eval(&self, ctx: &BuildContext) -> Result<Address, BuildError> {
        self(ctx)
    }
}

pub trait OutputExpr: 'static + Send + Sync {
    fn eval(
        &mut self,
        ctx: &BuildContext,
    ) -> Result<conway::TransactionOutput<'static>, BuildError>;
}

pub struct ChangeAddress(pub UtxoSource);

impl AddressExpr for ChangeAddress {
    fn eval(&self, ctx: &BuildContext) -> Result<Address, BuildError> {
        let utxo_set = &self.0.resolve(ctx)?;

        if utxo_set.is_empty() {
            return Err(BuildError::EmptyUtxoSet);
        }

        let addresses: HashSet<_> = utxo_set
            .txos()
            .map(|x| x.address())
            .collect::<Result<HashSet<_>, _>>()
            .map_err(|_| BuildError::UtxoDecode)?;

        if addresses.len() > 1 {
            return Err(BuildError::Conflicting);
        }

        Ok(addresses.into_iter().next().unwrap())
    }
}

pub struct TotalChange;

impl ValueExpr for TotalChange {
    fn eval(&self, ctx: &BuildContext) -> Result<conway::Value, BuildError> {
        let change = asset_math::subtract_value(&ctx.total_input, &ctx.spent_output)?;
        let fee = ctx.estimated_fee;
        let diff = asset_math::value_saturating_add_coin(change, -(fee as i64));
        Ok(diff)
    }
}

pub struct FeeChangeReturn(pub UtxoSource);

impl OutputExpr for FeeChangeReturn {
    fn eval(
        &mut self,
        ctx: &BuildContext,
    ) -> Result<conway::TransactionOutput<'static>, BuildError> {
        OutputBuilder::new()
            .address(ChangeAddress(self.0.clone()))
            .with_value(TotalChange)
            .eval(ctx)
    }
}

pub trait PlutusDataExpr: 'static + Send + Sync {
    fn eval(&self, ctx: &BuildContext) -> Result<conway::PlutusData, BuildError>;
}

impl PlutusDataExpr for conway::PlutusData {
    fn eval(&self, _ctx: &BuildContext) -> Result<conway::PlutusData, BuildError> {
        Ok(self.clone())
    }
}

impl<F> PlutusDataExpr for F
where
    F: Fn(&BuildContext) -> Result<conway::PlutusData, BuildError> + 'static + Send + Sync,
{
    fn eval(&self, ctx: &BuildContext) -> Result<conway::PlutusData, BuildError> {
        self(ctx)
    }
}

impl PlutusDataExpr for Box<dyn PlutusDataExpr> {
    fn eval(&self, ctx: &BuildContext) -> Result<conway::PlutusData, BuildError> {
        (**self).eval(ctx)
    }
}

impl PlutusDataExpr for () {
    fn eval(&self, _ctx: &BuildContext) -> Result<conway::PlutusData, BuildError> {
        Ok(conway::PlutusData::Constr(conway::Constr {
            tag: 121,
            any_constructor: None,
            fields: MaybeIndefArray::Def(vec![]),
        }))
    }
}

pub trait MintExpr: 'static + Send + Sync {
    fn eval(&self, ctx: &BuildContext) -> Result<Option<conway::Mint>, BuildError>;
    fn eval_redeemer(&self, ctx: &BuildContext) -> Result<Option<conway::Redeemer>, BuildError>;
}

#[derive(Default)]
pub struct MintBuilder {
    pub assets: Vec<Box<dyn ValueExpr>>,
    pub burn: Vec<Box<dyn ValueExpr>>,
    pub redeemer: Option<Box<dyn PlutusDataExpr>>,
}

impl MintBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_asset(mut self, asset: impl ValueExpr) -> Self {
        self.assets.push(Box::new(asset));
        self
    }

    pub fn with_burn(mut self, burn: impl ValueExpr) -> Self {
        self.burn.push(Box::new(burn));
        self
    }

    pub fn using_redeemer(mut self, redeemer: impl PlutusDataExpr) -> Self {
        self.redeemer = Some(Box::new(redeemer));
        self
    }
}

impl MintExpr for MintBuilder {
    fn eval(&self, ctx: &BuildContext) -> Result<Option<primitives::Mint>, BuildError> {
        let out = HashMap::new();

        let out = self.assets.iter().try_fold(out, |mut acc, v| {
            let v = v.eval_as_mint(ctx)?;
            asset_math::fold_multiassets(&mut acc, v);
            Result::<_, BuildError>::Ok(acc)
        })?;

        let out = self.burn.iter().try_fold(out, |mut acc, v| {
            let v = v.eval_as_burn(ctx)?;
            asset_math::fold_multiassets(&mut acc, v);
            Result::<_, BuildError>::Ok(acc)
        })?;

        let mint = out
            .into_iter()
            .filter_map(|(policy, assets)| {
                if assets.is_empty() {
                    None
                } else {
                    Some((policy, assets.into_iter().collect()))
                }
            })
            .collect();

        Ok(Some(mint))
    }

    fn eval_redeemer(&self, ctx: &BuildContext) -> Result<Option<conway::Redeemer>, BuildError> {
        let Some(mint) = self.eval(ctx)? else {
            return Ok(None);
        };

        if mint.is_empty() {
            return Err(BuildError::Incomplete);
        }

        if mint.len() > 1 {
            return Err(BuildError::Conflicting);
        }

        let (policy, _) = mint.iter().next().unwrap();

        let data = self
            .redeemer
            .as_ref()
            .ok_or(BuildError::Incomplete)?
            .eval(ctx)?;

        let out = conway::Redeemer {
            tag: conway::RedeemerTag::Mint,
            index: ctx.mint_redeemer_index(*policy)?,
            ex_units: ctx.eval_ex_units(*policy, &data),
            data,
        };

        Ok(Some(out))
    }
}

pub trait ScriptExpr: 'static + Send + Sync {
    fn eval(&self, ctx: &BuildContext) -> Result<conway::ScriptRef<'static>, BuildError>;
}

impl ScriptExpr for conway::ScriptRef<'static> {
    fn eval(&self, _ctx: &BuildContext) -> Result<conway::ScriptRef<'static>, BuildError> {
        Ok(self.clone())
    }
}

impl ScriptExpr for conway::PlutusScript<3> {
    fn eval(&self, _ctx: &BuildContext) -> Result<conway::ScriptRef<'static>, BuildError> {
        Ok(conway::ScriptRef::PlutusV3Script(self.clone()))
    }
}

#[derive(Default)]
pub struct OutputBuilder {
    pub previous: Option<conway::TransactionOutput<'static>>,
    pub address: Option<Box<dyn dsl::AddressExpr>>,
    pub values: Vec<Box<dyn dsl::ValueExpr>>,
    pub script: Option<Box<dyn ScriptExpr>>,
    // TODO: inline / hash datum
}

impl OutputBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn address(mut self, address: impl AddressExpr + 'static) -> Self {
        self.address = Some(Box::new(address));
        self
    }

    pub fn with_value(mut self, value: impl ValueExpr + 'static) -> Self {
        self.values.push(Box::new(value));
        self
    }

    pub fn with_script(mut self, script: impl ScriptExpr + 'static) -> Self {
        self.script = Some(Box::new(script));
        self
    }
}

impl OutputExpr for OutputBuilder {
    fn eval(
        &mut self,
        ctx: &BuildContext,
    ) -> Result<conway::TransactionOutput<'static>, BuildError> {
        let ctx = match &self.previous {
            Some(x) => &ctx.with_parent_output(x.clone()),
            None => ctx,
        };

        let value = self.values.eval(ctx)?;

        let address = self.address.eval(ctx)?.to_vec().into();

        let script_ref = self
            .script
            .as_ref()
            .map(|s| s.eval(ctx))
            .transpose()?
            .map(pallas_codec::utils::CborWrap);

        let output = conway::TransactionOutput::PostAlonzo(
            conway::PostAlonzoTransactionOutput {
                value,
                address,
                script_ref,
                datum_option: None, // TODO
            }
            .into(),
        );

        self.previous = Some(output.clone());

        Ok(output)
    }
}

pub trait TxExpr: 'static + Send + Sync {
    fn eval_body(
        &mut self,
        ctx: &BuildContext,
    ) -> Result<conway::TransactionBody<'static>, BuildError>;
    fn eval_witness_set(
        &mut self,
        ctx: &BuildContext,
    ) -> Result<conway::WitnessSet<'static>, BuildError>;
}

impl<T: TxExpr> TxExpr for &'static mut T {
    fn eval_body(
        &mut self,
        ctx: &BuildContext,
    ) -> Result<conway::TransactionBody<'static>, BuildError> {
        (**self).eval_body(ctx)
    }

    fn eval_witness_set(
        &mut self,
        ctx: &BuildContext,
    ) -> Result<conway::WitnessSet<'static>, BuildError> {
        (**self).eval_witness_set(ctx)
    }
}

impl TxExpr for Box<dyn TxExpr> {
    fn eval_body(
        &mut self,
        ctx: &BuildContext,
    ) -> Result<conway::TransactionBody<'static>, BuildError> {
        (**self).eval_body(ctx)
    }

    fn eval_witness_set(
        &mut self,
        ctx: &BuildContext,
    ) -> Result<conway::WitnessSet<'static>, BuildError> {
        (**self).eval_witness_set(ctx)
    }
}

#[derive(Default)]
pub struct TxBuilder {
    pub reference_inputs: Vec<Box<dyn InputExpr>>,
    pub inputs: Vec<Box<dyn InputExpr>>,
    pub outputs: Vec<Box<dyn OutputExpr>>,
    pub mint: Vec<Box<dyn MintExpr>>,
    pub fee: Option<u64>,
    // pub valid_from_slot: Option<u64>,
    // pub invalid_from_slot: Option<u64>,
    // pub network_id: Option<u8>,
    // pub collateral_inputs: Option<Vec<TxoRef>>,
    // pub collateral_output: Option<Output>,
    // pub disclosed_signers: Option<Vec<PubKeyHash>>,
    // pub scripts: Option<HashMap<ScriptHash, Script>>,
    // pub datums: Option<HashMap<DatumHash, DatumBytes>>,
    // pub redeemers: Option<Redeemers>,
    // pub script_data_hash: Option<ScriptDataHash>,
    // pub signature_amount_override: Option<u8>,
    // #[serde_as(as = "Option<DisplayFromStr>")]
    // pub change_address: Option<Address>,
    // pub certificates: TODO
    // pub withdrawals: TODO
    // pub updates: TODO
    // pub auxiliary_data: TODO
    // pub phase_2_valid: TODO
}

impl TxBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_reference_input(mut self, input: impl InputExpr) -> Self {
        self.reference_inputs.push(Box::new(input));
        self
    }

    pub fn with_input(mut self, input: impl InputExpr) -> Self {
        self.inputs.push(Box::new(input));
        self
    }

    pub fn with_output(mut self, output: impl OutputExpr) -> Self {
        self.outputs.push(Box::new(output));
        self
    }

    pub fn with_mint(mut self, mint: impl MintExpr) -> Self {
        self.mint.push(Box::new(mint));
        self
    }

    pub fn with_fee(mut self, fee: u64) -> Self {
        self.fee = Some(fee);
        self
    }
}

impl TxExpr for TxBuilder {
    fn eval_body(
        &mut self,
        ctx: &BuildContext,
    ) -> Result<conway::TransactionBody<'static>, BuildError> {
        let out = conway::TransactionBody {
            inputs: self
                .inputs
                .iter()
                .map(|i| i.eval(ctx))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .into(),
            outputs: self
                .outputs
                .iter_mut()
                .map(|o| o.eval(ctx))
                .collect::<Result<Vec<_>, _>>()?,
            fee: ctx.estimated_fee,
            ttl: None,
            validity_interval_start: None,
            certificates: None,
            withdrawals: None,
            auxiliary_data_hash: None,
            mint: {
                let mints = self
                    .mint
                    .iter()
                    .map(|m| m.eval(ctx))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten();

                asset_math::aggregate_assets(mints)
            },
            script_data_hash: None,
            collateral: None,
            required_signers: None,
            network_id: None,
            collateral_return: None,
            total_collateral: None,
            reference_inputs: {
                let refs: Vec<_> = self
                    .reference_inputs
                    .iter()
                    .map(|i| i.eval(ctx))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect();

                if refs.is_empty() {
                    None
                } else {
                    NonEmptySet::from_vec(refs)
                }
            },
            voting_procedures: None,
            proposal_procedures: None,
            treasury_value: None,
            donation: None,
        };

        Ok(out)
    }

    fn eval_witness_set(
        &mut self,
        ctx: &BuildContext,
    ) -> Result<conway::WitnessSet<'static>, BuildError> {
        let out = conway::WitnessSet {
            redeemer: {
                let redeemers: Vec<_> = self
                    .mint
                    .iter()
                    .map(|m| m.eval_redeemer(ctx))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect();

                if redeemers.is_empty() {
                    None
                } else {
                    Some(conway::Redeemers::List(redeemers).into())
                }
            },
            vkeywitness: None,
            native_script: None,
            bootstrap_witness: None,
            plutus_v1_script: None,
            plutus_data: None,
            plutus_v2_script: None,
            plutus_v3_script: None,
        };

        Ok(out)
    }
}

#[macro_export]
macro_rules! define_asset_class {
    ($struct_name:ident, $policy:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub struct $struct_name($crate::txbuilder::Bytes, u64);

        impl $struct_name {
            pub fn value(name: $crate::txbuilder::AssetName, quantity: u64) -> Self {
                Self(name.into(), quantity)
            }
        }

        impl $crate::txbuilder::ValueExpr for $struct_name {
            fn eval(
                &self,
                _: &$crate::txbuilder::BuildContext,
            ) -> std::result::Result<$crate::txbuilder::Value, $crate::txbuilder::BuildError> {
                let policy = $crate::txbuilder::Hash::from(*$policy);
                let name = $crate::txbuilder::Bytes::from(self.0.clone());
                let Ok(amount) = self.1.try_into() else {
                    return Ok($crate::txbuilder::Value::Coin(0));
                };
                let asset = [(name, amount)].into_iter().collect();
                let val = $crate::txbuilder::Value::Multiasset(
                    0,
                    [(policy, asset)].into_iter().collect(),
                );

                Ok(val)
            }
        }
    };
}

define_asset_class!(MyAssetClass, b"abcabcababcabcababcabcababca");
