//! MoveEvent registry — XML equip/deequip + revscript StepIn/StepOut/AddItem.
//!
//! C++ reference: `src/movement.cpp` `MoveEvents`, `MoveEvent::fireEquip` /
//! `executeStep`, `executeAddRemItem`, `MoveEvents::registerLuaEvent`,
//! `MoveEvents::getEvent(Item*)`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::combat_scripts::collect_lua_files;
use crate::runtime::{CallbackRef, LuaError, LuaRuntime, PendingMoveEvent};

/// `MoveEvent_t` — `movement.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveEventKind {
    Equip,
    DeEquip,
    AddItem,
    RemoveItem,
    /// `MOVE_EVENT_ADD_ITEM_ITEMTILE` — sibling tile item, not the moved item.
    AddItemItemTile,
    /// `MOVE_EVENT_REMOVE_ITEM_ITEMTILE` — sibling tile item, not the moved item.
    RemoveItemItemTile,
    StepIn,
    StepOut,
}

impl MoveEventKind {
    /// Parse `:type("stepin")` / XML `@event` names (case-insensitive).
    pub fn from_type_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "equip" => Some(Self::Equip),
            "deequip" => Some(Self::DeEquip),
            "additem" => Some(Self::AddItem),
            "removeitem" => Some(Self::RemoveItem),
            "stepin" => Some(Self::StepIn),
            "stepout" => Some(Self::StepOut),
            _ => None,
        }
    }

    /// TFS `registerLuaEvent` `:tileItem(true)` remap — `movement.cpp:243-255`.
    ///
    /// Only AddItem/RemoveItem change bucket; StepIn/Out keep their kind (TFS no-op).
    pub fn with_tile_item(self, tile_item: bool) -> Self {
        if !tile_item {
            return self;
        }
        match self {
            Self::AddItem => Self::AddItemItemTile,
            Self::RemoveItem => Self::RemoveItemItemTile,
            other => other,
        }
    }

    /// Lua callback field on the pending `MoveEvent` table (`compat.lua` `__newindex`).
    pub fn script_callback_field(self) -> &'static str {
        match self {
            Self::StepIn => "onStepIn",
            Self::StepOut => "onStepOut",
            Self::Equip => "onEquip",
            Self::DeEquip => "onDeEquip",
            Self::AddItem | Self::AddItemItemTile => "onAddItem",
            Self::RemoveItem | Self::RemoveItemItemTile => "onRemoveItem",
        }
    }
}

#[derive(Debug)]
pub struct MoveEventEntry {
    pub kind: MoveEventKind,
    pub item_id: u16,
    pub slot_mask: u32,
    pub req_level: u32,
    pub callback: CallbackRef,
}

/// Action-id keyed step/equip events (`MoveEvents::actionIdMap`).
#[derive(Debug)]
pub struct MoveEventAidEntry {
    pub kind: MoveEventKind,
    pub action_id: u16,
    pub slot_mask: u32,
    pub req_level: u32,
    pub callback: CallbackRef,
}

/// Revscript MoveEvent definition drained from `MoveEvent():register()`.
///
/// C++ reference: `movement.h` `MoveEvent` — item/action id ranges + script callback.
#[derive(Debug)]
pub struct MoveEventDef {
    pub kind: MoveEventKind,
    pub item_ids: Vec<u16>,
    pub action_ids: Vec<u16>,
    pub slot_mask: u32,
    pub req_level: u32,
    pub callback: Option<Arc<mlua::RegistryKey>>,
}

impl From<PendingMoveEvent> for MoveEventDef {
    fn from(pending: PendingMoveEvent) -> Self {
        Self {
            kind: pending.kind,
            item_ids: pending.item_ids,
            action_ids: pending.action_ids,
            slot_mask: pending.slot_mask,
            req_level: pending.req_level,
            callback: pending.callback.map(Arc::new),
        }
    }
}

/// Registry of move-event callbacks keyed by `(kind, item_id)` (and aid map).
#[derive(Debug, Default)]
pub struct MoveEventsRegistry {
    by_item: HashMap<(MoveEventKind, u16), MoveEventEntry>,
    by_aid: HashMap<(MoveEventKind, u16), MoveEventAidEntry>,
}

