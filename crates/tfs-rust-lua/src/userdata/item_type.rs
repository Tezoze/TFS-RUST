//! ItemType userdata for Lua (`ItemType` in TFS scripts).
//!
//! C++ reference: `src/luascript.cpp` — `LuaScriptInterface` ItemType userdata
//! (`ItemType::getID`, `isStackable`, `isFluidContainer`, `getDestroyId`,
//! `getFluidSource`, `getName`, `getArticle`, `getPluralName`, `getWeight`,
//! `isContainer`, …). Wraps a server item type id; resolves name→id lookups
//! through `ScriptContext`.

use mlua::{MetaMethod, UserData, UserDataMethods, Value};
use std::cell::RefCell;

use crate::context::{CURRENT_CTX, LuaContext};

/// Item type handle wrapping a server item type id. `id = 0` means "not found"
/// (matching C++ `ItemType::id == 0` for invalid items).
#[derive(Clone, Copy, Debug)]
pub struct ItemTypeRef(pub u16);

impl UserData for ItemTypeRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "ItemType");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `ItemType:getId()` — `ItemType::getID` (`src/items.h`). Returns the
        // server item type id, or `0` if the item was not found.
        methods.add_method("getId", |_, this, ()| Ok(this.0));

        // `ItemType:isStackable()` — `ItemType::stackable` (`src/items.h`).
        // CH-6 talkaction `/i` count clamping.
        methods.add_method("isStackable", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_is_stackable(this.0))
            })
        });

        // `ItemType:isFluidContainer()` — `ItemType::isFluidContainer`
        // (`src/items.h`). CH-6 talkaction `/i` count clamping.
        methods.add_method("isFluidContainer", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_is_fluid_container(this.0))
            })
        });

        // `ItemType:getCharges()` — `ItemType::charges` (`src/items.h`).
        // PC-3a Phase 5: `Player:conjureItem` falls back to charges when count
        // is omitted.
        methods.add_method("getCharges", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_charges(this.0))
            })
        });

        // `ItemType:getDestroyId()` — `luaItemTypeGetDestroyId` (`luascript.cpp`).
        // `ItemType::destroyTo`; XML `destroyto`; 772 `DESTROYTARGET`.
        methods.add_method("getDestroyId", |_, this, ()| {
            crate::context::current_ctx(|ctx| ctx.get_item_type_destroy_id(this.0))
                .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))
        });

        // `ItemType:getFluidSource()` — `luaItemTypeGetFluidSource` (`luascript.cpp`).
        // `ItemType::fluidSource`; XML `fluidsource` as 772 sequential `FLUID_*`.
        methods.add_method("getFluidSource", |_, this, ()| {
            crate::context::current_ctx(|ctx| ctx.get_item_type_fluid_source(this.0))
                .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))
        });

        // `ItemType:getName()` — `luaItemTypeGetName` (`luascript.cpp`).
        // `ItemType::name`; XML `name`. Empty string when the type is unknown.
        methods.add_method("getName", |_, this, ()| {
            crate::context::current_ctx(|ctx| ctx.get_item_type_name(this.0))
                .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))
        });

        // `ItemType:getArticle()` — `luaItemTypeGetArticle` (`luascript.cpp`).
        // `ItemType::article`; XML `article`. Empty string when unset.
        methods.add_method("getArticle", |_, this, ()| {
            crate::context::current_ctx(|ctx| ctx.get_item_type_article(this.0))
                .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))
        });

        // `ItemType:getPluralName()` — `luaItemTypeGetPluralName` (`luascript.cpp`).
        // `ItemType::getPluralName` (`items.h`): XML `plural`, else `name` / `name+"s"`.
        methods.add_method("getPluralName", |_, this, ()| {
            crate::context::current_ctx(|ctx| ctx.get_item_type_plural_name(this.0))
                .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))
        });

        // `ItemType:getWeight([count = 1])` — `luaItemTypeGetWeight` (`luascript.cpp`).
        // `weight * max(1, count)` (`uint16_t` count, default 1).
        methods.add_method("getWeight", |_, this, count: Option<u16>| {
            let n = u64::from(count.unwrap_or(1).max(1));
            crate::context::current_ctx(|ctx| u64::from(ctx.get_item_type_weight(this.0)) * n)
                .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))
        });

        // `ItemType:isContainer()` — `luaItemTypeIsContainer` (`luascript.cpp`).
        // `ItemType::isContainer`: `group == ITEM_GROUP_CONTAINER` (`items.h`).
        methods.add_method("isContainer", |_, this, ()| {
            crate::context::current_ctx(|ctx| ctx.get_item_type_is_container(this.0))
                .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))
        });

        // `ItemType:getDescription(lookDistance[, subType])` — native `item_get_description_cpp`.
        methods.add_method(
            "getDescription",
            |_, this, (look_distance, sub_type): (Option<i32>, Option<u16>)| {
                let look_distance = look_distance.unwrap_or(1);
                let sub_type = sub_type.unwrap_or(1);
                crate::context::current_ctx(|ctx| {
                    ctx.script_item_type_description(this.0, look_distance, sub_type)
                })
                .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))
            },
        );

        // `ItemType:getNameDescription([subType], addArticle)` — `item.cpp` ~1582–1615.
        methods.add_method(
            "getNameDescription",
            |_, this, (sub_type, add_article): (Option<u16>, Option<bool>)| {
                let sub_type = sub_type.unwrap_or(1);
                let add_article = add_article.unwrap_or(true);
                crate::context::current_ctx(|ctx| {
                    ctx.script_item_type_name_description(this.0, sub_type, add_article)
                })
                .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))
            },
        );

        // `ItemType.usesSlot(slot)` — `data/lib/core/itemtype.lua` (`SlotPositionBits`).
        methods.add_method("usesSlot", |_, this, slot: u8| {
            let slot_bit = slot_position_bit(slot);
            crate::context::current_ctx(|ctx| {
                ctx.get_item_type_slot_position(this.0) & slot_bit != 0
            })
            .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))
        });

        methods.add_method("isCorpse", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_is_corpse(this.0))
            })
        });

        methods.add_method("isMovable", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_is_movable(this.0))
            })
        });

        methods.add_method("isGroundTile", |_, this, ()| {
            CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_item_type_is_ground_tile(this.0))
            })
        });

        // Gap 7b — `__index` fallback so `itemtype:usesSlot(slot)` resolves
        // `function ItemType.usesSlot(self, ...)` from `data/lib/core/itemtype.lua`.
        // Native methods above keep priority. C++ `LuaScriptInterface::registerClass`.
        methods.add_meta_method(MetaMethod::Index, |lua, _this, key: mlua::LuaString| {
            crate::class_registry::class_index_lookup(
                lua,
                crate::class_registry::ITEM_TYPE_INDEX_CHAIN,
                key,
            )
        });
    }
}

