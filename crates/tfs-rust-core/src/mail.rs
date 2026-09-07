//! Mailbox delivery — 772 `SendMail` / `SendMails`.
//!
//! Corpus: `SendMail` / `SendMails` (`moveuse.cc:712-919`); Collision dat
//! `IsType(Obj1,3501|3508) -> SendMail(Obj2)`. 772 lands in the **locker**
//! (`Player->Depot` / `LoadDepotBox`) beside the chest, not inside it.
//! 1098 locker `queryAdd` blocks that; gated path still uses the town chest.

use slotmap::Key;
use tfs_rust_common::GameCommand;
use tfs_rust_common::Position;
use tfs_rust_db::player::PlayerStore;

use crate::container_ui::ContainerContentChange;
use crate::creature::CreatureKind;
use crate::cylinder::{Cylinder, CylinderFlags};
use crate::formulas::DepotLockerStructure;
use crate::game_world::GameWorld;
use crate::ids::ItemId;
use crate::item_constants::{
    ITEM_LABEL, ITEM_LETTER, ITEM_LETTER_STAMPED, ITEM_PARCEL, ITEM_PARCEL_STAMPED,
};
use crate::tile::flags as tilestate;

/// `strlen(Addressee) >= 30` (`moveuse.cc:761`).
const MAX_ADDRESSEE_BYTES: usize = 29;
const MESSAGE_INFO_DESCR: u8 = 0x16;
const NEW_MAIL: &str = "New mail has arrived.";

/// Parse line 1 (name) + line 2 (town) — `ReadLine` (`moveuse.cc:693-763`).
pub fn parse_mail_address(text: &str) -> Option<(String, String)> {
    let mut lines = text.split('\n').map(|l| l.trim().trim_end_matches('\r'));
    let addressee = lines.next().unwrap_or("").to_string();
    let town = lines.next().unwrap_or("").to_string();
    if addressee.is_empty() || town.is_empty() {
        return None;
    }
    if addressee.len() > MAX_ADDRESSEE_BYTES {
        return None;
    }
    Some((addressee, town))
}

fn is_new_letter(item_type: u16) -> bool {
    item_type == ITEM_LETTER
}

fn is_new_parcel(item_type: u16) -> bool {
    item_type == ITEM_PARCEL
}

impl GameWorld {
    /// Collision `SendMail(Obj2)` after an item lands on a mailbox tile.
    pub(crate) fn apply_mailbox_send(&mut self, pos: Position, item_id: ItemId) {
        if self.mail_skip_mailbox_specials.remove(&item_id) {
            return;
        }
        let flags = self.map.get_tile(pos).map(|t| t.body().flags).unwrap_or(0);
        if flags & tilestate::MAILBOX == 0 {
            return;
        }
        let Some(item) = self.items.get(item_id) else {
            return;
        };
        if self
            .items_db
            .items
            .get(&item.item_type)
            .is_some_and(|t| t.is_mailbox())
        {
            return;
        }
        let _ = self.send_mail(item_id);
    }

    /// `SendMail` (`moveuse.cc:712-851`). Failures are silent — item stays.
    pub(crate) fn send_mail(&mut self, item_id: ItemId) -> bool {
        let Some((kind, text)) = self.mail_kind_and_text(item_id) else {
            return false;
        };
        let Some((addressee, town_name)) = parse_mail_address(&text) else {
            return false;
        };
        let Some(town_id) = self.depot_number_for_town(&town_name) else {
            return false;
        };
        if let Some(cid) = self.find_online_player_by_name(&addressee) {
            return self.deliver_mail_to_online(item_id, cid, town_id, kind);
        }
        if let Some(guid) = self.guid_from_name_cache(&addressee) {
            return self.deliver_mail_offline(item_id, guid, town_id, kind);
        }
        self.spawn_mail_lookup(item_id, town_id, addressee);
        false
    }

