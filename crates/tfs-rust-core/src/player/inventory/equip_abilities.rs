//! Item ability apply/remove on equip — TFS `MoveEvent::EquipItem` / `DeEquipItem`.
//!
//! Domain: `src/movement.cpp` `EquipItem` / `DeEquipItem` (abilities from `items.xml`).
//! Outcomes: transform-on-equip, speed / skill / stat / regen / suppress / mana shield /
//! invisible — matching TFS `changeSpeed` + `setVarSkill` / `setVarStats` + conditions.

use tfs_rust_common::enums::ConditionType;
use tfs_rust_content::item_abilities::{
    ItemAbilities, CONDITION_DRUNK, STAT_MAGICPOINTS, STAT_MAXHITPOINTS, STAT_MAXMANAPOINTS,
    STAT_SOULPOINTS,
};

use crate::condition::{add_condition_merge, ActiveCondition, ConditionData};
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};

impl GameWorld {
    /// TFS `Game::changeSpeed` — adjust `varSpeed` and announce (`game.cpp` ~3855).
    pub(crate) fn change_creature_speed(&mut self, cid: CreatureId, delta: i32) {
        if let Some(kind) = self.creatures.get_mut(cid) {
            kind.base_mut().var_speed += delta;
        }
        self.announce_creature_speed(cid);
    }

    /// TFS `Player::sendIcons` — `0xA2` condition icons (`player.cpp` / `protocolgame.cpp`).
    pub(crate) fn send_player_icons(&mut self, cid: CreatureId) {
        let Some(conn_id) = self.conn_for_creature(cid) else {
            return;
        };
        let icons = self.player_client_icons(cid);
        use tfs_rust_net::outgoing_extra::{send_icons, send_icons_classic};
        let packet = match &self.codec {
            tfs_rust_net::Codec::V772(_) => send_icons_classic(icons).into_bytes(),
            tfs_rust_net::Codec::V1098(_) => send_icons(icons).into_bytes(),
        };
        self.enqueue_outgoing(conn_id, packet);
    }

