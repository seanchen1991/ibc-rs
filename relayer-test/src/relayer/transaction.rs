use core::cmp::min;
use ibc::core::ics24_host::identifier::ChainId;
use ibc_proto::cosmos::tx::v1beta1::{AuthInfo, Fee, Tx, TxBody, TxRaw};
use ibc_relayer::chain::cosmos::{
    auth_info_and_bytes, broadcast_tx_sync, calculate_fee, encode_key_bytes, encode_sign_doc,
    encode_signer_info, mul_ceil, send_tx_simulate, tx_body_and_bytes,
    DEFAULT_GAS_PRICE_ADJUSTMENT, DEFAULT_MAX_GAS,
};
use ibc_relayer::config::types::Memo;
use ibc_relayer::config::AddressType;
use ibc_relayer::config::ChainConfig;
use ibc_relayer::error::Error;
use ibc_relayer::keyring::KeyEntry;
use prost_types::Any;
use tendermint_rpc::endpoint::broadcast::tx_sync::Response;
use tendermint_rpc::{HttpClient, Url};
use tonic::codegen::http::Uri;
use tracing::{debug, error};

pub async fn send_tx(
    chain_id: &ChainId,
    rpc_client: &HttpClient,
    rpc_address: &Url,
    messages: Vec<Any>,
    account_sequence: u64,
    key_entry: &KeyEntry,
    address_type: &AddressType,
    fee: Fee,
    tx_memo: &Memo,
    account_number: u64,
) -> Result<Response, Error> {
    let tx_bytes = sign_and_encode_tx(
        chain_id,
        messages,
        account_sequence,
        key_entry,
        address_type,
        fee,
        tx_memo,
        account_number,
    )?;

    let response = broadcast_tx_sync(rpc_client, rpc_address, tx_bytes).await?;

    Ok(response)
}

pub async fn simulate_tx_fees(
    config: &ChainConfig,
    chain_id: &ChainId,
    grpc_address: &Uri,
    messages: Vec<Any>,
    account_sequence: u64,
    key_entry: &KeyEntry,
    address_type: &AddressType,
    max_fee: Fee,
    tx_memo: &Memo,
    account_number: u64,
) -> Result<Fee, Error> {
    let signed_tx = encode_tx_to_raw(
        chain_id,
        messages,
        account_sequence,
        key_entry,
        address_type,
        max_fee,
        tx_memo,
        account_number,
    )?;

    let max_gas = max_gas_from_config(config);
    let default_gas = default_gas_from_config(config);

    let tx = Tx {
        body: Some(signed_tx.body),
        auth_info: Some(signed_tx.auth_info),
        signatures: signed_tx.signatures,
    };

    let estimated_gas =
        estimate_gas_with_raw_tx(chain_id, tx, grpc_address, default_gas, max_gas).await?;

    let adjusted_gas = adjust_gas_with_simulated_fees(config, estimated_gas);

    let adjusted_fee = gas_amount_to_fees(config, adjusted_gas);

    Ok(adjusted_fee)
}

pub fn sign_and_encode_tx(
    chain_id: &ChainId,
    messages: Vec<Any>,
    account_sequence: u64,
    key_entry: &KeyEntry,
    address_type: &AddressType,
    fee: Fee,
    tx_memo: &Memo,
    account_number: u64,
) -> Result<Vec<u8>, Error> {
    let signed_tx = encode_tx_to_raw(
        chain_id,
        messages,
        account_sequence,
        key_entry,
        address_type,
        fee,
        tx_memo,
        account_number,
    )?;

    let tx_raw = TxRaw {
        body_bytes: signed_tx.body_bytes,
        auth_info_bytes: signed_tx.auth_info_bytes,
        signatures: signed_tx.signatures,
    };

    encode_tx_raw(tx_raw)
}

pub fn encode_tx_raw(tx_raw: TxRaw) -> Result<Vec<u8>, Error> {
    let mut tx_bytes = Vec::new();
    prost::Message::encode(&tx_raw, &mut tx_bytes).map_err(Error::protobuf_encode)?;

    Ok(tx_bytes)
}

