//! Bounded serde inputs shared by genre parsers (G8, G9, I5).
use serde::{
    Deserialize, Deserializer,
    de::{self, SeqAccess, Visitor},
};
use std::{fmt, marker::PhantomData};
/// Deserialize at most MAX entries, rejecting excess before growing the result.
pub fn bounded_vec<'de, D, T, const MAX: usize>(d: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Limited<T, const N: usize>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const N: usize> Visitor<'de> for Limited<T, N> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "at most {N} entries")
        }
        fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Self::Value, S::Error> {
            let mut result = Vec::new();
            while let Some(value) = seq.next_element()? {
                if result.len() == N {
                    return Err(de::Error::custom("too many rows or columns"));
                }
                result.push(value);
            }
            Ok(result)
        }
    }
    d.deserialize_seq(Limited::<T, MAX>(PhantomData))
}
/// A string checked for byte length before taking ownership.
#[non_exhaustive]
pub struct BoundedText<const MAX: usize>(pub String);
impl<'de, const MAX: usize> Deserialize<'de> for BoundedText<MAX> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Text<const N: usize>;
        impl<const N: usize> Visitor<'_> for Text<N> {
            type Value = BoundedText<N>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "at most {N} characters")
            }
            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                if s.len() > N {
                    return Err(E::custom("the row is too long"));
                }
                Ok(BoundedText(s.into()))
            }
        }
        d.deserialize_str(Text::<MAX>)
    }
}
