//! Corpus spell outcomes the data pack does not express on its own.
//!
//! - Healing spells clear paralyze — `THealingImpact::handleCreature` `magic.cc:202-205`,
//!   spell `Heal` `magic.cc:2113-2115`. Life fluid does not: `DrinkPotion` shares that
//!   helper (`magic.cc:4333`), and the clear there is a reconstruction of the spell impact.
//! - Berserk — `magic.cc:3557-3561`: one `ComputeDamage(80, 20)`, then `level * scaled / 25`.
//!   Mana is `level * 4`.
//! - Fluids — `DrinkPotion` `magic.cc:4327-4334`: mana `100 + random(-50, 50)`,
//!   life `50 + random(-25, 25)` hit points only.

use crate::formulas::{FormulaHooks, MechanicsProfile};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use tfs_rust_common::enums::ConditionType;

use super::spell_damage;

impl GameWorld {
    /// Drop paralyze after a healing spell lands on a living creature.
    ///
    /// `health_before` is `SKILL_HITPOINTS` before the change (`magic.cc:198`).
    /// Life fluid and `addHealth` must not call this.
    pub(crate) fn clear_paralyze_after_heal_spell(
        &mut self,
        target: CreatureId,
        health_before: i32,
    ) {
        if health_before <= 0 {
            return;
        }
        let removed = self.creatures.get_mut(target).is_some_and(|kind| {
            let before = kind.base().active_conditions.len();
            kind.base_mut()
                .active_conditions
                .retain(|c| c.ctype != ConditionType::Paralyze);
            before != kind.base().active_conditions.len()
        });
        if removed {
            self.on_condition_ended(target, ConditionType::Paralyze);
        }
    }

    /// `DrinkPotion` (`magic.cc:4327-4334`). One glibc roll. Life does not clear paralyze.
    pub(crate) fn drink_potion(&mut self, creature_u64: u64, life: bool) -> Result<(), String> {
        let amount = if life {
            50 + self.parity_random(-25, 25)
        } else {
            100 + self.parity_random(-50, 50)
        };
        if life {
            self.lua_script_player_add_health(creature_u64, amount)
        } else {
            self.lua_script_player_add_mana(creature_u64, amount)
        }
    }
}

/// Berserk damage after the variation roll (`magic.cc:3559`).
///
/// `rolled` is `80 + random(-20, 20)`. The multiplier is `ComputeDamage` with the
/// spell's `& 4` cap (`clamp_max_100`). Then `(level * scaled) / 25`.
pub(crate) fn berserk_hit(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    level: i32,
    magic_level: i32,
    rolled: i32,
    clamp_max_100: bool,
    clamp_min_100: bool,
) -> i32 {
    let scaled = spell_damage(
        profile,
        hooks,
        level,
        magic_level,
        rolled,
        clamp_max_100,
        clamp_min_100,
    );
    (level * scaled) / 25
}

