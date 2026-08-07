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

/// 772 `TCombat` weapon fields — `crcombat.cc:19-25`, filled by `GetWeapon` (`:36-102`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CombatWeapons {
    pub shield: Option<crate::ids::ItemId>,
    pub close: Option<crate::ids::ItemId>,
    pub missile: Option<crate::ids::ItemId>,
    pub throw_: Option<crate::ids::ItemId>,
    pub wand: Option<crate::ids::ItemId>,
    pub ammo: Option<crate::ids::ItemId>,
    /// `true` until a WEAPON/BOW/THROW/WAND clears it — SHIELD never clears Fist.
    pub fist: bool,
}

impl Default for CombatWeapons {
    fn default() -> Self {
        Self {
            shield: None,
            close: None,
            missile: None,
            throw_: None,
            wand: None,
            ammo: None,
            fist: true,
        }
    }
}

impl GameWorld {
    /// 772 `TCombat::GetAttackValue` — `crcombat.cc:164-189`.
    ///
    /// Priority: `Close` → `Missile` (ammo attack) → `Throw` → `Wand` (0) → fist race attack.
    /// Returns `(max_value, SkillNr)` — the raw unscaled attack value and its skill index.
    pub fn player_get_attack_value(&self, cid: CreatureId) -> (i32, SkillNr) {
        let sim_melee_attack = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.sim_melee_attack,
            _ => return (0, SkillNr::Fist),
        };

        let weapons = self.player_resolve_combat_weapons(cid);

