//! Circle-ring disc offsets for radius-based AoE — PC-3a.
//!
//! C++ reference:
//! - 772 `InitCircles` — `tibia-game-master/src/magic.cc:4344` loads `circles.dat`
//!   (21×21 grid, rings 0–7, 101 tiles) into `TCircle Circle[10]`.
//! - 772 `ExecuteCircleSpell` — `magic.cc:459` iterates rings `0..=R` and applies
//!   `ThrowPossible` + `handleField`/`handleCreature` per tile.
//! - 1098 `AreaCombat::setupArea(radius)` — `src/combat.cpp:1391` hardcodes a 13×13
//!   grid with rings 1–8. Verified identical to 772 rings 0–7 (offset by 1).
//!
//! Both eras use the **same** 772 disc-ring model — there is no era variance.
//! The 772 `circles.dat` ring-offset data is the canonical AoE shape for all
// spell/area combat. Rings are stored 0-indexed (0–7); a spell with radius R
//! hits all tiles in rings 0 through R.

/// Ring 0–7 offsets baked from `circles.dat` (772) / `combat.cpp:setupArea` (1098).
///
/// Each ring is a list of `(dx, dy)` offsets relative to the spell center.
/// Ring 0 is the center tile; ring N is the set of tiles at "ring distance" N.
///
/// Tile counts per ring: 1, 4, 4, 12, 16, 20, 16, 28 = 101 total.
pub const DISC_RINGS: &[&[(i32, i32)]] = &[
    // Ring 0 — center (1 tile)
    &[(0, 0)],
    // Ring 1 — orthogonal neighbours (4 tiles)
    &[(0, -1), (-1, 0), (1, 0), (0, 1)],
    // Ring 2 — diagonal neighbours (4 tiles)
    &[(-1, -1), (1, -1), (-1, 1), (1, 1)],
    // Ring 3 (12 tiles)
    &[
        (-1, -2),
        (0, -2),
        (1, -2),
        (-2, -1),
        (2, -1),
        (-2, 0),
        (2, 0),
        (-2, 1),
        (2, 1),
        (-1, 2),
        (0, 2),
        (1, 2),
    ],
    // Ring 4 (16 tiles)
    &[
        (-1, -3),
        (0, -3),
        (1, -3),
        (-2, -2),
        (2, -2),
        (-3, -1),
        (3, -1),
        (-3, 0),
        (3, 0),
        (-3, 1),
        (3, 1),
        (-2, 2),
        (2, 2),
        (-1, 3),
        (0, 3),
        (1, 3),
    ],
    // Ring 5 (20 tiles)
    &[
        (-1, -4),
        (0, -4),
        (1, -4),
        (-2, -3),
        (2, -3),
        (-3, -2),
        (3, -2),
        (-4, -1),
        (4, -1),
        (-4, 0),
        (4, 0),
        (-4, 1),
        (4, 1),
        (-3, 2),
        (3, 2),
        (-2, 3),
        (2, 3),
        (-1, 4),
        (0, 4),
        (1, 4),
    ],
    // Ring 6 (16 tiles)
    &[
        (0, -5),
        (-2, -4),
        (2, -4),
        (-3, -3),
        (3, -3),
        (-4, -2),
        (4, -2),
        (-5, 0),
        (5, 0),
        (-4, 2),
        (4, 2),
        (-3, 3),
        (3, 3),
        (-2, 4),
        (2, 4),
        (0, 5),
    ],
    // Ring 7 (28 tiles)
    &[
        (0, -6),
        (-2, -5),
        (-1, -5),
        (1, -5),
        (2, -5),
        (-3, -4),
        (3, -4),
        (-4, -3),
        (4, -3),
        (-5, -2),
        (5, -2),
        (-5, -1),
        (5, -1),
        (-6, 0),
        (6, 0),
        (-5, 1),
        (5, 1),
        (-5, 2),
        (5, 2),
        (-4, 3),
        (4, 3),
        (-3, 4),
        (3, 4),
        (-2, 5),
        (-1, 5),
        (1, 5),
        (2, 5),
        (0, 6),
    ],
];

/// Maximum ring index supported by the disc model (both eras).
pub const MAX_DISC_RADIUS: usize = 7;

/// Returns all `(dx, dy)` offsets for rings `0..=radius`.
///
/// Mirrors 772 `ExecuteCircleSpell` (`magic.cc:468`): `for R = 0; R <= Radius; R++`
/// collects every point in `Circle[R]`. The caller adds these to the spell center
/// to get target tile positions, then checks `throw_possible` per tile.
///
/// Clamps `radius` to `MAX_DISC_RADIUS` (matching `magic.cc:463`).
pub fn disc_offsets(radius: usize) -> Vec<(i32, i32)> {
    let r = radius.min(MAX_DISC_RADIUS);
    DISC_RINGS[..=r]
        .iter()
        .flat_map(|ring| ring.iter().copied())
        .collect()
}

