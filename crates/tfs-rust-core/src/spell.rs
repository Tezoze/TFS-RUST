//! Instant / rune spell gating (mana, soul, level, vocation, cooldowns).
// C++ reference: `spells.cpp` `Spell::playerSpellCheck`, `playerInstantSpellCheck`.

use std::collections::HashMap;

use crate::creature::Player;
use crate::matrix_area::MatrixArea;

#[derive(Debug, Clone)]
pub struct SpellDefinition {
    pub id: u16,
    pub level: u16,
    pub mana: u16,
    pub soul: u8,
    /// Game ticks before the same spell can be cast again.
    pub cooldown_ticks: u64,
    pub group_id: u8,
    /// Ticks before any spell in `group_id` can be cast again.
    pub group_cooldown_ticks: u64,
    /// Bit `1 << vocation_id` allowed (simplified vs TFS vocation mask).
    pub vocation_mask: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellFailReason {
    Level,
    Mana,
    Soul,
    Vocation,
    Cooldown,
    GroupCooldown,
}

/// Returns `Ok` if the player may cast this spell at `now_tick` (ignores path / line of sight).
pub fn can_cast_instant(
    player: &Player,
    spell: &SpellDefinition,
    now_tick: u64,
) -> Result<(), SpellFailReason> {
    if (player.level as u16) < spell.level {
        return Err(SpellFailReason::Level);
    }
    if (player.mana as u16) < spell.mana {
        return Err(SpellFailReason::Mana);
    }
    if (player.economy.soul as u8) < spell.soul {
        return Err(SpellFailReason::Soul);
    }
    let bit = 1u32 << (player.vocation_id.max(0) as u32);
    if spell.vocation_mask & bit == 0 {
        return Err(SpellFailReason::Vocation);
    }
    if player
        .spell_cooldown_end
        .get(&spell.id)
        .is_some_and(|&t| now_tick < t)
    {
        return Err(SpellFailReason::Cooldown);
    }
    if player
        .spell_group_cooldown_end
        .get(&spell.group_id)
        .is_some_and(|&t| now_tick < t)
    {
        return Err(SpellFailReason::GroupCooldown);
    }
    Ok(())
}

/// Record cooldowns after a successful cast at `now_tick`.
pub fn register_cast_cooldowns(
    cooldowns: &mut HashMap<u16, u64>,
    group_cooldowns: &mut HashMap<u8, u64>,
    spell: &SpellDefinition,
    now_tick: u64,
) {
    cooldowns.insert(spell.id, now_tick.saturating_add(spell.cooldown_ticks));
    group_cooldowns.insert(
        spell.group_id,
        now_tick.saturating_add(spell.group_cooldown_ticks),
    );
}

/// Relative `(dx, dy)` offsets from the spell anchor for each set tile in `area` (for map application).
pub fn matrix_tile_offsets(area: &MatrixArea) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for r in 0..area.rows {
        for c in 0..area.cols {
            if area.get(r, c) {
                let dx = c as i32 - area.center_x as i32;
                let dy = r as i32 - area.center_y as i32;
                v.push((dx, dy));
            }
        }
    }
    v
}
