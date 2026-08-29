//! Native handlers for AID-keyed MoveEvents (3000–3123) — zero Lua on hot quest tiles.
//!
//! Corpus: `moveuse.cc` `MoveTop` / `EffectOnMap` (Collision actions).
//! Pack: TFS `MoveEvent::executeStep` / `executeAddRemItem` — `movement.cpp`.
//!
//! Compiled from movement revscripts at boot via [`tfs_rust_lua::compile_aid_move_handlers`].

use rustc_hash::FxHashMap;
use slotmap::Key;
use tfs_rust_common::Position;
use tfs_rust_lua::{
    AidMoveGate, AidMoveRelocSpec, CompiledAidMoveEntry, EffectPosition, MoveEventKind, RelocFrom,
    RelocTo,
};

use crate::creature::CreatureKind;
use crate::cylinder::CylinderFlags;
use crate::event_dispatcher::TileMoveEventItem;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};

const ITEM_MAGICWALL: u16 = 1497;
const ITEM_WILDGROWTH: u16 = 1499;
const CONST_ME_POFF: u8 = 3;

/// Boot-built registry of native move handlers keyed by `(kind, aid)`.
#[derive(Debug, Default, Clone)]
pub struct NativeAidMoveRegistry {
    entries: FxHashMap<(MoveEventKind, u16), CompiledAidMoveEntry>,
    town_ids: FxHashMap<String, u32>,
}

