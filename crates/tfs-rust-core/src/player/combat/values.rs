//! PC-1 — Player attack/defend/armor value resolution.
//!
//! Returns **raw unscaled** values — the item/skill number before fight-mode scaling and probe
//! rolls, matching `GetAttackValue`/`GetDefendValue`/`GetArmorStrength` in the C++ reference
//! (which return `WEAPONATTACKVALUE`/`SHIELDDEFENDVALUE`/`ARMORVALUE` before `GetAttackDamage`/
//! `GetDefendDamage`/`Damage(PHYSICAL)` apply mode + probe). The downstream consumers that apply
//! era-tunable multipliers already exist in `combat::math` and read from `MechanicsProfile` /
//! `data/formulas/772.lua` (`apply_attack_mode`, `apply_defense_mode`, `probe_value`,
//! `armor_reduction`). PC-1 adds no new `772.lua` keys and hardcodes no fight-mode multipliers or
//! probe constants.
//!
//! C++ reference (mechanics, `tibia-game-master/src/crcombat.cc`):
//! - `TCombat::GetAttackValue` — `:164-189`.
//! - `TCombat::GetDefendValue` — `:191-218`.
//! - `TCombat::GetArmorStrength` — `:286-307`.
//! - `WeaponTypeToSkill` — `:150-162`.
//! - `TCombat::GetWeapon` (hand-slot scan) — `:36-102`.
//! - `TCombat::GetAmmo` (ammo resolution) — `:104-126`.
//!
//! C++ reference (race data): `runtime/mon/human.mon` `Attack=7`, `Defend=5`, `Armor=0`.

use crate::creature::{CreatureKind, PlayerSkills};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::inventory::{
    slot_type_for_item_type, InventorySlot, PLAYER_INVENTORY_SLOT_FIRST,
    PLAYER_INVENTORY_SLOT_LAST, WEAPON_AMMO, WEAPON_AXE, WEAPON_CLUB, WEAPON_DISTANCE,
    WEAPON_SHIELD, WEAPON_SWORD,
};

/// Skill index used by `ProbeValue` — C++ `SKILL_*` (`enums.hh:555-566`).
///
/// Maps 1:1 to `WeaponTypeToSkill` (`crcombat.cc:150-162`): `WEAPON_NONE`→`Fist`,
/// `WEAPON_SWORD`→`Sword`, `WEAPON_CLUB`→`Club`, `WEAPON_AXE`→`Axe`,
/// `WEAPON_SHIELD`→`Shielding`, `WEAPON_AMMO`/`WEAPON_THROW`→`Distance`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillNr {
    Fist,
    Club,
    Sword,
    Axe,
    Distance,
    Shielding,
    /// Not used by weapon resolution; used by skill-tries / death loss (PC-5).
    Fishing,
}

impl SkillNr {
    /// C++ `WeaponTypeToSkill` — `crcombat.cc:150-162`.
    pub fn from_weapon_type(weapon_type: u8) -> Self {
        match weapon_type {
            WEAPON_SWORD => SkillNr::Sword,
            WEAPON_CLUB => SkillNr::Club,
            WEAPON_AXE => SkillNr::Axe,
            WEAPON_SHIELD => SkillNr::Shielding,
            // `WEAPON_AMMO` falls through to `Distance` (`crcombat.cc:158-159`).
            WEAPON_DISTANCE | WEAPON_AMMO => SkillNr::Distance,
            // `WEAPON_NONE`, `WEAPON_WAND` → `SKILL_FIST` (`crcombat.cc:153,179-181`).
            _ => SkillNr::Fist,
        }
    }

    /// Resolve to the player's current skill level — C++ `Master->Skills[SkillNr]`.
    pub fn level(self, skills: &PlayerSkills) -> i32 {
        match self {
            SkillNr::Fist => skills.fist,
            SkillNr::Club => skills.club,
            SkillNr::Sword => skills.sword,
            SkillNr::Axe => skills.axe,
            SkillNr::Distance => skills.dist,
            SkillNr::Shielding => skills.shielding,
            SkillNr::Fishing => skills.fishing,
        }
    }