    /// Async `GetCharacterID` result — stamp+queue a held (already detached) item.
    pub(crate) fn apply_mail_lookup_finished(
        &mut self,
        item_ffi: u64,
        town_id: u32,
        guid: Option<u32>,
    ) {
        let item_id = ItemId::from(slotmap::KeyData::from_ffi(item_ffi));
        let held_pos = self.mail_lookup_holds.remove(&item_ffi);
        let Some(guid) = guid else {
            if let Some(pos) = held_pos {
                self.restore_mail_to_mailbox(item_id, pos);
            }
            return;
        };
        if held_pos.is_none() && !self.item_still_on_mailbox(item_id) {
            return;
        }
        let Some(kind) = self.mail_kind(item_id) else {
            if let Some(pos) = held_pos {
                self.restore_mail_to_mailbox(item_id, pos);
            }
            return;
        };
        let ok = if let Some(&cid) = self.player_by_guid.get(&guid) {
            self.deliver_mail_to_online(item_id, cid, town_id, kind)
        } else {
            self.deliver_mail_offline(item_id, guid, town_id, kind)
        };
        if !ok && let Some(pos) = held_pos {
            self.restore_mail_to_mailbox(item_id, pos);
        }
    }

    fn mail_kind(&self, item_id: ItemId) -> Option<MailKind> {
        let ty = self.items.get(item_id)?.item_type;
        if is_new_letter(ty) {
            Some(MailKind::Letter)
        } else if is_new_parcel(ty) {
            Some(MailKind::Parcel)
        } else {
            None
        }
    }

    fn mail_kind_and_text(&mut self, item_id: ItemId) -> Option<(MailKind, String)> {
        let ty = self.items.get(item_id)?.item_type;
        if is_new_letter(ty) {
            let text = self.items.get(item_id)?.text().to_string();
            return Some((MailKind::Letter, text));
        }
        if is_new_parcel(ty) {
            self.hydrate_container_if_needed(item_id);
            let label = self.parcel_label_text(item_id)?;
            return Some((MailKind::Parcel, label));
        }
        None
    }

    fn parcel_label_text(&self, parcel_id: ItemId) -> Option<String> {
        let children = self.container_registry.get(parcel_id)?.items.clone();
        for child in children {
            if self.items.get(child).is_some_and(|i| i.item_type == ITEM_LABEL) {
                return Some(self.items.get(child)?.text().to_string());
            }
        }
        None
    }

    fn depot_number_for_town(&self, name: &str) -> Option<u32> {
        self.map
            .towns
            .iter()
            .find(|(_, t)| t.name.eq_ignore_ascii_case(name))
            .map(|(id, _)| *id)
    }

