use byteorder::ByteOrder;
use std::{borrow::Cow, error, fmt, str};

use crate::utils::padding_for_n_bytes;
use crate::{DictEntry, ObjectPath, Signature, Structure, Variant};

#[derive(Debug)]
pub enum VariantError {
    ExcessData,
    IncorrectType,
    IncorrectValue,
    InvalidUtf8,
    InsufficientData,
    UnsupportedType(String),
}

impl error::Error for VariantError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl fmt::Display for VariantError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VariantError::ExcessData => write!(f, "excess data"),
            VariantError::IncorrectType => write!(f, "incorrect type"),
            VariantError::IncorrectValue => write!(f, "incorrect value"),
            VariantError::InvalidUtf8 => write!(f, "invalid UTF-8"),
            VariantError::InsufficientData => write!(f, "insufficient data"),
            VariantError::UnsupportedType(s) => {
                write!(f, "unsupported type (signature: \"{}\")", s)
            }
        }
    }
}

pub trait VariantType<'a>: std::fmt::Debug {
    const SIGNATURE_CHAR: char;
    const SIGNATURE_STR: &'static str;
    const ALIGNMENT: usize;

    // FIXME: Would be nice if this returned a slice
    fn encode(&self, n_bytes_before: usize) -> Vec<u8>;

    // Default implementation works for constant-sized types where size is the same as their
    // alignment
    fn slice_data<'b>(
        bytes: &'b [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<&'b [u8], VariantError>
    where
        Self: Sized,
    {
        Self::ensure_correct_signature(signature)?;
        let len = Self::ALIGNMENT + padding_for_n_bytes(n_bytes_before, Self::ALIGNMENT);
        ensure_sufficient_bytes(bytes, len)?;

        Ok(&bytes[0..len])
    }
    fn ensure_correct_signature(signature: &str) -> Result<(), VariantError>
    where
        Self: Sized,
    {
        if signature != Self::SIGNATURE_STR {
            return Err(VariantError::IncorrectType);
        }

        Ok(())
    }

    fn decode(
        bytes: &'a [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<Self, VariantError>
    where
        Self: Sized;

    fn signature<'b>(&'b self) -> Cow<'b, str>
    where
        Self: Sized,
    {
        Cow::from(Self::SIGNATURE_STR)
    }

    fn slice_signature(signature: &str) -> Result<&str, VariantError>
    where
        Self: Sized,
    {
        let slice = &signature[0..1];
        Self::ensure_correct_signature(slice)?;

        Ok(slice)
    }

    fn create_bytes_vec(n_bytes_before: usize) -> Vec<u8>
    where
        Self: Sized,
    {
        let padding = padding_for_n_bytes(n_bytes_before, Self::ALIGNMENT);

        std::iter::repeat(0).take(padding).collect()
    }

    // Helper for decode() implementation
    fn slice_for_decoding<'b>(
        bytes: &'b [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<&'b [u8], VariantError>
    where
        Self: Sized,
    {
        Self::ensure_correct_signature(signature)?;
        let padding = Self::padding(n_bytes_before);
        let len = Self::ALIGNMENT + padding;
        ensure_sufficient_bytes(bytes, len)?;

        Ok(&bytes[padding..])
    }

    fn padding(n_bytes_before: usize) -> usize
    where
        Self: Sized,
    {
        padding_for_n_bytes(n_bytes_before, Self::ALIGNMENT)
    }
}

pub trait SimpleVariantType<'a>: VariantType<'a> {
    fn slice_data_simple<'b>(
        data: &'b [u8],
        n_bytes_before: usize,
    ) -> Result<&'b [u8], VariantError>
    where
        Self: Sized,
    {
        Self::slice_data(data, Self::SIGNATURE_STR, n_bytes_before)
    }

    fn decode_simple(bytes: &'a [u8], n_bytes_before: usize) -> Result<Self, VariantError>
    where
        Self: Sized,
    {
        Self::decode(bytes, Self::SIGNATURE_STR, n_bytes_before)
    }
}

impl<'a> VariantType<'a> for u8 {
    const SIGNATURE_CHAR: char = 'y';
    const SIGNATURE_STR: &'static str = "y";
    const ALIGNMENT: usize = 1;

    fn encode(&self, n_bytes_before: usize) -> Vec<u8> {
        let mut bytes = Self::create_bytes_vec(n_bytes_before);
        bytes.extend(&self.to_ne_bytes());

        bytes
    }

