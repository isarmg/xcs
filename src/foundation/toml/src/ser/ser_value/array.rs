use super::write_value;
use super::{Error, ValueSerializer};

type InnerSerializeValueSeq = <toml_edit::ser::ValueSerializer as data_serialization::Serializer>::SerializeSeq;

#[doc(hidden)]
pub struct SerializeValueArray<'d> {
    inner: InnerSerializeValueSeq,
    dst: &'d mut String,
}

impl<'d> SerializeValueArray<'d> {
    pub(crate) fn new(ser: ValueSerializer<'d>, inner: InnerSerializeValueSeq) -> Self {
        Self {
            inner,
            dst: ser.dst,
        }
    }
}

impl data_serialization::ser::SerializeSeq for SerializeValueArray<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: data_serialization::ser::Serialize + ?Sized,
    {
        self.inner.serialize_element(value).map_err(Error::wrap)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, self.inner.end())
    }
}

impl data_serialization::ser::SerializeTuple for SerializeValueArray<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: data_serialization::ser::Serialize + ?Sized,
    {
        self.inner.serialize_element(value).map_err(Error::wrap)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, self.inner.end())
    }
}

impl data_serialization::ser::SerializeTupleStruct for SerializeValueArray<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: data_serialization::ser::Serialize + ?Sized,
    {
        self.inner.serialize_field(value).map_err(Error::wrap)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, self.inner.end())
    }
}

type InnerSerializeValueTupleVariant =
    <toml_edit::ser::ValueSerializer as data_serialization::Serializer>::SerializeTupleVariant;

#[doc(hidden)]
pub struct SerializeValueTupleVariant<'d> {
    inner: InnerSerializeValueTupleVariant,
    dst: &'d mut String,
}

impl<'d> SerializeValueTupleVariant<'d> {
    pub(crate) fn new(ser: ValueSerializer<'d>, inner: InnerSerializeValueTupleVariant) -> Self {
        Self {
            inner,
            dst: ser.dst,
        }
    }
}

impl data_serialization::ser::SerializeTupleVariant for SerializeValueTupleVariant<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: data_serialization::ser::Serialize + ?Sized,
    {
        self.inner.serialize_field(value).map_err(Error::wrap)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        write_value(self.dst, self.inner.end())
    }
}