    fn guid_from_name_cache(&self, name: &str) -> Option<u32> {
        if let Some(&g) = self.houses.name_to_guid.get(name) {
            return Some(g);
        }
        self.houses
            .name_to_guid
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, &g)| g)
    }

    fn item_still_on_mailbox(&self, item_id: ItemId) -> bool {
        let Some(item) = self.items.get(item_id) else {
            return false;
        };
        let Some(Cylinder::Tile { pos }) = item.parent else {
            return false;
        };
        self.map
            .get_tile(pos)
            .is_some_and(|t| t.body().flags & tilestate::MAILBOX != 0)
    }

    fn deliver_mail_to_online(
        &mut self,
        item_id: ItemId,
        cid: crate::ids::CreatureId,
        town_id: u32,
        kind: MailKind,
    ) -> bool {
        if self.mail_destination_full(cid, town_id) {
            return false;
        }
        // Corpus: `"New mail has arrived."` only when `Player->Depot` (locker) is open
        // (`moveuse.cc:791-803`). Snapshot before auto-create.
        let locker_open = self.mail_window_open(cid, town_id);
        if !self.take_unstamped_mail(item_id) {
            return false;
        }
        self.stamp_mail(item_id, kind);
        self.place_mail_in_depot(cid, town_id, item_id);
        if locker_open {
            let _ = self.lua_script_player_send_text_message(
                cid.data().as_ffi(),
                MESSAGE_INFO_DESCR,
                NEW_MAIL.to_string(),
            );
        }
        true
    }

    fn deliver_mail_offline(
        &mut self,
        item_id: ItemId,
        guid: u32,
        town_id: u32,
        kind: MailKind,
    ) -> bool {
        if !self.take_unstamped_mail(item_id) {
            return false;
        }
        self.stamp_mail(item_id, kind);
        self.queue_offline_mail(item_id, guid, town_id)
    }

    /// Take the letter off the mailbox, or accept an already-detached lookup hold.
    fn take_unstamped_mail(&mut self, item_id: ItemId) -> bool {
        if let Some(pos) = self.mail_tile_pos(item_id) {
            return self.detach_item_from_tile(pos, item_id).is_ok();
        }
        self.items
            .get(item_id)
            .is_some_and(|i| i.parent.is_none())
    }

    fn restore_mail_to_mailbox(&mut self, item_id: ItemId, pos: Position) {
        if self.items.get(item_id).is_none() {
            return;
        }
        self.mail_skip_mailbox_specials.insert(item_id);
        if self
            .internal_add_item_to_tile(pos, item_id, CylinderFlags::NO_LIMIT)
            .is_err()
        {
            self.mail_skip_mailbox_specials.remove(&item_id);
        }
    }

    fn existing_depot_locker(
        &self,
        cid: crate::ids::CreatureId,
        town_id: u32,
    ) -> Option<ItemId> {
        match self.creatures.get(cid)? {
            CreatureKind::Player(p) => p.depot_lockers.get(&town_id).copied(),
            _ => None,
        }
    }

    fn existing_depot_chest(
        &self,
        cid: crate::ids::CreatureId,
        town_id: u32,
    ) -> Option<ItemId> {
        match self.creatures.get(cid)? {
            CreatureKind::Player(p) => p.depot_chests.get(&town_id).copied(),
            _ => None,
        }
    }

    fn mail_destination_full(&self, cid: crate::ids::CreatureId, town_id: u32) -> bool {
        match self.mechanics.profile.depot_locker_structure {
            DepotLockerStructure::ClassicDepotChest => self
                .existing_depot_locker(cid, town_id)
                .is_some_and(|id| self.container_is_slot_full(id)),
            DepotLockerStructure::TfsMarketInbox => self
                .existing_depot_chest(cid, town_id)
                .is_some_and(|id| self.container_is_slot_full(id)),
        }
    }

    fn mail_window_open(&self, cid: crate::ids::CreatureId, town_id: u32) -> bool {
        let root = match self.mechanics.profile.depot_locker_structure {
            DepotLockerStructure::ClassicDepotChest => self.existing_depot_locker(cid, town_id),
            DepotLockerStructure::TfsMarketInbox => self.existing_depot_chest(cid, town_id),
        };
        root.is_some_and(|id| {
            self.container_registry
                .get_cid_for_container(cid, id)
                .is_some()
        })
    }

    /// 772: prepend into the locker. 1098: town chest (`queryAdd` rejects locker adds).
    pub(crate) fn place_mail_in_depot(
        &mut self,
        cid: crate::ids::CreatureId,
        town_id: u32,
        item_id: ItemId,
    ) {
        match self.mechanics.profile.depot_locker_structure {
            DepotLockerStructure::ClassicDepotChest => {
                let Some(locker) = self.player_get_depot_locker(cid, town_id) else {
                    return;
                };
                crate::house::add_to_container_front(self, locker, item_id);
                self.player_set_last_depot_id(cid, town_id);
                self.notify_container_content_changed(
                    locker,
                    ContainerContentChange::Add { slot: 0 },
                );
            }
            DepotLockerStructure::TfsMarketInbox => {
                self.house_add_item_to_town_depot(cid, town_id, item_id);
                self.player_set_last_depot_id(cid, town_id);
                if let Some(chest) = self.player_get_depot_chest(cid, town_id, false) {
                    self.notify_container_content_changed(
                        chest,
                        ContainerContentChange::Add { slot: 0 },
                    );
                }
            }
        }
    }

    fn mail_tile_pos(&self, item_id: ItemId) -> Option<Position> {
        match self.items.get(item_id)?.parent {
            Some(Cylinder::Tile { pos }) => Some(pos),
            _ => None,
        }
    }

    fn stamp_mail(&mut self, item_id: ItemId, kind: MailKind) {
        let stamped = match kind {
            MailKind::Letter => ITEM_LETTER_STAMPED,
            MailKind::Parcel => ITEM_PARCEL_STAMPED,
        };
        self.change_item_type(item_id, stamped);
    }

    fn container_is_slot_full(&self, chest: ItemId) -> bool {
        self.container_registry
            .get(chest)
            .is_some_and(|c| c.items.len() as u32 >= c.capacity)
    }

    fn spawn_mail_lookup(&mut self, item_id: ItemId, town_id: u32, name: String) {
        let Some(tx) = self.scheduler.as_ref().map(|s| s.ctrl_sender()) else {
            return;
        };
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let Some(pos) = self.mail_tile_pos(item_id) else {
            return;
        };
        if self.detach_item_from_tile(pos, item_id).is_err() {
            return;
        }
        let item_ffi = item_id.data().as_ffi();
        self.mail_lookup_holds.insert(item_ffi, pos);
        let db = self.db.clone();
        handle.spawn(async move {
            let found = PlayerStore::new(&db)
                .guid_and_name_by_name(&name)
                .await
                .ok()
                .flatten();
            let _ = tx.send(GameCommand::MailLookupFinished {
                item_id: item_ffi,
                town_id,
                guid: found.map(|(g, _)| g),
            });
        });
    }
}