/// `ItemType(nameOrId)` constructor — `luascript.cpp` `luaItemTypeCreate`.
/// Resolves a name string to a server item type id via `ScriptContext`, or
/// wraps a numeric id directly. Returns a `ItemTypeRef` userdata (id=0 if
/// the name was not found, matching C++ behavior).
pub fn register_item_type_constructor(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    let constructor = lua.create_function(|lua, arg: Value| {
        let id: u16 = match arg {
            Value::Integer(n) => n as u16,
            Value::Number(n) => n as u16,
            Value::String(s) => {
                let name = s.to_str()?.to_string();
                CURRENT_CTX.with(|c: &RefCell<Option<*const dyn LuaContext>>| {
                    let ptr =
                        (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                    if ptr.is_null() {
                        return Err(mlua::Error::runtime("LuaContext not set"));
                    }
                    let ctx = unsafe { &*ptr };
                    Ok(ctx.get_item_type_id_by_name(&name).unwrap_or(0))
                })?
            }
            _ => 0,
        };
        let ud = lua.create_userdata(ItemTypeRef(id))?;
        Ok(Value::UserData(ud))
    })?;
    // `ItemType` is a class table (extensible by `function ItemType.usesSlot(...)`
    // in `data/lib/core/itemtype.lua`) with a `__call` ctor. Gap 7a.
    crate::class_registry::register_class(lua, "ItemType", Some(constructor))?;
    Ok(())
}

/// Register the ItemType metatable in the Lua runtime.
pub fn register_item_type_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<ItemTypeRef>(|_registry| {})
}