/// Spoken mana. Berserk is `level * 4` (`magic.cc:3560`), not `manaPercent`.
pub(crate) fn spoken_mana_cost(
    words: &str,
    level: i32,
    max_mana: i32,
    flat: u32,
    percent: u32,
) -> u32 {
    if words.eq_ignore_ascii_case("ex,ori") {
        return u32::try_from(level.max(0))
            .unwrap_or(u32::MAX)
            .saturating_mul(4);
    }
    if percent > 0 {
        let max = u32::try_from(max_mana.max(0)).unwrap_or(0);
        max.saturating_mul(percent) / 100
    } else {
        flat
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::{ActiveCondition, ConditionData};
    use crate::test_support::{
        ensure_walkable_tile_if_absent, insert_player, minimal_world, sim_hero_player,
    };
    use slotmap::Key;
    use tfs_rust_common::Position;
    use tfs_rust_common::enums::ConditionType;
    use tfs_rust_lua::CombatExecuteRequest;

    fn paralyze() -> ActiveCondition {
        ActiveCondition::new(
            1,
            0,
            ConditionType::Paralyze,
            ConditionData::Speed { flat_delta: -100 },
            Some(10),
        )
    }

    fn place_player(world: &mut GameWorld, name: &str) -> (crate::ids::CreatureId, u64) {
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player(name, pos);
        player.base.health = 80;
        player.base.max_health = 400;
        player.mana = 0;
        player.max_mana = 200;
        player.base.active_conditions.push(paralyze());
        let cid = insert_player(world, player);
        ensure_walkable_tile_if_absent(&mut world.map, pos);
        world.map.register_creature_at(pos, cid);
        (cid, cid.data().as_ffi())
    }

    #[test]
    fn heal_spell_clears_paralyze_without_dispel() {
        let mut world = minimal_world();
        let (cid, ffi) = place_player(&mut world, "Healer");
        let pos = Position::new(100, 100, 7);
        let req = CombatExecuteRequest {
            caster_id: ffi,
            center_x: pos.x,
            center_y: pos.y,
            center_z: pos.z,
            caster_x: pos.x,
            caster_y: pos.y,
            caster_z: pos.z,
            combat_type: 128,
            effect: 0,
            aggressive: false,
            block_armor: false,
            block_shield: false,
            area_offsets: vec![(0, 0)],
            damage_min: 30,
            damage_max: 30,
            conditions: vec![],
            dispel_type: None,
            create_item: 0,
            no_damage: false,
            distance_effect: 0,
            target_caster_or_topmost: false,
        };
        world.combat_execute_from_lua(&req).expect("heal");
        let kind = world.creatures.get(cid).unwrap();
        assert_eq!(kind.base().health, 110);
        assert!(
            !kind
                .base()
                .active_conditions
                .iter()
                .any(|c| c.ctype == ConditionType::Paralyze),
            "a healing spell clears paralyze without COMBAT_PARAM_DISPEL"
        );
    }

    #[test]
    fn life_fluid_leaves_paralyze() {
        let mut world = minimal_world();
        world.seed_parity_rng(3);
        let expected = 50 + {
            let probe = minimal_world();
            let mut probe = probe;
            probe.seed_parity_rng(3);
            probe.parity_random(-25, 25)
        };
        let (cid, ffi) = place_player(&mut world, "Drinker");
        world.drink_potion(ffi, true).expect("life");
        let kind = world.creatures.get(cid).unwrap();
        assert_eq!(kind.base().health, 80 + expected);
        assert!(
            kind.base()
                .active_conditions
                .iter()
                .any(|c| c.ctype == ConditionType::Paralyze),
            "life fluid does not clear paralyze"
        );
    }

    #[test]
    fn mana_fluid_uses_glibc_roll() {
        let mut world = minimal_world();
        world.seed_parity_rng(9);
        let mut probe = minimal_world();
        probe.seed_parity_rng(9);
        let expected = 100 + probe.parity_random(-50, 50);
        let (cid, ffi) = place_player(&mut world, "Mage");
        world.drink_potion(ffi, false).expect("mana");
        let mana = match world.creatures.get(cid).unwrap() {
            crate::creature::CreatureKind::Player(p) => p.mana,
            _ => panic!("player"),
        };
        assert_eq!(mana, expected);
        assert!((50..=150).contains(&mana));
    }

    #[test]
    fn berserk_scales_the_roll_then_divides_by_level() {
        let world = minimal_world();
        let profile = &world.mechanics.profile;
        let hooks = &world.mechanics.hooks;
        // level 20, ml 5, roll 60: mult 55, scaled 33, hit 20*33/25 = 26.
        assert_eq!(berserk_hit(profile, hooks, 20, 5, 60, true, false), 26);
        // Cap at 100: level 50, ml 10, roll 80 → 50*80/25 = 160.
        assert_eq!(berserk_hit(profile, hooks, 50, 10, 80, true, false), 160);
    }

    #[test]
    fn berserk_mana_is_four_times_level() {
        assert_eq!(spoken_mana_cost("ex,ori", 35, 500, 0, 80), 140);
        assert_eq!(spoken_mana_cost("ex,ura", 35, 500, 0, 0), 0);
        assert_eq!(spoken_mana_cost("ex,evo, vis, lux", 20, 400, 0, 0), 0);
        assert_eq!(spoken_mana_cost("ex,ura", 8, 100, 20, 0), 20);
    }

    #[test]
    fn poison_storm_uses_the_full_disc() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/scripts/spells/attack/poison_storm.lua");
        let src = std::fs::read_to_string(&path).expect("poison_storm.lua");
        assert!(
            src.contains("AREA_CIRCLE6X6"),
            "radius 8 is the 101-tile disc"
        );
        assert!(
            !src.contains("AREA_CIRCLE5X5"),
            "radius 6 is ultimate explosion, not poison storm"
        );
        assert!(
            src.contains("combat:execute"),
            "the disc must show EFFECT_POISON"
        );
        assert!(
            src.contains("target:getId() ~= casterId"),
            "the caster is not poisoned"
        );
        assert_eq!(super::super::disc_tile_count(7), 101);
    }
}
