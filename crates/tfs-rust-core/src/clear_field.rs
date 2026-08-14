//! Clear occupants when an item becomes `UNPASS` (door close).
//!
//! Outcomes: 772 `ClearField` / `MoveAllObjects` / `JumpPossible`
//! (`moveuse.cc:569-617`, `505-535`; `info.cc:702-726`).
//! Domain: TFS door scripts call this before `item:transform` to closed.

use slotmap::Key;
use tfs_rust_common::Position;
use tfs_rust_common::enums::Direction;

use crate::cylinder::{Cylinder, CylinderFlags};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::tile::flags as tilestate;

/// Direction scan order — must match `ClearField` OffsetX/Y (`moveuse.cc:586-587`).
const CLEAR_FIELD_DIRS: [Direction; 4] = [
    Direction::East,
    Direction::South,
    Direction::West,
    Direction::North,
];

impl GameWorld {
    /// 772 `ClearField(Obj, Exclude)` — shove stack mates off `exclude_item`'s tile before
    /// the item becomes `UNPASS` (open→closed door).
    ///
    /// `exclude_creature`: SeparationEvent passes the walker so they are not moved again
    /// (`moveuse.cc:2338`). Click-close uses `None`.
    pub(crate) fn clear_field(
        &mut self,
        exclude_item: ItemId,
        exclude_creature: Option<CreatureId>,
    ) {
        let Some(pos) = self.script_item_position(exclude_item) else {
            return;
        };

        let Some(dest) = self.clear_field_find_dest(pos) else {
            return;
        };
        if dest == pos {
            return;
        }

        self.move_all_objects_from_tile(pos, dest, exclude_item, exclude_creature);
    }

    /// First `BANK && !UNPASS`, else `JumpPossible` — same E/S/W/N order as decompile.
    fn clear_field_find_dest(&self, origin: Position) -> Option<Position> {
        for dir in CLEAR_FIELD_DIRS {
            let dest = origin.offset(dir);
            if self.tile_is_bank_and_passable_for_clear(dest) {
                return Some(dest);
            }
        }
        for dir in CLEAR_FIELD_DIRS {
            let dest = origin.offset(dir);
            if self.jump_possible(dest, false) {
                return Some(dest);
            }
        }
        None
    }

