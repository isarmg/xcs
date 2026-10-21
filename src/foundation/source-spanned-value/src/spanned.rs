use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

// Currently serde itself doesn't have a spanned type, so we map our `Spanned`
// to a special value in the serde data model. Namely one with these special
// fields/struct names.
//
// In general, supported deserializers should catch this and not literally emit
// these strings but rather emit `Spanned` as they're intended.
#[doc(hidden)]
#[cfg(feature = "serialization")]
pub const NAME: &str = "$__serde_spanned_private_Spanned";
#[doc(hidden)]
#[cfg(feature = "serialization")]
pub const START_FIELD: &str = "$__serde_spanned_private_start";
#[doc(hidden)]
#[cfg(feature = "serialization")]
pub const END_FIELD: &str = "$__serde_spanned_private_end";
#[doc(hidden)]
#[cfg(feature = "serialization")]
pub const VALUE_FIELD: &str = "$__serde_spanned_private_value";
#[doc(hidden)]
#[cfg(feature = "serialization")]
pub fn is_spanned(name: &'static str, fields: &'static [&'static str]) -> bool {
    name == NAME && fields == [START_FIELD, END_FIELD, VALUE_FIELD]
}

/// A spanned value, indicating the range at which it is defined in the source.
#[derive(Clone, Debug)]
pub struct Spanned<T> {
    /// Byte range
    span: std::ops::Range<usize>,
    /// The spanned value.
    value: T,
}

impl<T> Spanned<T> {
    /// Create a spanned value, indicating the range at which it is defined in the source.
    ///
    /// # Example
    ///
    /// ```
    /// use source_spanned_value::Spanned;
    /// let field = Spanned::new(8..12, 123_u32);
    /// assert_eq!(field.span(), 8..12);
    /// assert_eq!(*field.get_ref(), 123);
    /// # #[cfg(feature = "serialization")]
    /// # {
    /// fn accepts_serializable<T: data_serialization::Serialize>(_: &T) {}
    /// accepts_serializable(&field);
    /// # }
    /// ```
    pub fn new(range: std::ops::Range<usize>, value: T) -> Self {
        Spanned { span: range, value }
    }

    /// Byte range
    pub fn span(&self) -> std::ops::Range<usize> {
        self.span.clone()
    }

    /// Consumes the spanned value and returns the contained value.
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Returns a reference to the contained value.
    pub fn get_ref(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the contained value.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl std::borrow::Borrow<str> for Spanned<String> {
    fn borrow(&self) -> &str {
        self.get_ref()
    }
}

impl<T> AsRef<T> for Spanned<T> {
    fn as_ref(&self) -> &T {
        self.get_ref()
    }
}

impl<T> AsMut<T> for Spanned<T> {
    fn as_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl<T: Eq> Eq for Spanned<T> {}

impl<T: Hash> Hash for Spanned<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T: PartialOrd> PartialOrd for Spanned<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T: Ord> Ord for Spanned<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

#[cfg(feature = "serialization")]
impl<'de, T> data_serialization::de::Deserialize<'de> for Spanned<T>
where
    T: data_serialization::de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Spanned<T>, D::Error>
    where
        D: data_serialization::de::Deserializer<'de>,
    {
        struct SpannedVisitor<T>(::std::marker::PhantomData<T>);

        impl<'de, T> data_serialization::de::Visitor<'de> for SpannedVisitor<T>
        where
            T: data_serialization::de::Deserialize<'de>,
        {
            type Value = Spanned<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a spanned value")
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Spanned<T>, V::Error>
            where
                V: data_serialization::de::MapAccess<'de>,
            {
                let mut start: Option<usize> = None;
                let mut end: Option<usize> = None;
                let mut value: Option<T> = None;
                while let Some(key) = visitor.next_key()? {
                    match key {
                        START_FIELD => {
                            if start.is_some() {
                                return Err(data_serialization::de::Error::duplicate_field(START_FIELD));
                            }
                            start = Some(visitor.next_value()?);
                        }
                        END_FIELD => {
                            if end.is_some() {
                                return Err(data_serialization::de::Error::duplicate_field(END_FIELD));
                            }
                            end = Some(visitor.next_value()?);
                        }
                        VALUE_FIELD => {
                            if value.is_some() {
                                return Err(data_serialization::de::Error::duplicate_field(VALUE_FIELD));
                            }
                            value = Some(visitor.next_value()?);
                        }
                        field => {
                            return Err(data_serialization::de::Error::unknown_field(
                                field,
                                &[START_FIELD, END_FIELD, VALUE_FIELD],
                            ));
                        }
                    }
                }
                match (start, end, value) {
                    (Some(start), Some(end), Some(value)) => Ok(Spanned {
                        span: start..end,
                        value,
                    }),
                    (None, _, _) => Err(data_serialization::de::Error::missing_field(START_FIELD)),
                    (_, None, _) => Err(data_serialization::de::Error::missing_field(END_FIELD)),
                    (_, _, None) => Err(data_serialization::de::Error::missing_field(VALUE_FIELD)),
                }
            }
        }

        static FIELDS: [&str; 3] = [START_FIELD, END_FIELD, VALUE_FIELD];

        let visitor = SpannedVisitor(::std::marker::PhantomData);

        deserializer.deserialize_struct(NAME, &FIELDS, visitor)
    }
}

#[cfg(feature = "serialization")]
impl<T: data_serialization::ser::Serialize> data_serialization::ser::Serialize for Spanned<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: data_serialization::ser::Serializer,
    {
        self.value.serialize(serializer)
    }
}