    /// TFS `Player::getClientIcons` — `player.cpp` ~387–415 (subset: conditions + suppress).
    fn player_client_icons(&self, cid: CreatureId) -> u16 {
        use tfs_rust_content::item_abilities::{
            CONDITION_BLEEDING, CONDITION_CURSED, CONDITION_DAZZLED, CONDITION_DROWN,
            CONDITION_ENERGY, CONDITION_FIRE, CONDITION_FREEZING, CONDITION_POISON,
        };
        // `src/const.h` ICON_* bits.
        const ICON_POISON: u16 = 1 << 0;
        const ICON_BURN: u16 = 1 << 1;
        const ICON_ENERGY: u16 = 1 << 2;
        const ICON_DRUNK: u16 = 1 << 3;
        const ICON_MANASHIELD: u16 = 1 << 4;
        const ICON_PARALYZE: u16 = 1 << 5;
        const ICON_HASTE: u16 = 1 << 6;
        const ICON_DROWNING: u16 = 1 << 8;
        const ICON_FREEZING: u16 = 1 << 9;
        const ICON_DAZZLED: u16 = 1 << 10;
        const ICON_CURSED: u16 = 1 << 11;
        const ICON_BLEEDING: u16 = 1 << 15;

        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return 0;
        };
        let suppress = p.condition_suppressions;
        let mut icons: u16 = 0;
        for cond in &p.base.active_conditions {
            let bit = match cond.ctype {
                ConditionType::Poison if suppress & CONDITION_POISON == 0 => ICON_POISON,
                ConditionType::Fire if suppress & CONDITION_FIRE == 0 => ICON_BURN,
                ConditionType::Energy if suppress & CONDITION_ENERGY == 0 => ICON_ENERGY,
                ConditionType::Drunk if suppress & CONDITION_DRUNK == 0 => ICON_DRUNK,
                ConditionType::ManaShield => ICON_MANASHIELD,
                ConditionType::Paralyze => ICON_PARALYZE,
                ConditionType::Haste => ICON_HASTE,
                ConditionType::Freezing if suppress & CONDITION_FREEZING == 0 => ICON_FREEZING,
                ConditionType::Dazzled if suppress & CONDITION_DAZZLED == 0 => ICON_DAZZLED,
                ConditionType::Cursed if suppress & CONDITION_CURSED == 0 => ICON_CURSED,
                ConditionType::Bleeding if suppress & CONDITION_BLEEDING == 0 => ICON_BLEEDING,
                _ => 0,
            };
            // Drown uses CONDITION_DROWN bit in suppressions; we have no Drown ctype yet.
            let _ = CONDITION_DROWN;
            let _ = ICON_DROWNING;
            icons |= bit;
        }
        if p.base.drunkenness > 0 && suppress & CONDITION_DRUNK == 0 {
            icons |= ICON_DRUNK;
        }
        icons
    }

    /// TFS `MoveEvent::EquipItem` ability body — `movement.cpp` ~716–813.
    pub(crate) fn apply_equip_item_abilities(
        &mut self,
        cid: CreatureId,
        item_id: ItemId,
        slot: u8,
    ) {
        let already = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.is_item_ability_enabled(slot),
            _ => return,
        };
        if already {
            return;
        }

        let Some(item_type) = self.items.get(item_id).map(|i| i.item_type) else {
            return;
        };

        // Transform inactive → active (time/skill rings). Abilities live on the **active**
        // type in this data pack — apply from the post-transform id (outcome parity).
        let transform_to = self
            .items_db
            .items
            .get(&item_type)
            .and_then(|it| xml_u16(&it.xml_attributes, "transformequipto"))
            .filter(|&id| id > 0);

        let ability_type = if let Some(new_type) = transform_to {
            self.transform_equipped_item(cid, item_id, slot, new_type);
            new_type
        } else {
            item_type
        };

        let abilities = self
            .items_db
            .items
            .get(&ability_type)
            .map(|it| it.abilities.clone())
            .unwrap_or_default();

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.set_item_ability(slot, true);
        }

        self.apply_item_abilities_delta(cid, slot, &abilities, 1);
    }

    /// TFS `MoveEvent::DeEquipItem` ability body — `movement.cpp` ~816–893.
    pub(crate) fn remove_equip_item_abilities(
        &mut self,
        cid: CreatureId,
        item_id: ItemId,
        slot: u8,
    ) {
        let enabled = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.is_item_ability_enabled(slot),
            _ => return,
        };
        if !enabled {
            return;
        }

        let Some(item_type) = self.items.get(item_id).map(|i| i.item_type) else {
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.set_item_ability(slot, false);
            }
            return;
        };

        let abilities = self
            .items_db
            .items
            .get(&item_type)
            .map(|it| it.abilities.clone())
            .unwrap_or_default();

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.set_item_ability(slot, false);
        }

        self.apply_item_abilities_delta(cid, slot, &abilities, -1);

        let transform_to = self
            .items_db
            .items
            .get(&item_type)
            .and_then(|it| xml_u16(&it.xml_attributes, "transformdeequipto"))
            .filter(|&id| id > 0);
        if let Some(new_type) = transform_to {
            self.transform_equipped_item(cid, item_id, slot, new_type);
        }
    }

    /// Change equipped item type + refresh `0x78` via shared decay type-change helper.
    ///
    /// Domain: `Game::transformItem` + equip/deequip (`game.cpp` / `movement.cpp`).
    /// Outcomes: `stopduration` pause on deequip; resume on re-equip (`ChangeObject`).
    fn transform_equipped_item(
        &mut self,
        _cid: CreatureId,
        item_id: ItemId,
        _slot: u8,
        new_type: u16,
    ) {
        self.change_item_type(item_id, new_type);
    }

    /// Apply (`sign = 1`) or reverse (`sign = -1`) one item's ability modifiers.
    fn apply_item_abilities_delta(
        &mut self,
        cid: CreatureId,
        slot: u8,
        abilities: &ItemAbilities,
        sign: i32,
    ) {
        let mut need_icons = false;

        if abilities.invisible {
            need_icons = true;
            if sign > 0 {
                if let Some(kind) = self.creatures.get_mut(cid) {
                    add_condition_merge(
                        &mut kind.base_mut().active_conditions,
                        ActiveCondition::new(
                            slot as u32,
                            slot as u32,
                            ConditionType::Invisible,
                            ConditionData::Generic { ticks: -1 },
                            None,
                        ),
                    );
                }
                // TFS `ConditionInvisible::startCondition` → `internalCreatureChangeVisible(false)`.
                self.announce_player_change_visible(cid, false);
            } else {
                let still_invisible = if let Some(kind) = self.creatures.get_mut(cid) {
                    kind.base_mut().active_conditions.retain(|c| {
                        !(c.ctype == ConditionType::Invisible && c.id == slot as u32)
                    });
                    kind.base().is_invisible()
                } else {
                    false
                };
                if !still_invisible {
                    // `ConditionInvisible::endCondition` → visible again when no other invis.
                    self.announce_player_change_visible(cid, true);
                }
            }
        }

        if abilities.mana_shield {
            need_icons = true;
            if sign > 0 {
                if let Some(kind) = self.creatures.get_mut(cid) {
                    add_condition_merge(
                        &mut kind.base_mut().active_conditions,
                        ActiveCondition::new(
                            slot as u32,
                            slot as u32,
                            ConditionType::ManaShield,
                            ConditionData::Generic { ticks: -1 },
                            None,
                        ),
                    );
                }
            } else if let Some(kind) = self.creatures.get_mut(cid) {
                kind.base_mut().active_conditions.retain(|c| {
                    !(c.ctype == ConditionType::ManaShield && c.id == slot as u32)
                });
            }
        }

        if abilities.speed != 0 {
            self.change_creature_speed(cid, abilities.speed * sign);
        }

        if abilities.condition_suppressions != 0 {
            need_icons = true;
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                if sign > 0 {
                    p.condition_suppressions |= abilities.condition_suppressions;
                    if abilities.condition_suppressions & CONDITION_DRUNK != 0 {
                        p.base.drunkenness = 0;
                    }
                } else {
                    p.condition_suppressions &= !abilities.condition_suppressions;
                }
            }
        }

        if abilities.regeneration {
            if sign > 0 {
                if let Some(kind) = self.creatures.get_mut(cid) {
                    add_condition_merge(
                        &mut kind.base_mut().active_conditions,
                        ActiveCondition::new(
                            slot as u32,
                            slot as u32,
                            ConditionType::Regeneration,
                            ConditionData::Regeneration {
                                health_gain: abilities.health_gain as i32,
                                health_ticks_ms: abilities.health_ticks,
                                mana_gain: abilities.mana_gain as i32,
                                mana_ticks_ms: abilities.mana_ticks,
                                health_elapsed_ms: 0,
                                mana_elapsed_ms: 0,
                            },
                            None,
                        ),
                    );
                }
            } else if let Some(kind) = self.creatures.get_mut(cid) {
                kind.base_mut().active_conditions.retain(|c| {
                    !(c.ctype == ConditionType::Regeneration && c.id == slot as u32)
                });
            }
        }

        let mut need_skills = false;
        let mut need_stats = false;

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            for (i, &delta) in abilities.skills.iter().enumerate() {
                if delta != 0 {
                    p.var_skills[i] += delta * sign;
                    need_skills = true;
                }
            }

            for (i, &delta) in abilities.stats.iter().enumerate() {
                if delta != 0 {
                    p.var_stats[i] += delta * sign;
                    need_stats = true;
                }
            }

            for (i, &pct) in abilities.stats_percent.iter().enumerate() {
                if pct == 0 {
                    continue;
                }
                let default = match i {
                    STAT_MAXHITPOINTS => p.base.max_health,
                    STAT_MAXMANAPOINTS => p.max_mana,
                    STAT_SOULPOINTS => p.economy.soul,
                    STAT_MAGICPOINTS => p.skills.maglevel,
                    _ => 0,
                };
                let adj =
                    ((default as f32) * (((pct - 100) as f32) / 100.0)).floor() as i32;
                if adj != 0 {
                    p.var_stats[i] += adj * sign;
                    need_stats = true;
                }
            }
        }

        if need_skills {
            self.send_player_skills(cid);
        }
        if need_stats {
            self.send_player_stats(cid);
        }
        if need_icons {
            self.send_player_icons(cid);
        }
    }

    /// TFS `Player::sendCreatureChangeVisible` player branch — empty outfit when invisible.
    ///
    /// C++: `player.h` ~789–800 (`Outfit_t{}` vs `getCurrentOutfit`).
    pub(crate) fn announce_player_change_visible(&mut self, cid: CreatureId, visible: bool) {
        let Some(kind) = self.creatures.get(cid) else {
            return;
        };
        let pos = kind.position();
        let wire_id = crate::login_out::creature_wire_id(cid, kind);
        let outfit = if visible {
            let o = &kind.base().outfit;
            tfs_rust_net::creature_encode::OutfitWire {
                look_type: o.look_type.max(0) as u16,
                look_head: o.look_head.clamp(0, 255) as u8,
                look_body: o.look_body.clamp(0, 255) as u8,
                look_legs: o.look_legs.clamp(0, 255) as u8,
                look_feet: o.look_feet.clamp(0, 255) as u8,
                look_addons: o.look_addons.clamp(0, 255) as u8,
                look_mount: 0,
                look_type_ex: 0,
            }
        } else {
            tfs_rust_net::creature_encode::OutfitWire {
                look_type: 0,
                look_head: 0,
                look_body: 0,
                look_legs: 0,
                look_feet: 0,
                look_addons: 0,
                look_mount: 0,
                look_type_ex: 0,
            }
        };
        let msg = self.codec.encode_creature_outfit(wire_id, &outfit);
        self.broadcast_to_spectators(pos, msg.into_bytes());
    }

    /// Reverse equip abilities without `transformDeEquipTo` (used when decay owns the type change).
    pub(crate) fn strip_equip_abilities_keep_type(
        &mut self,
        cid: CreatureId,
        item_id: ItemId,
        slot: u8,
    ) {
        let enabled = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.is_item_ability_enabled(slot),
            _ => return,
        };
        if !enabled {
            return;
        }
        let Some(item_type) = self.items.get(item_id).map(|i| i.item_type) else {
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.set_item_ability(slot, false);
            }
            return;
        };
        let abilities = self
            .items_db
            .items
            .get(&item_type)
            .map(|it| it.abilities.clone())
            .unwrap_or_default();
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.set_item_ability(slot, false);
        }
        self.apply_item_abilities_delta(cid, slot, &abilities, -1);
    }

    pub(crate) fn find_equipment_owner(&self, item_id: ItemId) -> Option<(CreatureId, u8)> {
        for (cid, kind) in self.creatures.iter() {
            let CreatureKind::Player(p) = kind else {
                continue;
            };
            for (idx, slot_item) in p.equipment_slots.iter().enumerate() {
                if *slot_item == Some(item_id) {
                    return Some((cid, (idx as u8) + 1));
                }
            }
        }
        None
    }

    pub(crate) fn unequip_decayed_item(&mut self, cid: CreatureId, slot: u8, item_id: ItemId) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            if let Some(idx) = crate::inventory::slot_to_array_index(slot) {
                if p.equipment_slots[idx] == Some(item_id) {
                    p.equipment_slots[idx] = None;
                }
            }
            p.set_item_ability(slot, false);
        }
        self.decay.cancel(item_id);
        self.items.remove(item_id);
        self.broadcast_player_inventory_slot(cid, slot, None);
        self.recompute_player_inventory_weight(cid);
        self.send_player_stats(cid);
    }
}

