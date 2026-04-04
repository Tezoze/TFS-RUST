//! Level and vocation stat progression (TFS `Player::getReqExperience`, vocation gains).
// C++ reference: `player.cpp`, `vocation.cpp`.

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

/// Recompute max health / mana / cap for current level (called on level-up).
pub fn recalculate_vitals(vocation_id: i32, level: i32) -> (i32, i32, i32) {
    let (hp_gain, mana_gain, cap_gain) = per_level_gains(vocation_id);
    let l = level.max(1);
    let max_health = 150 + hp_gain * (l - 1);
    let max_mana = mana_gain * (l - 1);
    let cap = 400 + cap_gain * (l - 1);
    (max_health, max_mana, cap)
}
