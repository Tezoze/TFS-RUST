//! VIP list add / remove / edit — TFS pack surface.
//!
//! Pack: `Game::playerAddVip` / `playerRemoveVip` / `playerEditVip` — `game.cpp`;
//! `Player::addVIP` / `removeVIP` / `getMaxVIPEntries` / `notifyStatusChange` — `player.cpp`.
//! DB: `IOLoginData::{add,remove,edit}VIPEntry` — `iologindata.cpp` (immediate SQL, not `savePlayer`).
//! Wire: 772 `gameserver/src/protocolgame.cpp` `sendVIP` / `sendVIPLogout`;
//! 1098 `src/protocolgame.cpp` `sendVIP` / `sendVIPStatus`.

use tfs_rust_common::GameCommand;
use tfs_rust_db::player::{PlayerStore, VipEntry};
use tfs_rust_net::outgoing_extra::send_text_message_simple;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

const VIP_HARD_CAP: usize = 200;
const VIP_FREE_DEFAULT: u32 = 20;
const VIP_PREMIUM_DEFAULT: u32 = 100;
const VIP_DESC_MAX: usize = 128;
const VIP_STATUS_ONLINE: u8 = 1;
const VIP_STATUS_OFFLINE: u8 = 0;

impl GameWorld {
    /// C++ `Player::getMaxVIPEntries` — `player.cpp`.
    pub fn player_get_max_vip_entries(&self, cid: CreatureId) -> u32 {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return VIP_FREE_DEFAULT;
        };
        if let Some(g) = self.groups.groups.get(&p.group_id)
            && g.max_vip_entries != 0
        {
            return g.max_vip_entries.min(VIP_HARD_CAP as u32);
        }
        if self.player_is_premium(cid) {
            VIP_PREMIUM_DEFAULT
        } else {
            VIP_FREE_DEFAULT
        }
    }

    /// C++ `Game::playerAddVip` — `game.cpp`.
    pub fn player_add_vip(&mut self, cid: CreatureId, name: String) {
        let name = name.trim().to_string();
        if name.is_empty() {
            self.send_vip_fail(cid, "A player with this name does not exist.");
            return;
        }
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        let requester_guid = p.guid;
        if name.eq_ignore_ascii_case(&p.base.name) {
            self.send_vip_fail(cid, "You cannot add yourself.");
            return;
        }
        if let Some(target) = self.find_online_player_by_name(&name) {
            let (vip_guid, vip_name, ghost) = match self.creatures.get(target) {
                Some(CreatureKind::Player(t)) => (t.guid, t.base.name.clone(), t.ghost_mode),
                _ => {
                    self.send_vip_fail(cid, "A player with this name does not exist.");
                    return;
                }
            };
            let status = if ghost && !self.player_is_access_player(cid) {
                VIP_STATUS_OFFLINE
            } else {
                VIP_STATUS_ONLINE
            };
            self.finish_add_vip(cid, vip_guid, vip_name, status);
            return;
        }
        if self.scheduler.is_none() {
            self.send_vip_fail(cid, "A player with this name does not exist.");
            return;
        }
        self.spawn_vip_lookup(requester_guid, name);
    }

    /// Offline `getGuidByNameEx` result — re-run add checks on the game thread.
    pub fn apply_vip_lookup_finished(
        &mut self,
        requester_guid: u32,
        target_guid: Option<u32>,
        target_name: String,
    ) {
        let Some(&cid) = self.player_by_guid.get(&requester_guid) else {
            return;
        };
        let Some(vip_guid) = target_guid else {
            self.send_vip_fail(cid, "A player with this name does not exist.");
            return;
        };
        let (vip_name, status) = if let Some(&tid) = self.player_by_guid.get(&vip_guid) {
            match self.creatures.get(tid) {
                Some(CreatureKind::Player(t)) => {
                    let status = if t.ghost_mode && !self.player_is_access_player(cid) {
                        VIP_STATUS_OFFLINE
                    } else {
                        VIP_STATUS_ONLINE
                    };
                    (t.base.name.clone(), status)
                }
                _ => (target_name, VIP_STATUS_OFFLINE),
            }
        } else {
            (target_name, VIP_STATUS_OFFLINE)
        };
        self.finish_add_vip(cid, vip_guid, vip_name, status);
    }

    /// C++ `Game::playerRemoveVip` / `Player::removeVIP`.
    pub fn player_remove_vip(&mut self, cid: CreatureId, guid: u32) {
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
            return;
        };
        let Some(i) = p.vip_list.iter().position(|e| e.player_id == guid) else {
            return;
        };
        p.vip_list.remove(i);
        let account_id = p.account_id;
        self.persist_vip_remove(account_id, guid);
    }

    /// C++ `Game::playerEditVip` — 1098 `0xDE` only.
    pub fn player_edit_vip(
        &mut self,
        cid: CreatureId,
        guid: u32,
        description: String,
        icon: u32,
        notify: bool,
    ) {
        if icon > u32::from(u8::MAX) {
            return;
        }
        let description: String = description.chars().take(VIP_DESC_MAX).collect();
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
            return;
        };
        let Some(entry) = p.vip_list.iter_mut().find(|e| e.player_id == guid) else {
            return;
        };
        entry.description = description.clone();
        entry.icon = icon;
        entry.notify = notify;
        let account_id = p.account_id;
        self.persist_vip_edit(account_id, guid, description, icon, notify);
    }

    /// C++ `Player::notifyStatusChange` — login/logout fan-out to watchers.
    pub fn broadcast_vip_status(&mut self, subject: CreatureId, online: bool) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(subject) else {
            return;
        };
        let guid = p.guid;
        let name = p.base.name.clone();
        let ghost = p.ghost_mode;
        let watchers: Vec<CreatureId> = self
            .player_by_guid
            .values()
            .copied()
            .filter(|&c| c != subject)
            .collect();
        for watcher in watchers {
            let has = matches!(
                self.creatures.get(watcher),
                Some(CreatureKind::Player(w)) if w.vip_list.iter().any(|e| e.player_id == guid)
            );
            if !has {
                continue;
            }
            let status = if online {
                if ghost && !self.player_is_access_player(watcher) {
                    VIP_STATUS_OFFLINE
                } else {
                    VIP_STATUS_ONLINE
                }
            } else {
                VIP_STATUS_OFFLINE
            };
            let Some(conn) = self.conn_for_creature(watcher) else {
                continue;
            };
            let pkt = self.codec.encode_vip_status(guid, status);
            self.enqueue_encoded(conn, pkt);
            if ghost {
                continue;
            }
            let text = if online {
                format!("{name} has logged in.")
            } else {
                format!("{name} has logged out.")
            };
            let ty = self.codec.failure_message_type();
            self.enqueue_outgoing(conn, send_text_message_simple(ty, &text).into_bytes());
        }
    }

    fn finish_add_vip(&mut self, cid: CreatureId, vip_guid: u32, vip_name: String, status: u8) {
        let (own_guid, account_id, already, len) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => (
                p.guid,
                p.account_id,
                p.vip_list.iter().any(|e| e.player_id == vip_guid),
                p.vip_list.len(),
            ),
            _ => return,
        };
        if own_guid == vip_guid {
            self.send_vip_fail(cid, "You cannot add yourself.");
            return;
        }
        if already {
            self.send_vip_fail(cid, "This player is already in your list.");
            return;
        }
        let cap = (self.player_get_max_vip_entries(cid) as usize).min(VIP_HARD_CAP);
        if len >= cap {
            self.send_vip_fail(cid, "You cannot add more buddies.");
            return;
        }
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.vip_list.push(VipEntry {
                player_id: vip_guid,
                name: vip_name.clone(),
                description: String::new(),
                icon: 0,
                notify: false,
            });
        }
        self.persist_vip_add(account_id, vip_guid);
        let Some(conn) = self.conn_for_creature(cid) else {
            return;
        };
        let pkt = self
            .codec
            .encode_vip_entry(vip_guid, &vip_name, "", 0, false, status);
        self.enqueue_encoded(conn, pkt);
    }

    fn find_online_player_by_name(&self, name: &str) -> Option<CreatureId> {
        if let Some(&cid) = self.player_by_name.get(name) {
            return Some(cid);
        }
        self.player_by_name
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, &cid)| cid)
    }

    fn send_vip_fail(&mut self, cid: CreatureId, text: &str) {
        let Some(conn) = self.conn_for_creature(cid) else {
            return;
        };
        let ty = self.codec.failure_message_type();
        self.enqueue_outgoing(conn, send_text_message_simple(ty, text).into_bytes());
    }

    fn spawn_vip_lookup(&self, requester_guid: u32, name: String) {
        let Some(sched) = self.scheduler.as_ref() else {
            return;
        };
        let tx = sched.ctrl_sender();
        let db = self.db.clone();
        tokio::spawn(async move {
            let found = PlayerStore::new(&db)
                .guid_and_name_by_name(&name)
                .await
                .ok()
                .flatten();
            let (target_guid, target_name) = match found {
                Some((guid, n)) => (Some(guid), n),
                None => (None, name),
            };
            let _ = tx.send(GameCommand::VipLookupFinished {
                requester_guid,
                target_guid,
                target_name,
            });
        });
    }

    fn persist_vip_add(&self, account_id: u32, player_id: u32) {
        let Ok(account_id) = i32::try_from(account_id) else {
            return;
        };
        let Ok(player_id) = i32::try_from(player_id) else {
            return;
        };
        let db = self.db.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(e) = PlayerStore::new(&db)
                    .add_vip_entry(account_id, player_id, "", 0, false)
                    .await
                {
                    tracing::error!(?e, account_id, player_id, "VIP add persist failed");
                }
            });
        }
    }

    fn persist_vip_remove(&self, account_id: u32, player_id: u32) {
        let Ok(account_id) = i32::try_from(account_id) else {
            return;
        };
        let Ok(player_id) = i32::try_from(player_id) else {
            return;
        };
        let db = self.db.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(e) = PlayerStore::new(&db)
                    .remove_vip_entry(account_id, player_id)
                    .await
                {
                    tracing::error!(?e, account_id, player_id, "VIP remove persist failed");
                }
            });
        }
    }

    fn persist_vip_edit(
        &self,
        account_id: u32,
        player_id: u32,
        description: String,
        icon: u32,
        notify: bool,
    ) {
        let Ok(account_id) = i32::try_from(account_id) else {
            return;
        };
        let Ok(player_id) = i32::try_from(player_id) else {
            return;
        };
        let db = self.db.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(e) = PlayerStore::new(&db)
                    .edit_vip_entry(account_id, player_id, &description, icon, notify)
                    .await
                {
                    tracing::error!(?e, account_id, player_id, "VIP edit persist failed");
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_harness::{ensure_walkable_tile, insert_player, minimal_world, test_player};
    use tfs_rust_common::ConnId;
    use tfs_rust_common::Position;
    use tfs_rust_common::protocol_opcodes::server;

    fn vip_fixture() -> (GameWorld, CreatureId, CreatureId) {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let mut alice = test_player("Alice", pos);
        alice.guid = 10;
        alice.account_id = 1;
        let mut bob = test_player("Bob", pos);
        bob.guid = 20;
        bob.account_id = 2;
        let alice_id = insert_player(&mut world, alice);
        let bob_id = insert_player(&mut world, bob);
        world.player_by_guid.insert(10, alice_id);
        world.player_by_guid.insert(20, bob_id);
        world.player_by_name.insert("Alice".into(), alice_id);
        world.player_by_name.insert("Bob".into(), bob_id);
        world.register_conn_mapping(ConnId(1), alice_id);
        world.register_conn_mapping(ConnId(2), bob_id);
        (world, alice_id, bob_id)
    }

    fn vip_list(world: &GameWorld, cid: CreatureId) -> Vec<VipEntry> {
        match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.vip_list.clone(),
            _ => Vec::new(),
        }
    }

    fn packets(world: &GameWorld, conn: ConnId) -> &[Vec<u8>] {
        world
            .pending_outgoing
            .get(&conn)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[test]
    fn add_online_buddy_appends_and_sends_entry() {
        let (mut world, alice, bob) = vip_fixture();
        world.player_add_vip(alice, "Bob".into());
        let list = vip_list(&world, alice);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].player_id, 20);
        assert_eq!(list[0].name, "Bob");
        let last = packets(&world, ConnId(1)).last().expect("0xD2");
        assert_eq!(last[0], server::VIP_ENTRY);
        let _ = bob;
    }

    #[test]
    fn add_self_is_rejected() {
        let (mut world, alice, _) = vip_fixture();
        world.player_add_vip(alice, "Alice".into());
        assert!(vip_list(&world, alice).is_empty());
        let last = packets(&world, ConnId(1)).last().expect("fail text");
        assert_eq!(last[0], 0xB4);
    }

    #[test]
    fn add_duplicate_is_rejected() {
        let (mut world, alice, _) = vip_fixture();
        world.player_add_vip(alice, "Bob".into());
        world.pending_outgoing.clear();
        world.player_add_vip(alice, "bob".into());
        assert_eq!(vip_list(&world, alice).len(), 1);
        let last = packets(&world, ConnId(1)).last().expect("fail text");
        assert_eq!(last[0], 0xB4);
    }

    #[test]
    fn add_over_free_cap_is_rejected() {
        let (mut world, alice, _) = vip_fixture();
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(alice) {
            p.vip_list = (100..=119)
                .map(|i| VipEntry {
                    player_id: i,
                    name: format!("n{i}"),
                    description: String::new(),
                    icon: 0,
                    notify: false,
                })
                .collect();
        }
        world.player_add_vip(alice, "Bob".into());
        assert_eq!(vip_list(&world, alice).len(), 20);
        assert!(!vip_list(&world, alice).iter().any(|e| e.player_id == 20));
    }

    #[test]
    fn remove_by_guid() {
        let (mut world, alice, _) = vip_fixture();
        world.player_add_vip(alice, "Bob".into());
        world.player_remove_vip(alice, 20);
        assert!(vip_list(&world, alice).is_empty());
    }

    #[test]
    fn edit_updates_memory() {
        let (mut world, alice, _) = vip_fixture();
        world.player_add_vip(alice, "Bob".into());
        world.player_edit_vip(alice, 20, "friend".into(), 3, true);
        let e = &vip_list(&world, alice)[0];
        assert_eq!(e.description, "friend");
        assert_eq!(e.icon, 3);
        assert!(e.notify);
    }

    #[test]
    fn login_notify_reaches_watcher() {
        let (mut world, alice, bob) = vip_fixture();
        world.player_add_vip(alice, "Bob".into());
        world.pending_outgoing.clear();
        world.broadcast_vip_status(bob, true);
        let out = packets(&world, ConnId(1));
        assert!(out.iter().any(|p| p.first() == Some(&server::VIP_STATUS)));
        assert!(out.iter().any(|p| p.first() == Some(&0xB4)));
    }

    #[test]
    fn logout_notify_reaches_watcher() {
        let (mut world, alice, bob) = vip_fixture();
        world.player_add_vip(alice, "Bob".into());
        world.pending_outgoing.clear();
        world.broadcast_vip_status(bob, false);
        let out = packets(&world, ConnId(1));
        assert!(out.iter().any(|p| p.first() == Some(&server::VIP_STATUS)));
    }

    #[test]
    fn max_vip_entries_free_default_is_20() {
        let (world, alice, _) = vip_fixture();
        assert_eq!(world.player_get_max_vip_entries(alice), 20);
    }
}
