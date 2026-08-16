//! Item userdata for Lua (`Item` in TFS scripts).
//!
//! C++ reference: `src/luascript.cpp` — `LuaScriptInterface` item userdata (`Item::getID`, `getName`, …).

use mlua::{Lua, MetaMethod, UserData, UserDataFields, UserDataMethods, Value};
use std::cell::RefCell;

use crate::context::{CURRENT_CTX, CreatureRef, ItemData, ItemRef, LuaContext};
use crate::lua_mutation::{
    LuaMoveDestination, call_lua_item_decay, call_lua_item_move_to, call_lua_item_remove,
    call_lua_item_transform, call_lua_set_action_id, call_lua_set_custom_attribute,
    call_lua_set_store_item, call_lua_set_unique_id,
};
use crate::userdata::container::ContainerRef;
use crate::userdata::position::PositionRef;
use crate::userdata::tile::TileRef;

/// Register the Item metatable in the Lua runtime.
pub fn register_item_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<ItemRef>(|_registry| {})
}

/// TFS `LuaScriptInterface::setItemMetatable` (`luascript.cpp`).
///
/// `Game.createItem` / `Container:addItem` push `Item*` then set Container vs
/// Item metatable from the live object. mlua userdata types are distinct, so
/// container types must be `ContainerRef` or `reward:addItem(...)` is nil
/// (`onUseQuest` bag/backpack `content`).
pub(crate) fn push_item_userdata(lua: &Lua, item_id: u64) -> Result<Value, mlua::Error> {
    let is_container = crate::context::current_ctx(|ctx| {
        ctx.is_registered_container(item_id)
            || ctx
                .get_item_data(item_id)
                .is_some_and(|d| ctx.get_item_type_is_container(d.item_type))
    })
    .unwrap_or(false);
    if is_container {
        Ok(Value::UserData(lua.create_userdata(ContainerRef(item_id))?))
    } else {
        Ok(Value::UserData(lua.create_userdata(ItemRef(item_id))?))
    }
}

/// Resolve Item or Container userdata to a script item id.
pub(crate) fn item_script_id_from_value(value: &Value) -> Option<u64> {
    let Value::UserData(ud) = value else {
        return None;
    };
    if let Ok(item) = ud.borrow::<ItemRef>() {
        return Some(item.0);
    }
    if let Ok(cont) = ud.borrow::<ContainerRef>() {
        return Some(cont.0);
    }
    None
}

/// Resolve a Lua item-id argument — number or name (`luaGameCreateItem` / `luaTileAddItem`).
pub(crate) fn parse_lua_item_type_id(value: Value) -> Result<Option<u16>, mlua::Error> {
    match value {
        Value::Integer(n) if n > 0 && n <= i64::from(u16::MAX) => Ok(Some(n as u16)),
        Value::Number(n) if n > 0.0 && n <= f64::from(u16::MAX) => Ok(Some(n as u16)),
        Value::String(s) => {
            let name = s.to_str()?.to_owned();
            Ok(crate::context::current_ctx(|ctx| ctx.get_item_type_id_by_name(&name)).flatten())
        }
        _ => Ok(None),
    }
}

fn with_ctx<F, R>(f: F) -> Result<R, mlua::Error>
where
    F: FnOnce(&dyn LuaContext) -> Result<R, mlua::Error>,
{
    CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
        let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
        if ptr.is_null() {
            return Err(mlua::Error::runtime("LuaContext not set"));
        }
        let ctx = unsafe { &*ptr };
        f(ctx)
    })
}

fn push_cylinder(lua: &Lua, cyl: tfs_rust_common::ScriptCylinder) -> Result<Value, mlua::Error> {
    match cyl {
        tfs_rust_common::ScriptCylinder::Player(id) => {
            let ud = lua.create_userdata(CreatureRef(id))?;
            Ok(Value::UserData(ud))
        }
        tfs_rust_common::ScriptCylinder::Container(id) => {
            let ud = lua.create_userdata(ContainerRef(id))?;
            Ok(Value::UserData(ud))
        }
        tfs_rust_common::ScriptCylinder::Tile(pos) => {
            let ud = lua.create_userdata(TileRef {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            })?;
            Ok(Value::UserData(ud))
        }
    }
}

