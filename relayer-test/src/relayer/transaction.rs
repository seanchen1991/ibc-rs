use eyre::eyre;
use ibc::core::ics24_host::identifier::ChainId;
use ibc_proto::cosmos::tx::v1beta1::{Fee, TxRaw};
use ibc_relayer::chain::cosmos::{
    auth_info_and_bytes, broadcast_tx_sync, encode_key_bytes, encode_sign_doc, encode_signer_info,
    tx_body_and_bytes,
};
use ibc_relayer::config::types::Memo;
use ibc_relayer::config::AddressType;
use ibc_relayer::keyring::KeyEntry;
use prost_types::Any;
use tendermint_rpc::{HttpClient, Url};

use crate::error::Error;

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
) -> Result<(), Error> {
    let key_bytes = encode_key_bytes(key_entry)?;
    let signer = encode_signer_info(key_bytes, address_type, account_sequence)?;

    let (_, body_buf) = tx_body_and_bytes(messages, tx_memo)?;

    let (_, auth_buf) = auth_info_and_bytes(signer.clone(), fee.clone())?;

    let signed_doc = encode_sign_doc(
        chain_id,
        key_entry,
        address_type,
        body_buf.clone(),
        auth_buf.clone(),
        account_number,
    )?;

    let tx_raw = TxRaw {
        body_bytes: body_buf,
        auth_info_bytes: auth_buf,
        signatures: vec![signed_doc],
    };

    let mut tx_bytes = Vec::new();
    prost::Message::encode(&tx_raw, &mut tx_bytes).unwrap();

    let response = broadcast_tx_sync(rpc_client, rpc_address, tx_bytes).await?;

    match response.code {
        tendermint::abci::Code::Ok => Ok(()),
        tendermint::abci::Code::Err(code) => Err(eyre!(
            "broadcast tx returns error code {}: {:?}",
            code,
            response
        )),
    }
}