/// TFS `CONST_SLOT_*` → `SlotPositionBits` — `data/lib/core/itemtype.lua`.
fn slot_position_bit(slot: u8) -> u32 {
    match slot {
        1 => 1 << 0,  // SLOTP_HEAD
        2 => 1 << 1,  // SLOTP_NECKLACE
        3 => 1 << 2,  // SLOTP_BACKPACK
        4 => 1 << 3,  // SLOTP_ARMOR
        5 => 1 << 4,  // SLOTP_RIGHT
        6 => 1 << 5,  // SLOTP_LEFT
        7 => 1 << 6,  // SLOTP_LEGS
        8 => 1 << 7,  // SLOTP_FEET
        9 => 1 << 8,  // SLOTP_RING
        10 => 1 << 9, // SLOTP_AMMO
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use crate::context::with_lua_context;
    use crate::runtime::LuaRuntime;
    use tfs_rust_common::{
        ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemId, ScriptItemRef,
    };

    /// Known `items.xml` rows: statue 1442 `destroyto=2256`, water cask 1771
    /// `fluidsource=water` (772 `FLUID_WATER=1`), muddy floor 355 `mud` (`FLUID_MUD=4`).
    struct ItemTypeAttrCtx;

    impl ScriptContext for ItemTypeAttrCtx {
        fn get_creature(&self, _: ScriptCreatureId) -> Option<ScriptCreatureData> {
            None
        }
        fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
            None
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_item_type_destroy_id(&self, item_type: u16) -> u16 {
            match item_type {
                1442 => 2256,
                _ => 0,
            }
        }
        fn get_item_type_fluid_source(&self, item_type: u16) -> u8 {
            match item_type {
                1771 => 1, // water
                355 => 4,  // mud
                _ => 0,
            }
        }
    }

    #[test]
    fn e3_get_destroy_id_and_fluid_source_from_known_xml_rows() {
        let runtime = LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        with_lua_context(&ItemTypeAttrCtx, || {
            let destroy: u16 = lua
                .load("return ItemType(1442):getDestroyId()")
                .eval()
                .expect("getDestroyId");
            assert_eq!(destroy, 2256);

            let none: u16 = lua
                .load("return ItemType(102):getDestroyId()")
                .eval()
                .expect("grass destroy");
            assert_eq!(none, 0);

            let water: u8 = lua
                .load("return ItemType(1771):getFluidSource()")
                .eval()
                .expect("getFluidSource water");
            assert_eq!(water, 1);

            let mud: u8 = lua
                .load("return ItemType(355):getFluidSource()")
                .eval()
                .expect("getFluidSource mud");
            assert_eq!(mud, 4);

            let empty: u8 = lua
                .load("return ItemType(102):getFluidSource()")
                .eval()
                .expect("grass fluid");
            assert_eq!(empty, 0);
        });
    }

    /// R1: `onUseQuest` found-text + capacity — TFS `luaItemTypeGetName` /
    /// `GetArticle` / `GetPluralName` / `GetWeight` (`weight * max(1,count)`) /
    /// `IsContainer`. Known `items.xml` rows: gold 2148, bag 1987, grass 102.
    struct ItemTypeLookCtx;

    impl ScriptContext for ItemTypeLookCtx {
        fn get_creature(&self, _: ScriptCreatureId) -> Option<ScriptCreatureData> {
            None
        }
        fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
            None
        }
        fn get_config_string(&self, _: &str) -> Option<String> {
            None
        }
        fn get_item_type_name(&self, item_type: u16) -> String {
            match item_type {
                2148 => "gold coin".into(),
                1987 => "bag".into(),
                102 => "grass".into(),
                _ => String::new(),
            }
        }
        fn get_item_type_article(&self, item_type: u16) -> String {
            match item_type {
                2148 | 1987 => "a".into(),
                _ => String::new(),
            }
        }
        fn get_item_type_plural_name(&self, item_type: u16) -> String {
            match item_type {
                2148 => "gold coins".into(),
                1987 => "bags".into(),
                102 => "grass".into(),
                _ => String::new(),
            }
        }
        fn get_item_type_weight(&self, item_type: u16) -> u32 {
            match item_type {
                2148 => 10,
                1987 => 800,
                _ => 0,
            }
        }
        fn get_item_type_is_container(&self, item_type: u16) -> bool {
            item_type == 1987
        }
    }

    #[test]
    fn r1_item_type_name_article_plural_weight_container() {
        let runtime = LuaRuntime::new().expect("runtime");
        let lua = &runtime.lua;
        with_lua_context(&ItemTypeLookCtx, || {
            let name: String = lua
                .load("return ItemType(2148):getName()")
                .eval()
                .expect("getName gold");
            assert_eq!(name, "gold coin");

            let article: String = lua
                .load("return ItemType(2148):getArticle()")
                .eval()
                .expect("getArticle gold");
            assert_eq!(article, "a");

            let plural: String = lua
                .load("return ItemType(2148):getPluralName()")
                .eval()
                .expect("getPluralName gold");
            assert_eq!(plural, "gold coins");

            let w1: u64 = lua
                .load("return ItemType(2148):getWeight()")
                .eval()
                .expect("getWeight default");
            assert_eq!(w1, 10);

            let w100: u64 = lua
                .load("return ItemType(2148):getWeight(100)")
                .eval()
                .expect("getWeight 100");
            assert_eq!(w100, 1000);

            let w0: u64 = lua
                .load("return ItemType(2148):getWeight(0)")
                .eval()
                .expect("getWeight 0");
            assert_eq!(w0, 10);

            let gold_cont: bool = lua
                .load("return ItemType(2148):isContainer()")
                .eval()
                .expect("isContainer gold");
            assert!(!gold_cont);

            let bag_name: String = lua
                .load("return ItemType(1987):getName()")
                .eval()
                .expect("getName bag");
            assert_eq!(bag_name, "bag");

            let bag_cont: bool = lua
                .load("return ItemType(1987):isContainer()")
                .eval()
                .expect("isContainer bag");
            assert!(bag_cont);

            let bag_w: u64 = lua
                .load("return ItemType(1987):getWeight()")
                .eval()
                .expect("getWeight bag");
            assert_eq!(bag_w, 800);

            let grass_art: String = lua
                .load("return ItemType(102):getArticle()")
                .eval()
                .expect("getArticle grass");
            assert_eq!(grass_art, "");

            let grass_plural: String = lua
                .load("return ItemType(102):getPluralName()")
                .eval()
                .expect("getPluralName grass");
            assert_eq!(grass_plural, "grass");

            let unknown: String = lua
                .load("return ItemType(9999):getName()")
                .eval()
                .expect("getName unknown");
            assert_eq!(unknown, "");
        });
    }
}