impl MoveEventsRegistry {
    pub fn get(&self, kind: MoveEventKind, item_id: u16) -> Option<&MoveEventEntry> {
        self.by_item.get(&(kind, item_id))
    }

    pub fn get_by_aid(&self, kind: MoveEventKind, action_id: u16) -> Option<&MoveEventAidEntry> {
        self.by_aid.get(&(kind, action_id))
    }

    /// TFS `MoveEvents::getEvent(Item*, MoveEvent_t)` — `movement.cpp:366-397`.
    ///
    /// Unique-id map skipped this pass (no `:uid` scripts). Aid hit returns the first
    /// registered event for that `(kind, aid)` and does not consult itemid. Aid miss
    /// (unset, or no event of this kind) falls through to itemid — C++ does the same
    /// when `actionIdMap` has no list for this `eventType`.
    pub fn get_event(
        &self,
        kind: MoveEventKind,
        item_type: u16,
        action_id: u16,
    ) -> Option<&CallbackRef> {
        if action_id != 0
            && let Some(entry) = self.get_by_aid(kind, action_id)
        {
            return Some(&entry.callback);
        }
        self.get(kind, item_type).map(|e| &e.callback)
    }

    pub fn len(&self) -> usize {
        self.by_item.len() + self.by_aid.len()
    }

    /// Count of action-id keyed events (`MoveEvents::actionIdMap`).
    pub fn aid_len(&self) -> usize {
        self.by_aid.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_item.is_empty() && self.by_aid.is_empty()
    }

    /// First registered event per `(kind, item_id)` wins — C++ `addEvent` + `begin()`.
    pub fn register(&mut self, entry: MoveEventEntry) {
        let key = (entry.kind, entry.item_id);
        if let std::collections::hash_map::Entry::Vacant(slot) = self.by_item.entry(key) {
            slot.insert(entry);
        } else {
            tracing::warn!(
                kind = ?key.0,
                item_id = key.1,
                "duplicate MoveEvent item id; keeping first"
            );
        }
    }

    /// First registered event per `(kind, action_id)` wins — C++ `addEvent` + `begin()`.
    pub fn register_aid(&mut self, entry: MoveEventAidEntry) {
        let key = (entry.kind, entry.action_id);
        if let std::collections::hash_map::Entry::Vacant(slot) = self.by_aid.entry(key) {
            slot.insert(entry);
        } else {
            tracing::warn!(
                kind = ?key.0,
                action_id = key.1,
                "duplicate MoveEvent action id; keeping first"
            );
        }
    }

    /// Load `movements/lib/movements.lua` and ensure default equip globals exist.
    ///
    /// C++ ref: `MoveEvents::load` loads lib before XML callback registration.
    fn ensure_movement_globals(runtime: &mut LuaRuntime, data_dir: &Path) -> Result<(), LuaError> {
        let lib_path = data_dir.join("movements/lib/movements.lua");
        if lib_path.exists() {
            let path_string = lib_path.display().to_string();
            if let Err(e) = runtime.load_script(&path_string) {
                tracing::warn!("Failed to load movements lib {}: {e}", lib_path.display());
            }
        }

        const DEFAULT_EQUIP_GLOBALS: &str = r#"
if onEquipItem == nil then
    function onEquipItem(player, item, slot, isCheck)
        return true
    end
end
if onDeEquipItem == nil then
    function onDeEquipItem(player, item, slot, isCheck)
        return true
    end
end
"#;
        runtime.exec_chunk("movements_defaults", DEFAULT_EQUIP_GLOBALS)?;
        Ok(())
    }

