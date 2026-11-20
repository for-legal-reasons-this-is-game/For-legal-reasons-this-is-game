//! Stores a protobuf message inside a serde struct as its protobuf bytes.

use prost::Message;
use serde::{Deserialize, Deserializer, Serializer, de::Error};

pub fn serialize<T: Message, S: Serializer>(message: &T, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_bytes(&message.encode_to_vec())
}

pub fn deserialize<'de, T: Message + Default, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<T, D::Error> {
    let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
    T::decode(bytes.as_slice()).map_err(D::Error::custom)
}
