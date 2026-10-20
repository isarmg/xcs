//! Check the original Serde data model before JSON converts NaN/infinity to null.
use serde::{
    Serialize,
    ser::{self, Serializer},
};

pub(crate) fn validate<T: Serialize + ?Sized>(value: &T) -> Result {
    value.serialize(IntegerJson)
}

#[derive(Clone, Copy)]
struct IntegerJson;
type Result<T = ()> = std::result::Result<T, serde_json::Error>;

fn invalid(message: &str) -> serde_json::Error {
    ser::Error::custom(message)
}

macro_rules! scalar {
    ($($name:ident: $ty:ty),* $(,)?) => {$(
        fn $name(self, _: $ty) -> Result { Ok(()) }
    )*};
}

impl Serializer for IntegerJson {
    type Ok = ();
    type Error = serde_json::Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    scalar!(serialize_bool: bool, serialize_i8: i8, serialize_i16: i16,
        serialize_i32: i32, serialize_u8: u8, serialize_u16: u16,
        serialize_u32: u32, serialize_char: char, serialize_str: &str,
        serialize_bytes: &[u8]);

    fn serialize_i64(self, value: i64) -> Result {
        if value.unsigned_abs() > (1 << 53) - 1 {
            Err(invalid("full-width integers must use decimal strings"))
        } else {
            Ok(())
        }
    }
    fn serialize_u64(self, value: u64) -> Result {
        if value > (1 << 53) - 1 {
            Err(invalid("full-width integers must use decimal strings"))
        } else {
            Ok(())
        }
    }
    fn serialize_i128(self, value: i128) -> Result {
        if value.unsigned_abs() > (1 << 53) - 1 {
            Err(invalid("full-width integers must use decimal strings"))
        } else {
            Ok(())
        }
    }
    fn serialize_u128(self, value: u128) -> Result {
        if value > (1 << 53) - 1 {
            Err(invalid("full-width integers must use decimal strings"))
        } else {
            Ok(())
        }
    }
    fn serialize_f32(self, _: f32) -> Result {
        Err(invalid("floating point is forbidden"))
    }
    fn serialize_f64(self, _: f64) -> Result {
        Err(invalid("floating point is forbidden"))
    }
    fn serialize_none(self) -> Result {
        Ok(())
    }
    fn serialize_unit(self) -> Result {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result {
        Ok(())
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result {
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result {
        value.serialize(self)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _: &'static str, value: &T) -> Result {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        value: &T,
    ) -> Result {
        value.serialize(self)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self> {
        Ok(self)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self> {
        Ok(self)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self> {
        Ok(self)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self> {
        Ok(self)
    }
}

macro_rules! sequence {
    ($trait:ident, $method:ident $(, $key:ident: $ty:ty)?) => {
        impl ser::$trait for IntegerJson {
            type Ok = ();
            type Error = serde_json::Error;
            fn $method<T: Serialize + ?Sized>(&mut self, $($key: $ty,)? value: &T) -> Result {
                $(let _ = $key;)?
                value.serialize(*self)
            }
            fn end(self) -> Result { Ok(()) }
        }
    };
}
sequence!(SerializeSeq, serialize_element);
sequence!(SerializeTuple, serialize_element);
sequence!(SerializeTupleStruct, serialize_field);
sequence!(SerializeTupleVariant, serialize_field);
sequence!(SerializeStruct, serialize_field, key: &'static str);
sequence!(SerializeStructVariant, serialize_field, key: &'static str);

impl ser::SerializeMap for IntegerJson {
    type Ok = ();
    type Error = serde_json::Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, value: &T) -> Result {
        value.serialize(*self)
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result {
        value.serialize(*self)
    }
    fn end(self) -> Result {
        Ok(())
    }
}
