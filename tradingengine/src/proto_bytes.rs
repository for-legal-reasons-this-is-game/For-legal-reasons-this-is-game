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

/// The same for a list of messages: each one is stored as its protobuf bytes.
pub mod list {
    use prost::Message;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

    struct AsBytes<'a, T>(&'a T);

    impl<T: Message> Serialize for AsBytes<'_, T> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            super::serialize(self.0, serializer)
        }
    }

    pub fn serialize<T: Message, S: Serializer>(
        messages: &[T],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(messages.iter().map(AsBytes))
    }

    pub fn deserialize<'de, T: Message + Default, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<T>, D::Error> {
        let list: Vec<Vec<u8>> = Deserialize::deserialize(deserializer)?;
        list.iter()
            .map(|bytes| T::decode(bytes.as_slice()).map_err(D::Error::custom))
            .collect()
    }
}
