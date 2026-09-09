//! Storage compatibility with the existing TypeScript Durable Objects.
//! Binary values remain ArrayBuffers and JSON metadata remains plain objects.
use futures::future::LocalBoxFuture;
use js_sys::{Array, ArrayBuffer, Map, Object, Reflect, Uint8Array};
use serde::{de::DeserializeOwned, Serialize};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue};
use worker::{Result, Storage, Transaction};

#[derive(Serialize)]
struct Raw(#[serde(with = "serde_wasm_bindgen::preserve")] JsValue);
pub fn to_js<T: Serialize>(value: &T) -> Result<JsValue> {
    Ok(value.serialize(
        &serde_wasm_bindgen::Serializer::new()
            .serialize_maps_as_objects(true),
    )?)
}
pub fn binary(bytes: &[u8]) -> JsValue {
    Uint8Array::from(bytes).buffer().into()
}
pub fn bytes(value: &JsValue) -> Result<Vec<u8>> {
    if !value.is_instance_of::<ArrayBuffer>() {
        return Err("Invalid binary storage value".into());
    }
    Ok(Uint8Array::new(value).to_vec())
}
pub fn entries(map: Map) -> Result<Vec<(String, JsValue)>> {
    let mut entries = Vec::new();
    for entry in map.entries() {
        let pair = Array::from(&entry?);
        let key = pair.get(0).as_string().ok_or("Invalid storage key")?;
        entries.push((key, pair.get(1)));
    }
    Ok(entries)
}
pub async fn get_raw(
    storage: &Storage,
    key: &str,
) -> Result<Option<JsValue>> {
    let map = storage.get_multiple(vec![key]).await?;
    let value = map.get(&JsValue::from_str(key));
    Ok((!value.is_undefined()).then_some(value))
}
pub async fn tx_raw(
    tx: &Transaction,
    key: &str,
) -> Result<Option<JsValue>> {
    let map = tx.get_multiple(vec![key]).await?;
    let value = map.get(&JsValue::from_str(key));
    Ok((!value.is_undefined()).then_some(value))
}
pub async fn tx_get<T: DeserializeOwned>(
    tx: &Transaction,
    key: &str,
) -> Result<Option<T>> {
    tx_raw(tx, key)
        .await?
        .map(serde_wasm_bindgen::from_value)
        .transpose()
        .map_err(Into::into)
}
pub async fn tx_put<T: Serialize>(
    tx: &Transaction,
    key: &str,
    value: &T,
) -> Result<()> {
    tx.put(key, Raw(to_js(value)?)).await
}
pub async fn tx_batch(
    tx: &Transaction,
    rows: Vec<(String, JsValue)>,
) -> Result<()> {
    for batch in rows.chunks(96) {
        let object = Object::new();
        for (key, value) in batch {
            Reflect::set(&object, &JsValue::from_str(key), value)?;
        }
        tx.put_multiple(Raw(object.into())).await?;
    }
    Ok(())
}
pub async fn tx_delete(tx: &Transaction, keys: Vec<String>) -> Result<()> {
    for keys in keys.chunks(96) {
        tx.delete_multiple(keys.to_vec()).await?;
    }
    Ok(())
}
pub async fn transaction<T: 'static>(
    storage: &Storage,
    action: impl FnOnce(Transaction) -> LocalBoxFuture<'static, Result<T>>
        + 'static,
) -> Result<T> {
    let result = Rc::new(RefCell::new(None));
    let returned = result.clone();
    storage
        .transaction(move |tx| async move {
            *returned.borrow_mut() = Some(action(tx).await?);
            Ok(())
        })
        .await?;
    let value = result
        .borrow_mut()
        .take()
        .ok_or("Transaction returned no value")?;
    Ok(value)
}