    /// `CoordinateFlag(BANK) && !CoordinateFlag(UNPASS)` for ClearField pass 1.
    fn tile_is_bank_and_passable_for_clear(&self, pos: Position) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        let body = tile.body();
        let Some(ground) = body.ground else {
            return false;
        };
        if !self.items_db.is_terrain_bank(ground) {
            return false;
        }
        if (body.flags & tilestate::BLOCKSOLID) != 0 {
            return false;
        }
        !body
            .down_items
            .iter()
            .chain(body.top_items.iter())
            .any(|&iid| {
                self.items
                    .get(iid)
                    .is_some_and(|it| self.items_db.is_unpassable(it.item_type))
            })
    }

    /// 772 `JumpPossible` (`info.cc:702-726`). `avoid_players` unused by ClearField (`false`).
    pub(crate) fn jump_possible(&self, pos: Position, avoid_players: bool) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        let body = tile.body();
        let Some(ground) = body.ground else {
            return false;
        };
        if !self.items_db.is_terrain_bank(ground) {
            return false;
        }

        for &iid in body.down_items.iter().chain(body.top_items.iter()) {
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                continue;
            };
            // `UNPASS && UNMOVE` → hard fail.
            if it.block_solid() && !it.moveable() {
                return false;
            }
        }

        if avoid_players {
            for &cid in &body.creatures {
                if matches!(
                    self.creatures.get(cid),
                    Some(crate::creature::CreatureKind::Player(_))
                ) {
                    return false;
                }
            }
        }
        true
    }

    /// 772 `MoveAllObjects` with `MoveUnmovable = true` (`moveuse.cc:505-535`).
    /// Processes stack in reverse (decompile recurses `getNextObject` first).
    fn move_all_objects_from_tile(
        &mut self,
        from: Position,
        to: Position,
        exclude_item: ItemId,
        exclude_creature: Option<CreatureId>,
    ) {
        let (item_ids, creature_ids) = {
            let Some(tile) = self.map.get_tile(from) else {
                return;
            };
            let body = tile.body();
            let mut items: Vec<ItemId> = body
                .down_items
                .iter()
                .chain(body.top_items.iter())
                .copied()
                .filter(|&iid| iid != exclude_item)
                .collect();
            items.reverse();
            let creatures: Vec<CreatureId> = body
                .creatures
                .iter()
                .copied()
                .filter(|&cid| exclude_creature != Some(cid))
                .collect();
            (items, creatures)
        };

        for cid in creature_ids {
            let _ = self.lua_script_creature_teleport(cid.data().as_ffi(), to.x, to.y, to.z, true);
        }

        for iid in item_ids {
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let typ = item.item_type;
            let count = item.count;
            let Some(it) = self.items_db.items.get(&typ) else {
                continue;
            };
            if it.is_magic_field() || it.is_splash() {
                let _ = self.internal_remove_item_from_tile(from, iid, u16::MAX);
                continue;
            }
            // MoveUnmovable=true — attempt move even for UNMOVE; failures are ignored
            // (decompile logs RESULT and continues).
            let _ = self.internal_move_item(
                None,
                Cylinder::Tile { pos: from },
                Cylinder::Tile { pos: to },
                iid,
                count,
                CylinderFlags::NO_LIMIT,
                None,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cylinder::CylinderFlags;
    use crate::item::Item;
    use crate::sim_harness::minimal_world;
    use crate::test_world::support::{insert_player, test_player};
    use crate::tile::{Tile, flags};
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

    fn place_bank(world: &mut GameWorld, pos: Position) {
        world.map.insert_tile(pos, Tile::empty_normal());
        if let Some(t) = world.map.get_tile_mut(pos) {
            t.body_mut().ground = Some(100);
        }
    }

    #[test]
    fn clear_field_prefers_east_then_south() {
        let mut world = minimal_world();
        let mut g = ItemType::default();
        g.group = 1;
        register_type(&mut world, 100, g);

        let door_pos = Position::new(100, 100, 7);
        let east = Position::new(101, 100, 7);
        let south = Position::new(100, 101, 7);
        for p in [door_pos, east, south] {
            place_bank(&mut world, p);
        }
        // Block east — must fall through to south.
        if let Some(t) = world.map.get_tile_mut(east) {
            t.body_mut().flags |= flags::BLOCKSOLID;
        }

        const DOOR: u16 = 9301;
        let mut door_ty = ItemType::default();
        door_ty.block_solid_override = Some(false);
        register_type(&mut world, DOOR, door_ty);

        let door = world.items.insert(Item::new_single(DOOR));
        world
            .internal_add_item_to_tile(door_pos, door, CylinderFlags::NO_LIMIT)
            .expect("door");

        let player = insert_player(&mut world, test_player("DoorStand", door_pos));
        world.map.register_creature_at(door_pos, player);
        world.clear_field(door, None);

        let after = world.creatures.get(player).unwrap().position();
        assert_eq!(after, south, "east blocked → shove south");
        assert!(
            world
                .map
                .get_tile(door_pos)
                .unwrap()
                .body()
                .creatures
                .is_empty()
        );
    }

    #[test]
    fn clear_field_jump_possible_when_all_unpass() {
        let mut world = minimal_world();
        let mut g = ItemType::default();
        g.group = 1;
        register_type(&mut world, 100, g);

        const BOX: u16 = 9302;
        let mut box_ty = ItemType::default();
        box_ty.block_solid_override = Some(true);
        box_ty.moveable_override = Some(true);
        register_type(&mut world, BOX, box_ty);

        const DOOR: u16 = 9303;
        register_type(&mut world, DOOR, ItemType::default());

        let door_pos = Position::new(110, 110, 7);
        let neighbors = [
            Position::new(111, 110, 7),
            Position::new(110, 111, 7),
            Position::new(109, 110, 7),
            Position::new(110, 109, 7),
        ];
        for p in std::iter::once(door_pos).chain(neighbors) {
            place_bank(&mut world, p);
        }
        // All four neighbors: movable UNPASS (fails pass-1, passes JumpPossible).
        for p in neighbors {
            let box_id = world.items.insert(Item::new_single(BOX));
            world
                .internal_add_item_to_tile(p, box_id, CylinderFlags::NO_LIMIT)
                .expect("box");
        }

        let door = world.items.insert(Item::new_single(DOOR));
        world
            .internal_add_item_to_tile(door_pos, door, CylinderFlags::NO_LIMIT)
            .expect("door");
        let player = insert_player(&mut world, test_player("JumpOut", door_pos));
        world.map.register_creature_at(door_pos, player);

        world.clear_field(door, None);
        assert_eq!(
            world.creatures.get(player).unwrap().position(),
            neighbors[0],
            "JumpPossible picks east first"
        );
    }

    #[test]
    fn clear_field_removes_magic_field_on_door_tile() {
        let mut world = minimal_world();
        let mut g = ItemType::default();
        g.group = 1;
        register_type(&mut world, 100, g);

        const FIELD: u16 = 9304;
        let mut field = ItemType::default();
        field.type_tag = 6; // MAGICFIELD
        register_type(&mut world, FIELD, field);
        register_type(&mut world, 9305, ItemType::default());

        let door_pos = Position::new(120, 120, 7);
        let east = Position::new(121, 120, 7);
        for p in [door_pos, east] {
            place_bank(&mut world, p);
        }

        let door = world.items.insert(Item::new_single(9305));
        world
            .internal_add_item_to_tile(door_pos, door, CylinderFlags::NO_LIMIT)
            .unwrap();
        let field_id = world.items.insert(Item::new_single(FIELD));
        world
            .internal_add_item_to_tile(door_pos, field_id, CylinderFlags::NO_LIMIT)
            .unwrap();

        world.clear_field(door, None);
        assert!(
            world.items.get(field_id).is_none(),
            "magic field deleted on door tile, not moved"
        );
    }
}
