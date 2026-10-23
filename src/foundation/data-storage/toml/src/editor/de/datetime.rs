use data_serialization::de::value::BorrowedStrDeserializer;
use data_serialization::de::IntoDeserializer;

use crate::editor::de::Error;

pub(crate) struct DatetimeDeserializer {
    date: Option<crate::editor::Datetime>,
}

impl DatetimeDeserializer {
    pub(crate) fn new(date: crate::editor::Datetime) -> Self {
        Self { date: Some(date) }
    }
}

impl<'de> data_serialization::de::MapAccess<'de> for DatetimeDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: data_serialization::de::DeserializeSeed<'de>,
    {
        if self.date.is_some() {
            seed.deserialize(BorrowedStrDeserializer::new(
                crate::datetime::__unstable::FIELD,
            ))
            .map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: data_serialization::de::DeserializeSeed<'de>,
    {
        if let Some(date) = self.date.take() {
            seed.deserialize(date.to_string().into_deserializer())
        } else {
            panic!("next_value_seed called before next_key_seed")
        }
    }
}