fn xml_u16(attrs: &std::collections::HashMap<String, String>, key: &str) -> Option<u16> {
    attrs.get(key).and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::InventorySlot;
    use crate::item_attributes::DecayState;
    use crate::sim_harness::{insert_player, minimal_world, test_player};
    use tfs_rust_common::Position;
    use tfs_rust_content::otb::ItemType;

    fn register_type(world: &mut GameWorld, item_type_id: u16, mut it: ItemType) {
        it.id = item_type_id;
        it.server_id = item_type_id;
        let mut items = std::collections::HashMap::clone(&world.items_db.items);
        items.insert(item_type_id, it);
        let client_to_server = std::collections::HashMap::clone(&world.items_db.client_to_server);
        world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
            items,
            client_to_server,
        });
    }

    fn equip_with_abilities(
        world: &mut GameWorld,
        cid: CreatureId,
        slot: u8,
        item_type_id: u16,
        abilities: ItemAbilities,
    ) -> ItemId {
        let mut it = ItemType::default();
        it.abilities = abilities;
        register_type(world, item_type_id, it);
        let iid = world.items.insert(crate::item::Item::new_single(item_type_id));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            let idx = crate::inventory::slot_to_array_index(slot).expect("slot");
            p.equipment_slots[idx] = Some(iid);
        }
        world.apply_equip_item_abilities(cid, iid, slot);
        iid
    }

    #[test]
    fn boots_of_haste_adds_var_speed_on_equip_and_clears_on_deequip() {
        let mut world = minimal_world();
        let cid = insert_player(
            &mut world,
            test_player("BootsTest", Position::new(100, 100, 7)),
        );
        let slot = InventorySlot::Feet as u8;
        let mut abl = ItemAbilities::default();
        abl.speed = 20;
        let iid = equip_with_abilities(&mut world, cid, slot, 2195, abl);
        assert_eq!(
            match world.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => p.base.var_speed,
                _ => panic!(),
            },
            20
        );
        world.remove_equip_item_abilities(cid, iid, slot);
        assert_eq!(
            match world.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => p.base.var_speed,
                _ => panic!(),
            },
            0
        );
    }

    #[test]
    fn skill_ring_adds_var_skill_for_combat_reads() {
        let mut world = minimal_world();
        let cid = insert_player(
            &mut world,
            test_player("SkillRing", Position::new(100, 100, 7)),
        );
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.skills.sword = 10;
        }
        let slot = InventorySlot::Ring as u8;
        let mut abl = ItemAbilities::default();
        abl.skills[tfs_rust_common::enums::Skill::Sword as usize] = 4;
        let _iid = equip_with_abilities(&mut world, cid, slot, 2210, abl);
        let p = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p,
            _ => panic!(),
        };
        assert_eq!(p.skill_level(crate::player::combat::SkillNr::Sword), 14);
    }

    #[test]
    fn inactive_time_ring_transforms_and_applies_speed() {
        let mut world = minimal_world();
        let cid = insert_player(
            &mut world,
            test_player("TimeRing", Position::new(100, 100, 7)),
        );
        let slot = InventorySlot::Ring as u8;

        let mut inactive = ItemType::default();
        inactive
            .xml_attributes
            .insert("transformequipto".into(), "2206".into());
        register_type(&mut world, 2169, inactive);

        let mut active = ItemType::default();
        active.abilities.speed = 30;
        active.decay_time = 600;
        active.decay_to = 0;
        active
            .xml_attributes
            .insert("duration".into(), "600".into());
        active
            .xml_attributes
            .insert("decayto".into(), "0".into());
        active
            .xml_attributes
            .insert("transformdeequipto".into(), "2169".into());
        register_type(&mut world, 2206, active);

        let iid = world.items.insert(crate::item::Item::new_single(2169));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.equipment_slots[slot_to_idx(slot)] = Some(iid);
        }
        world.apply_equip_item_abilities(cid, iid, slot);

        assert_eq!(world.items.get(iid).map(|i| i.item_type), Some(2206));
        assert_eq!(
            match world.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => p.base.var_speed,
                _ => panic!(),
            },
            30
        );
        assert!(
            world.items.get(iid).is_some_and(|i| i.decaying() == DecayState::True),
            "active ring should be decaying"
        );

        world.remove_equip_item_abilities(cid, iid, slot);
        assert_eq!(world.items.get(iid).map(|i| i.item_type), Some(2169));
        assert_eq!(
            match world.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => p.base.var_speed,
                _ => panic!(),
            },
            0
        );
    }

    #[test]
    fn life_ring_adds_regeneration_condition() {
        let mut world = minimal_world();
        let cid = insert_player(
            &mut world,
            test_player("LifeRing", Position::new(100, 100, 7)),
        );
        let slot = InventorySlot::Ring as u8;
        let mut abl = ItemAbilities::default();
        abl.regeneration = true;
        abl.health_gain = 1;
        abl.health_ticks = 3000;
        abl.mana_gain = 4;
        abl.mana_ticks = 3000;
        let iid = equip_with_abilities(&mut world, cid, slot, 2205, abl);
        let has_regen = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p
                .base
                .active_conditions
                .iter()
                .any(|c| c.ctype == ConditionType::Regeneration),
            _ => false,
        };
        assert!(has_regen);
        world.remove_equip_item_abilities(cid, iid, slot);
        let has_regen = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p
                .base
                .active_conditions
                .iter()
                .any(|c| c.ctype == ConditionType::Regeneration),
            _ => true,
        };
        assert!(!has_regen);
    }

    #[test]
    fn dwarven_ring_suppresses_drunk() {
        let mut world = minimal_world();
        let cid = insert_player(
            &mut world,
            test_player("Dwarven", Position::new(100, 100, 7)),
        );
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.base.drunkenness = 10;
        }
        let slot = InventorySlot::Ring as u8;
        let mut abl = ItemAbilities::default();
        abl.condition_suppressions = CONDITION_DRUNK;
        let _iid = equip_with_abilities(&mut world, cid, slot, 2215, abl);
        let (suppress, drunk) = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => (p.condition_suppressions, p.base.drunkenness),
            _ => panic!(),
        };
        assert_eq!(suppress & CONDITION_DRUNK, CONDITION_DRUNK);
        assert_eq!(drunk, 0);
    }

    #[test]
    fn energy_ring_mana_shield_condition() {
        let mut world = minimal_world();
        let cid = insert_player(
            &mut world,
            test_player("Energy", Position::new(100, 100, 7)),
        );
        let slot = InventorySlot::Ring as u8;
        let mut abl = ItemAbilities::default();
        abl.mana_shield = true;
        let _iid = equip_with_abilities(&mut world, cid, slot, 2204, abl);
        let has = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p
                .base
                .active_conditions
                .iter()
                .any(|c| c.ctype == ConditionType::ManaShield),
            _ => false,
        };
        assert!(has);
    }

    #[test]
    fn stealth_ring_adds_invisible_condition() {
        let mut world = minimal_world();
        let cid = insert_player(
            &mut world,
            test_player("Stealth", Position::new(100, 100, 7)),
        );
        let slot = InventorySlot::Ring as u8;
        let mut abl = ItemAbilities::default();
        abl.invisible = true;
        let iid = equip_with_abilities(&mut world, cid, slot, 2202, abl);
        let has = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.base.is_invisible(),
            _ => false,
        };
        assert!(has);
        world.remove_equip_item_abilities(cid, iid, slot);
        let has = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.base.is_invisible(),
            _ => true,
        };
        assert!(!has);
    }

    fn slot_to_idx(slot: u8) -> usize {
        crate::inventory::slot_to_array_index(slot).unwrap()
    }
}
