//! Objects with a leading name and an internally tagged rest: a node (`id` + `op`) and an
//! output (`name` + `kind`). They serialize head first through `#[serde(flatten)]` (the
//! canonical field order of PR §2), and deserialize by splitting the head off and reading
//! the rest strictly: `flatten` itself cannot deny unknown fields.

use serde::de::{DeserializeOwned, Error};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

/// Reads an object, takes its string field `head` and reads the remaining fields as `T`.
pub(super) fn split<'de, D, T>(d: D, head: &'static str) -> Result<(String, T), D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let mut map = Map::<String, Value>::deserialize(d)?;
    let name = match map.remove(head) {
        Some(Value::String(s)) => s,
        Some(_) => return Err(D::Error::custom(format!("`{head}` must be a string"))),
        None => return Err(D::Error::missing_field(head)),
    };
    let rest = T::deserialize(Value::Object(map)).map_err(D::Error::custom)?;
    Ok((name, rest))
}