    /// Parse `data/movements/movements.xml` equip/deequip entries with `function="..."`.
    pub fn load_from_xml(
        &mut self,
        runtime: &mut LuaRuntime,
        data_dir: &Path,
    ) -> Result<(), LuaError> {
        Self::ensure_movement_globals(runtime, data_dir)?;

        let xml_path = data_dir.join("movements/movements.xml");
        if !xml_path.exists() {
            tracing::warn!("movements.xml not found: {}", xml_path.display());
            return Ok(());
        }
        let xml = std::fs::read_to_string(&xml_path)
            .map_err(|e| LuaError::ScriptIo(xml_path.display().to_string(), e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct MovementsXml {
            #[serde(rename = "movevent", default)]
            movevents: Vec<MoveventXml>,
        }
        #[derive(serde::Deserialize)]
        struct MoveventXml {
            #[serde(rename = "@event")]
            event: String,
            #[serde(rename = "@itemid")]
            itemid: Option<u16>,
            #[serde(rename = "@fromid")]
            fromid: Option<u16>,
            #[serde(rename = "@toid")]
            toid: Option<u16>,
            #[serde(rename = "@slot")]
            slot: Option<String>,
            #[serde(rename = "@level")]
            level: Option<u32>,
            #[serde(rename = "@function")]
            function: Option<String>,
        }

        let parsed: MovementsXml =
            quick_xml::de::from_str(&xml).map_err(|e| LuaError::SyntaxError(e.to_string()))?;

        for mv in parsed.movevents {
            let Some(function) = mv.function else {
                continue;
            };
            let Some(kind) = MoveEventKind::from_type_name(&mv.event) else {
                continue;
            };

            let ids: Vec<u16> = if let Some(id) = mv.itemid {
                vec![id]
            } else if let (Some(from), Some(to)) = (mv.fromid, mv.toid) {
                (from..=to).collect()
            } else {
                continue;
            };

            let slot_mask = parse_slot_mask(mv.slot.as_deref().unwrap_or(""));
            let req_level = mv.level.unwrap_or(0);

            for item_id in ids {
                let callback = runtime.register_callback(
                    format!("move:{function}:{:?}:{item_id}", kind),
                    &function,
                )?;
                self.register(MoveEventEntry {
                    kind,
                    item_id,
                    slot_mask,
                    req_level,
                    callback,
                });
            }
        }

        Ok(())
    }
}

/// Load all revscript MoveEvents from `data/scripts/movements/**/*.lua`.
///
/// C++ reference: `MoveEvents::registerLuaEvent` (adapted to revscript scan).
pub fn load_move_event_scripts(
    runtime: &mut LuaRuntime,
    data_dir: &Path,
) -> Result<Vec<MoveEventDef>, LuaError> {
    let dir = data_dir.join("scripts/movements");
    if !dir.exists() {
        tracing::warn!("Movements directory not found: {}", dir.display());
        return Ok(Vec::new());
    }

    let mut lua_files: Vec<PathBuf> = Vec::new();
    collect_lua_files(&dir, &mut lua_files);
    lua_files.sort();

    for path in &lua_files {
        let path_string = path.display().to_string();
        if let Err(e) = runtime.load_move_event_script(&path_string) {
            tracing::warn!("Failed to load movement script {path_string}: {e}");
        }
    }

    let pending = runtime.drain_pending_move_events();
    let defs: Vec<MoveEventDef> = pending.into_iter().map(Into::into).collect();
    tracing::info!(
        count = defs.len(),
        files = lua_files.len(),
        "Loaded move event scripts"
    );
    Ok(defs)
}

/// Apply revscript defs into an existing registry (after XML load).
pub fn merge_move_event_defs(
    registry: &mut MoveEventsRegistry,
    runtime: &LuaRuntime,
    defs: Vec<MoveEventDef>,
) {
    for def in defs {
        let Some(key) = def.callback.as_ref() else {
            tracing::warn!(
                ?def.kind,
                item_ids = ?def.item_ids,
                "MoveEvent has no callback; skipping"
            );
            continue;
        };
        let Ok(func) = runtime.lua.registry_value::<mlua::Function>(key.as_ref()) else {
            tracing::warn!(?def.kind, "MoveEvent callback registry lookup failed");
            continue;
        };

        for &item_id in &def.item_ids {
            let Ok(reg_key) = runtime.lua.create_registry_value(func.clone()) else {
                continue;
            };
            registry.register(MoveEventEntry {
                kind: def.kind,
                item_id,
                slot_mask: def.slot_mask,
                req_level: def.req_level,
                callback: CallbackRef::from_registry_key(reg_key),
            });
        }
        for &action_id in &def.action_ids {
            let Ok(reg_key) = runtime.lua.create_registry_value(func.clone()) else {
                continue;
            };
            registry.register_aid(MoveEventAidEntry {
                kind: def.kind,
                action_id,
                slot_mask: def.slot_mask,
                req_level: def.req_level,
                callback: CallbackRef::from_registry_key(reg_key),
            });
        }
    }
}

fn parse_slot_mask(slot: &str) -> u32 {
    match slot.to_ascii_lowercase().as_str() {
        "head" => 1 << 0,
        "necklace" => 1 << 1,
        "backpack" => 1 << 2,
        "armor" => 1 << 3,
        "right-hand" | "right" => 1 << 4,
        "left-hand" | "left" | "hand" | "shield" => (1 << 4) | (1 << 5),
        "legs" => 1 << 6,
        "feet" => 1 << 7,
        "ring" => 1 << 8,
        "ammo" => 1 << 9,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::inject_door_tables_from_global;
    use std::path::PathBuf;

    fn workspace_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    #[test]
    fn slot_mask_feet_and_hand() {
        assert_eq!(parse_slot_mask("feet"), 1 << 7);
        assert_eq!(parse_slot_mask("hand"), (1 << 4) | (1 << 5));
    }

    #[test]
    fn closing_doors_registers_open_quest_and_level() {
        let data_root = workspace_data_root();
        let closing = data_root.join("scripts/movements/other/closing_doors.lua");
        if !closing.exists() {
            eprintln!("closing_doors.lua not found — skipping");
            return;
        }

        let mut runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");
        runtime
            .load_move_event_script(closing.to_str().expect("utf8"))
            .expect("closing_doors.lua should load");
        let pending = runtime.drain_pending_move_events();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].kind, MoveEventKind::StepOut);
        // Open quest door 1224 (= closed 1223 + 1); open level 1228 (= 1227 + 1).
        assert!(
            pending[0].item_ids.contains(&1224),
            "expected open quest door 1224; got {:?}",
            pending[0].item_ids
        );
        assert!(
            pending[0].item_ids.contains(&1228),
            "expected open level door 1228; got {:?}",
            pending[0].item_ids
        );
        assert!(pending[0].callback.is_some());
    }

    #[test]
    fn level_doors_registers_step_in() {
        let data_root = workspace_data_root();
        let level = data_root.join("scripts/movements/other/level_doors.lua");
        if !level.exists() {
            eprintln!("level_doors.lua not found — skipping");
            return;
        }

        let mut runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");
        runtime
            .load_move_event_script(level.to_str().expect("utf8"))
            .expect("level_doors.lua should load");
        let pending = runtime.drain_pending_move_events();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].kind, MoveEventKind::StepIn);
        assert!(pending[0].item_ids.contains(&1228));
        assert!(pending[0].callback.is_some());
    }

    fn dummy_step_callback(runtime: &mut LuaRuntime, name: &str) -> CallbackRef {
        runtime
            .exec_chunk(
                name,
                &format!(
                    "function {name}(creature, item, position, fromPosition) _G.fired = '{name}' return true end"
                ),
            )
            .expect("chunk");
        runtime
            .register_callback(name.to_string(), name)
            .expect("callback")
    }

    fn fired_marker(runtime: &LuaRuntime) -> String {
        runtime
            .lua
            .globals()
            .get::<String>("fired")
            .unwrap_or_default()
    }

    fn fire_step(runtime: &LuaRuntime, callback: &CallbackRef) {
        let pos = tfs_rust_common::Position::new(100, 100, 7);
        runtime
            .call_move_step(callback, 1, 1, pos, pos)
            .expect("call_move_step");
    }

    /// M1: tile item with actionid 3052 fires the aid callback, not a same-type `:id()` trap.
    #[test]
    fn get_event_aid_wins_over_itemid_and_fires() {
        let mut runtime = LuaRuntime::new().expect("runtime");
        let mut registry = MoveEventsRegistry::default();
        let trap = dummy_step_callback(&mut runtime, "trap_id");
        let aid = dummy_step_callback(&mut runtime, "aid_3052");
        registry.register(MoveEventEntry {
            kind: MoveEventKind::StepIn,
            item_id: 1510,
            slot_mask: 0,
            req_level: 0,
            callback: trap,
        });
        registry.register_aid(MoveEventAidEntry {
            kind: MoveEventKind::StepIn,
            action_id: 3052,
            slot_mask: 0,
            req_level: 0,
            callback: aid,
        });

        let resolved = registry
            .get_event(MoveEventKind::StepIn, 1510, 3052)
            .expect("aid event");
        fire_step(&runtime, resolved);
        assert_eq!(fired_marker(&runtime), "aid_3052");

        let id_only = registry
            .get_event(MoveEventKind::StepIn, 1510, 0)
            .expect("itemid event");
        fire_step(&runtime, id_only);
        assert_eq!(fired_marker(&runtime), "trap_id");
    }

    /// C++ `getEvent`: aid attribute set but no StepIn for that aid → fall through to itemid.
    #[test]
    fn get_event_aid_miss_falls_through_to_itemid() {
        let mut runtime = LuaRuntime::new().expect("runtime");
        let mut registry = MoveEventsRegistry::default();
        let trap = dummy_step_callback(&mut runtime, "trap_fallthrough");
        registry.register(MoveEventEntry {
            kind: MoveEventKind::StepIn,
            item_id: 1510,
            slot_mask: 0,
            req_level: 0,
            callback: trap,
        });
        registry.register_aid(MoveEventAidEntry {
            kind: MoveEventKind::AddItem,
            action_id: 3052,
            slot_mask: 0,
            req_level: 0,
            callback: dummy_step_callback(&mut runtime, "aid_additem_only"),
        });

        let resolved = registry
            .get_event(MoveEventKind::StepIn, 1510, 3052)
            .expect("itemid fallthrough");
        fire_step(&runtime, resolved);
        assert_eq!(fired_marker(&runtime), "trap_fallthrough");
    }

    #[test]
    fn register_keeps_first_aid_and_item_event() {
        let mut runtime = LuaRuntime::new().expect("runtime");
        let mut registry = MoveEventsRegistry::default();
        registry.register_aid(MoveEventAidEntry {
            kind: MoveEventKind::StepIn,
            action_id: 3052,
            slot_mask: 0,
            req_level: 0,
            callback: dummy_step_callback(&mut runtime, "first_aid"),
        });
        registry.register_aid(MoveEventAidEntry {
            kind: MoveEventKind::StepIn,
            action_id: 3052,
            slot_mask: 0,
            req_level: 0,
            callback: dummy_step_callback(&mut runtime, "second_aid"),
        });
        registry.register(MoveEventEntry {
            kind: MoveEventKind::StepIn,
            item_id: 1510,
            slot_mask: 0,
            req_level: 0,
            callback: dummy_step_callback(&mut runtime, "first_id"),
        });
        registry.register(MoveEventEntry {
            kind: MoveEventKind::StepIn,
            item_id: 1510,
            slot_mask: 0,
            req_level: 0,
            callback: dummy_step_callback(&mut runtime, "second_id"),
        });

        fire_step(
            &runtime,
            registry
                .get_event(MoveEventKind::StepIn, 1510, 3052)
                .expect("aid"),
        );
        assert_eq!(fired_marker(&runtime), "first_aid");
        fire_step(
            &runtime,
            registry
                .get_event(MoveEventKind::StepIn, 1510, 0)
                .expect("id"),
        );
        assert_eq!(fired_marker(&runtime), "first_id");
    }

    /// Load the pack: 133 files, non-zero `by_aid`, rookgaard bridge aids, live aid-over-id lookup.
    #[test]
    fn load_movements_registers_aid_and_prefers_bridge_over_trap() {
        let data_root = workspace_data_root();
        let dir = data_root.join("scripts/movements");
        if !dir.exists() {
            eprintln!("scripts/movements not found — skipping");
            return;
        }

        let mut files = Vec::new();
        collect_lua_files(&dir, &mut files);
        assert_eq!(
            files.len(),
            133,
            "expected 133 movement scripts; got {}",
            files.len()
        );

        let mut runtime = LuaRuntime::new().expect("runtime");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");
        let defs = load_move_event_scripts(&mut runtime, &data_root).expect("load");
        let mut registry = MoveEventsRegistry::default();
        merge_move_event_defs(&mut registry, &runtime, defs);

        assert!(
            registry.aid_len() > 0,
            "aid map must be populated after load"
        );
        assert!(
            registry.get_by_aid(MoveEventKind::StepIn, 3052).is_some(),
            "rookgaard premium_bridge.lua aid 3052"
        );
        assert!(
            registry.get_by_aid(MoveEventKind::StepIn, 3051).is_some(),
            "rookgaard level_2_bridge.lua aid 3051"
        );
        assert!(
            registry
                .get_by_aid(MoveEventKind::AddItemItemTile, 3052)
                .is_some(),
            "premium_bridge.lua :tileItem(true) remaps to ITEMTILE"
        );
        assert!(
            registry.get_by_aid(MoveEventKind::AddItem, 3052).is_none(),
            "aid 3052 AddItem must not stay in the non-ITEMTILE bucket"
        );

        let trap = registry
            .get(MoveEventKind::StepIn, 1510)
            .expect("trap.lua :id(1510)");
        let aid = registry
            .get_by_aid(MoveEventKind::StepIn, 3052)
            .expect("aid 3052");
        let resolved = registry
            .get_event(MoveEventKind::StepIn, 1510, 3052)
            .expect("get_event aid");
        assert!(
            std::ptr::eq(resolved, &aid.callback),
            "aid 3052 must win over trap itemid 1510"
        );
        let id_only = registry
            .get_event(MoveEventKind::StepIn, 1510, 0)
            .expect("get_event itemid");
        assert!(
            std::ptr::eq(id_only, &trap.callback),
            "aid 0 must hit :id() trap"
        );
        assert!(
            registry.get(MoveEventKind::StepIn, 1511).is_some(),
            "trap.lua :id(1511) blades"
        );
        assert!(
            registry.get(MoveEventKind::StepIn, 1506).is_none(),
            "fields.lua must not register StepIn 1506"
        );
    }

    fn dummy_add_item_callback(runtime: &mut LuaRuntime, name: &str) -> CallbackRef {
        runtime
            .exec_chunk(
                name,
                &format!(
                    "function {name}(item, tileitem, position) _G.move_id = item.itemid; _G.tile_id = tileitem.itemid; _G.pos_x = position.x; _G.fired = '{name}' return true end"
                ),
            )
            .expect("chunk");
        runtime
            .register_callback(name.to_string(), name)
            .expect("callback")
    }

    struct AddItemCtx {
        moved_type: u16,
        tile_type: u16,
    }

    impl tfs_rust_common::ScriptContext for AddItemCtx {
        fn get_creature(
            &self,
            _: tfs_rust_common::ScriptCreatureId,
        ) -> Option<tfs_rust_common::ScriptCreatureData> {
            None
        }
        fn get_item(
            &self,
            id: tfs_rust_common::ScriptItemId,
        ) -> Option<tfs_rust_common::ScriptItemRef> {
            Some(tfs_rust_common::ScriptItemRef(id))
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_item_data(
            &self,
            id: tfs_rust_common::ScriptItemId,
        ) -> Option<tfs_rust_common::ScriptItemData> {
            let item_type = if id == 1 {
                self.moved_type
            } else {
                self.tile_type
            };
            Some(tfs_rust_common::ScriptItemData {
                item_type,
                count: 1,
                weight: 0,
                name: "test".into(),
                action_id: if id == 1 { 0 } else { 3052 },
                unique_id: 0,
                is_store_item: false,
                fluid_type: 0,
                sub_type: 0,
            })
        }
    }

    /// `:tileItem(true)` remaps AddItem → ITEMTILE; StepIn on the same file stays StepIn.
    #[test]
    fn tile_item_true_remaps_additem_to_itemtile() {
        let data_root = workspace_data_root();
        let bridge = data_root.join("scripts/movements/map/rookgaard/premium_bridge.lua");
        if !bridge.exists() {
            eprintln!("premium_bridge.lua not found — skipping");
            return;
        }

        let mut runtime = LuaRuntime::new().expect("runtime");
        runtime
            .load_move_event_script(bridge.to_str().expect("utf8"))
            .expect("premium_bridge.lua should load");
        let pending = runtime.drain_pending_move_events();
        assert_eq!(pending.len(), 2);
        assert!(
            pending
                .iter()
                .any(|e| e.kind == MoveEventKind::StepIn && e.action_ids.contains(&3052)),
            "StepIn aid 3052; got {pending:?}"
        );
        assert!(
            pending.iter().any(|e| {
                e.kind == MoveEventKind::AddItemItemTile && e.action_ids.contains(&3052)
            }),
            "AddItem+tileItem → ITEMTILE; got {pending:?}"
        );
        assert!(
            pending.iter().all(|e| e.kind != MoveEventKind::AddItem),
            "non-ITEMTILE AddItem must not remain after remap"
        );
    }

    /// M2: `onAddItem` Lua sees `tileitem.itemid` of the tile sibling, not the dropped item.
    /// No actor is passed (TFS `executeAddRemItem`).
    #[test]
    fn call_move_item_sees_tileitem_id_not_moved_item() {
        let mut runtime = LuaRuntime::new().expect("runtime");
        let callback = dummy_add_item_callback(&mut runtime, "on_add");
        let pos = tfs_rust_common::Position::new(100, 100, 7);
        let ctx = AddItemCtx {
            moved_type: 2148,
            tile_type: 1510,
        };
        crate::context::with_lua_context(&ctx, || {
            runtime
                .call_move_item(&callback, 1, Some(2), pos)
                .expect("call_move_item");
        });
        assert_eq!(fired_marker(&runtime), "on_add");
        let move_id: u16 = runtime.lua.globals().get("move_id").unwrap_or(0);
        let tile_id: u16 = runtime.lua.globals().get("tile_id").unwrap_or(0);
        let pos_x: u16 = runtime.lua.globals().get("pos_x").unwrap_or(0);
        assert_eq!(move_id, 2148, "moveitem is the dropped item");
        assert_eq!(tile_id, 1510, "tileitem is the sibling, not the drop");
        assert_eq!(pos_x, 100);

        crate::context::with_lua_context(&ctx, || {
            runtime
                .call_move_item(&callback, 1, None, pos)
                .expect("call_move_item nullptr tileitem");
        });
        let tile_id: u16 = runtime.lua.globals().get("tile_id").unwrap_or(99);
        assert_eq!(tile_id, 0, "nullptr tileitem is pushThing zero table");
    }

    /// ITEMTILE aid is a distinct kind from ADD_ITEM — M1 get_event must not mix them.
    #[test]
    fn get_event_itemtile_kind_is_separate_from_additem() {
        let mut runtime = LuaRuntime::new().expect("runtime");
        let mut registry = MoveEventsRegistry::default();
        registry.register_aid(MoveEventAidEntry {
            kind: MoveEventKind::AddItemItemTile,
            action_id: 3052,
            slot_mask: 0,
            req_level: 0,
            callback: dummy_add_item_callback(&mut runtime, "itemtile_3052"),
        });
        registry.register(MoveEventEntry {
            kind: MoveEventKind::AddItem,
            item_id: 2148,
            slot_mask: 0,
            req_level: 0,
            callback: dummy_add_item_callback(&mut runtime, "additem_2148"),
        });

        assert!(
            registry
                .get_event(MoveEventKind::AddItem, 2148, 3052)
                .is_some()
        );
        let additem = registry
            .get_event(MoveEventKind::AddItem, 2148, 3052)
            .expect("ADD_ITEM fallthrough");
        let tile = registry
            .get_event(MoveEventKind::AddItemItemTile, 100, 3052)
            .expect("ITEMTILE aid");
        let pos = tfs_rust_common::Position::new(1, 1, 7);
        let ctx = AddItemCtx {
            moved_type: 2148,
            tile_type: 1510,
        };
        crate::context::with_lua_context(&ctx, || {
            runtime
                .call_move_item(additem, 1, None, pos)
                .expect("additem");
        });
        assert_eq!(fired_marker(&runtime), "additem_2148");
        crate::context::with_lua_context(&ctx, || {
            runtime
                .call_move_item(tile, 1, Some(2), pos)
                .expect("itemtile");
        });
        assert_eq!(fired_marker(&runtime), "itemtile_3052");
    }
}
