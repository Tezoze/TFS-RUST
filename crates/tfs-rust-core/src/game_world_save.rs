//! Build `PlayerSaveData` for `PlayerStore::savePlayer` from live simulation state.
// C++ reference: `src/iologindata.cpp` `IOLoginData::savePlayer`, `saveItems`

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use tfs_rust_common::enums::{Direction, SkullType};
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_db::player::{PlayerItemPayload, PlayerSaveData};
use tfs_rust_db::ItemRecord;

use crate::creature::{write_outfits_into_storage, CreatureKind};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::slot_to_array_index;
use crate::item_attributes::DecayState;
use crate::item_blob::{write_item_blob, write_item_blob_with_duration};

fn direction_to_u8(d: Direction) -> u8 {
    match d {
        Direction::North => 0,
        Direction::East => 1,
        Direction::South => 2,
        Direction::West => 3,
        Direction::SouthWest
        | Direction::NorthWest
        | Direction::NorthEast
        | Direction::SouthEast => 2,
    }
}

fn item_to_record(world: &GameWorld, pid: i32, sid: i32, item_id: ItemId) -> Result<ItemRecord> {
    let Some(item) = world.items.get(item_id) else {
        return Err(TfsRustError::Protocol(format!(
            "build_player_save_data: item {item_id:?} missing from SlotMap",
        )));
    };
    let count = (item.count.min(10000)) as i16;
    // When actively decaying, persist live remaining ms (TFS `getDuration()`), not schedule snapshot.
    let attributes = if item.decaying() == DecayState::True {
        let rem = world
            .item_decay_remaining_ms(item_id)
            .map(|m| m.min(i32::MAX as u64) as i32);
        write_item_blob_with_duration(item, world.items_db.as_ref(), rem)
    } else {
        write_item_blob(item, world.items_db.as_ref())
    };
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
        row.offlinetraining_time =
            (player.offline_training_ms / 1000).min(u32::from(u16::MAX)) as u16;
        row.balance = player.economy.balance;
        row.direction = direction_to_u8(player.base.direction);
        // 772 red mark: persist `PlayerkillerEnd` in `skulltime`; set skull=Red when active
        // so TFS tools see a red skull. White/yellow are session-only (observer-relative).
        row.skulltime = player.playerkiller_end;
        row.murder_timestamps = crate::player::combat::skulls::encode_murder_timestamps(
            &player.murder_timestamps,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        );
        row.skull = if player.playerkiller_end != 0 {
            SkullType::Red as u8 as i8
        } else {
            SkullType::None as u8 as i8
        };

        row.skill_fist = player.skills.fist.max(0) as u32;
        row.skill_club = player.skills.club.max(0) as u32;
        row.skill_sword = player.skills.sword.max(0) as u32;
        row.skill_axe = player.skills.axe.max(0) as u32;
        row.skill_dist = player.skills.dist.max(0) as u32;
        row.skill_shielding = player.skills.shielding.max(0) as u32;
        row.skill_fishing = player.skills.fishing.max(0) as u32;
        row.skill_fist_tries = player.skills.fist_tries;
        row.skill_club_tries = player.skills.club_tries;
        row.skill_sword_tries = player.skills.sword_tries;
        row.skill_axe_tries = player.skills.axe_tries;
        row.skill_dist_tries = player.skills.dist_tries;
        row.skill_shielding_tries = player.skills.shielding_tries;
        row.skill_fishing_tries = player.skills.fishing_tries;
        row.manaspent = player.skills.manaspent;
        row.blessings = player.blessings;

        // TFS `players.conditions` blob — `IOLoginData::savePlayer` (`iologindata.cpp:647-654`).
        // 772 persists the same buffs as skill Cycle fields (`crplayer.cc` skill Save).
        let conditions_blob =
            crate::condition_blob::serialize_conditions(&player.base.active_conditions);
        row.conditions = if conditions_blob.is_empty() {
            None
        } else {
            Some(conditions_blob)
        };

        // 772 `SKILL_FED` persistence — `crplayer.cc:2496` save Cycle, `crplayer.cc:2486` save Act.
        row.food_remaining = player.food_remaining as i32;
        row.food_level = player.food_level;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        row.lastlogout = now;
        if baseline.player_row.lastlogin > 0 {
            let delta = (now.saturating_sub(baseline.player_row.lastlogin)) as i64;
            row.onlinetime = baseline.player_row.onlinetime.saturating_add(delta);
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

        let mut storage = baseline.storage.clone();
        write_outfits_into_storage(&mut storage, &player.outfits);

        Ok(PlayerSaveData {
            player: row,
            spells: baseline.spells.clone(),
            storage,
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
    use crate::item_attributes::DecayState;
    use crate::item_blob::parse_item_blob;
    use crate::test_world::support::{insert_player, minimal_world, test_player};
    use tfs_rust_content::otb::ItemType;

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

    #[test]
    fn save_writes_decay_remaining_not_schedule_snapshot() {
        let mut world = minimal_world();
        world.server_ms = 10_000;
        let mut it = ItemType::default();
        it.id = 2169;
        it.server_id = 2169;
        it.decay_time = 600;
        it.decay_to = 2168;
        let mut items = std::collections::HashMap::clone(&world.items_db.items);
        items.insert(2169, it);
        let client_to_server = std::collections::HashMap::clone(&world.items_db.client_to_server);
        world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
            items,
            client_to_server,
        });

        let pos = Position::new(50, 50, 7);
        let cid = insert_player(&mut world, test_player("decay_save", pos));
        let mut ring = Item::new_single(2169);
        ring.set_duration(200_000);
        let ring_id = world.items.insert(ring);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.equipment_slots[5] = Some(ring_id); // CONST_SLOT_RING
        }
        world.start_decay(ring_id);
        world.server_ms = 60_000; // 50s elapsed → 150_000 remaining

        let save = world.build_player_save_data(cid).expect("save");
        let rec = save
            .items
            .inventory
            .iter()
            .find(|r| r.itemtype == 2169)
            .expect("ring row");
        let parsed = parse_item_blob(&rec.attributes, false).expect("parse");
        assert_eq!(parsed.attrs.get_duration_raw(), 150_000);
        assert_eq!(parsed.attrs.get_decaying(), DecayState::Pending); // non-zero → Pending on parse
    }
}
