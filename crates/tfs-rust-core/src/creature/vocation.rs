//! Level and vocation stat progression (TFS `Player::getReqExperience`, vocation gains).
// C++ reference: `player.cpp`, `vocation.cpp`; 772 base speed — `gameserver/src/player.h` `updateBaseSpeed`.

/// Total experience required to **reach** `level` (level >= 2). Matches common TFS polynomial.
// C++ reference: `Player::getExpForLevel` / level progression tables.
pub fn total_experience_for_level(level: u32) -> u64 {
    if level <= 1 {
        return 0;
    }
    let l = level as i64;
    let v = (50 * l * l * l) / 3 - 100 * l * l + (850 * l) / 3 - 200;
    v.max(0) as u64
}

/// Experience needed to go from `level` to `level + 1`.
pub fn experience_to_next_level(level: i32) -> u64 {
    if level < 1 {
        return 0;
    }
    let next = total_experience_for_level(level as u32 + 1);
    let cur = total_experience_for_level(level as u32);
    next.saturating_sub(cur)
}

/// Per-level resource gains by vocation id (from `vocations.xml` — stub defaults).
// C++ reference: `Vocation::getHealthGain`, `getManaGain`, `getCapGain`.
pub fn per_level_gains(vocation_id: i32) -> (i32, i32, i32) {
    match vocation_id {
        1 | 5 => (5, 30, 10), // sorc / druid (example)
        2 | 6 => (15, 5, 25), // paladin
        3 | 7 => (25, 5, 45), // knight
        _ => (10, 10, 15),
    }
}

use crate::formulas::StepSpeedModel;

/// Vocation `basespeed` from `data/XML/vocations.xml` (all 220 in shipped 772 pack).
fn vocation_base_speed(vocation_id: i32) -> i32 {
    let _ = vocation_id;
    220
}

/// Stored `Creature::baseSpeed` (GoStrength) before `GetSpeed = 2*base+80`.
///
/// - **1098** — TFS `vocation->getBaseSpeed() + 2*(level-1)` (`src/player.h` `updateBaseSpeed`).
/// - **772** — TVP `vocation->getBaseSpeed() + (level > 1 ? level : 0)` (`gameserver/src/player.h`).
pub fn base_walk_speed(model: StepSpeedModel, vocation_id: i32, level: i32) -> i32 {
    let voc_base = vocation_base_speed(vocation_id);
    let l = level.max(1);
    match model {
        StepSpeedModel::LinearGo => voc_base + if l > 1 { l } else { 0 },
        StepSpeedModel::TfsLog => (voc_base + 2 * (l - 1)).clamp(10, 1500),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::StepSpeedModel;

    #[test]
    fn base_walk_speed_matches_gameserver_player_update() {
        // voc 220, level 8 → base 228, GetSpeed 536
        assert_eq!(base_walk_speed(StepSpeedModel::LinearGo, 1, 8), 228);
        assert_eq!(
            crate::formulas::linear_go_effective_speed(base_walk_speed(StepSpeedModel::LinearGo, 1, 8)),
            536
        );
        // TFS 1098: 220 + 2*7 = 234
        assert_eq!(base_walk_speed(StepSpeedModel::TfsLog, 1, 8), 234);
    }
}

/// Recompute max health / mana / cap for current level (called on level-up).
pub fn recalculate_vitals(vocation_id: i32, level: i32) -> (i32, i32, i32) {
    let (hp_gain, mana_gain, cap_gain) = per_level_gains(vocation_id);
    let l = level.max(1);
    let max_health = 150 + hp_gain * (l - 1);
    let max_mana = mana_gain * (l - 1);
    let cap = 400 + cap_gain * (l - 1);
    (max_health, max_mana, cap)
}