fn parse_move_destination(_lua: &Lua, value: Value) -> Result<LuaMoveDestination, mlua::Error> {
    match value {
        Value::UserData(ud) => {
            // TFS `luaItemMoveTo`: Position userdata uses `getPosition` (`luascript.cpp`).
            if let Ok(pos) = ud.borrow::<PositionRef>() {
                return Ok(LuaMoveDestination::Tile {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                });
            }
            if let Ok(cref) = ud.borrow::<CreatureRef>() {
                return Ok(LuaMoveDestination::Player {
                    creature_id: cref.0,
                });
            }
            if let Ok(cont) = ud.borrow::<ContainerRef>() {
                return Ok(LuaMoveDestination::Container { item_id: cont.0 });
            }
            if let Ok(item) = ud.borrow::<ItemRef>() {
                return Ok(LuaMoveDestination::Container { item_id: item.0 });
            }
            Err(mlua::Error::runtime("invalid moveTo destination"))
        }
        Value::Table(t) => {
            let x: u16 = t.get("x")?;
            let y: u16 = t.get("y")?;
            let z: u8 = t.get("z")?;
            Ok(LuaMoveDestination::Tile { x, y, z })
        }
        _ => Err(mlua::Error::runtime("invalid moveTo destination")),
    }
}

impl UserData for ItemRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "Item");
    }

    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        // TFS compat `item.itemid` → `Item:getId()` (`data/lib/compat/compat.lua`).
        // Required by `food.lua` / door key branches without loading full compat.
        fields.add_field_method_get("itemid", |_, this| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d: ItemData| d.item_type)
                    .unwrap_or(0))
            })
        });
        // TFS compat `item.actionid` → `Item:getActionId()` (`compat.lua` ItemIndex).
        // Gap 3: `functions.lua` reads `ground.actionid` / `target.actionid`.
        fields.add_field_method_get("actionid", |_, this| {
            with_ctx(|ctx| Ok(ctx.get_item_data(this.0).map(|d| d.action_id).unwrap_or(0)))
        });
        // TFS compat `item.uid` → `Item:getUniqueId()` (`compat.lua` ItemIndex).
        fields.add_field_method_get("uid", |_, this| {
            with_ctx(|ctx| Ok(ctx.get_item_data(this.0).map(|d| d.unique_id).unwrap_or(0)))
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // C++ `Item::getID()` — server item type id (e.g. 2260 blank rune), not the
        // SlotMap instance key. `luascript.cpp` `luaItemGetId`. Required by
        // `Player:conjureItem` (`leftItem:getId() == reagentId`) and door/lever scripts.
        methods.add_method("getId", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d: ItemData| d.item_type)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getType", |lua, this, ()| {
            use crate::userdata::item_type::ItemTypeRef;
            let typ = with_ctx(|ctx| {
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d: ItemData| d.item_type)
                    .unwrap_or(0))
            })?;
            let ud = lua.create_userdata(ItemTypeRef(typ))?;
            Ok(Value::UserData(ud))
        });

        methods.add_method("getCount", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d: ItemData| d.count)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getWeight", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d: ItemData| d.weight)
                    .unwrap_or(0))
            })
        });

        methods.add_method("getName", |_, this, ()| {
            with_ctx(|ctx| {
                ctx.get_item_data(this.0)
                    .map(|d: ItemData| d.name)
                    .ok_or_else(|| mlua::Error::runtime("item not found"))
            })
        });

        methods.add_method("getActionId", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.get_item_data(this.0).map(|d| d.action_id).unwrap_or(0)))
        });

        // `item:getFluidType()` — `luascript.cpp` `luaItemGetFluidType`.
        // `Tile.relocateTo` (`lib/core/tile.lua`) skips splash/fluid items via this.
        methods.add_method("getFluidType", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.get_item_data(this.0).map(|d| d.fluid_type).unwrap_or(0)))
        });

        methods.add_method("setActionId", |_, this, action_id: u16| {
            call_lua_set_action_id(this.0, action_id).map_err(mlua::Error::runtime)
        });

        methods.add_method("getUniqueId", |_, this, ()| {
            // TFS `luaItemGetUniqueId` (`luascript.cpp:6495`): returns ATTR_UNIQUE_ID
            // if set, otherwise registers via `addThing` and returns a local UID > 65535.
            with_ctx(|ctx| {
                let uid = ctx.get_item_data(this.0).map(|d| d.unique_id).unwrap_or(0);
                if uid != 0 {
                    return Ok(uid);
                }
                Ok(ctx.register_script_item_uid(this.0))
            })
        });

        methods.add_method("setUniqueId", |_, this, unique_id: u16| {
            call_lua_set_unique_id(this.0, unique_id).map_err(mlua::Error::runtime)
        });

        methods.add_method("isStoreItem", |_, this, ()| {
            with_ctx(|ctx| {
                Ok(ctx
                    .get_item_data(this.0)
                    .map(|d| d.is_store_item)
                    .unwrap_or(false))
            })
        });

        methods.add_method("setStoreItem", |_, this, store: bool| {
            call_lua_set_store_item(this.0, store).map_err(mlua::Error::runtime)
        });

        methods.add_method("isContainer", |_, this, ()| {
            with_ctx(|ctx| Ok(ctx.is_registered_container(this.0)))
        });

        methods.add_method("getContainer", |lua, this, ()| {
            let is_cont = with_ctx(|ctx| Ok(ctx.is_registered_container(this.0)))?;
            if is_cont {
                let ud = lua.create_userdata(ContainerRef(this.0))?;
                Ok(Value::UserData(ud))
            } else {
                Ok(Value::Nil)
            }
        });

        methods.add_method("getParent", |lua, this, ()| {
            let parent = with_ctx(|ctx| Ok(ctx.get_item_parent(this.0)))?;
            match parent {
                Some(cyl) => push_cylinder(lua, cyl),
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getTopParent", |lua, this, ()| {
            let parent = with_ctx(|ctx| Ok(ctx.get_item_top_parent(this.0)))?;
            match parent {
                Some(cyl) => push_cylinder(lua, cyl),
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("getPosition", |lua, this, ()| {
            let pos = with_ctx(|ctx| {
                ctx.get_item_position(this.0)
                    .ok_or_else(|| mlua::Error::runtime("item not found"))
            })?;
            let ud = lua.create_userdata(PositionRef {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            })?;
            Ok(Value::UserData(ud))
        });

        methods.add_method(
            "moveTo",
            |lua, this, (dest, flags): (Value, Option<u32>)| {
                let dest = parse_move_destination(lua, dest)?;
                let flags = flags.unwrap_or(0);
                call_lua_item_move_to(this.0, dest, flags).map_err(mlua::Error::runtime)
            },
        );

        methods.add_method("remove", |_, this, count: Option<i32>| {
            let count = count.unwrap_or(-1);
            call_lua_item_remove(this.0, count).map_err(mlua::Error::runtime)
        });

        // `item:isItem()` — `luascript.cpp` `luaItemIsItem` (always true for Item userdata).
        methods.add_method("isItem", |_, _this, ()| Ok(true));

        // `item:isCreature()` — Thing discriminator used by doors.lua / talkactions scripts.
        methods.add_method("isCreature", |_, _this, ()| Ok(false));

        // `item:hasAttribute(key)` — `luascript.cpp` `luaItemHasAttribute`.
        // Number = TFS bitflag; string = Remere custom attr (`keynumber`, …).
        methods.add_method("hasAttribute", |_, this, key: Value| match key {
            Value::Nil => Ok(false),
            Value::Integer(n) => {
                let attr_bits = n as u32;
                if attr_bits == 0 {
                    return Ok(false);
                }
                with_ctx(|ctx| Ok(ctx.item_has_attribute(this.0, attr_bits)))
            }
            Value::Number(n) => {
                let attr_bits = n as u32;
                if attr_bits == 0 {
                    return Ok(false);
                }
                with_ctx(|ctx| Ok(ctx.item_has_attribute(this.0, attr_bits)))
            }
            Value::String(s) => {
                let key = s.to_str()?.to_string();
                with_ctx(|ctx| Ok(ctx.item_has_custom_attribute(this.0, &key)))
            }
            _ => Ok(false),
        });

        // `item:getAttribute(key)` — `luascript.cpp` `luaItemGetAttribute`.
        methods.add_method("getAttribute", |lua, this, key: Value| match key {
            Value::Integer(n) => {
                let attr_bits = n as u32;
                let v = with_ctx(|ctx| Ok(ctx.item_get_int_attribute(this.0, attr_bits)))?;
                Ok(match v {
                    Some(i) => Value::Integer(i),
                    None => Value::Integer(0),
                })
            }
            Value::Number(n) => {
                let attr_bits = n as u32;
                let v = with_ctx(|ctx| Ok(ctx.item_get_int_attribute(this.0, attr_bits)))?;
                Ok(match v {
                    Some(i) => Value::Integer(i),
                    None => Value::Integer(0),
                })
            }
            Value::String(s) => {
                let key = s.to_str()?.to_string();
                let v = with_ctx(|ctx| Ok(ctx.item_get_custom_attribute(this.0, &key)))?;
                Ok(match v {
                    Some(tfs_rust_common::ScriptAttrValue::Integer(i)) => Value::Integer(i),
                    Some(tfs_rust_common::ScriptAttrValue::Float(f)) => Value::Number(f),
                    Some(tfs_rust_common::ScriptAttrValue::Boolean(b)) => Value::Boolean(b),
                    Some(tfs_rust_common::ScriptAttrValue::String(s)) => {
                        Value::String(lua.create_string(&s)?)
                    }
                    None => Value::Nil,
                })
            }
            _ => Ok(Value::Nil),
        });

        // `item:setAttribute(key, value)` — bitflag ints + Remere string custom attrs.
        methods.add_method("setAttribute", |_, this, (key, value): (Value, Value)| {
            let int_val = match value {
                Value::Integer(i) => i,
                Value::Number(f) => f as i64,
                _ => {
                    return Err(mlua::Error::runtime("setAttribute: expected integer value"));
                }
            };
            let bits = match &key {
                Value::Integer(i) => Some(*i as u32),
                Value::Number(f) => Some(*f as u32),
                _ => None,
            };
            if let Some(bits) = bits {
                const ACTION_ID: u32 = 1 << 0;
                const UNIQUE_ID: u32 = 1 << 1;
                if bits == ACTION_ID {
                    call_lua_set_action_id(this.0, int_val as u16).map_err(mlua::Error::runtime)?;
                    return Ok(true);
                }
                if bits == UNIQUE_ID {
                    call_lua_set_unique_id(this.0, int_val as u16).map_err(mlua::Error::runtime)?;
                    return Ok(true);
                }
                return Ok(false);
            }
            if let Value::String(s) = key {
                let key = s.to_str()?.to_string();
                call_lua_set_custom_attribute(this.0, key, int_val)
                    .map_err(mlua::Error::runtime)?;
                return Ok(true);
            }
            Ok(false)
        });

        // `item:transform(itemId[, count/subType])` — `luascript.cpp`
        // `luaItemTransform` → `Game::transformItem`. PC-3a Phase 5.
        methods.add_method(
            "transform",
            |_, this, (item_id, sub_type): (mlua::Value, Option<i32>)| {
                let new_type: u16 = match item_id {
                    mlua::Value::Integer(n) => n as u16,
                    mlua::Value::Number(n) => n as u16,
                    mlua::Value::String(s) => {
                        let name = s.to_str()?.to_string();
                        with_ctx(|ctx| {
                            ctx.get_item_type_id_by_name(&name)
                                .ok_or_else(|| mlua::Error::runtime("unknown item name"))
                        })?
                    }
                    _ => return Err(mlua::Error::runtime("invalid item type")),
                };
                let sub_type = sub_type.unwrap_or(-1);
                call_lua_item_transform(this.0, new_type, sub_type).map_err(mlua::Error::runtime)
            },
        );

        // `item:decay()` — TFS `luaItemDecay` → `Item::startDecaying` → `Game::startDecay`.
        methods.add_method("decay", |_, this, ()| {
            call_lua_item_decay(this.0).map_err(mlua::Error::runtime)
        });

        // Gap 7b — `__index` fallback so `item:getType()` / `item:isCreature()`
        // resolve methods defined as `function Item.getType(self, ...)` in
        // `data/lib/core/item.lua`. Native methods above keep priority.
        // C++ `LuaScriptInterface::registerClass`; shared helper in
        // `class_registry`.
        methods.add_meta_method(MetaMethod::Index, |lua, _this, key: mlua::LuaString| {
            crate::class_registry::class_index_lookup(
                lua,
                crate::class_registry::ITEM_INDEX_CHAIN,
                key,
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::with_lua_context;
    use mlua::Lua;
    use std::collections::HashMap;
    use tfs_rust_common::{
        ScriptAttrValue, ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemData,
        ScriptItemId, ScriptItemRef, ScriptThing, remere_attr,
    };

    struct KeyAttrCtx {
        attrs: HashMap<String, i64>,
        top: Option<ScriptThing>,
    }

    impl ScriptContext for KeyAttrCtx {
        fn get_creature(&self, _: ScriptCreatureId) -> Option<ScriptCreatureData> {
            None
        }
        fn get_item(&self, id: ScriptItemId) -> Option<ScriptItemRef> {
            Some(ScriptItemRef(id))
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_item_data(&self, id: ScriptItemId) -> Option<ScriptItemData> {
            Some(ScriptItemData {
                item_type: if id == 1 { 1209 } else { 2088 },
                count: 1,
                weight: 0,
                name: "test".into(),
                action_id: if id == 1 { 4004 } else { 0 },
                unique_id: 0,
                is_store_item: false,
                fluid_type: if id == 1 { 5 } else { 0 },
            })
        }
        fn item_has_custom_attribute(&self, _: ScriptItemId, key: &str) -> bool {
            self.attrs.contains_key(key)
        }
        fn item_get_custom_attribute(&self, _: ScriptItemId, key: &str) -> Option<ScriptAttrValue> {
            self.attrs.get(key).copied().map(ScriptAttrValue::Integer)
        }
        fn tile_get_top_visible_thing(
            &self,
            _: u16,
            _: u16,
            _: u8,
            _: Option<ScriptCreatureId>,
        ) -> Option<ScriptThing> {
            self.top
        }
        fn tile_exists(&self, _: u16, _: u16, _: u8) -> bool {
            true
        }
    }

    #[test]
    fn item_is_item_and_keyhole_attr_roundtrip_via_lua() {
        let lua = Lua::new();
        register_item_metatable(&lua).expect("item mt");
        crate::userdata::register_tile_constructor(&lua).expect("tile");
        crate::constants::register_constants(&lua).expect("constants");

        let mut attrs = HashMap::new();
        attrs.insert(remere_attr::KEYHOLENUMBER.to_string(), 42);
        attrs.insert(remere_attr::KEYNUMBER.to_string(), 42);
        let ctx = KeyAttrCtx {
            attrs,
            top: Some(ScriptThing::Item(1)),
        };

        with_lua_context(&ctx, || {
            let door = lua.create_userdata(ItemRef(1)).expect("door");
            let key = lua.create_userdata(ItemRef(2)).expect("key");
            lua.globals().set("door", door).unwrap();
            lua.globals().set("key", key).unwrap();

            let is_item: bool = lua.load("return door:isItem()").eval().unwrap();
            assert!(is_item);

            let has: bool = lua
                .load("return door:hasAttribute(ITEM_ATTRIBUTE_KEYHOLENUMBER)")
                .eval()
                .unwrap();
            assert!(has);

            let match_ok: bool = lua
                .load(
                    "return key:getAttribute(ITEM_ATTRIBUTE_KEYNUMBER) == door:getAttribute(ITEM_ATTRIBUTE_KEYHOLENUMBER)",
                )
                .eval()
                .unwrap();
            assert!(match_ok);

            let itemid: u16 = lua.load("return door.itemid").eval().unwrap();
            assert_eq!(itemid, 1209);

            let aid: u16 = lua.load("return door.actionid").eval().unwrap();
            assert_eq!(aid, 4004);

            let fluid: u16 = lua.load("return door:getFluidType()").eval().unwrap();
            assert_eq!(fluid, 5);

            let top_is_item: bool = lua
                .load("return Tile(100, 100, 7):getTopVisibleThing():isItem()")
                .eval()
                .unwrap();
            assert!(top_is_item);
        });
    }

    /// Rope `thing:moveTo(toPosition:moveUpstairs())` — TFS `luaItemMoveTo` accepts
    /// Position userdata via `getPosition` (`luascript.cpp`).
    #[test]
    fn move_to_parses_position_userdata() {
        let lua = Lua::new();
        crate::userdata::register_position_metatable(&lua).expect("Position");
        let dest: Value = lua
            .load("return Position(32316, 32226, 6)")
            .eval()
            .expect("Position()");
        let parsed = parse_move_destination(&lua, dest).expect("parse");
        assert_eq!(
            parsed,
            LuaMoveDestination::Tile {
                x: 32316,
                y: 32226,
                z: 6
            }
        );
    }

    /// R2: `Game.createItem` returns Container userdata for bag 1987 so
    /// `reward:addItem` works (`onUseQuest` content). Gold 2148 stays Item.
    struct R2CreateItemCtx;

    impl ScriptContext for R2CreateItemCtx {
        fn get_creature(&self, _: ScriptCreatureId) -> Option<ScriptCreatureData> {
            None
        }
        fn get_item(&self, id: ScriptItemId) -> Option<ScriptItemRef> {
            Some(ScriptItemRef(id))
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_item_data(&self, id: ScriptItemId) -> Option<ScriptItemData> {
            let item_type = match id {
                1 => 1987,
                2 => 2148,
                3 => 2148,
                _ => return None,
            };
            Some(ScriptItemData {
                item_type,
                count: 1,
                weight: if item_type == 1987 { 800 } else { 10 },
                name: if item_type == 1987 {
                    "bag".into()
                } else {
                    "gold coin".into()
                },
                action_id: 0,
                unique_id: 0,
                is_store_item: false,
                fluid_type: 0,
            })
        }
        fn get_item_type_is_container(&self, item_type: u16) -> bool {
            item_type == 1987
        }
        fn is_registered_container(&self, item_id: ScriptItemId) -> bool {
            item_id == 1
        }
        fn get_container_data(
            &self,
            item_id: ScriptItemId,
        ) -> Option<tfs_rust_common::ScriptContainerData> {
            if item_id != 1 {
                return None;
            }
            Some(tfs_rust_common::ScriptContainerData {
                size: 0,
                capacity: 20,
                empty_slots: 20,
                item_holding_count: 0,
                corpse_owner: 0,
            })
        }
    }

    fn r2_create_item_applier(
        _: *mut (),
        mutation: crate::lua_mutation::LuaMutation,
    ) -> Result<(), String> {
        match mutation {
            crate::lua_mutation::LuaMutation::GameCreateItem { item_type, .. } => {
                let id = match item_type {
                    1987 => 1,
                    2148 => 2,
                    _ => 99,
                };
                crate::lua_mutation::set_mutation_item_result(id);
            }
            crate::lua_mutation::LuaMutation::ContainerAddItem { .. } => {
                crate::lua_mutation::set_mutation_item_result(3);
            }
            crate::lua_mutation::LuaMutation::ItemRemove { .. } => {
                crate::lua_mutation::set_mutation_bool_result(true);
            }
            _ => {}
        }
        Ok(())
    }

    #[test]
    fn r2_create_item_returns_container_userdata_for_container_type() {
        crate::lua_mutation::register_lua_mutation_applier(r2_create_item_applier);
        let runtime = crate::runtime::LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        crate::lua_mutation::with_lua_mutation_scope(std::ptr::without_provenance_mut(1), || {
            with_lua_context(&R2CreateItemCtx, || {
                let bag_size: u32 = lua
                    .load("local bag = Game.createItem(1987); return bag:getSize()")
                    .eval()
                    .expect("bag:getSize");
                assert_eq!(bag_size, 0, "bag must be Container userdata");

                let added: bool = lua
                    .load(
                        "local bag = Game.createItem(1987)
                         local gold = bag:addItem(2148, 40)
                         return gold ~= nil",
                    )
                    .eval()
                    .expect("bag:addItem");
                assert!(added, "Container:addItem must succeed on createItem bag");

                let gold_has_size: bool = lua
                    .load(
                        "local gold = Game.createItem(2148)
                         return pcall(function() return gold:getSize() end)",
                    )
                    .eval()
                    .expect("gold:getSize pcall");
                assert!(
                    !gold_has_size,
                    "non-container createItem must stay Item userdata"
                );

                let removed: bool = lua
                    .load("local bag = Game.createItem(1987); return bag:remove()")
                    .eval()
                    .expect("bag:remove");
                assert!(removed);
            });
        });
    }

    /// R3: `addItemEx` is bound on Player / Container / Tile and returns `RETURNVALUE_*`.
    #[test]
    fn r3_add_item_ex_returns_returnvalue() {
        crate::lua_mutation::register_lua_mutation_applier(|_, mutation| {
            if let crate::lua_mutation::LuaMutation::AddItemEx { .. } = mutation {
                crate::lua_mutation::set_mutation_i32_result(0);
            }
            Ok(())
        });
        let runtime = crate::runtime::LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        let player = lua
            .create_userdata(crate::context::CreatureRef(1))
            .expect("player");
        let gold = lua.create_userdata(ItemRef(2)).expect("gold");
        lua.globals().set("player", player).unwrap();
        lua.globals().set("gold", gold).unwrap();
        crate::lua_mutation::with_lua_mutation_scope(std::ptr::without_provenance_mut(1), || {
            with_lua_context(&R2CreateItemCtx, || {
                let rv: i32 = lua
                    .load("return player:addItemEx(gold)")
                    .eval()
                    .expect("addItemEx");
                assert_eq!(rv, 0);
                let bag = lua
                    .create_userdata(crate::userdata::container::ContainerRef(1))
                    .expect("bag");
                lua.globals().set("bag", bag).unwrap();
                let crv: i32 = lua
                    .load("return bag:addItemEx(gold)")
                    .eval()
                    .expect("container addItemEx");
                assert_eq!(crv, 0);
                let tile = lua
                    .create_userdata(crate::userdata::tile::TileRef { x: 50, y: 50, z: 7 })
                    .expect("tile");
                lua.globals().set("tile", tile).unwrap();
                let trv: i32 = lua
                    .load("return tile:addItemEx(gold)")
                    .eval()
                    .expect("tile addItemEx");
                assert_eq!(trv, 0);
            });
        });
    }

    /// Floor `item:getParent()` is Tile userdata so `parent:isContainer()` can resolve.
    #[test]
    fn get_parent_tile_arm_pushes_tile_userdata() {
        use tfs_rust_common::{Position, ScriptCylinder};

        struct ParentCtx;
        impl ScriptContext for ParentCtx {
            fn get_creature(&self, _: ScriptCreatureId) -> Option<ScriptCreatureData> {
                None
            }
            fn get_item(&self, id: ScriptItemId) -> Option<ScriptItemRef> {
                Some(ScriptItemRef(id))
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_item_data(&self, _: ScriptItemId) -> Option<ScriptItemData> {
                Some(ScriptItemData {
                    item_type: 2580,
                    count: 1,
                    weight: 0,
                    name: "fishing rod".into(),
                    action_id: 0,
                    unique_id: 0,
                    is_store_item: false,
                    fluid_type: 0,
                })
            }
            fn get_item_parent(&self, _: ScriptItemId) -> Option<ScriptCylinder> {
                Some(ScriptCylinder::Tile(Position::new(100, 100, 7)))
            }
            fn tile_exists(&self, _: u16, _: u16, _: u8) -> bool {
                true
            }
        }

        let lua = Lua::new();
        register_item_metatable(&lua).expect("item");
        crate::userdata::register_tile_constructor(&lua).expect("tile");
        crate::userdata::register_position_metatable(&lua).expect("position");
        let item = lua.create_userdata(ItemRef(1)).expect("item");
        lua.globals().set("item", item).unwrap();
        with_lua_context(&ParentCtx, || {
            let z: u8 = lua
                .load("local p = item:getParent(); return p:getPosition().z")
                .eval()
                .expect("getParent Tile");
            assert_eq!(z, 7);
        });
    }
}