#[derive(Clone, Copy)]
enum MailKind {
    Letter,
    Parcel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::Container;
    use crate::item::Item;
    use crate::sim_harness::{
        beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
    };
    use crate::tile::flags as tile_flags;
    use std::sync::Arc;
    use tfs_rust_content::items::ITEM_TYPE_MAILBOX;
    use tfs_rust_content::otb::ItemType;
    use tfs_rust_content::otbm::TownData;

    fn setup_mailbox_world() -> (crate::game_world::GameWorld, Position) {
        let mut world = beat_driven_test_world();
        let mut db = (*world.items_db).clone();
        db.items.insert(
            2593,
            ItemType {
                id: 2593,
                server_id: 2593,
                type_tag: ITEM_TYPE_MAILBOX,
                ..ItemType::default()
            },
        );
        db.items.insert(
            ITEM_LETTER,
            ItemType {
                id: ITEM_LETTER,
                server_id: ITEM_LETTER,
                flags: 1 << 6,
                ..ItemType::default()
            },
        );
        db.items.insert(
            ITEM_LETTER_STAMPED,
            ItemType {
                id: ITEM_LETTER_STAMPED,
                server_id: ITEM_LETTER_STAMPED,
                flags: 1 << 6,
                ..ItemType::default()
            },
        );
        db.items.insert(
            ITEM_PARCEL,
            ItemType {
                id: ITEM_PARCEL,
                server_id: ITEM_PARCEL,
                flags: 1 << 6,
                ..ItemType::default()
            },
        );
        db.items.insert(
            ITEM_PARCEL_STAMPED,
            ItemType {
                id: ITEM_PARCEL_STAMPED,
                server_id: ITEM_PARCEL_STAMPED,
                flags: 1 << 6,
                ..ItemType::default()
            },
        );
        db.items.insert(
            ITEM_LABEL,
            ItemType {
                id: ITEM_LABEL,
                server_id: ITEM_LABEL,
                flags: 1 << 6,
                ..ItemType::default()
            },
        );
        world.items_db = Arc::new(db);
        world.map.towns.insert(
            1,
            TownData {
                id: 1,
                name: "Thais".into(),
                temple_position: Position::new(100, 100, 7),
            },
        );
        let pos = Position::new(80, 80, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        if let Some(t) = world.map.get_tile_mut(pos) {
            t.body_mut().flags |= tile_flags::MAILBOX;
        }
        let box_id = world.items.insert(Item::new_single(2593));
        world
            .internal_add_item_to_tile(pos, box_id, crate::cylinder::CylinderFlags::NO_LIMIT)
            .expect("place mailbox");
        (world, pos)
    }

    #[test]
    fn parse_trims_and_requires_two_lines() {
        assert_eq!(
            parse_mail_address("  Hero  \n  Thais  \nHello"),
            Some(("Hero".into(), "Thais".into()))
        );
        assert!(parse_mail_address("Hero\n").is_none());
        assert!(parse_mail_address("\nThais").is_none());
        let long = format!("{}\nThais", "a".repeat(30));
        assert!(parse_mail_address(&long).is_none());
        let ok = format!("{}\nThais", "a".repeat(29));
        assert!(parse_mail_address(&ok).is_some());
    }

    #[test]
    fn letter_to_online_player_stamps_and_lands_in_town_locker() {
        let (mut world, pos) = setup_mailbox_world();
        let start = Position::new(81, 80, 7);
        ensure_walkable_tile(&mut world.map, start, 100);
        let mut hero = test_player("Hero", start);
        hero.guid = 42;
        let cid = insert_player(&mut world, hero);
        world.player_by_name.insert("Hero".into(), cid);
        world.player_by_guid.insert(42, cid);

        let mut letter = Item::new_single(ITEM_LETTER);
        letter.set_text("Hero\nThais\nHi");
        let letter_id = world.items.insert(letter);
        world
            .internal_add_item_to_tile(pos, letter_id, crate::cylinder::CylinderFlags::NONE)
            .expect("drop letter");

        assert!(
            world.items.get(letter_id).is_some(),
            "item instance survives"
        );
        assert_eq!(
            world.items.get(letter_id).map(|i| i.item_type),
            Some(ITEM_LETTER_STAMPED)
        );
        assert!(
            !world.item_still_on_mailbox(letter_id),
            "must leave the mailbox tile"
        );
        let locker = world
            .player_get_depot_locker(cid, 1)
            .expect("locker created");
        let chest = world
            .player_get_depot_chest(cid, 1, false)
            .expect("chest still nested in locker");
        assert!(
            world
                .container_registry
                .get(locker)
                .is_some_and(|c| c.items.contains(&letter_id)),
            "772 SendMail lands in the locker, beside the chest"
        );
        assert!(
            world
                .container_registry
                .get(chest)
                .is_some_and(|c| !c.items.contains(&letter_id)),
            "must not bury the letter inside the depot chest"
        );
        assert!(
            world
                .container_registry
                .get(locker)
                .is_some_and(|c| c.items.contains(&chest)),
            "chest remains a locker child"
        );
        assert_eq!(
            world
                .script_container_data(locker)
                .map(|d| d.item_holding_count),
            Some(1),
            "depot-tile holding count must include the letter without a depot shuffle"
        );
        let last = match world.creatures.get(cid) {
            Some(crate::creature::CreatureKind::Player(p)) => p.last_depot_id,
            _ => -1,
        };
        assert_eq!(last, 1, "mail must mark last_depot_id so logout saves the locker");
    }

    #[test]
    fn unknown_town_leaves_letter_unstamped() {
        let (mut world, pos) = setup_mailbox_world();
        let mut letter = Item::new_single(ITEM_LETTER);
        letter.set_text("Hero\nCarlin");
        let letter_id = world.items.insert(letter);
        world
            .internal_add_item_to_tile(pos, letter_id, crate::cylinder::CylinderFlags::NONE)
            .expect("drop");
        assert_eq!(
            world.items.get(letter_id).map(|i| i.item_type),
            Some(ITEM_LETTER)
        );
        assert!(world.item_still_on_mailbox(letter_id));
    }

    #[test]
    fn parcel_uses_label_text() {
        let (mut world, pos) = setup_mailbox_world();
        let start = Position::new(81, 80, 7);
        ensure_walkable_tile(&mut world.map, start, 100);
        let mut hero = test_player("Hero", start);
        hero.guid = 7;
        let cid = insert_player(&mut world, hero);
        world.player_by_name.insert("Hero".into(), cid);
        world.player_by_guid.insert(7, cid);

        let parcel_id = world.items.insert(Item::new_single(ITEM_PARCEL));
        let mut label = Item::new_single(ITEM_LABEL);
        label.set_text("Hero\nThais");
        let label_id = world.items.insert(label);
        let mut reg = std::mem::take(&mut world.container_registry);
        reg.register(Container::new(parcel_id, 10));
        if let Some(c) = reg.get_mut(parcel_id) {
            c.internal_add_item_front(label_id);
        }
        world.container_registry = reg;

        world
            .internal_add_item_to_tile(pos, parcel_id, crate::cylinder::CylinderFlags::NONE)
            .expect("drop parcel");
        assert_eq!(
            world.items.get(parcel_id).map(|i| i.item_type),
            Some(ITEM_PARCEL_STAMPED)
        );
        let locker = world.player_get_depot_locker(cid, 1).expect("locker");
        let chest = world.player_get_depot_chest(cid, 1, false).expect("chest");
        assert!(
            world
                .container_registry
                .get(locker)
                .is_some_and(|c| c.items.contains(&parcel_id))
        );
        assert!(
            world
                .container_registry
                .get(chest)
                .is_some_and(|c| !c.items.contains(&parcel_id))
        );
        assert_eq!(
            world
                .script_container_data(locker)
                .map(|d| d.item_holding_count),
            Some(2),
            "holding count is parcel + label (chest excluded)"
        );
    }

    #[test]
    fn offline_guid_queues_mail_outbox() {
        let (mut world, pos) = setup_mailbox_world();
        world.houses.name_to_guid.insert("Offline".into(), 99);
        let mut letter = Item::new_single(ITEM_LETTER);
        letter.set_text("Offline\nThais");
        let letter_id = world.items.insert(letter);
        world
            .internal_add_item_to_tile(pos, letter_id, crate::cylinder::CylinderFlags::NONE)
            .expect("drop");
        assert_eq!(
            world.items.get(letter_id).map(|i| i.item_type),
            Some(ITEM_LETTER_STAMPED)
        );
        assert!(
            world.houses.pending_depot_dumps.get(&99).is_none(),
            "mail must not use house pending_depot_dumps"
        );
        let pending = &world.mail_outbox.get(&99).expect("mail outbox").pending;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].item_ids, vec![letter_id]);
        assert_eq!(pending[0].town_id, 1);
        assert!(
            pending[0]
                .records
                .iter()
                .any(|r| r.pid == crate::depot_append::LOCKER_ROOT_PID_BASE + 1
                    && r.itemtype == ITEM_LETTER_STAMPED),
            "offline serialize uses locker-root pid so load_depot_table places beside the chest"
        );
        assert!(
            !pending[0].records.is_empty(),
            "serialize now for DB append / login splice"
        );
    }

    fn stub_loaded(guid: i32) -> tfs_rust_db::player::LoadedPlayerData {
        tfs_rust_db::player::LoadedPlayerData {
            player: tfs_rust_db::player::PlayerRecord {
                id: guid,
                name: "Offline".into(),
                account_id: 1,
                group_id: 1,
                sex: 0,
                vocation: 0,
                experience: 0,
                level: 8,
                maglevel: 0,
                health: 100,
                healthmax: 100,
                blessings: 0,
                mana: 50,
                manamax: 50,
                manaspent: 0,
                soul: 100,
                lookbody: 0,
                lookfeet: 0,
                lookhead: 0,
                looklegs: 0,
                looktype: 128,
                lookaddons: 0,
                posx: 100,
                posy: 100,
                posz: 7,
                cap: 400,
                lastlogin: 0,
                lastlogout: 0,
                lastip: 0,
                conditions: None,
                skulltime: 0,
                murder_timestamps: String::new(),
                skull: 0,
                town_id: 1,
                balance: 0,
                offlinetraining_time: 0,
                offlinetraining_skill: 0,
                stamina: 2520,
                skill_fist: 10,
                skill_fist_tries: 0,
                skill_club: 10,
                skill_club_tries: 0,
                skill_sword: 10,
                skill_sword_tries: 0,
                skill_axe: 10,
                skill_axe_tries: 0,
                skill_dist: 10,
                skill_dist_tries: 0,
                skill_shielding: 10,
                skill_shielding_tries: 0,
                skill_fishing: 10,
                skill_fishing_tries: 0,
                direction: 0,
                save: 1,
                onlinetime: 0,
                deletion: 0,
                food_remaining: 0,
                soul_cycle: 0,
                soul_count: 0,
                soul_max_count: 0,
            },
            premium_ends_at: 0,
            account_type: 1,
            spells: Vec::new(),
            storage: Vec::new(),
            vip_list: Vec::new(),
            guild: None,
            items: tfs_rust_db::player::PlayerItemPayload::default(),
        }
    }

    fn queue_offline_letter(world: &mut crate::game_world::GameWorld, pos: Position) -> ItemId {
        world.houses.name_to_guid.insert("Offline".into(), 99);
        let mut letter = Item::new_single(ITEM_LETTER);
        letter.set_text("Offline\nThais");
        let letter_id = world.items.insert(letter);
        world
            .internal_add_item_to_tile(pos, letter_id, crate::cylinder::CylinderFlags::NONE)
            .expect("drop");
        letter_id
    }

    #[test]
    fn login_before_ack_sees_mail() {
        let (mut world, pos) = setup_mailbox_world();
        queue_offline_letter(&mut world, pos);
        // No Tokio runtime in this test → append is not in flight; splice on apply.
        assert!(!world.mail_login_should_defer(99));
        let mut loaded = stub_loaded(99);
        world.apply_outbox_to_loaded(99, &mut loaded);
        assert!(
            loaded
                .items
                .depot
                .iter()
                .any(|r| r.itemtype == ITEM_LETTER_STAMPED),
            "stale load must receive spliced mail"
        );
        assert!(!world.mail_outbox.contains_key(&99));
    }

    #[test]
    fn login_after_ack_does_not_duplicate() {
        let (mut world, pos) = setup_mailbox_world();
        queue_offline_letter(&mut world, pos);
        let records = world.mail_outbox.get(&99).expect("outbox").pending[0]
            .records
            .clone();
        let appended: Vec<(i32, i32, u16)> = records
            .iter()
            .map(|r| (r.pid, r.sid, r.itemtype))
            .collect();
        world.apply_mail_delivery_finished(99, true, appended.clone());
        assert!(!world.mail_login_should_defer(99));
        let mut loaded = stub_loaded(99);
        loaded.items.depot = records;
        let n = loaded.items.depot.len();
        world.apply_outbox_to_loaded(99, &mut loaded);
        assert_eq!(
            loaded.items.depot.len(),
            n,
            "fresh load already has appended sids — no splice"
        );
        assert!(!world.mail_outbox.contains_key(&99));
    }

    #[test]
    fn house_dump_skips_online_guid() {
        let (mut world, _pos) = setup_mailbox_world();
        let start = Position::new(81, 80, 7);
        ensure_walkable_tile(&mut world.map, start, 100);
        let mut hero = test_player("Hero", start);
        hero.guid = 42;
        let cid = insert_player(&mut world, hero);
        world.player_by_name.insert("Hero".into(), cid);
        world.player_by_guid.insert(42, cid);
        let letter_id = world.items.insert(Item::new_single(ITEM_LETTER_STAMPED));
        assert!(world.apply_house_depot_dump_if_online(42, vec![letter_id], 1));
        let chest = world
            .player_get_depot_chest(cid, 1, false)
            .expect("live depot");
        assert!(
            world
                .container_registry
                .get(chest)
                .is_some_and(|c| c.items.contains(&letter_id))
        );
    }

    #[test]
    fn two_offline_letters_accumulate_in_outbox() {
        let (mut world, pos) = setup_mailbox_world();
        queue_offline_letter(&mut world, pos);
        queue_offline_letter(&mut world, pos);
        let pending = &world.mail_outbox.get(&99).expect("outbox").pending;
        assert_eq!(pending.len(), 2, "second letter must not overwrite the first");
        assert_eq!(
            pending
                .iter()
                .filter(|p| p.records.iter().any(|r| r.itemtype == ITEM_LETTER_STAMPED))
                .count(),
            2
        );
    }

    #[test]
    fn failed_append_keeps_outbox() {
        let (mut world, pos) = setup_mailbox_world();
        let letter_id = queue_offline_letter(&mut world, pos);
        world.apply_mail_delivery_finished(99, false, Vec::new());
        let pending = &world.mail_outbox.get(&99).expect("kept after fail").pending;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].item_ids, vec![letter_id]);
    }

    #[test]
    fn stale_load_after_ack_splices_missing_rows() {
        let (mut world, pos) = setup_mailbox_world();
        queue_offline_letter(&mut world, pos);
        let records = world.mail_outbox.get(&99).expect("outbox").pending[0]
            .records
            .clone();
        let appended: Vec<(i32, i32, u16)> = records
            .iter()
            .map(|r| (r.pid, r.sid, r.itemtype))
            .collect();
        world.apply_mail_delivery_finished(99, true, appended);
        let mut loaded = stub_loaded(99);
        world.apply_outbox_to_loaded(99, &mut loaded);
        assert!(
            loaded
                .items
                .depot
                .iter()
                .any(|r| r.itemtype == ITEM_LETTER_STAMPED),
            "ack then stale PlayerLoaded must still splice"
        );
        assert!(!world.mail_outbox.contains_key(&99));
    }

    #[test]
    fn stamp_after_detach_keeps_parent_none() {
        let (mut world, pos) = setup_mailbox_world();
        let letter_id = world.items.insert(Item::new_single(ITEM_LETTER));
        world
            .internal_add_item_to_tile(pos, letter_id, crate::cylinder::CylinderFlags::NONE)
            .expect("drop");
        world.detach_item_from_tile(pos, letter_id).expect("detach");
        assert!(
            world.items.get(letter_id).is_some_and(|i| i.parent.is_none()),
            "SendMail holds the letter with no cylinder parent"
        );
        assert!(
            world.discover_item_parent(letter_id).is_none(),
            "must not invent a tile parent (full-map scan) for detached mail"
        );
        world.change_item_type(letter_id, ITEM_LETTER_STAMPED);
        assert_eq!(
            world.items.get(letter_id).map(|i| i.item_type),
            Some(ITEM_LETTER_STAMPED)
        );
        assert!(
            world.items.get(letter_id).is_some_and(|i| i.parent.is_none()),
            "stamp must not re-parent onto a tile"
        );
    }

    #[test]
    fn failed_name_lookup_restores_letter_to_mailbox() {
        use slotmap::Key;
        let (mut world, pos) = setup_mailbox_world();
        let mut letter = Item::new_single(ITEM_LETTER);
        letter.set_text("Nobody\nThais");
        let letter_id = world.items.insert(letter);
        world
            .internal_add_item_to_tile(pos, letter_id, crate::cylinder::CylinderFlags::NONE)
            .expect("drop");
        world
            .detach_item_from_tile(pos, letter_id)
            .expect("hold");
        let ffi = letter_id.data().as_ffi();
        world.mail_lookup_holds.insert(ffi, pos);
        world.apply_mail_lookup_finished(ffi, 1, None);
        assert_eq!(
            world.items.get(letter_id).map(|i| i.item_type),
            Some(ITEM_LETTER)
        );
        assert!(world.item_still_on_mailbox(letter_id));
        assert!(world.mail_lookup_holds.is_empty());
    }

    #[test]
    fn lookup_success_delivers_held_letter_to_locker() {
        use slotmap::Key;
        let (mut world, pos) = setup_mailbox_world();
        let start = Position::new(81, 80, 7);
        ensure_walkable_tile(&mut world.map, start, 100);
        let mut hero = test_player("Hero", start);
        hero.guid = 42;
        let cid = insert_player(&mut world, hero);
        world.player_by_guid.insert(42, cid);

        let mut letter = Item::new_single(ITEM_LETTER);
        letter.set_text("Hero\nThais");
        let letter_id = world.items.insert(letter);
        world
            .internal_add_item_to_tile(pos, letter_id, crate::cylinder::CylinderFlags::NONE)
            .expect("drop");
        world
            .detach_item_from_tile(pos, letter_id)
            .expect("hold");
        let ffi = letter_id.data().as_ffi();
        world.mail_lookup_holds.insert(ffi, pos);
        world.apply_mail_lookup_finished(ffi, 1, Some(42));
        let locker = world.player_get_depot_locker(cid, 1).expect("locker");
        assert!(
            world
                .container_registry
                .get(locker)
                .is_some_and(|c| c.items.contains(&letter_id))
        );
        assert_eq!(
            world.items.get(letter_id).map(|i| i.item_type),
            Some(ITEM_LETTER_STAMPED)
        );
    }

    #[test]
    fn open_locker_gets_mail_at_front_slot() {
        let (mut world, pos) = setup_mailbox_world();
        let start = Position::new(81, 80, 7);
        ensure_walkable_tile(&mut world.map, start, 100);
        let mut hero = test_player("Hero", start);
        hero.guid = 42;
        let cid = insert_player(&mut world, hero);
        world.player_by_name.insert("Hero".into(), cid);
        world.player_by_guid.insert(42, cid);
        let locker = world.player_get_depot_locker(cid, 1).expect("open locker");
        world
            .container_registry
            .add_container(cid, locker, Some(0), 0);

        let mut letter = Item::new_single(ITEM_LETTER);
        letter.set_text("Hero\nThais");
        let letter_id = world.items.insert(letter);
        world
            .internal_add_item_to_tile(pos, letter_id, crate::cylinder::CylinderFlags::NONE)
            .expect("drop");

        let items = world
            .container_registry
            .get(locker)
            .map(|c| c.items.clone())
            .unwrap_or_default();
        assert_eq!(items.first().copied(), Some(letter_id));
        assert!(world.mail_window_open(cid, 1));
    }
}
