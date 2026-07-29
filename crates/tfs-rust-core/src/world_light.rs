//! World ambient light and game time — cipsoft 772 `GetAmbiente` / `GetTime`.
//!
//! C++ reference:
//! - `tibia-game-master/src/time.cc` `GetAmbiente` (ambient level/color), `GetTime` (game clock).
//! - `tibia-game-master/src/main.cc:361-372` `AdvanceGame` ambient broadcast.
//! - `tibia-game-master/src/sending.cc:894-906` `SendAmbiente` (`0x82`).

use chrono::Timelike;

/// Cipsoft 772 `GetTime` (`time.cc:43-49`).
///
/// Maps each real-time hour to a game-time day:
/// `Hour = (sec_in_hour / 150)`, `Minute = (sec_in_hour % 150) * 2 / 5`.
pub fn world_time_from_local_clock() -> i16 {
    let lt = chrono::Local::now();
    let sec_in_hour = lt.second() as i32 + lt.minute() as i32 * 60;
    let hour = sec_in_hour / 150;
    let minute = (sec_in_hour % 150) * 2 / 5;
    (hour * 60 + minute) as i16
}

/// Cipsoft 772 `GetAmbiente` (`time.cc:60-92`).
///
/// Returns `(brightness, color)` for the given world time in game-minutes.
pub fn ambient_from_world_time(wt: i16) -> (u8, u8) {
    if wt < 60 {
        (0x33, 0xD7)
    } else if wt < 120 {
        (0x66, 0xD7)
    } else if wt < 180 {
        (0x99, 0xAD)
    } else if wt < 240 {
        (0xCC, 0xAD)
    } else if wt <= 1200 {
        (0xFF, 0xD7)
    } else if wt <= 1260 {
        (0xCC, 0xD0)
    } else if wt <= 1320 {
        (0x99, 0xD0)
    } else if wt <= 1380 {
        (0x66, 0xD7)
    } else {
        (0x33, 0xD7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_time_is_within_one_game_day() {
        let wt = world_time_from_local_clock();
        assert!((0..1440).contains(&wt), "world time should be within one game day");
    }

    #[test]
    fn ambiente_boundaries() {
        assert_eq!(ambient_from_world_time(0), (0x33, 0xD7));
        assert_eq!(ambient_from_world_time(59), (0x33, 0xD7));
        assert_eq!(ambient_from_world_time(60), (0x66, 0xD7));
        assert_eq!(ambient_from_world_time(119), (0x66, 0xD7));
        assert_eq!(ambient_from_world_time(120), (0x99, 0xAD));
        assert_eq!(ambient_from_world_time(179), (0x99, 0xAD));
        assert_eq!(ambient_from_world_time(180), (0xCC, 0xAD));
        assert_eq!(ambient_from_world_time(239), (0xCC, 0xAD));
        assert_eq!(ambient_from_world_time(240), (0xFF, 0xD7));
        assert_eq!(ambient_from_world_time(1200), (0xFF, 0xD7));
        assert_eq!(ambient_from_world_time(1201), (0xCC, 0xD0));
        assert_eq!(ambient_from_world_time(1260), (0xCC, 0xD0));
        assert_eq!(ambient_from_world_time(1261), (0x99, 0xD0));
        assert_eq!(ambient_from_world_time(1320), (0x99, 0xD0));
        assert_eq!(ambient_from_world_time(1321), (0x66, 0xD7));
        assert_eq!(ambient_from_world_time(1380), (0x66, 0xD7));
        assert_eq!(ambient_from_world_time(1381), (0x33, 0xD7));
        assert_eq!(ambient_from_world_time(1439), (0x33, 0xD7));
    }
}
