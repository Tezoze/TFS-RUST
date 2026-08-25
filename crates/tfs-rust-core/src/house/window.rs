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
            let msg = self.codec.encode_house_window(window_text_id, &text);
            self.enqueue_encoded(conn, msg);
        }
        true
    }

    /// `Game::playerUpdateHouseWindow` (`game.cpp` ~2598).
    pub fn player_update_house_window(
        &mut self,
        cid: CreatureId,
        door_id: u8,
        window_text_id: u32,
        text: String,
    ) -> bool {
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
        self.houses.apply_list_row(session.house_id, session.list_id, &text, |name| {
            cache.get(name).copied().or_else(|| online.get(name).copied())
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
                let msg = self.codec.encode_house_window(session.window_text_id, &text);
                self.enqueue_encoded(conn, msg);
            }
            return true;
        }
        self.player_open_house_window(cid, house_id, list_id)
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
}
