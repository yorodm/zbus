use serde::{Deserialize, Deserializer, Serialize};
use std::convert::TryFrom;

use crate::{Array, Dict, Fd, ObjectPath, Signature, Structure, Type, Value};

/// Owned [`Value`](enum.Value.html)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OwnedValue(Value<'static>);

impl OwnedValue {
    pub(crate) fn into_inner(self) -> Value<'static> {
        self.0
    }
}

macro_rules! ov_try_from {
    ($to:ty) => {
        impl<'a> TryFrom<OwnedValue> for $to {
            type Error = crate::Error;

            fn try_from(v: OwnedValue) -> Result<Self, Self::Error> {
                <$to>::try_from(v.0)
            }
        }
    };
}

macro_rules! ov_try_from_ref {
    ($to:ty) => {
        impl<'a> TryFrom<&'a OwnedValue> for $to {
            type Error = crate::Error;

            fn try_from(v: &'a OwnedValue) -> Result<Self, Self::Error> {
                <$to>::try_from(&v.0)
            }
        }
    };
}

ov_try_from!(u8);
ov_try_from!(bool);
ov_try_from!(i16);
ov_try_from!(u16);
ov_try_from!(i32);
ov_try_from!(u32);
ov_try_from!(i64);
ov_try_from!(u64);
ov_try_from!(f64);
ov_try_from!(String);
ov_try_from!(Signature<'a>);
ov_try_from!(ObjectPath<'a>);
ov_try_from!(Array<'a>);
ov_try_from!(Dict<'a, 'a>);
ov_try_from!(Structure<'a>);
ov_try_from!(Fd);

ov_try_from_ref!(u8);
ov_try_from_ref!(bool);
ov_try_from_ref!(i16);
ov_try_from_ref!(u16);
ov_try_from_ref!(i32);
ov_try_from_ref!(u32);
ov_try_from_ref!(i64);
ov_try_from_ref!(u64);
ov_try_from_ref!(f64);
ov_try_from_ref!(&'a str);
ov_try_from_ref!(&'a Signature<'a>);
ov_try_from_ref!(&'a ObjectPath<'a>);
ov_try_from_ref!(&'a Array<'a>);
ov_try_from_ref!(&'a Dict<'a, 'a>);
ov_try_from_ref!(&'a Structure<'a>);
ov_try_from_ref!(Fd);

impl<'a, T> TryFrom<OwnedValue> for Vec<T>
where
    T: TryFrom<Value<'a>, Error = crate::Error> + 'a,
{
    type Error = crate::Error;

    fn try_from(value: OwnedValue) -> Result<Self, Self::Error> {
        if let Value::Array(v) = value.0 {
            Self::try_from(v)
        } else {
            Err(crate::Error::IncorrectType)
        }
    }
}

impl<'a> From<Value<'a>> for OwnedValue {
    fn from(v: Value<'a>) -> Self {
        // TODO: add into_owned, avoiding copy if already owned..
        OwnedValue(v.to_owned())
    }
}

impl<'a> From<&Value<'a>> for OwnedValue {
    fn from(v: &Value<'a>) -> Self {
        OwnedValue(v.to_owned())
    }
}

impl<'a> Type for OwnedValue {
    fn signature() -> Signature<'static> {
        Value::signature()
    }
}

impl std::ops::Deref for OwnedValue {
    type Target = Value<'static>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for OwnedValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Value::deserialize(deserializer)?.into())
    }
}

#[cfg(test)]
mod tests {
    use byteorder::LE;
    use std::convert::TryFrom;
    use std::error::Error;
    use std::result::Result;

    use crate::{from_slice, to_bytes, EncodingContext, OwnedValue, Value};

    #[test]
    fn from_value() -> Result<(), Box<dyn Error>> {
        let v = Value::from("hi!");
        let ov: OwnedValue = v.into();
        assert_eq!(<&str>::try_from(&ov)?, "hi!");
        Ok(())
    }

    #[test]
    fn serde() -> Result<(), Box<dyn Error>> {
        let ec = EncodingContext::<LE>::new_dbus(0);
        let ov: OwnedValue = Value::from("hi!").into();
        let ser = to_bytes(ec, &ov)?;
        let de: Value = from_slice(&ser, ec)?;
        assert_eq!(<&str>::try_from(&de)?, "hi!");
        Ok(())
    }
}