    fn decode(
        bytes: &'a [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<Self, VariantError> {
        let slice = Self::slice_for_decoding(bytes, signature, n_bytes_before)?;

        Ok(slice[0])
    }
}
impl<'a> SimpleVariantType<'a> for u8 {}

impl<'a> VariantType<'a> for bool {
    const SIGNATURE_CHAR: char = 'b';
    const SIGNATURE_STR: &'static str = "b";
    const ALIGNMENT: usize = 4;

    fn encode(&self, n_bytes_before: usize) -> Vec<u8> {
        let mut bytes = Self::create_bytes_vec(n_bytes_before);
        bytes.extend(&(*self as u32).to_ne_bytes());

        bytes
    }

    fn decode(
        bytes: &'a [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<Self, VariantError> {
        let slice = Self::slice_for_decoding(bytes, signature, n_bytes_before)?;

        match byteorder::NativeEndian::read_u32(&slice) {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(VariantError::IncorrectValue),
        }
    }
}
impl<'a> SimpleVariantType<'a> for bool {}

impl<'a> VariantType<'a> for i16 {
    const SIGNATURE_CHAR: char = 'n';
    const SIGNATURE_STR: &'static str = "n";
    const ALIGNMENT: usize = 2;

    fn encode(&self, n_bytes_before: usize) -> Vec<u8> {
        let mut bytes = Self::create_bytes_vec(n_bytes_before);
        bytes.extend(&self.to_ne_bytes());

        bytes
    }

    fn decode(
        bytes: &'a [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<Self, VariantError> {
        let slice = Self::slice_for_decoding(bytes, signature, n_bytes_before)?;

        Ok(byteorder::NativeEndian::read_i16(slice))
    }
}
impl<'a> SimpleVariantType<'a> for i16 {}

impl<'a> VariantType<'a> for u16 {
    const SIGNATURE_CHAR: char = 'q';
    const SIGNATURE_STR: &'static str = "q";
    const ALIGNMENT: usize = 2;

    fn encode(&self, n_bytes_before: usize) -> Vec<u8> {
        let mut bytes = Self::create_bytes_vec(n_bytes_before);
        bytes.extend(&self.to_ne_bytes());

        bytes
    }

    fn decode(
        bytes: &'a [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<Self, VariantError> {
        let slice = Self::slice_for_decoding(bytes, signature, n_bytes_before)?;

        Ok(byteorder::NativeEndian::read_u16(slice))
    }
}
impl<'a> SimpleVariantType<'a> for u16 {}

impl<'a> VariantType<'a> for i32 {
    const SIGNATURE_CHAR: char = 'i';
    const SIGNATURE_STR: &'static str = "i";
    const ALIGNMENT: usize = 4;

    fn encode(&self, n_bytes_before: usize) -> Vec<u8> {
        let mut bytes = Self::create_bytes_vec(n_bytes_before);
        bytes.extend(&self.to_ne_bytes());

        bytes
    }

    fn decode(
        bytes: &'a [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<Self, VariantError> {
        let slice = Self::slice_for_decoding(bytes, signature, n_bytes_before)?;

        Ok(byteorder::NativeEndian::read_i32(slice))
    }
}
impl<'a> SimpleVariantType<'a> for i32 {}

impl<'a> VariantType<'a> for u32 {
    const SIGNATURE_CHAR: char = 'u';
    const SIGNATURE_STR: &'static str = "u";
    const ALIGNMENT: usize = 4;

    fn encode(&self, n_bytes_before: usize) -> Vec<u8> {
        let mut bytes = Self::create_bytes_vec(n_bytes_before);
        bytes.extend(&self.to_ne_bytes());

        bytes
    }

    fn decode(
        bytes: &'a [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<Self, VariantError> {
        let slice = Self::slice_for_decoding(bytes, signature, n_bytes_before)?;

        Ok(byteorder::NativeEndian::read_u32(slice))
    }
}
impl<'a> SimpleVariantType<'a> for u32 {}

impl<'a> VariantType<'a> for i64 {
    const SIGNATURE_CHAR: char = 'x';
    const SIGNATURE_STR: &'static str = "x";
    const ALIGNMENT: usize = 8;

