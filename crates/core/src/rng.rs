//! A small random number generator with a stable, documented sequence.

/// The SplitMix64 random number generator.
///
/// Generated puzzles must be reproducible from their seed on every platform
/// and with every dependency version, so every genre uses this instead of an
/// external crate whose output could change between releases. Its output
/// for a given seed will never change.
///
/// It is not suitable for cryptography.
///
/// ```
/// use pencil_puzzles_core::SplitMix64;
///
/// let mut rng = SplitMix64::new(42);
/// let roll = rng.below(6) + 1;
/// assert!((1..=6).contains(&roll));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// Creates a generator from a seed.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next number in the sequence.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..bound`.
    ///
    /// Uses a plain remainder, so numbers are very slightly biased towards
    /// the low end when `bound` is not a power of two. The bias is
    /// negligible for the small bounds puzzle generators use.
    ///
    /// # Panics
    ///
    /// Panics if `bound` is zero.
    pub fn below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "the bound must be positive");
        self.next_u64() % bound
    }

    /// Shuffles `values` in place (Fisher-Yates, from the end), drawing one
    /// number per position after the first.
    ///
    /// ```
    /// use pencil_puzzles_core::SplitMix64;
    ///
    /// let mut order = [0, 1, 2, 3];
    /// SplitMix64::new(7).shuffle(&mut order);
    /// order.sort_unstable();
    /// assert_eq!(order, [0, 1, 2, 3]);
    /// ```
    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            values.swap(i, self.below(i as u64 + 1) as usize);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_is_stable() {
        // Changing these values changes every generated puzzle in every
        // genre. They must never change.
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        let mut values = [0, 1, 2, 3, 4];
        SplitMix64::new(0).shuffle(&mut values);
        assert_eq!(values, [2, 3, 1, 4, 0]);
    }
}
