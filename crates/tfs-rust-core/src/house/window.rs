//! House access-list edit window — TFS `Player::setEditHouse` / `Game::playerUpdateHouseWindow`.
//! Wire: S→C `0x97` (`sendHouseWindow`), C→S `0x8A` (`parseHouseWindow`).

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::player_flags::{PLAYER_FLAG_CAN_EDIT_HOUSES, has_player_flag};

/// In-progress list editor (`Player::editHouse` + `editListId` + `windowTextId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HouseEditSession {
    pub house_id: u32,
    pub list_id: u32,
    pub window_text_id: u32,
}

/// C++ `Game::playerUpdateHouseWindow` (`game.cpp` ~2598): first byte must be 0.
pub fn house_window_door_id_ok(door_id: u8) -> bool {
    door_id == 0
}

/// Session `windowTextId` must match the client submission.
pub fn house_window_id_ok(session: &HouseEditSession, window_text_id: u32) -> bool {
    session.window_text_id == window_text_id
}

impl GameWorld {
    /// `Player::setEditHouse` + `sendHouseWindow` (`protocolgame.cpp` ~1937).
    pub fn player_open_house_window(
        &mut self,
        cid: CreatureId,
        house_id: u32,
        list_id: u32,
    ) -> bool {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        let guid = p.guid;
        let edit = has_player_flag(self.player_group_flags(cid), PLAYER_FLAG_CAN_EDIT_HOUSES);
        if !self.houses.can_edit_list(house_id, list_id, guid, edit) {
            return false;
        }
        let text = self
            .houses
            .get_access_list_text(house_id, list_id)
            .unwrap_or_default();
        self.next_window_text_id = self.next_window_text_id.wrapping_add(1);
        let window_text_id = self.next_window_text_id;
        self.houses.edit_sessions.insert(
            guid,
            HouseEditSession {
                house_id,
                list_id,
                window_text_id,
            },
        );
        if let Some(conn) = self.conn_for_creature(cid) {
            self.enqueue_house_list_window(conn, house_id, list_id, window_text_id, &text);
        }
        true
    }

    /// `Game::playerUpdateHouseWindow` (`game.cpp` ~2598).
    /// 772 `CEditList` (`receiving.cc:679`) uses list type `1`/`2`/`3` plus the house or door id.
    pub fn player_update_house_window(
        &mut self,
        cid: CreatureId,
        door_id: u8,
        window_text_id: u32,
        text: String,
    ) -> bool {
        if self.codec.version().raw() < 1098 {
            return self.player_update_classic_house_list(cid, door_id, window_text_id, text);
        }
        if !house_window_door_id_ok(door_id) {
            return false;
        }
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        let guid = p.guid;
        let edit = has_player_flag(self.player_group_flags(cid), PLAYER_FLAG_CAN_EDIT_HOUSES);
        let Some(session) = self.houses.edit_sessions.get(&guid).copied() else {
            return false;
        };
        if !house_window_id_ok(&session, window_text_id) {
            return false;
        }
        if !self
            .houses
            .can_edit_list(session.house_id, session.list_id, guid, edit)
        {
            return false;
        }
        let cache = self.houses.name_to_guid.clone();
        let mut online: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for (name, &oid) in &self.player_by_name {
            if let Some(CreatureKind::Player(p)) = self.creatures.get(oid) {
                online.insert(name.to_ascii_lowercase(), p.guid);
            }
        }
        self.houses
            .apply_list_row(session.house_id, session.list_id, &text, |name| {
                cache
                    .get(name)
                    .copied()
                    .or_else(|| online.get(name).copied())
            });
        self.houses.edit_sessions.remove(&guid);
        self.queue_unresolved_house_names(session.house_id, session.list_id, &text);
        true
    }