    /// 772 `getSkillName` — `tools.cpp:764-796` (used in "You advanced in …").
    pub fn display_name(self) -> &'static str {
        match self {
            SkillNr::Fist => "fist fighting",
            SkillNr::Club => "club fighting",
            SkillNr::Sword => "sword fighting",
            SkillNr::Axe => "axe fighting",
            SkillNr::Distance => "distance fighting",
            SkillNr::Shielding => "shielding",
            SkillNr::Fishing => "fishing",
        }
    }

    /// All combat skills that take death try-loss (fist..fishing).
    pub const COMBAT_ALL: [SkillNr; 7] = [
        SkillNr::Fist,
        SkillNr::Club,
        SkillNr::Sword,
        SkillNr::Axe,
        SkillNr::Distance,
        SkillNr::Shielding,
        SkillNr::Fishing,
    ];
}

/// Per-hand-slot weapon categorization — mirrors C++ `GetWeapon` flag checks (`crcombat.cc:78-100`).
#[derive(Debug, Clone, Copy)]
pub(crate) enum HandWeapon {
    /// `SHIELD` flag → `this->Shield`.
    Shield,
    /// `WEAPON` flag → `this->Close`.
    Close,
    /// `BOW` flag → `this->Missile` (distance weapon requiring ammo).
    Missile,
    /// `THROW` flag → `this->Throw` (distance weapon without ammo).
    Throw,
    /// `WAND` flag → `this->Wand`.
    Wand,
}

pub(crate) fn classify_weapon(weapon_type: u8, ammo_type: u8) -> Option<HandWeapon> {
    match weapon_type {
        WEAPON_SHIELD => Some(HandWeapon::Shield),
        WEAPON_SWORD | WEAPON_CLUB | WEAPON_AXE => Some(HandWeapon::Close),
        WEAPON_DISTANCE => {
            if ammo_type != 0 {
                Some(HandWeapon::Missile)
            } else {
                Some(HandWeapon::Throw)
            }
        }
        // `WEAPON_WAND` — wands aren't wired in `ItemType` yet (§0.7); classified here for parity.
        _ if weapon_type == crate::inventory::WEAPON_WAND => Some(HandWeapon::Wand),
        _ => None,
    }
}

impl GameWorld {
    /// 772 `TCombat::GetAttackValue` — `crcombat.cc:164-189`.
    ///
    /// Priority: `Close` (melee `WEAPONATTACKVALUE`) → `Missile` (ammo `AMMOATTACKVALUE`) →
    /// `Throw` (`THROWATTACKVALUE`) → `Wand` (attack 0) → fist (`RaceData[Race].Attack`).
    /// Returns `(max_value, SkillNr)` — the raw unscaled attack value and its skill index.
    pub fn player_get_attack_value(&self, cid: CreatureId) -> (i32, SkillNr) {
        let sim_melee_attack = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.sim_melee_attack,
            _ => return (0, SkillNr::Fist),
        };

        // `player_get_weapon(cid, false)` resolves ammo for bows (returns the ammo item) and
        // returns the weapon itself for melee/throw. `None` = no weapon → fist fallback.
        let Some(weapon_iid) = self.player_get_weapon(cid, false) else {
            // Fist — `RaceData[Race].Attack` (`crcombat.cc:183`).
            return (sim_melee_attack, SkillNr::Fist);
        };
        let Some(item) = self.items.get(weapon_iid) else {
            return (sim_melee_attack, SkillNr::Fist);
        };
        let Some(it) = self.items_db.items.get(&item.item_type) else {
            return (sim_melee_attack, SkillNr::Fist);
        };