        if let Some(iid) = weapons.close {
            let Some(item) = self.items.get(iid) else {
                return (sim_melee_attack, SkillNr::Fist);
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                return (sim_melee_attack, SkillNr::Fist);
            };
            return (it.attack, SkillNr::from_weapon_type(it.weapon_type));
        }
        if weapons.missile.is_some() {
            // Bow without ammo: attack 0, still `WEAPON_AMMO` → Distance (`crcombat.cc:171-174`).
            let attack = weapons
                .ammo
                .and_then(|aid| self.items.get(aid))
                .and_then(|i| self.items_db.items.get(&i.item_type))
                .map(|it| it.attack)
                .unwrap_or(0);
            return (attack, SkillNr::Distance);
        }
        if let Some(iid) = weapons.throw_ {
            let Some(item) = self.items.get(iid) else {
                return (sim_melee_attack, SkillNr::Fist);
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                return (sim_melee_attack, SkillNr::Fist);
            };
            return (it.attack, SkillNr::Distance);
        }
        if weapons.wand.is_some() {
            // Wand attack value is 0; skill maps via `WEAPON_NONE` → Fist (`crcombat.cc:180-181`).
            return (0, SkillNr::Fist);
        }
        (sim_melee_attack, SkillNr::Fist)
    }

    /// 772 `TCombat::GetDefendValue` — `crcombat.cc:191-218`.
    ///
    /// Priority: `Shield` → `Close` → `Throw` → `Missile`(0) → fist race defend.
    pub fn player_get_defend_value(&self, cid: CreatureId) -> (i32, SkillNr) {
        let sim_melee_defense = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.sim_melee_defense,
            _ => return (0, SkillNr::Fist),
        };

        let weapons = self.player_resolve_combat_weapons(cid);

        if let Some(iid) = weapons.shield {
            let def = self
                .items
                .get(iid)
                .and_then(|i| self.items_db.items.get(&i.item_type))
                .map(|it| it.defense)
                .unwrap_or(0);
            return (def, SkillNr::Shielding);
        }
        if let Some(iid) = weapons.close {
            let Some(item) = self.items.get(iid) else {
                return (sim_melee_defense, SkillNr::Fist);
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                return (sim_melee_defense, SkillNr::Fist);
            };
            return (it.defense, SkillNr::from_weapon_type(it.weapon_type));
        }
        if let Some(iid) = weapons.throw_ {
            let def = self
                .items
                .get(iid)
                .and_then(|i| self.items_db.items.get(&i.item_type))
                .map(|it| it.defense)
                .unwrap_or(0);
            return (def, SkillNr::Distance);
        }
        if weapons.missile.is_some() {
            return (0, SkillNr::Distance);
        }
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

    /// 772 `TCombat::GetDistance` — `crcombat.cc:309-319`.
    ///
    /// Categorical weapon distance: Close/Fist → 1, Throw → 2, Missile/Wand → 3.
    /// Item `shoot_range` is **not** returned here — use [`Self::player_weapon_max_range`].
    pub fn player_weapon_distance(&self, cid: CreatureId) -> i32 {
        let w = self.player_get_combat_weapons(cid);
        if w.close.is_some() || w.fist {
            1
        } else if w.throw_.is_some() {
            2
        } else if w.missile.is_some() || w.wand.is_some() {
            3
        } else {
            0
        }
    }

    /// Per-weapon max range for in-strike `TARGETOUTOFRANGE` (`BOWRANGE` / `THROWRANGE` /
    /// `WANDRANGE`). Not used at Attack() arm time — that uses categorical distance + viewport.
    pub fn player_weapon_max_range(&self, cid: CreatureId) -> i32 {
        let w = self.player_get_combat_weapons(cid);
        if let Some(iid) = w.missile.or(w.throw_) {
            return self
                .items
                .get(iid)
                .and_then(|i| self.items_db.items.get(&i.item_type))
                .map(|it| it.shoot_range.max(1))
                .unwrap_or(1);
        }
        if w.wand.is_some() {
            return 3;
        }
        1
    }

    /// Deprecated name for categorical distance — prefer [`Self::player_weapon_distance`].
    /// Call sites that meant item range should migrate to [`Self::player_weapon_max_range`].
    pub fn player_weapon_range(&self, cid: CreatureId) -> i32 {
        self.player_weapon_distance(cid)
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

    /// Synthetic Close+Missile in opposite hands → melee dispatch, distance 1 (audit B1/B2).
    #[test]
    fn category_precedence_close_over_missile() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        equip_item(
            &mut world,
            cid,
            InventorySlot::Right as u8,
            2377,
            make_weapon(2377, WEAPON_SWORD, 15, 8),
        );
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2456,
            make_bow(2456, 2),
        );
        let (value, skill) = world.player_get_attack_value(cid);
        assert_eq!(value, 15);
        assert_eq!(skill, SkillNr::Sword);
        assert_eq!(world.player_weapon_distance(cid), 1);
    }

    /// Later hand slot overwrites the same category — Left (6) wins over Right (5) for shields.
    #[test]
    fn later_hand_slot_overwrites_same_category() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        equip_item(
            &mut world,
            cid,
            InventorySlot::Right as u8,
            2510,
            make_shield(2510, 10),
        );
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2511,
            make_shield(2511, 20),
        );
        let w = world.player_get_combat_weapons(cid);
        let left_iid = world
            .get_player_inventory_item(cid, InventorySlot::Left as u8)
            .unwrap();
        assert_eq!(w.shield, Some(left_iid));
        let (def, _) = world.player_get_defend_value(cid);
        assert_eq!(def, 20);
    }

    /// Bow without matching ammo keeps Distance skill (not Fist) — audit B1.
    #[test]
    fn bow_without_ammo_keeps_distance_skill() {
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
        let (value, skill) = world.player_get_attack_value(cid);
        assert_eq!(value, 0);
        assert_eq!(skill, SkillNr::Distance);
        assert_eq!(world.player_weapon_distance(cid), 3);
    }

    /// Shield only → Fist true, distance 1 — audit B3.
    #[test]
    fn shield_only_still_fists() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2510,
            make_shield(2510, 18),
        );
        let w = world.player_get_combat_weapons(cid);
        assert!(w.fist);
        assert!(w.shield.is_some());
        assert!(w.close.is_none());
        assert_eq!(world.player_weapon_distance(cid), 1);
        let (atk, skill) = world.player_get_attack_value(cid);
        assert_eq!(atk, 7);
        assert_eq!(skill, SkillNr::Fist);
    }

    /// Underleveled item-flag weapon skipped in resolution; armor path unaffected — B3.
    #[test]
    fn underleveled_weapon_skipped_in_resolution() {
        let mut world = minimal_world();
        let mut hero = sim_hero_player("Hero", Position::new(100, 100, 7));
        hero.level = 8;
        let cid = world.creatures.insert(CreatureKind::Player(hero));
        let mut axe = make_weapon(2387, WEAPON_AXE, 40, 12);
        axe.min_req_level = 30;
        equip_item(&mut world, cid, InventorySlot::Left as u8, 2387, axe);
        let (value, skill) = world.player_get_attack_value(cid);
        assert_eq!(value, 7);
        assert_eq!(skill, SkillNr::Fist);
        assert!(world.player_get_combat_weapons(cid).fist);
    }

    /// Wrong vocation on item-flag weapon → skipped — B3.
    #[test]
    fn wrong_profession_weapon_skipped() {
        use std::sync::Arc;
        use tfs_rust_content::vocations::{VocationDef, VocationFormula, VocationRegistry};

        let mut world = minimal_world();
        let mut vocations = std::collections::HashMap::new();
        vocations.insert(
            1u16,
            VocationDef {
                id: 1,
                client_id: 3,
                name: "Sorcerer".into(),
                description: "a sorcerer".into(),
                from_vocation: 1,
                gain_cap: 0,
                gain_hp: 0,
                gain_mana: 0,
                gain_hp_ticks: 0,
                gain_hp_amount: 0,
                gain_mana_ticks: 0,
                gain_mana_amount: 0,
                mana_multiplier: 0.0,
                attack_speed_ms: 0,
                base_speed: 0,
                soul_max: 0,
                gain_soul_ticks: 0,
                allow_pvp: false,
                base_hp: 0,
                base_mana: 0,
                base_cap: 0,
                formula: VocationFormula::default(),
                skill_multipliers: [0.0; 7],
            },
        );
        world.vocations = Arc::new(VocationRegistry { vocations });

        let mut hero = sim_hero_player("Hero", Position::new(100, 100, 7));
        hero.vocation_id = 1;
        hero.level = 50;
        let cid = world.creatures.insert(CreatureKind::Player(hero));
        let mut axe = make_weapon(2387, WEAPON_AXE, 40, 12);
        axe.voc_equip_names = vec!["knight".into()];
        equip_item(&mut world, cid, InventorySlot::Left as u8, 2387, axe);
        let (value, skill) = world.player_get_attack_value(cid);
        assert_eq!(value, 7);
        assert_eq!(skill, SkillNr::Fist);
    }

    /// Equipped wand → CombatWeapons.wand set — B1 (strike still uses WandDef separately).
    #[test]
    fn wand_resolution_still_uses_wand_def() {
        let mut world = minimal_world();
        let cid = world.creatures.insert(CreatureKind::Player(sim_hero_player(
            "Hero",
            Position::new(100, 100, 7),
        )));
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2190,
            ItemType {
                server_id: 2190,
                weapon_type: crate::inventory::WEAPON_WAND,
                ..Default::default()
            },
        );
        let w = world.player_resolve_combat_weapons(cid);
        assert!(w.wand.is_some());
        assert!(!w.fist);
        assert_eq!(world.player_weapon_distance(cid), 3);
        assert_eq!(world.player_weapon_max_range(cid), 3);
        let (atk, skill) = world.player_get_attack_value(cid);
        assert_eq!(atk, 0);
        assert_eq!(skill, SkillNr::Fist);
    }

    /// G9 — ranged GetDistance≠1 → enqueue prepends Wait(100) (`cract.cc:1358-1360`).
    #[test]
    fn ranged_attack_builder_prepends_wait_100() {
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
        assert_eq!(world.player_weapon_distance(cid), 3);
        assert!(world.enqueue_creature_attack(cid));
        let todo = &world.creatures.get(cid).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 2);
        assert!(matches!(
            todo.queue[0],
            crate::creature_todo::CreatureAction::Wait { deadline_ms: 100 }
        ));
        assert!(matches!(
            todo.queue[1],
            crate::creature_todo::CreatureAction::Attack
        ));
    }
}