    fn queue_unresolved_house_names(&self, house_id: u32, list_id: u32, text: &str) {
        let unresolved: Vec<String> = crate::house::AccessList::candidate_names(text)
            .into_iter()
            .filter(|n| !self.houses.name_to_guid.contains_key(n))
            .collect();
        if unresolved.is_empty() {
            return;
        }
        let Some(sched) = self.scheduler.as_ref() else {
            return;
        };
        let tx = sched.ctrl_sender();
        let db = self.db.clone();
        let text = text.to_string();
        tokio::spawn(async move {
            let store = tfs_rust_db::HouseStore::new(&db);
            let mut resolved = Vec::new();
            for name in unresolved {
                if let Ok(Some(guid)) = store.guid_by_name(&name).await {
                    resolved.push((name, guid));
                }
            }
            if resolved.is_empty() {
                return;
            }
            let _ = tx.send(tfs_rust_common::GameCommand::HouseNamesResolved {
                house_id,
                list_id,
                text,
                resolved,
            });
        });
    }

    /// `Player::setEditHouse` — store session, do not send the window yet.
    pub fn lua_script_player_set_edit_house(
        &mut self,
        creature_u64: u64,
        house_id: u32,
        list_id: u32,
    ) -> bool {
        let Some(cid) = self.resolve_creature_u64(creature_u64) else {
            return false;
        };
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        let guid = p.guid;
        let edit = has_player_flag(self.player_group_flags(cid), PLAYER_FLAG_CAN_EDIT_HOUSES);
        if !self.houses.can_edit_list(house_id, list_id, guid, edit) {
            return false;
        }
        self.next_window_text_id = self.next_window_text_id.wrapping_add(1);
        let window_text_id = self.next_window_text_id;
        self.houses.edit_sessions.insert(
            guid,
            HouseEditSession {
                house_id,
                list_id,
                window_text_id,
            },
        );
        true
    }

    /// `Player::sendHouseWindow` — send 0x97 for the stored (or newly opened) session.
    pub fn lua_script_player_send_house_window(
        &mut self,
        creature_u64: u64,
        house_id: u32,
        list_id: u32,
    ) -> bool {
        let Some(cid) = self.resolve_creature_u64(creature_u64) else {
            return false;
        };
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        let guid = p.guid;
        if let Some(session) = self.houses.edit_sessions.get(&guid).copied()
            && session.house_id == house_id
            && session.list_id == list_id
        {
            let text = self
                .houses
                .get_access_list_text(house_id, list_id)
                .unwrap_or_default();
            if let Some(conn) = self.conn_for_creature(cid) {
                self.enqueue_house_list_window(
                    conn,
                    house_id,
                    list_id,
                    session.window_text_id,
                    &text,
                );
            }
            return true;
        }
        self.player_open_house_window(cid, house_id, list_id)
    }

    /// 772 `SendEditList` / `CEditList`. Types: guest `1`, subowner `2`, door `3`.
    fn enqueue_house_list_window(
        &mut self,
        conn: tfs_rust_common::ConnId,
        house_id: u32,
        list_id: u32,
        window_text_id: u32,
        list_text: &str,
    ) {
        if self.codec.version().raw() >= 1098 {
            let msg = self.codec.encode_house_window(window_text_id, list_text);
            self.enqueue_encoded(conn, msg);
            return;
        }
        let (list_type, id, text) = classic_edit_list(self, house_id, list_id, window_text_id);
        let text = text.unwrap_or_else(|| list_text.to_string());
        let msg = tfs_rust_net::outgoing_extra::send_edit_list(list_type, id, &text);
        self.enqueue_encoded(conn, msg);
    }

