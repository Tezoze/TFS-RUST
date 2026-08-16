//! Tile specials: trashholder consume + teleport dest + item-type magic puffs.
//!
//! Domain: TFS `TrashHolder::addThing` / `Teleport::addThing` (`trashholder.cpp`, `teleport.cpp`).
//! Outcomes: XML `effect` as 772 `CONST_ME_*` wire byte.

use tfs_rust_common::Position;

use crate::cylinder::CylinderFlags;
use crate::game_world::GameWorld;
use crate::ids::ItemId;
use crate::tile::flags as tilestate;
use crate::walk::internal_teleport_player;

impl GameWorld {
    /// After an item is on a tile — trash consume or teleport dest (`Tile::addThing` specials).
    pub(crate) fn apply_tile_item_specials(&mut self, pos: Position, item_id: ItemId) {
        let flags = self
            .map
            .get_tile(pos)
            .map(|t| t.body().flags)
            .unwrap_or(0);
        if flags & tilestate::TRASHHOLDER != 0 {
            self.apply_trashholder_consume(pos, item_id);
            return;
        }
        if flags & tilestate::TELEPORT != 0 {
            self.apply_teleport_item(pos, item_id);
        }
    }

    /// After a creature lands on a tile — TFS `Teleport::addThing` creature arm.
    pub(crate) fn apply_tile_creature_specials(&mut self, cid: crate::ids::CreatureId, pos: Position) {
        let flags = self
            .map
            .get_tile(pos)
            .map(|t| t.body().flags)
            .unwrap_or(0);
        if flags & tilestate::TELEPORT == 0 {
            return;
        }
        let Some((dest, effect)) = self.tile_teleport_dest_and_effect(pos) else {
            return;
        };
        if dest == pos {
            return;
        }
        if effect != 0 {
            self.broadcast_magic_effect(pos, effect);
            self.broadcast_magic_effect(dest, effect);
        }
        if let Some(conn) = self.conn_for_creature(cid) {
            let _ = internal_teleport_player(self, conn, cid, dest, false);
        } else {
            self.move_creature_on_map(cid, pos, dest);
        }
    }

    fn apply_trashholder_consume(&mut self, pos: Position, item_id: ItemId) {
        let Some(item) = self.items.get(item_id) else {
            return;
        };
        let Some(it) = self.items_db.items.get(&item.item_type) else {
            return;
        };
        if it.is_trashholder() {
            return;
        }
        if it.is_hangable() {
            let supports = self
                .map
                .get_tile(pos)
                .is_some_and(|t| t.body().flags & tilestate::SUPPORTS_HANGABLE != 0);
            if supports {
                return;
            }
        }
        let effect = self.tile_special_magic_effect(pos, |t| t.is_trashholder());
        let count = item.count.max(1);
        let _ = self.internal_remove_item_from_tile(pos, item_id, count);
        if effect != 0 {
            self.broadcast_magic_effect(pos, effect);
        }
    }

    fn apply_teleport_item(&mut self, pos: Position, item_id: ItemId) {
        let Some(item) = self.items.get(item_id) else {
            return;
        };
        if self
            .items_db
            .items
            .get(&item.item_type)
            .is_some_and(|t| t.is_teleport())
        {
            return;
        }
        let Some((dest, effect)) = self.tile_teleport_dest_and_effect(pos) else {
            return;
        };
        if dest == pos {
            return;
        }
        if effect != 0 {
            self.broadcast_magic_effect(pos, effect);
            self.broadcast_magic_effect(dest, effect);
        }
        if self.detach_item_from_tile(pos, item_id).is_ok() {
            let _ = self.internal_add_item_to_tile(dest, item_id, CylinderFlags::NO_LIMIT);
        }
    }

    fn tile_teleport_dest_and_effect(&self, pos: Position) -> Option<(Position, u8)> {
        let tele_id = self.tile_special_item(pos, |t| t.is_teleport())?;
        let dest = self.items.get(tele_id).and_then(|i| i.tele_dest())?;
        let effect = self
            .items
            .get(tele_id)
            .and_then(|i| self.items_db.items.get(&i.item_type))
            .map(|t| t.magic_effect)
            .unwrap_or(0);
        Some((dest, effect))
    }

    fn tile_special_magic_effect(
        &self,
        pos: Position,
        pred: impl Fn(&tfs_rust_content::otb::ItemType) -> bool,
    ) -> u8 {
        self.tile_special_item(pos, pred)
            .and_then(|id| self.items.get(id))
            .and_then(|i| self.items_db.items.get(&i.item_type))
            .map(|t| t.magic_effect)
            .unwrap_or(0)
    }

    fn tile_special_item(
        &self,
        pos: Position,
        pred: impl Fn(&tfs_rust_content::otb::ItemType) -> bool,
    ) -> Option<ItemId> {
        let body = self.map.get_tile(pos)?.body();
        body.ground_item
            .into_iter()
            .chain(body.down_items.iter().copied())
            .chain(body.top_items.iter().copied())
            .find(|&id| {
                self.items
                    .get(id)
                    .and_then(|i| self.items_db.items.get(&i.item_type))
                    .is_some_and(&pred)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Item;
    use crate::sim_harness::{beat_driven_test_world, ensure_walkable_tile};
    use crate::tile::flags as tile_flags;
    use std::sync::Arc;
    use tfs_rust_content::items::ITEM_TYPE_TRASHHOLDER;
    use tfs_rust_content::otb::ItemType;

    #[test]
    fn trashholder_consumes_dropped_item_and_has_effect() {
        let mut world = beat_driven_test_world();
        let mut db = (*world.items_db).clone();
        db.items.insert(
            708,
            ItemType {
                id: 708,
                server_id: 708,
                type_tag: ITEM_TYPE_TRASHHOLDER,
                magic_effect: 3,
                ..ItemType::default()
            },
        );
        db.items.insert(
            3031,
            ItemType {
                id: 3031,
                server_id: 3031,
                ..ItemType::default()
            },
        );
        world.items_db = Arc::new(db);

        let pos = Position::new(80, 80, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        if let Some(t) = world.map.get_tile_mut(pos) {
            t.body_mut().flags |= tile_flags::TRASHHOLDER;
        }
        let tar = world.items.insert(Item::new_single(708));
        world
            .internal_add_item_to_tile(pos, tar, CylinderFlags::NO_LIMIT)
            .expect("place tar");
        let gold = world.items.insert(Item::new_single(3031));
        world
            .internal_add_item_to_tile(pos, gold, CylinderFlags::NONE)
            .expect("drop gold");
        assert!(
            world.items.get(gold).is_none(),
            "trashholder must remove the dropped item"
        );
    }
}