pub struct SignedTx {
    pub body: TxBody,
    pub body_bytes: Vec<u8>,
    pub auth_info: AuthInfo,
    pub auth_info_bytes: Vec<u8>,
    pub signatures: Vec<Vec<u8>>,
}

pub fn encode_tx_to_raw(
    chain_id: &ChainId,
    messages: Vec<Any>,
    account_sequence: u64,
    key_entry: &KeyEntry,
    address_type: &AddressType,
    fee: Fee,
    tx_memo: &Memo,
    account_number: u64,
) -> Result<SignedTx, Error> {
    let key_bytes = encode_key_bytes(key_entry)?;

    let signer = encode_signer_info(key_bytes, address_type, account_sequence)?;

    let (body, body_bytes) = tx_body_and_bytes(messages, tx_memo)?;

    let (auth_info, auth_info_bytes) = auth_info_and_bytes(signer.clone(), fee.clone())?;

    let signed_doc = encode_sign_doc(
        chain_id,
        key_entry,
        address_type,
        body_bytes.clone(),
        auth_info_bytes.clone(),
        account_number,
    )?;

    Ok(SignedTx {
        body,
        body_bytes,
        auth_info,
        auth_info_bytes,
        signatures: vec![signed_doc],
    })
}

pub async fn estimate_gas_with_raw_tx(
    chain_id: &ChainId,
    tx: Tx,
    grpc_address: &Uri,
    default_gas: u64,
    max_gas: u64,
) -> Result<u64, Error> {
    let response = send_tx_simulate(tx, grpc_address).await;

    let estimated_gas = match response {
        Ok(response) => {
            let m_gas_info = response.gas_info;

            debug!(
                "[{}] send_tx: tx simulation successful, simulated gas: {:?}",
                chain_id, m_gas_info,
            );

            match m_gas_info {
                Some(gas) => gas.gas_used,
                None => default_gas,
            }
        }
        Err(e) => {
            error!(
                "[{}] send_tx: failed to estimate gas, falling back on default gas, error: {}",
                chain_id,
                e.detail()
            );

            default_gas
        }
    };

    if estimated_gas > max_gas {
        debug!(
            estimated = ?estimated_gas,
            max = ?max_gas,
            "[{}] send_tx: estimated gas is higher than max gas",
            chain_id,
        );

        return Err(Error::tx_simulate_gas_estimate_exceeded(
            chain_id.clone(),
            estimated_gas,
            max_gas,
        ));
    } else {
        Ok(estimated_gas)
    }
}

pub fn adjust_gas_with_simulated_fees(config: &ChainConfig, gas_amount: u64) -> u64 {
    let gas_adjustment = gas_adjustment_from_config(config);

    let max_gas = max_gas_from_config(config);

    min(gas_amount + mul_ceil(gas_amount, gas_adjustment), max_gas)
}

pub fn default_gas_from_config(config: &ChainConfig) -> u64 {
    config
        .default_gas
        .unwrap_or_else(|| max_gas_from_config(config))
}

pub fn max_gas_from_config(config: &ChainConfig) -> u64 {
    config.max_gas.unwrap_or(DEFAULT_MAX_GAS)
}

pub fn gas_adjustment_from_config(config: &ChainConfig) -> f64 {
    config
        .gas_adjustment
        .unwrap_or(DEFAULT_GAS_PRICE_ADJUSTMENT)
}

pub fn gas_amount_to_fees(config: &ChainConfig, gas_amount: u64) -> Fee {
    let gas_limit = adjust_gas_with_simulated_fees(config, gas_amount);

    let amount = calculate_fee(gas_limit, &config.gas_price);

    Fee {
        amount: vec![amount],
        gas_limit,
        payer: "".to_string(),
        granter: "".to_string(),
    }
}

pub fn max_fee_from_config(config: &ChainConfig) -> Fee {
    let gas_limit = max_gas_from_config(config);
    let amount = calculate_fee(gas_limit, &config.gas_price);

    Fee {
        amount: vec![amount],
        gas_limit,
        payer: "".to_string(),
        granter: "".to_string(),
    }
}
