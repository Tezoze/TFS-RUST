//! Build `PlayerSaveData` for `PlayerStore::savePlayer` from live simulation state.
// C++ reference: `src/iologindata.cpp` `IOLoginData::savePlayer`, `saveItems`

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use tfs_rust_common::enums::Direction;
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_db::player::{PlayerItemPayload, PlayerSaveData};
use tfs_rust_db::ItemRecord;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::slot_to_array_index;
use crate::item_blob::write_item_blob;

fn direction_to_u8(d: Direction) -> u8 {
    match d {
        Direction::North => 0,
        Direction::East => 1,
        Direction::South => 2,
        Direction::West => 3,
        Direction::SouthWest | Direction::NorthWest | Direction::NorthEast | Direction::SouthEast => 2,
    }
}

fn item_to_record(
    world: &GameWorld,
    pid: i32,
    sid: i32,
    item_id: ItemId,
) -> Result<ItemRecord> {
    let Some(item) = world.items.get(item_id) else {
        return Err(TfsRustError::Protocol(format!(
            "build_player_save_data: item {item_id:?} missing from SlotMap",
        )));
    };
    let count = (item.count.min(10000)) as i16;
    let attributes = write_item_blob(item, world.items_db.as_ref());
    Ok(ItemRecord {
        pid,
        sid,
        itemtype: item.item_type,
        count,
        attributes,
    })
}

/// C++ `IOLoginData::saveItems` — `runningId` starts at 100; BFS over open containers.
fn append_save_item_tree(
    world: &GameWorld,
    roots: &[(i32, ItemId)],
    out: &mut Vec<ItemRecord>,
) -> Result<()> {
    let mut running_id: i32 = 100;
    let mut queue: VecDeque<(ItemId, i32)> = VecDeque::new();

    for &(pid, item_id) in roots {
        running_id += 1;
        let sid = running_id;
        out.push(item_to_record(world, pid, sid, item_id)?);
        if let Some(cont) = world.container_registry.get(item_id) {
            if !cont.items.is_empty() {
                queue.push_back((item_id, sid));
            }
        }
    }

    while let Some((container_item_id, parent_sid)) = queue.pop_front() {
        let Some(cont) = world.container_registry.get(container_item_id) else {
            continue;
        };
        for &child_id in &cont.items {
            running_id += 1;
            let sid = running_id;
            out.push(item_to_record(world, parent_sid, sid, child_id)?);
            if let Some(sub) = world.container_registry.get(child_id) {
                if !sub.items.is_empty() {
                    queue.push_back((child_id, sid));
                }
            }
        }
    }
    Ok(())
}

impl GameWorld {
    /// Snapshot in-memory player + login baseline into `PlayerSaveData`.
    // C++ ref: `IOLoginData::savePlayer` (`iologindata.cpp`)
    pub fn build_player_save_data(&self, cid: CreatureId) -> Result<PlayerSaveData> {
        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return Err(TfsRustError::Protocol(
                "build_player_save_data: not a player creature".into(),
            ));
        };
        let Some(ref baseline) = player.persist else {
            return Err(TfsRustError::Protocol(
                "build_player_save_data: missing persist baseline (character not DB-loaded)".into(),
            ));
        };

        let mut row = baseline.player_row.clone();
        let pos = player.base.position;
        row.posx = i32::from(pos.x);
        row.posy = i32::from(pos.y);
        row.posz = i32::from(pos.z);
        row.name = player.base.name.clone();
        row.level = player.level;
        row.experience = player.experience;
        row.vocation = player.vocation_id;
        row.health = player.base.health;
        row.healthmax = player.base.max_health;
        if row.health <= 0 {
            row.health = 1;
        }
        row.mana = player.mana;
        row.manamax = player.max_mana;
        row.maglevel = player.skills.maglevel;
        row.looktype = player.base.outfit.look_type;
        row.lookhead = player.base.outfit.look_head;
        row.lookbody = player.base.outfit.look_body;
        row.looklegs = player.base.outfit.look_legs;
        row.lookfeet = player.base.outfit.look_feet;
        row.lookaddons = player.base.outfit.look_addons;
        row.cap = (player.capacity / 100).max(0);
        row.soul = player.economy.soul.max(0) as u32;
        row.town_id = player.town_id;
        row.stamina = player.stamina_minutes;
        row.offlinetraining_time = (player.offline_training_ms / 1000).min(u32::from(u16::MAX)) as u16;
        row.balance = player.economy.balance;
        row.direction = direction_to_u8(player.base.direction);
        row.skull = player.base.skull as u8 as i8;

