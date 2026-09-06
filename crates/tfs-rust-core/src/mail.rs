//! Mailbox delivery — 772 `SendMail` / `SendMails`.
//!
//! Corpus: `SendMail` / `SendMails` (`moveuse.cc:712-919`); Collision dat
//! `IsType(Obj1,3501|3508) -> SendMail(Obj2)`. Pack: TFS `mailbox.cpp` cylinder
//! `addThing` (same outcome; deliver to **town depot**, never 1098 inbox).

use slotmap::Key;
use tfs_rust_common::GameCommand;
use tfs_rust_common::Position;
use tfs_rust_db::player::PlayerStore;

use crate::cylinder::Cylinder;
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

    /// Async `GetCharacterID` result — stamp+queue if the item is still on a mailbox.
    pub(crate) fn apply_mail_lookup_finished(
        &mut self,
        item_ffi: u64,
        town_id: u32,
        guid: Option<u32>,
    ) {
        let item_id = ItemId::from(slotmap::KeyData::from_ffi(item_ffi));
        let Some(guid) = guid else {
            return;
        };
        if !self.item_still_on_mailbox(item_id) {
            return;
        }
        let Some(kind) = self.mail_kind(item_id) else {
            return;
        };
        if let Some(&cid) = self.player_by_guid.get(&guid) {
            let _ = self.deliver_mail_to_online(item_id, cid, town_id, kind);
        } else {
            let _ = self.deliver_mail_offline(item_id, guid, town_id, kind);
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
        let depot_open = self.player_get_depot_chest(cid, town_id, false).is_some();
        if depot_open
            && let Some(chest) = self.player_get_depot_chest(cid, town_id, false)
            && self.container_is_slot_full(chest)
        {
            return false;
        }
        let Some(pos) = self.mail_tile_pos(item_id) else {
            return false;
        };
        if self.detach_item_from_tile(pos, item_id).is_err() {
            return false;
        }
        self.stamp_mail(item_id, kind);
        self.house_add_item_to_town_depot(cid, town_id, item_id);
        if depot_open {
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
        let Some(pos) = self.mail_tile_pos(item_id) else {
            return false;
        };
        if self.detach_item_from_tile(pos, item_id).is_err() {
            return false;
        }
        self.stamp_mail(item_id, kind);
        self.houses
            .pending_depot_dumps
            .entry(guid)
            .or_default()
            .push(item_id);
        self.houses.pending_depot_town.insert(guid, town_id);
        true
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

    fn spawn_mail_lookup(&self, item_id: ItemId, town_id: u32, name: String) {
        let Some(sched) = self.scheduler.as_ref() else {
            return;
        };
        let tx = sched.ctrl_sender();
        let db = self.db.clone();
        let item_ffi = item_id.data().as_ffi();
        tokio::spawn(async move {
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
    fn letter_to_online_player_stamps_and_lands_in_town_depot() {
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
        let chest = world
            .player_get_depot_chest(cid, 1, false)
            .expect("depot created");
        assert!(
            world
                .container_registry
                .get(chest)
                .is_some_and(|c| c.items.contains(&letter_id))
        );
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
    }

    #[test]
    fn offline_guid_queues_pending_depot_dump() {
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
        assert_eq!(
            world.houses.pending_depot_dumps.get(&99),
            Some(&vec![letter_id])
        );
        assert_eq!(world.houses.pending_depot_town.get(&99), Some(&1));
    }
}
