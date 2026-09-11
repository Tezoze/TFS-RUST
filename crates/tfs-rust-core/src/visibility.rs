//! Floor visibility — 772 `TCreature::CanSeeFloor`.
//! C++ reference: `cr.hh:576-582`.

/// Surface viewers (`z ≤ 7`) see floors 0..=7; underground viewers see `|Δz| ≤ 2`.
pub fn can_see_floor(viewer_z: u8, floor_z: u8) -> bool {
    if viewer_z <= 7 {
        floor_z <= 7
    } else {
        (viewer_z as i32 - floor_z as i32).abs() <= 2
    }
}

#[cfg(test)]
mod tests {
    use super::can_see_floor;

    #[test]
    fn surface_sees_surface_not_underground() {
        assert!(can_see_floor(7, 6));
        assert!(can_see_floor(6, 7));
        assert!(!can_see_floor(7, 8));
        assert!(!can_see_floor(7, 11));
    }

    #[test]
    fn underground_sees_plus_minus_two() {
        assert!(can_see_floor(11, 9));
        assert!(can_see_floor(11, 13));
        assert!(!can_see_floor(11, 7));
        assert!(!can_see_floor(11, 14));
    }
}