        row.skill_fist = player.skills.fist.max(0) as u32;
        row.skill_club = player.skills.club.max(0) as u32;
        row.skill_sword = player.skills.sword.max(0) as u32;
        row.skill_axe = player.skills.axe.max(0) as u32;
        row.skill_dist = player.skills.dist.max(0) as u32;
        row.skill_shielding = player.skills.shielding.max(0) as u32;
        row.skill_fishing = player.skills.fishing.max(0) as u32;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        row.lastlogout = now;
        if baseline.player_row.lastlogin > 0 {
            let delta = (now.saturating_sub(baseline.player_row.lastlogin)) as i64;
            row.onlinetime = baseline
                .player_row
                .onlinetime
                .saturating_add(delta);
        }

        let mut roots: Vec<(i32, ItemId)> = Vec::new();
        for slot in 1u8..=10u8 {
            if let Some(idx) = slot_to_array_index(slot) {
                if let Some(iid) = player.equipment_slots[idx] {
                    roots.push((i32::from(slot), iid));
                }
            }
        }

        let mut inventory = Vec::new();
        append_save_item_tree(self, &roots, &mut inventory)?;

        let mut store_roots: Vec<(i32, ItemId)> = Vec::new();
        if let Some(idx) = slot_to_array_index(11) {
            if let Some(root_iid) = player.equipment_slots[idx] {
                if let Some(cont) = self.container_registry.get(root_iid) {
                    for &child in &cont.items {
                        store_roots.push((0, child));
                    }
                }
            }
        }
        let mut store_inbox = Vec::new();
        append_save_item_tree(self, &store_roots, &mut store_inbox)?;

        let skip_depot_save = player.last_depot_id == -1;

        let mut depot = Vec::new();
        if !skip_depot_save {
            let mut depot_roots: Vec<(i32, ItemId)> = Vec::new();
            for (&town_id, &chest_id) in &player.depot_chests {
                if let Some(cont) = self.container_registry.get(chest_id) {
                    for &child in &cont.items {
                        depot_roots.push((town_id as i32, child));
                    }
                }
            }
            append_save_item_tree(self, &depot_roots, &mut depot)?;
        }

        let mut inbox_roots: Vec<(i32, ItemId)> = Vec::new();
        if let Some(inbox_id) = player.inbox_root {
            if let Some(cont) = self.container_registry.get(inbox_id) {
                for &child in &cont.items {
                    inbox_roots.push((0, child));
                }
            }
        }
        let mut inbox = Vec::new();
        append_save_item_tree(self, &inbox_roots, &mut inbox)?;

        Ok(PlayerSaveData {
            player: row,
            spells: baseline.spells.clone(),
            storage: baseline.storage.clone(),
            items: PlayerItemPayload {
                inventory,
                depot,
                inbox,
                store_inbox,
            },
            skip_depot_save,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_common::Position;

    use crate::item::Item;
    use crate::test_world::support::{insert_player, minimal_world, test_player};

    #[test]
    fn save_depot_skipped_when_never_opened() {
        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        let cid = insert_player(&mut world, test_player("save", pos));
        let save = world.build_player_save_data(cid).expect("save data");
        assert!(save.skip_depot_save);
        assert!(save.items.depot.is_empty());
    }

    #[test]
    fn save_depot_live_after_open_and_mutation() {
        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        let cid = insert_player(&mut world, test_player("save", pos));
        let chest = world
            .player_get_depot_chest(cid, 1, true)
            .expect("depot chest");
        world.player_set_last_depot_id(cid, 1);
        let coin = world.items.insert(Item::new_single(2148));
        if let Some(cont) = world.container_registry.get_mut(chest) {
            let _ = cont.add_item(coin);
        }
        world.refresh_container_derived(chest);

        let save = world.build_player_save_data(cid).expect("save data");
        assert!(!save.skip_depot_save);
        assert_eq!(save.items.depot.len(), 1);
        assert_eq!(save.items.depot[0].pid, 1);
        assert_eq!(save.items.depot[0].itemtype, 2148);
    }
}