/// Number of tiles covered by `disc_offsets(radius)` — useful for pre-allocation.
pub fn disc_tile_count(radius: usize) -> usize {
    let r = radius.min(MAX_DISC_RADIUS);
    DISC_RINGS[..=r].iter().map(|ring| ring.len()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disc_ring_counts_match_circles_dat() {
        // Verified from circles.dat: 21×21 grid, rings 0–7, 101 tiles total.
        assert_eq!(DISC_RINGS.len(), 8, "rings 0-7");
        assert_eq!(DISC_RINGS[0].len(), 1, "ring 0: center only");
        assert_eq!(DISC_RINGS[1].len(), 4, "ring 1: 4 orthogonal");
        assert_eq!(DISC_RINGS[2].len(), 4, "ring 2: 4 diagonal");
        assert_eq!(DISC_RINGS[3].len(), 12, "ring 3");
        assert_eq!(DISC_RINGS[4].len(), 16, "ring 4");
        assert_eq!(DISC_RINGS[5].len(), 20, "ring 5");
        assert_eq!(DISC_RINGS[6].len(), 16, "ring 6");
        assert_eq!(DISC_RINGS[7].len(), 28, "ring 7");
        let total: usize = DISC_RINGS.iter().map(|r| r.len()).sum();
        assert_eq!(total, 101, "total tiles from circles.dat");
    }

    #[test]
    fn disc_offsets_radius_0_is_center() {
        let offsets = disc_offsets(0);
        assert_eq!(offsets, vec![(0, 0)]);
    }

    #[test]
    fn disc_offsets_radius_1_is_plus_shape() {
        let offsets = disc_offsets(1);
        assert_eq!(offsets.len(), 5); // center + 4 orthogonal
        assert!(offsets.contains(&(0, 0)));
        assert!(offsets.contains(&(0, -1)));
        assert!(offsets.contains(&(0, 1)));
        assert!(offsets.contains(&(-1, 0)));
        assert!(offsets.contains(&(1, 0)));
        // No diagonals at radius 1
        assert!(!offsets.contains(&(-1, -1)));
    }

    #[test]
    fn disc_offsets_radius_6_matches_ue() {
        // UE (case 24) uses radius 6 in 772 — `MassCombat(..., 6, ...)`.
        // AREA_CIRCLE5X5 (1098 Lua matrix) produces the same 73 tiles.
        let offsets = disc_offsets(6);
        assert_eq!(offsets.len(), 73); // 1+4+4+12+16+20+16
    }

    #[test]
    fn disc_offsets_radius_7_is_full_disc() {
        let offsets = disc_offsets(7);
        assert_eq!(offsets.len(), 101);
    }

    #[test]
    fn disc_offsets_clamps_above_max() {
        // magic.cc:463 clamps Radius to NARRAY(Circle)-1 = 9.
        // Our MAX_DISC_RADIUS is 7 (the highest populated ring).
        let offsets = disc_offsets(99);
        assert_eq!(offsets.len(), 101, "clamped to max ring 7");
    }

    #[test]
    fn disc_tile_count_matches_offsets() {
        for r in 0..=MAX_DISC_RADIUS {
            assert_eq!(disc_tile_count(r), disc_offsets(r).len());
        }
    }

    #[test]
    fn disc_offsets_match_1098_tfs_grid() {
        // 1098 TFS combat.cpp:setupArea uses a 13×13 grid with rings 1–8.
        // 772 ring N == 1098 ring N+1 (verified numerically).
        // So disc_offsets(R) should match setupArea(R+1) for all R.
        let area_1098: [[i32; 13]; 13] = [
            [0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 8, 8, 7, 8, 8, 0, 0, 0, 0],
            [0, 0, 0, 8, 7, 6, 6, 6, 7, 8, 0, 0, 0],
            [0, 0, 8, 7, 6, 5, 5, 5, 6, 7, 8, 0, 0],
            [0, 8, 7, 6, 5, 4, 4, 4, 5, 6, 7, 8, 0],
            [0, 8, 6, 5, 4, 3, 2, 3, 4, 5, 6, 8, 0],
            [8, 7, 6, 5, 4, 2, 1, 2, 4, 5, 6, 7, 8],
            [0, 8, 6, 5, 4, 3, 2, 3, 4, 5, 6, 8, 0],
            [0, 8, 7, 6, 5, 4, 4, 4, 5, 6, 7, 8, 0],
            [0, 0, 8, 7, 6, 5, 5, 5, 6, 7, 8, 0, 0],
            [0, 0, 0, 8, 7, 6, 6, 6, 7, 8, 0, 0, 0],
            [0, 0, 0, 0, 8, 8, 7, 8, 8, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0],
        ];
        let center_1098 = 6; // center of 13×13

        for r772 in 0..=7usize {
            let r1098 = (r772 + 1) as i32;
            let mut expected: Vec<(i32, i32)> = Vec::new();
            for y in 0..13 {
                for x in 0..13 {
                    if area_1098[y][x] > 0 && area_1098[y][x] <= r1098 {
                        expected.push((x as i32 - center_1098, y as i32 - center_1098));
                    }
                }
            }
            expected.sort();
            let mut actual = disc_offsets(r772);
            actual.sort();
            assert_eq!(
                actual, expected,
                "772 ring 0..={r772} must match 1098 setupArea({})",
                r1098
            );
        }
    }
}
