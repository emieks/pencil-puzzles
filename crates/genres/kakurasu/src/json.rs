//! Bounded JSON dimensions (G8, G9).
use serde::{Deserialize, Deserializer};
pub(crate) fn bounded<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    pencil_puzzles_core::serde_helpers::bounded_vec::<D, T, { crate::MAX_SIZE }>(d)
}
pub(crate) type Row = pencil_puzzles_core::serde_helpers::BoundedText<{ crate::MAX_SIZE }>;
