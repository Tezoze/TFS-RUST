//! Instant / rune spell gating (mana, soul, level, vocation, cooldowns).
// C++ reference: `spells.cpp` `Spell::playerSpellCheck`, `playerInstantSpellCheck`.

use std::collections::HashMap;
use std::time::Instant;

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
    /// TFS `Player::canDoAction` — walking step `nextAction` lockout (`player.cpp` `onWalk`).
    NextAction,
}

/// Returns `Ok` if the player may cast this spell at `now_tick` (ignores path / line of sight).
pub fn can_cast_instant(
    player: &Player,
    spell: &SpellDefinition,
    now: Instant,
    now_tick: u64,
) -> Result<(), SpellFailReason> {
    if !player.timed_action_ready(now) {
        return Err(SpellFailReason::NextAction);
    }
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

/// Profile-driven spell damage (B4.7) — delegates to [`crate::combat::math::spell_damage`].
///
/// `base` is the spell's pre-scaling damage; `clamp_max_100` / `clamp_min_100` come from the spell's
/// flag bits (CipSoft `Flags & 4` / `& 8`, `magic.cc:786`). Returns the scaled damage for the active
/// era (`2*level + 3*magicLevel` % multiplier by default; Tier-2 `getSpellDamage` overrides).
#[allow(clippy::too_many_arguments)]
pub fn spell_damage_scaled(
    profile: &crate::formulas::MechanicsProfile,
    hooks: &crate::formulas::FormulaHooks,
    level: i32,
    magic_level: i32,
    base: i32,
    clamp_max_100: bool,
    clamp_min_100: bool,
) -> i32 {
    crate::combat::math::spell_damage(
        profile,
        hooks,
        level,
        magic_level,
        base,
        clamp_max_100,
        clamp_min_100,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::time::{Duration, Instant};

    use crate::creature::{CreatureBase, Outfit};
    use crate::creature::Player;
    use crate::creature::PlayerEconomy;
    use crate::creature::PlayerInventory;
    use crate::creature::PlayerSkills;
    use crate::creature::PlayerSocial;
    use tfs_rust_common::enums::{Direction, SkullType};
    use tfs_rust_common::Position;

    fn minimal_player(next_action_until: Option<Instant>) -> Player {
        Player {
            base: CreatureBase {
                name: "t".into(),
                position: Position::new(0, 0, 7),
                direction: Direction::North,
                health: 100,
                max_health: 100,
                outfit: Outfit::default(),
                speed: 220,
                base_speed: 220,
                skull: SkullType::None,
                drunkenness: 0,
                active_conditions: Vec::new(),
                walk_queue: VecDeque::new(),
                last_step: None,
                last_step_cost: 1,
                last_step_ground_speed: 150,
                next_walk_check: None,
                walk_timer: Default::default(),
                cancel_next_walk: false,
                force_update_follow_path: false,
                walk_update_ticks: 0,
                is_updating_path: false,
                has_follow_path: false,
                movement_blocked: false,
                stairhop_blocked_until: None,
                follow_target: None,
                attack_target: None,
                master: None,
                damage_map: Default::default(),
                think_check_bucket: None,
            },
            account_id: 1,
            guid: 1,
            group_id: 1,
            vocation_id: 1,
            level: 50,
            experience: 0,
            mana: 100,
            max_mana: 100,
            capacity: 100,
            inventory: PlayerInventory::default(),
            skills: PlayerSkills {
                fist: 10,
                club: 10,
                sword: 10,
                axe: 10,
                dist: 10,
                shielding: 10,
                fishing: 10,
                maglevel: 10,
            },
            economy: PlayerEconomy { balance: 0, soul: 100 },
            social: PlayerSocial::default(),
            town_id: 1,
            premium_ends_at: 0,
            stamina_minutes: 0,
            offline_training_ms: 0,
            spell_cooldown_end: HashMap::new(),
            spell_group_cooldown_end: HashMap::new(),
            operating_system: 0,
            otclient_v8: 0,
            ghost_mode: false,
            equipment_slots: std::array::from_fn(|_| None),
            inventory_weight: 0,
            items_light: crate::creature::LightInfo::default(),
            inventory_abilities: [false; 11],
            shop_owner: None,
            vip_list: Vec::new(),
            health_hidden: false,
            last_activity: Instant::now(),
            last_ping_sent: Instant::now(),
            last_pong_at: Instant::now(),
            next_action_until,
            walk_action: None,
            walk_action_due: None,
            depot_chests: HashMap::new(),
            depot_lockers: HashMap::new(),
            inbox_root: None,
            last_depot_id: -1,
            persist: None,
        }
    }

    #[test]
    fn can_cast_instant_blocks_while_next_action_in_future() {
        let spell = SpellDefinition {
            id: 1,
            level: 1,
            mana: 0,
            soul: 0,
            cooldown_ticks: 0,
            group_id: 0,
            group_cooldown_ticks: 0,
            vocation_mask: 0xFFFF_FFFF,
        };
        let now = Instant::now();
        let p = minimal_player(Some(now + Duration::from_secs(60)));
        assert_eq!(
            can_cast_instant(&p, &spell, now, 0),
            Err(SpellFailReason::NextAction)
        );
        let p2 = minimal_player(Some(now - Duration::from_millis(1)));
        assert!(can_cast_instant(&p2, &spell, now, 0).is_ok());
        let p3 = minimal_player(None);
        assert!(can_cast_instant(&p3, &spell, now, 0).is_ok());
    }
}