impl NativeAidMoveRegistry {
    /// Build from compiled script specs; resolve `setTown` names via OTBM town table.
    pub fn from_compiled(
        compiled: Vec<CompiledAidMoveEntry>,
        towns: &std::collections::HashMap<u32, tfs_rust_content::otbm::TownData>,
    ) -> Self {
        let mut town_ids = FxHashMap::default();
        for town in towns.values() {
            town_ids.insert(town.name.to_ascii_lowercase(), town.id);
        }
        let mut entries = FxHashMap::default();
        for entry in compiled {
            entries.insert((entry.kind, entry.aid), entry);
        }
        Self { entries, town_ids }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_native(&self, kind: MoveEventKind, aid: u16) -> bool {
        aid != 0 && self.entries.contains_key(&(kind, aid))
    }

    fn get(&self, kind: MoveEventKind, aid: u16) -> Option<&CompiledAidMoveEntry> {
        self.entries.get(&(kind, aid))
    }

    fn resolve_town_id(&self, name: &str) -> Option<u32> {
        self.town_ids.get(&name.to_ascii_lowercase()).copied()
    }
}

/// Native StepIn/StepOut dispatch — runs before Lua filtering in `fire_creature_step_events`.
pub fn on_creature_step(
    world: &mut GameWorld,
    cid: CreatureId,
    from: Position,
    to: Position,
    step_out_items: &[TileMoveEventItem],
    step_in_items: &[TileMoveEventItem],
) {
    let _ = (from, step_out_items);
    for item in step_in_items {
        if item.action_id == 0 {
            continue;
        }
        let Some(entry) = world
            .aid_move_handlers
            .get(MoveEventKind::StepIn, item.action_id)
            .cloned()
        else {
            continue;
        };
        if !gate_allows(&entry.gate, world, Some(cid)) {
            continue;
        }
        apply_entry(world, &entry, Some(cid), item.item_id, to);
    }
}

/// Native AddItem / AddItemItemTile — called from `fire_item_move_events` before Lua.
pub fn on_item_move_add(
    world: &mut GameWorld,
    moved_item: ItemId,
    pos: Position,
    tile_items: &[TileMoveEventItem],
) {
    for sibling in tile_items {
        if sibling.item_id == moved_item {
            continue;
        }
        if sibling.action_id == 0 {
            continue;
        }
        let kind = MoveEventKind::AddItemItemTile;
        let Some(entry) = world
            .aid_move_handlers
            .get(kind, sibling.action_id)
            .cloned()
        else {
            continue;
        };
        if !gate_allows(&entry.gate, world, None) {
            continue;
        }
        apply_entry(world, &entry, None, sibling.item_id, pos);
    }
}

/// True when any move-item handler for this transfer still requires Lua.
pub fn item_move_needs_lua(
    registry: &NativeAidMoveRegistry,
    moved_item: ItemId,
    item_action_id: u16,
    is_add: bool,
    tile_items: &[TileMoveEventItem],
) -> bool {
    let (kind, tile_kind) = if is_add {
        (MoveEventKind::AddItem, MoveEventKind::AddItemItemTile)
    } else {
        (MoveEventKind::RemoveItem, MoveEventKind::RemoveItemItemTile)
    };
    if item_action_id != 0 && !registry.is_native(kind, item_action_id) {
        return true;
    }
    tile_items.iter().any(|s| {
        s.item_id != moved_item && s.action_id != 0 && !registry.is_native(tile_kind, s.action_id)
    })
}

fn gate_allows(gate: &AidMoveGate, world: &GameWorld, actor: Option<CreatureId>) -> bool {
    match gate {
        AidMoveGate::None => true,
        AidMoveGate::IsPlayer => actor.is_some_and(|cid| matches!(world.creatures.get(cid), Some(CreatureKind::Player(_)))),
        AidMoveGate::PlayerLevelBelow { level } => {
            let Some(cid) = actor else {
                return false;
            };
            let Some(CreatureKind::Player(p)) = world.creatures.get(cid) else {
                return false;
            };
            p.level < i32::try_from(*level).unwrap_or(i32::MAX)
        }
        AidMoveGate::PlayerNotPremium => {
            let Some(cid) = actor else {
                return false;
            };
            !world.player_is_premium(cid)
        }
        AidMoveGate::VocationBranch { is_player, .. } => {
            if *is_player {
                actor.is_some_and(|cid| matches!(world.creatures.get(cid), Some(CreatureKind::Player(_))))
            } else {
                true
            }
        }
    }
}

fn apply_entry(
    world: &mut GameWorld,
    entry: &CompiledAidMoveEntry,
    actor: Option<CreatureId>,
    trigger_item: ItemId,
    pos: Position,
) {
    let item_pos = world
        .items
        .get(trigger_item)
        .and_then(|i| i.parent.as_ref())
        .and_then(|cyl| cyl.as_tile())
        .unwrap_or(pos);

    let reloc_to = resolve_reloc_to(world, &entry.gate, &entry.reloc, item_pos, actor);
    let from = resolve_reloc_from(&entry.reloc, item_pos);

    if from != reloc_to {
        let _ = native_do_relocate(world, from, reloc_to);
    }

    if let Some(effect) = &entry.effect {
        let effect_pos = resolve_effect_pos(world, &effect.position, item_pos);
        world.broadcast_magic_effect(effect_pos, effect.effect_id);
    }

    if let Some(town_name) = &entry.set_town
        && let (Some(cid), Some(town_id)) =
            (actor, world.aid_move_handlers.resolve_town_id(town_name))
    {
        let _ = world.lua_script_player_set_town(cid.data().as_ffi(), town_id);
    }
}

fn resolve_reloc_from(spec: &AidMoveRelocSpec, item_pos: Position) -> Position {
    let from = match spec {
        AidMoveRelocSpec::Single { from, .. } => *from,
        AidMoveRelocSpec::VocationBranch { from, .. } => *from,
    };
    match from {
        RelocFrom::ItemPosition => item_pos,
        RelocFrom::Absolute { x, y, z } => Position::new(x, y, z),
    }
}

fn resolve_reloc_to(
    world: &GameWorld,
    gate: &AidMoveGate,
    spec: &AidMoveRelocSpec,
    item_pos: Position,
    actor: Option<CreatureId>,
) -> Position {
    match spec {
        AidMoveRelocSpec::Single { to, .. } => resolve_reloc_to_pos(to, item_pos),
        AidMoveRelocSpec::VocationBranch {
            then_to,
            else_to,
            ..
        } => {
            let voc_match = match gate {
                AidMoveGate::VocationBranch { vocation_ids, .. } => {
                    vocation_matches(world, actor, vocation_ids)
                }
                _ => false,
            };
            if voc_match {
                resolve_reloc_to_pos(then_to, item_pos)
            } else {
                resolve_reloc_to_pos(else_to, item_pos)
            }
        }
    }
}

fn vocation_matches(world: &GameWorld, actor: Option<CreatureId>, ids: &[u8]) -> bool {
    let Some(cid) = actor else {
        return false;
    };
    let Some(CreatureKind::Player(p)) = world.creatures.get(cid) else {
        return false;
    };
    let vid = u8::try_from(p.vocation_id).unwrap_or(0);
    ids.contains(&vid)
}

fn resolve_reloc_to_pos(to: &RelocTo, item_pos: Position) -> Position {
    match to {
        RelocTo::Absolute { x, y, z } => Position::new(*x, *y, *z),
        RelocTo::ItemXOffset { dx, y, z } => Position::new(
            apply_i16_offset(item_pos.x, *dx),
            *y,
            *z,
        ),
        RelocTo::ItemRelative { dx, dy, z } => Position::new(
            apply_i16_offset(item_pos.x, *dx),
            apply_i16_offset(item_pos.y, *dy),
            *z,
        ),
    }
}

fn resolve_effect_pos(_world: &GameWorld, spec: &EffectPosition, item_pos: Position) -> Position {
    match spec {
        EffectPosition::Absolute { x, y, z } => Position::new(*x, *y, *z),
        EffectPosition::ItemPosition => item_pos,
        EffectPosition::ItemXOffset { dx, y, z } => Position::new(
            apply_i16_offset(item_pos.x, *dx),
            *y,
            *z,
        ),
        EffectPosition::ItemRelative { dx, dy, z } => Position::new(
            apply_i16_offset(item_pos.x, *dx),
            apply_i16_offset(item_pos.y, *dy),
            *z,
        ),
    }
}

fn apply_i16_offset(base: u16, delta: i16) -> u16 {
    i32::from(base)
        .saturating_add(i32::from(delta))
        .clamp(0, i32::from(u16::MAX)) as u16
}

/// Native `doRelocate(fromPos, toPos)` — `register_do_relocate` in `runtime.rs`.
fn native_do_relocate(world: &mut GameWorld, from: Position, to: Position) -> bool {
    if from == to {
        return false;
    }
    if world.map.get_tile(to).is_none() {
        return false;
    }
    let Some(from_tile) = world.map.get_tile(from) else {
        return false;
    };
    let body = from_tile.body().clone();
    let mut item_ids: Vec<ItemId> = Vec::new();
    if let Some(gid) = body.ground_item {
        item_ids.push(gid);
    }
    item_ids.extend(body.top_items.iter().copied());
    item_ids.extend(body.down_items.iter().copied());
    let creatures: Vec<CreatureId> = body.creatures.clone();

    for iid in item_ids.iter().rev() {
        let Some(item) = world.items.get(*iid) else {
            continue;
        };
        let Some(it) = world.items_db.items.get(&item.item_type) else {
            continue;
        };
        if it.is_ground_tile() || !it.moveable() {
            continue;
        }
        let _ = world.detach_item_from_tile(from, *iid);
        let _ = world.internal_add_item_to_tile(to, *iid, CylinderFlags::NONE);
    }

    for cid in creatures {
        if let Some(conn) = world.conn_for_creature(cid) {
            let _ = crate::walk::internal_teleport_player(world, conn, cid, to, true);
        } else {
            let old = world.creatures.get(cid).map(|k| k.position()).unwrap_or(from);
            world.move_creature_on_map(cid, old, to);
        }
    }

    strip_reloc_obstacles(world, from);
    true
}

fn strip_reloc_obstacles(world: &mut GameWorld, from: Position) {
    let Some(tile) = world.map.get_tile(from) else {
        return;
    };
    let ids: Vec<ItemId> = tile
        .body()
        .ground_item
        .into_iter()
        .chain(tile.body().top_items.iter().copied())
        .chain(tile.body().down_items.iter().copied())
        .collect();
    let mut poff = false;
    for iid in ids {
        let Some(item) = world.items.get(iid) else {
            continue;
        };
        let Some(it) = world.items_db.items.get(&item.item_type) else {
            continue;
        };
        let remove = item.item_type == ITEM_MAGICWALL
            || item.item_type == ITEM_WILDGROWTH
            || it.is_splash()
            || it.is_magic_field();
        if remove {
            let count = item.count.max(1);
            let _ = world.internal_remove_item_from_tile(from, iid, count);
            poff = true;
        }
    }
    if poff {
        world.broadcast_magic_effect(from, CONST_ME_POFF);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cylinder::Cylinder;
    use crate::item::Item;
    use crate::sim_harness::{insert_player, minimal_world, test_player};
    use crate::tile::{Tile, TileBody};
    use tfs_rust_common::ZoneType;

    fn place_ground(world: &mut GameWorld, pos: Position, type_id: u16, aid: u16) -> ItemId {
        let mut item = Item::new_single(type_id);
        if aid != 0 {
            item.set_action_id(aid);
        }
        item.parent = Some(Cylinder::Tile { pos });
        let iid = world.items.insert(item);
        world.map.insert_tile(
            pos,
            Tile::Normal(TileBody {
                ground: Some(type_id),
                ground_item: Some(iid),
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: 0,
                zone: ZoneType::Normal,
            }),
        );
        iid
    }

    #[test]
    fn native_registry_premium_bridge_entry() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("scripts/movements").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }
        let compiled = tfs_rust_lua::compile_aid_move_handlers(&data);
        let reg = NativeAidMoveRegistry::from_compiled(compiled, &std::collections::HashMap::new());
        assert!(
            reg.is_native(MoveEventKind::StepIn, 3052),
            "premium_bridge StepIn should compile"
        );
    }

