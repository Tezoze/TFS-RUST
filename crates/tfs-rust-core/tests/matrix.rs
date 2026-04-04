//! Property 11: `MatrixArea` transforms (flip / mirror / rotate90) consistency.
// C++ reference: `combat.cpp` `MatrixArea::flip`, `mirror`, `rotate90`.

use proptest::prelude::*;
use tfs_rust_core::MatrixArea;

fn random_matrix(rows: u32, cols: u32, bits: Vec<bool>) -> MatrixArea {
    let mut m = MatrixArea::new(rows, cols);
    let mut i = 0;
    for r in 0..rows {
        for c in 0..cols {
            if i < bits.len() && bits[i] {
                m.set(r, c, true);
            }
            i += 1;
        }
    }
    m.set_center(0, 0);
    m
}

proptest! {
    #[test]
    fn flip_is_involution(
        rows in 1u32..6,
        cols in 1u32..6,
    ) {
        let n = (rows * cols) as usize;
        let bits = vec![true; n];
        let m = random_matrix(rows, cols, bits);
        prop_assert_eq!(m.flip().flip(), m);
    }

    #[test]
    fn mirror_is_involution(
        rows in 1u32..6,
        cols in 1u32..6,
    ) {
        let n = (rows * cols) as usize;
        let bits = vec![true; n];
        let m = random_matrix(rows, cols, bits);
        prop_assert_eq!(m.mirror().mirror(), m);
    }

    #[test]
    fn rotate90_four_times_square(
        sz in 2u32..8,
    ) {
        let mut m = MatrixArea::new(sz, sz);
        m.set(0, 0, true);
        m.set(sz - 1, sz - 1, true);
        m.set_center(0, 0);
        let m4 = m.rotate90().rotate90().rotate90().rotate90();
        prop_assert_eq!(m4, m);
    }
}