        // For bow+ammo, `player_get_weapon` returned the ammo item (`WEAPON_AMMO`); its
        // `weapon_type` maps to `SkillNr::Distance` via `from_weapon_type` (`crcombat.cc:158`).
        // For wand, attack is 0 (`crcombat.cc:180`); `WEAPON_WAND` → `SkillNr::Fist`.
        let attack = it.attack;
        let skill = SkillNr::from_weapon_type(it.weapon_type);
        (attack, skill)
    }

    /// 772 `TCombat::GetDefendValue` — `crcombat.cc:191-218`.
    ///
    /// Priority: `Shield` (`SHIELDDEFENDVALUE`) → `Close` (`WEAPONDEFENDVALUE`) → `Throw`
    /// (`THROWDEFENDVALUE`) → `Missile` (0, bow reduces defense) → fist (`RaceData[Race].Defend`).
    /// Returns `(max_value, SkillNr)` — the raw unscaled defense value and its skill index.
    pub fn player_get_defend_value(&self, cid: CreatureId) -> (i32, SkillNr) {
        let sim_melee_defense = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.sim_melee_defense,
            _ => return (0, SkillNr::Fist),
        };

        // Scan hand slots (Left=6, Right=5) — C++ `GetWeapon` scans `INVENTORY_HAND_FIRST..LAST`.
        // Collect Copy values to avoid holding `&ItemType` across slot iterations.
        let mut shield_def: Option<i32> = None;
        let mut close_def: Option<(i32, SkillNr)> = None;
        let mut throw_def: Option<i32> = None;
        let mut has_missile = false;

        for slot in [InventorySlot::Left as u8, InventorySlot::Right as u8] {
            let Some(iid) = self.get_player_inventory_item(cid, slot) else {
                continue;
            };
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                continue;
            };
            match classify_weapon(it.weapon_type, it.ammo_type) {
                Some(HandWeapon::Shield) => shield_def = Some(it.defense),
                Some(HandWeapon::Close) => {
                    close_def = Some((it.defense, SkillNr::from_weapon_type(it.weapon_type)));
                }
                Some(HandWeapon::Throw) => throw_def = Some(it.defense),
                Some(HandWeapon::Missile) => has_missile = true,
                Some(HandWeapon::Wand) | None => {}
            }
        }

        // `GetDefendValue` priority: Shield > Close > Throw > Missile(0) > fist.
        if let Some(def) = shield_def {
            return (def, SkillNr::Shielding);
        }
        if let Some((def, skill)) = close_def {
            return (def, skill);
        }
        if let Some(def) = throw_def {
            return (def, SkillNr::Distance);
        }
        if has_missile {
            // Bow reduces defense to 0 (`crcombat.cc:207-210`).
            return (0, SkillNr::Distance);
        }

        // Fist — `RaceData[Race].Defend` (`crcombat.cc:212`).
        (sim_melee_defense, SkillNr::Fist)
    }

    /// 772 `TCombat::GetArmorStrength` — `crcombat.cc:286-307`.
    ///
    /// Sums `ARMORVALUE` of equipped CLOTHES+ARMOR at correct `BODYPOSITION`, adds
    /// `RaceData[Race].Armor`, and returns the **raw** sum (the `(A/2)+rand%(A/2)` randomization
    /// is applied by `combat::math::armor_reduction`, the downstream consumer).
    pub fn player_get_armor_strength(&self, cid: CreatureId) -> i32 {
        match self.creatures.get(cid) {
            Some(CreatureKind::Player(_)) => (),
            _ => return 0,
        };

        let mut armor = 0i32;
        for slot in PLAYER_INVENTORY_SLOT_FIRST..=PLAYER_INVENTORY_SLOT_LAST {
            let Some(iid) = self.get_player_inventory_item(cid, slot) else {
                continue;
            };
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                continue;
            };
            // C++ checks `CLOTHES` + `ARMOR` flags + `BODYPOSITION == Position` (`crcombat.cc:295-297`).
            // Rust: item has an armor value and is at its designated body slot.
            if it.armor > 0 && slot_type_for_item_type(it) == slot {
                armor += it.armor;
            }
        }

        // `RaceData[human].Armor = 0` (`human.mon:15`).
        armor
    }

    /// 772 `TCombat::GetDistance` — `crcombat.cc:611`. Returns the weapon's `Range` value:
    /// `1` for melee (close weapons, fist), `2`/`3` for ranged (bow `shoot_range`, wand
    /// `WANDRANGE=3`, throw `THROWRANGE`). The caller (`player_execute_attack`) uses this to
    /// dispatch to `CloseAttack` (range 1) or `DistanceAttack`/`WandAttack` (range 2/3).
    ///
    /// Bow `shoot_range` comes from `items.xml` (`range` attribute → `ItemType.shoot_range`).
    /// Wand range is hardcoded to 3 (`WANDRANGE`, `crcombat.cc:706`) — `WandDef` doesn't carry
    /// range. Throwing weapon range comes from `items.xml` (`range` → `ItemType.shoot_range`).
    pub fn player_weapon_range(&self, cid: CreatureId) -> i32 {
        // Scan hand slots for a distance weapon or wand — `crcombat.cc:632-638` classification.
        for slot in [InventorySlot::Left as u8, InventorySlot::Right as u8] {
            let Some(iid) = self.get_player_inventory_item(cid, slot) else {
                continue;
            };
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                continue;
            };
            match it.weapon_type {
                WEAPON_DISTANCE => return it.shoot_range.max(1),
                // `WEAPON_WAND` — `WANDRANGE = 3` (`crcombat.cc:706`).
                w if w == crate::inventory::WEAPON_WAND => return 3,
                _ => {}
            }
        }
        // Melee or fist — range 1 (`crcombat.cc:612`).
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::CreatureKind;
    use crate::item::Item;
    use crate::sim_harness::{minimal_world, sim_hero_player};
    use tfs_rust_common::Position;
    use tfs_rust_content::otb::ItemType;

    /// Insert an item into the world + items_db, equip it in `slot`.
    fn equip_item(world: &mut GameWorld, cid: CreatureId, slot: u8, item_type: u16, it: ItemType) {
        // Register the ItemType if not already present.
        if !world.items_db.items.contains_key(&item_type) {
            let mut items = std::collections::HashMap::clone(&world.items_db.items);
            items.insert(item_type, it);
            // Rebuild the Arc — tests own the only reference.
            let client_to_server =
                std::collections::HashMap::clone(&world.items_db.client_to_server);
            world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
                items,
                client_to_server,
            });
        }
        let iid = world.items.insert(Item::new_single(item_type));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            let idx = crate::inventory::slot_to_array_index(slot).unwrap();
            p.equipment_slots[idx] = Some(iid);
        }
    }

    fn make_weapon(server_id: u16, weapon_type: u8, attack: i32, defense: i32) -> ItemType {
        ItemType {
            server_id,
            weapon_type,
            attack,
            defense,
            ..Default::default()
        }
    }

    fn make_shield(server_id: u16, defense: i32) -> ItemType {
        ItemType {
            server_id,
            weapon_type: WEAPON_SHIELD,
            defense,
            ..Default::default()
        }
    }

    fn make_armor_piece(server_id: u16, slot_position: u32, armor: i32) -> ItemType {
        ItemType {
            server_id,
            slot_position,
            armor,
            ..Default::default()
        }
    }

    fn make_bow(server_id: u16, ammo_type: u8) -> ItemType {
        ItemType {
            server_id,
            weapon_type: WEAPON_DISTANCE,
            ammo_type,
            attack: 0, // bows themselves have 0 attack; ammo provides it
            ..Default::default()
        }
    }

    fn make_ammo(server_id: u16, ammo_type: u8, attack: i32) -> ItemType {
        ItemType {
            server_id,
            weapon_type: WEAPON_AMMO,
            ammo_type,
            attack,
            ..Default::default()
        }
    }

    #[test]
    fn attack_value_fist_fallback() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        // No weapon equipped → fist fallback: RaceData.Attack=7, SKILL_FIST.
        let (value, skill) = world.player_get_attack_value(cid);
        assert_eq!(value, 7);
        assert_eq!(skill, SkillNr::Fist);
    }

    #[test]
    fn attack_value_melee_sword() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2377,
            make_weapon(2377, WEAPON_SWORD, 15, 8),
        );
        let (value, skill) = world.player_get_attack_value(cid);
        assert_eq!(value, 15);
        assert_eq!(skill, SkillNr::Sword);
    }

    #[test]
    fn attack_value_bow_uses_ammo() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        // Bow in Left slot, arrows in Ammo slot.
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2456,
            make_bow(2456, 2),
        );
        equip_item(
            &mut world,
            cid,
            InventorySlot::Ammo as u8,
            2544,
            make_ammo(2544, 2, 30),
        );
        let (value, skill) = world.player_get_attack_value(cid);
        assert_eq!(value, 30); // ammo's attack value
        assert_eq!(skill, SkillNr::Distance);
    }

    #[test]
    fn attack_value_throw_weapon() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        // Throw weapon (distance, no ammo_type) in Left slot.
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2389,
            make_weapon(2389, WEAPON_DISTANCE, 25, 0),
        );
        let (value, skill) = world.player_get_attack_value(cid);
        assert_eq!(value, 25);
        assert_eq!(skill, SkillNr::Distance);
    }

    #[test]
    fn defend_value_shield_priority() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        // Sword in Left, shield in Right.
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2377,
            make_weapon(2377, WEAPON_SWORD, 15, 8),
        );
        equip_item(
            &mut world,
            cid,
            InventorySlot::Right as u8,
            2513,
            make_shield(2513, 22),
        );
        let (value, skill) = world.player_get_defend_value(cid);
        assert_eq!(value, 22); // shield defense takes priority
        assert_eq!(skill, SkillNr::Shielding);
    }

    #[test]
    fn defend_value_weapon_without_shield() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2377,
            make_weapon(2377, WEAPON_SWORD, 15, 8),
        );
        let (value, skill) = world.player_get_defend_value(cid);
        assert_eq!(value, 8); // weapon defense
        assert_eq!(skill, SkillNr::Sword);
    }

    #[test]
    fn defend_value_bow_zero_defense() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2456,
            make_bow(2456, 2),
        );
        let (value, skill) = world.player_get_defend_value(cid);
        assert_eq!(value, 0); // bow reduces defense to 0
        assert_eq!(skill, SkillNr::Distance);
    }

    #[test]
    fn defend_value_fist_fallback() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        let (value, skill) = world.player_get_defend_value(cid);
        assert_eq!(value, 5); // RaceData.Defend=5
        assert_eq!(skill, SkillNr::Fist);
    }

    #[test]
    fn armor_strength_sum_equipped_pieces() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        use crate::inventory::{SLOTP_ARMOR, SLOTP_HEAD, SLOTP_LEGS};
        equip_item(
            &mut world,
            cid,
            InventorySlot::Head as u8,
            2457,
            make_armor_piece(2457, SLOTP_HEAD, 5),
        );
        equip_item(
            &mut world,
            cid,
            InventorySlot::Armor as u8,
            2463,
            make_armor_piece(2463, SLOTP_ARMOR, 10),
        );
        equip_item(
            &mut world,
            cid,
            InventorySlot::Legs as u8,
            2647,
            make_armor_piece(2647, SLOTP_LEGS, 3),
        );
        let armor = world.player_get_armor_strength(cid);
        assert_eq!(armor, 18); // 5 + 10 + 3, raw sum (no randomization)
    }

    #[test]
    fn armor_strength_empty_equipment() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        let armor = world.player_get_armor_strength(cid);
        assert_eq!(armor, 0); // RaceData.Armor=0 for human
    }

    #[test]
    fn skill_nr_from_weapon_type() {
        use crate::inventory::WEAPON_NONE;
        assert_eq!(SkillNr::from_weapon_type(WEAPON_SWORD), SkillNr::Sword);
        assert_eq!(SkillNr::from_weapon_type(WEAPON_CLUB), SkillNr::Club);
        assert_eq!(SkillNr::from_weapon_type(WEAPON_AXE), SkillNr::Axe);
        assert_eq!(SkillNr::from_weapon_type(WEAPON_SHIELD), SkillNr::Shielding);
        assert_eq!(
            SkillNr::from_weapon_type(WEAPON_DISTANCE),
            SkillNr::Distance
        );
        assert_eq!(SkillNr::from_weapon_type(WEAPON_AMMO), SkillNr::Distance);
        assert_eq!(SkillNr::from_weapon_type(WEAPON_NONE), SkillNr::Fist);
    }

    #[test]
    fn skill_nr_level_resolution() {
        let skills = PlayerSkills::with_levels(10, 15, 20, 25, 30, 35, 40, 5);
        assert_eq!(SkillNr::Fist.level(&skills), 10);
        assert_eq!(SkillNr::Club.level(&skills), 15);
        assert_eq!(SkillNr::Sword.level(&skills), 20);
        assert_eq!(SkillNr::Axe.level(&skills), 25);
        assert_eq!(SkillNr::Distance.level(&skills), 30);
        assert_eq!(SkillNr::Shielding.level(&skills), 35);
    }

    #[test]
    fn attack_value_non_player_returns_zero() {
        let world = minimal_world();
        // No creature at all — should return (0, Fist) gracefully.
        let fake_id = CreatureId::from(slotmap::KeyData::default());
        let (value, skill) = world.player_get_attack_value(fake_id);
        assert_eq!(value, 0);
        assert_eq!(skill, SkillNr::Fist);
    }
}
