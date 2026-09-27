//! Admission bounds apply even when callers deserialize the public artifact type.
use serde::{
    de::{Error, SeqAccess, Visitor},
    Deserializer,
};
use std::fmt;

fn string<'de, D: Deserializer<'de>, const MAX: usize>(d: D) -> Result<String, D::Error> {
    struct Bounded<const MAX: usize>;
    impl<const MAX: usize> Visitor<'_> for Bounded<MAX> {
        type Value = String;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "a string of at most {MAX} bytes")
        }
        fn visit_str<E: Error>(self, value: &str) -> Result<String, E> {
            if value.len() > MAX {
                return Err(E::custom("private artifact metadata limit"));
            }
            Ok(value.to_owned())
        }
        fn visit_string<E: Error>(self, value: String) -> Result<String, E> {
            if value.len() > MAX {
                return Err(E::custom("private artifact metadata limit"));
            }
            Ok(value)
        }
    }
    d.deserialize_str(Bounded::<MAX>)
}

pub(super) fn name<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    string::<D, 4096>(d)
}
pub(super) fn identity<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    string::<D, 128>(d)
}
pub(super) fn assembly<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    string::<D, { 256 * 1024 }>(d)
}
pub(super) fn proof<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    const MAX: usize = 256 * 1024 * 1024;
    struct Bytes;
    impl<'de> Visitor<'de> for Bytes {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("bounded native proof bytes")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<u8>, A::Error> {
            if seq.size_hint().is_some_and(|n| n > MAX) {
                return Err(A::Error::custom("private proof allocation limit"));
            }
            let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(4096));
            while let Some(byte) = seq.next_element()? {
                if bytes.len() == MAX {
                    return Err(A::Error::custom("private proof allocation limit"));
                }
                bytes.push(byte);
            }
            Ok(bytes)
        }
    }
    d.deserialize_seq(Bytes)
}