    /// 772 `CEditList` (`receiving.cc:694`). `id` is the house id for types 1 and 2.
    fn player_update_classic_house_list(
        &mut self,
        cid: CreatureId,
        list_type: u8,
        id: u32,
        text: String,
    ) -> bool {
        let Some(crate::creature::CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        let guid = p.guid;
        let edit = has_player_flag(self.player_group_flags(cid), PLAYER_FLAG_CAN_EDIT_HOUSES);
        let (house_id, list_id) = match list_type {
            1 => (id, crate::house::GUEST_LIST),
            2 => (id, crate::house::SUBOWNER_LIST),
            3 => {
                let Some(session) = self.houses.edit_sessions.get(&guid).copied() else {
                    return false;
                };
                if session.window_text_id != id {
                    return false;
                }
                (session.house_id, session.list_id)
            }
            _ => return false,
        };
        if !self.houses.can_edit_list(house_id, list_id, guid, edit) {
            return false;
        }
        if list_type == 3
            && let Ok(door_id) = u8::try_from(list_id)
            && let Some(item_id) = house_door_item(self, house_id, door_id)
            && let Some(item) = self.items.get_mut(item_id)
        {
            item.set_text(text.clone());
        }
        let cache = self.houses.name_to_guid.clone();
        let mut online: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for (name, &oid) in &self.player_by_name {
            if let Some(crate::creature::CreatureKind::Player(p)) = self.creatures.get(oid) {
                online.insert(name.to_ascii_lowercase(), p.guid);
            }
        }
        self.houses
            .apply_list_row(house_id, list_id, &text, |name| {
                cache
                    .get(name)
                    .copied()
                    .or_else(|| online.get(name).copied())
            });
        self.houses.edit_sessions.remove(&guid);
        self.queue_unresolved_house_names(house_id, list_id, &text);
        true
    }

    pub fn apply_house_names_resolved(
        &mut self,
        house_id: u32,
        list_id: u32,
        text: String,
        resolved: Vec<(String, u32)>,
    ) {
        for (name, guid) in resolved {
            self.houses.name_to_guid.insert(name, guid);
        }
        let cache = self.houses.name_to_guid.clone();
        self.houses
            .apply_list_row(house_id, list_id, &text, |n| cache.get(n).copied());
    }
}

/// 772 list type + id. Door lists echo `window_text_id` and prefer the door item text.
fn classic_edit_list(
    world: &GameWorld,
    house_id: u32,
    list_id: u32,
    window_text_id: u32,
) -> (u8, u32, Option<String>) {
    match list_id {
        crate::house::GUEST_LIST => (1, house_id, None),
        crate::house::SUBOWNER_LIST => (2, house_id, None),
        door => {
            let text = u8::try_from(door).ok().and_then(|door_id| {
                let item_id = house_door_item(world, house_id, door_id)?;
                let text = world.items.get(item_id)?.text();
                if text.is_empty() {
                    None
                } else {
                    Some(text.to_string())
                }
            });
            (3, window_text_id, text)
        }
    }
}

fn house_door_item(world: &GameWorld, house_id: u32, door_id: u8) -> Option<crate::ids::ItemId> {
    if door_id == 0 {
        return None;
    }
    world.items.iter().find_map(|(id, item)| {
        let got = item
            .attributes
            .as_ref()
            .map(|a| a.get_door_id())
            .unwrap_or(0);
        if got != door_id {
            return None;
        }
        let crate::cylinder::Cylinder::Tile { pos } = item.parent? else {
            return None;
        };
        match world.map.get_tile(pos) {
            Some(crate::tile::Tile::House(h)) if h.house_id == house_id => Some(id),
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_nonzero_door_id_and_mismatched_window() {
        assert!(house_window_door_id_ok(0));
        assert!(!house_window_door_id_ok(1));
        let session = HouseEditSession {
            house_id: 1,
            list_id: 0x100,
            window_text_id: 7,
        };
        assert!(house_window_id_ok(&session, 7));
        assert!(!house_window_id_ok(&session, 8));
    }

    #[test]
    fn classic_guest_list_type_updates_guests() {
        use crate::test_support::{beat_driven_test_world, insert_player, test_player};
        use tfs_rust_common::Position;
        let mut world = beat_driven_test_world();
        let cid = insert_player(&mut world, test_player("Owner", Position::new(100, 100, 7)));
        world.houses.set_owner(4, 1);
        world.houses.name_to_guid.insert("alice".into(), 42);
        assert!(world.player_update_house_window(cid, 1, 4, "alice".into()));
        let guests = &world.houses.houses.get(&4).unwrap().guests;
        assert!(
            guests.contains(&42),
            "type 1 writes the guest list, got {guests:?}"
        );
    }
}
