//! Deterministic guest PRNG shared by `libc` and (future) kernel modules.

/// Advance the xorshift64 state and return a deterministic u32-masked value.
pub(crate) fn next_rand(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x & 0xFFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hle::context::RAND_SEED;

    #[test]
    fn sequence_is_deterministic_from_seed() {
        let mut state = RAND_SEED;
        let seq: Vec<u64> = (0..10).map(|_| next_rand(&mut state)).collect();
        assert_eq!(
            seq,
            vec![
                200_494_509,
                40_788_086,
                3_851_444_534,
                915_262_580,
                2_714_061_548,
                1_316_748_153,
                3_605_590_735,
                452_227_306,
                2_966_872_715,
                1_229_098_382,
            ]
        );
    }
}
