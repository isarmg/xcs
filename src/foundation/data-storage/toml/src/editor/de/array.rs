use crate::editor::de::Error;

pub(crate) struct ArrayDeserializer {
    input: Vec<crate::editor::Item>,
    span: Option<std::ops::Range<usize>>,
}

impl ArrayDeserializer {
    pub(crate) fn new(
        input: Vec<crate::editor::Item>,
        span: Option<std::ops::Range<usize>>,
    ) -> Self {
        Self { input, span }
    }
}

// Note: this is wrapped by `ValueDeserializer` and any trait methods
// implemented here need to be wrapped there
impl<'de> data_serialization::Deserializer<'de> for ArrayDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: data_serialization::de::Visitor<'de>,
    {
        visitor.visit_seq(ArraySeqAccess::new(self.input))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: data_serialization::de::Visitor<'de>,
    {
        if crate::span::__unstable::is_spanned(name, fields) {
            if let Some(span) = self.span.clone() {
                return visitor.visit_map(super::SpannedDeserializer::new(self, span));
            }
        }

        self.deserialize_any(visitor)
    }

    data_serialization::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

impl data_serialization::de::IntoDeserializer<'_, Error> for ArrayDeserializer {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl crate::editor::Array {
    pub(crate) fn into_deserializer(self) -> ArrayDeserializer {
        ArrayDeserializer::new(self.values, self.span)
    }
}

impl crate::editor::ArrayOfTables {
    pub(crate) fn into_deserializer(self) -> ArrayDeserializer {
        ArrayDeserializer::new(self.values, self.span)
    }
}

pub(crate) struct ArraySeqAccess {
    iter: std::vec::IntoIter<crate::editor::Item>,
}

impl ArraySeqAccess {
    pub(crate) fn new(input: Vec<crate::editor::Item>) -> Self {
        Self {
            iter: input.into_iter(),
        }
    }
}

impl<'de> data_serialization::de::SeqAccess<'de> for ArraySeqAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: data_serialization::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(v) => seed
                .deserialize(crate::editor::de::ValueDeserializer::new(v))
                .map(Some),
            None => Ok(None),
        }
    }
}