    fn encode(&self, n_bytes_before: usize) -> Vec<u8> {
        let mut bytes = Self::create_bytes_vec(n_bytes_before);
        bytes.extend(&self.to_ne_bytes());

        bytes
    }

    fn decode(
        bytes: &'a [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<Self, VariantError> {
        let slice = Self::slice_for_decoding(bytes, signature, n_bytes_before)?;

        Ok(byteorder::NativeEndian::read_i64(slice))
    }
}
impl<'a> SimpleVariantType<'a> for i64 {}

impl<'a> VariantType<'a> for u64 {
    const SIGNATURE_CHAR: char = 't';
    const SIGNATURE_STR: &'static str = "t";
    const ALIGNMENT: usize = 8;

    fn encode(&self, n_bytes_before: usize) -> Vec<u8> {
        let mut bytes = Self::create_bytes_vec(n_bytes_before);
        bytes.extend(&self.to_ne_bytes());

        bytes
    }

    fn decode(
        bytes: &'a [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<Self, VariantError> {
        let slice = Self::slice_for_decoding(bytes, signature, n_bytes_before)?;

        Ok(byteorder::NativeEndian::read_u64(slice))
    }
}
impl<'a> SimpleVariantType<'a> for u64 {}

impl<'a> VariantType<'a> for f64 {
    const SIGNATURE_CHAR: char = 'd';
    const SIGNATURE_STR: &'static str = "d";
    const ALIGNMENT: usize = 8;

    fn encode(&self, n_bytes_before: usize) -> Vec<u8> {
        let mut v = Self::create_bytes_vec(n_bytes_before);
        let mut bytes = [0; 8];
        byteorder::NativeEndian::write_f64(&mut bytes, *self);
        v.extend(&bytes);

        v
    }

    fn decode(
        bytes: &'a [u8],
        signature: &str,
        n_bytes_before: usize,
    ) -> Result<Self, VariantError> {
        let slice = Self::slice_for_decoding(bytes, signature, n_bytes_before)?;

        Ok(byteorder::NativeEndian::read_f64(slice))
    }
}
impl<'a> SimpleVariantType<'a> for f64 {}

pub(crate) fn ensure_sufficient_bytes(bytes: &[u8], size: usize) -> Result<(), VariantError> {
    if bytes.len() < size {
        return Err(VariantError::InsufficientData);
    }

    Ok(())
}

pub(crate) fn slice_data<'a>(
    data: &'a [u8],
    signature: &str,
    n_bytes_before: usize,
) -> Result<&'a [u8], VariantError> {
    match signature
        .chars()
        .next()
        .ok_or(VariantError::InsufficientData)?
    {
        // FIXME: There has to be a shorter way to do this
        u8::SIGNATURE_CHAR => u8::slice_data_simple(data, n_bytes_before),
        bool::SIGNATURE_CHAR => bool::slice_data_simple(data, n_bytes_before),
        i16::SIGNATURE_CHAR => i16::slice_data_simple(data, n_bytes_before),
        u16::SIGNATURE_CHAR => u16::slice_data_simple(data, n_bytes_before),
        i32::SIGNATURE_CHAR => i32::slice_data_simple(data, n_bytes_before),
        u32::SIGNATURE_CHAR => u32::slice_data_simple(data, n_bytes_before),
        i64::SIGNATURE_CHAR => i64::slice_data_simple(data, n_bytes_before),
        u64::SIGNATURE_CHAR => u64::slice_data_simple(data, n_bytes_before),
        f64::SIGNATURE_CHAR => f64::slice_data_simple(data, n_bytes_before),
        <(&str)>::SIGNATURE_CHAR => <(&str)>::slice_data_simple(data, n_bytes_before),
        // Doesn't matter what type for T we use here, signature is the same but we're also assuming `slice_data` to
        // be independent of `T` (an internal detail).
        Vec::<bool>::SIGNATURE_CHAR => Vec::<bool>::slice_data(data, signature, n_bytes_before),
        ObjectPath::SIGNATURE_CHAR => ObjectPath::slice_data_simple(data, n_bytes_before),
        Signature::SIGNATURE_CHAR => Signature::slice_data_simple(data, n_bytes_before),
        Structure::SIGNATURE_CHAR => Structure::slice_data(data, signature, n_bytes_before),
        Variant::SIGNATURE_CHAR => Variant::slice_data(data, signature, n_bytes_before),
        // Doesn't matter what type for T we use here, signature is the same but we're also assuming `slice_data` to
        // be independent of `T` (an internal detail).
        DictEntry::<bool, bool>::SIGNATURE_CHAR => {
            DictEntry::<bool, bool>::slice_data(data, signature, n_bytes_before)
        }
        _ => return Err(VariantError::UnsupportedType(String::from(signature))),
    }
}

