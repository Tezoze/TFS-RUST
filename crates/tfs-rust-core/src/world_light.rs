//! World ambient light — `Game::updateWorldLightLevel` / `getWorldLightInfo` (`src/game.cpp`).

use chrono::Timelike;

/// Matches `Game::worldTime` update (`Game::updateWorldTime`).
pub fn world_time_from_local_clock() -> i16 {
    let lt = chrono::Local::now();
    let sec_in_hour = lt.second() as i32 + lt.minute() as i32 * 60;
    ((sec_in_hour as f32) / 2.5) as i16
}

/// `Game::updateWorldLightLevel` — returns `light_level` (default color `215` elsewhere).
pub fn light_level_from_world_time(wt: i16) -> u8 {
    const GAME_SUNRISE: i16 = 360;
    const GAME_DAYTIME: i16 = 480;
    const GAME_SUNSET: i16 = 1080;
    const GAME_NIGHTTIME: i16 = 1200;
    const LIGHT_DAY: f32 = 250.0;
    const LIGHT_NIGHT: f32 = 40.0;

    if (GAME_SUNRISE..=GAME_DAYTIME).contains(&wt) {
        let t = (wt - GAME_SUNRISE) as f32 / (GAME_DAYTIME - GAME_SUNRISE) as f32;
        (LIGHT_NIGHT + t * (LIGHT_DAY - LIGHT_NIGHT)) as u8
    } else if (GAME_SUNSET..=GAME_NIGHTTIME).contains(&wt) {
        let t = (wt - GAME_SUNSET) as f32 / (GAME_NIGHTTIME - GAME_SUNSET) as f32;
        (LIGHT_DAY - t * (LIGHT_DAY - LIGHT_NIGHT)) as u8
    } else if !(GAME_SUNRISE..GAME_NIGHTTIME).contains(&wt) {
        LIGHT_NIGHT as u8
    } else {
        LIGHT_DAY as u8
    }
}
