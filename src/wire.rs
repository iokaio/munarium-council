// SPDX-License-Identifier: Apache-2.0
//! Independent candidate shape, canonical-byte and digest validation.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::LazyLock;

/// Non-disclosing failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Invalid or unsupported input.
    Invalid,
    /// Current authority does not permit the operation.
    Refused,
    /// Immutable identity or expected prior state conflicts.
    Conflict,
    /// A required dependency or durable store is unavailable.
    Unavailable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self {
        Self::Unavailable
    }
}
/// A bounded, non-disclosing result.
pub type Result<T> = std::result::Result<T, Error>;

fn bounded(value: &Value, depth: usize) -> Result<()> {
    match value {
        Value::Object(map) => {
            if depth >= 16 || map.keys().any(|k| !k.is_ascii()) {
                return Err(Error::Invalid);
            }
            for v in map.values() {
                bounded(v, depth + 1)?;
            }
        }
        Value::Array(items) => {
            if depth >= 16 {
                return Err(Error::Invalid);
            }
            for v in items {
                bounded(v, depth + 1)?;
            }
        }
        Value::Number(n)
            if n.as_i64()
                .is_none_or(|n| !(-9007199254740991..=9007199254740991).contains(&n)) =>
        {
            return Err(Error::Invalid);
        }
        _ => {}
    }
    Ok(())
}
/// Encode the candidate's restricted canonical JSON.
pub fn canonical(value: &Value) -> Result<String> {
    bounded(value, 0)?;
    let raw = serde_json::to_string(value).map_err(|_| Error::Invalid)?;
    if raw.len() > 65536 {
        return Err(Error::Invalid);
    }
    Ok(raw)
}
/// Parse exact canonical bytes; duplicate members and alternate encodings refuse.
pub fn parse(raw: &str) -> Result<Value> {
    if raw.len() > 65536 {
        return Err(Error::Invalid);
    }
    let value = serde_json::from_str(raw).map_err(|_| Error::Invalid)?;
    if canonical(&value)? != raw {
        return Err(Error::Invalid);
    }
    Ok(value)
}
/// Calculate the candidate's domain-separated digest.
pub fn digest(domain: &str, value: &Value) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(format!("munarium:stage2:{domain}:v1\0{}", canonical(value)?).as_bytes())
    ))
}
static SCHEMA: LazyLock<Option<jsonschema::Validator>> = LazyLock::new(|| {
    let value: Value =
        serde_json::from_str(include_str!("../contracts/stage2-v1/schema.json")).ok()?;
    jsonschema::validator_for(&value).ok()
});
/// Validate a closed record's shape; this never establishes current authority.
pub fn shape(value: &Value, kind: &str) -> Result<()> {
    canonical(value)?;
    if value["type"] != kind || !SCHEMA.as_ref().ok_or(Error::Unavailable)?.is_valid(value) {
        return Err(Error::Invalid);
    }
    Ok(())
}
/// Require every qualified reference to belong to the independently admitted scope.
pub fn scope(value: &Value, expected: &Value) -> Result<()> {
    match value {
        Value::Object(map) => {
            if map.get("scope").is_some_and(|s| s != expected) {
                return Err(Error::Refused);
            }
            for child in map.values() {
                scope(child, expected)?;
            }
        }
        Value::Array(items) => {
            for child in items {
                scope(child, expected)?;
            }
        }
        _ => {}
    }
    Ok(())
}
/// Extract a bounded unsigned timestamp or revision.
pub fn number(value: &Value) -> Result<u64> {
    value
        .as_u64()
        .filter(|n| *n <= 9007199254740991)
        .ok_or(Error::Invalid)
}