pub(crate) fn alignment_for_signature(signature: &str) -> Result<usize, VariantError> {
    match signature
        .chars()
        .next()
        .ok_or(VariantError::InsufficientData)?
    {
        // FIXME: There has to be a shorter way to do this
        u8::SIGNATURE_CHAR => Ok(u8::ALIGNMENT),
        bool::SIGNATURE_CHAR => Ok(bool::ALIGNMENT),
        i16::SIGNATURE_CHAR => Ok(i16::ALIGNMENT),
        u16::SIGNATURE_CHAR => Ok(u16::ALIGNMENT),
        i32::SIGNATURE_CHAR => Ok(i32::ALIGNMENT),
        u32::SIGNATURE_CHAR => Ok(u32::ALIGNMENT),
        i64::SIGNATURE_CHAR => Ok(i64::ALIGNMENT),
        u64::SIGNATURE_CHAR => Ok(u64::ALIGNMENT),
        f64::SIGNATURE_CHAR => Ok(f64::ALIGNMENT),
        <(&str)>::SIGNATURE_CHAR => Ok(<(&str)>::ALIGNMENT),
        // Doesn't matter what type for T we use here, alignment is the same
        Vec::<bool>::SIGNATURE_CHAR => Ok(Vec::<bool>::ALIGNMENT),
        ObjectPath::SIGNATURE_CHAR => Ok(ObjectPath::ALIGNMENT),
        Signature::SIGNATURE_CHAR => Ok(Signature::ALIGNMENT),
        Structure::SIGNATURE_CHAR => Ok(Structure::ALIGNMENT),
        Variant::SIGNATURE_CHAR => Ok(Variant::ALIGNMENT),
        // Doesn't matter what type for T we use here, alignment is the same
        DictEntry::<bool, bool>::SIGNATURE_CHAR => Ok(DictEntry::<bool, bool>::ALIGNMENT),
        _ => return Err(VariantError::UnsupportedType(String::from(signature))),
    }
}

pub(crate) fn slice_signature(signature: &str) -> Result<&str, VariantError> {
    match signature
        .chars()
        .next()
        .ok_or(VariantError::InsufficientData)?
    {
        // FIXME: There has to be a shorter way to do this
        u8::SIGNATURE_CHAR => u8::slice_signature(signature),
        bool::SIGNATURE_CHAR => bool::slice_signature(signature),
        i16::SIGNATURE_CHAR => i16::slice_signature(signature),
        u16::SIGNATURE_CHAR => u16::slice_signature(signature),
        i32::SIGNATURE_CHAR => i32::slice_signature(signature),
        u32::SIGNATURE_CHAR => u32::slice_signature(signature),
        i64::SIGNATURE_CHAR => i64::slice_signature(signature),
        u64::SIGNATURE_CHAR => u64::slice_signature(signature),
        f64::SIGNATURE_CHAR => f64::slice_signature(signature),
        <(&str)>::SIGNATURE_CHAR => <(&str)>::slice_signature(signature),
        // Doesn't matter what type for T we use here, signature is the same but we're also assuming `slice_signature`
        // to be independent of `T` (an internal detail).
        Vec::<bool>::SIGNATURE_CHAR => Vec::<bool>::slice_signature(signature),
        ObjectPath::SIGNATURE_CHAR => ObjectPath::slice_signature(signature),
        Signature::SIGNATURE_CHAR => Signature::slice_signature(signature),
        Structure::SIGNATURE_CHAR => Structure::slice_signature(signature),
        Variant::SIGNATURE_CHAR => Variant::slice_signature(signature),
        // Doesn't matter what type for T we use here, signature is the same but we're also assuming `slice_signature`
        // to be independent of `T` (an internal detail).
        DictEntry::<bool, bool>::SIGNATURE_CHAR => {
            DictEntry::<bool, bool>::slice_signature(signature)
        }
        _ => return Err(VariantError::UnsupportedType(String::from(signature))),
    }
}