    #[test]
    fn native_step_in_relocates_non_premium_player() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("scripts/movements").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }
        let compiled = tfs_rust_lua::compile_aid_move_handlers(&data);
        let mut world = minimal_world();
        world.aid_move_handlers =
            NativeAidMoveRegistry::from_compiled(compiled, &world.map.towns);

        let bridge = Position::new(32057, 32192, 7);
        let dest = Position::new(32060, 32192, 7);
        let pad = Position::new(32057, 32191, 7);
        place_ground(&mut world, bridge, 452, 3052);
        world.map.insert_tile(
            pad,
            Tile::Normal(TileBody {
                ground: Some(100),
                ground_item: None,
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: 0,
                zone: ZoneType::Normal,
            }),
        );
        world.map.insert_tile(
            dest,
            Tile::Normal(TileBody {
                ground: Some(100),
                ground_item: None,
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: 0,
                zone: ZoneType::Normal,
            }),
        );

        let cid = insert_player(&mut world, test_player("Walker", pad));
        world.map.register_creature_at(pad, cid);
        world.move_creature_on_map(cid, pad, bridge);
        world.flush_pending_creature_step_events();

        assert_eq!(
            world.creatures.get(cid).map(|k| k.position()),
            Some(dest),
            "native premium_bridge should relocate non-premium player"
        );
    }
}
