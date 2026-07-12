//! `Spell` userdata for Lua — PC-2b.
//!
//! C++ reference:
//! - `luascript.cpp:3095-3137` — `luaSpellCreate` / Spell metatable registration.
//! - `luascript.cpp:15847-15873` — `luaSpellOnCastSpell`.
//! - `luascript.cpp:15875-15905` — `luaSpellRegister`.
//! - `spells.h:108-380` — `InstantSpell` / `RuneSpell`.
//!
//! The Lua `Spell` is a config bag: `Spell(SPELL_INSTANT)` creates a `SpellBuilder`,
//! `:words`/`:level`/`:mana`/`:vocation`/`:name` populate it, `:onCastSpell` registers
//! a Lua callback, and `:register()` pushes a `PendingSpell` into the pending buffer.

use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Lua, RegistryKey, UserData, UserDataMethods, Value};

/// Spell type constants — mirrors `enums.h:75-76`.
const SPELL_INSTANT: i32 = 1;
const SPELL_RUNE: i32 = 2;

/// A pending spell registration — drained from the Lua runtime into a `SpellRegistry`.
#[derive(Debug, Clone, Default)]
pub struct PendingSpell {
    pub spell_type: i32,
    pub name: String,
    pub words: String,
    pub level: u32,
    pub magic_level: u32,
    pub mana: u32,
    pub mana_percent: u32,
    pub soul: u32,
    pub group: u8,
    pub cooldown: u32,
    pub group_cooldown: u32,
    pub is_premium: bool,
    pub is_aggressive: bool,
    pub need_target: bool,
    pub need_weapon: bool,
    pub need_learn: bool,
    pub is_self_target: bool,
    /// Whether the spell accepts a parameter (text after spell words).
    /// C++ `InstantSpell::hasParam` — set via `spell:hasParams(true)`.
    pub has_param: bool,
    /// Whether the spell accepts a player name parameter.
    /// C++ `InstantSpell::hasPlayerNameParam` — set via `spell:hasPlayerNameParam(true)`.
    pub has_player_name_param: bool,
    pub vocations: Vec<String>,
    /// Rune-specific fields.
    pub rune_id: u16,
    pub charges: u32,
    /// Lua callback function name for `onCastSpell`. C++ stores a script ID; we store
    /// the function name (set via `spell.onCastSpell = function(...) end` assignment,
    /// or via the `:onCastSpell("functionName")` method).
    pub on_cast_callback: Option<String>,
}

impl PendingSpell {
    pub fn new(spell_type: i32) -> Self {
        Self {
            spell_type,
            ..Default::default()
        }
    }

    pub fn is_instant(&self) -> bool {
        self.spell_type == SPELL_INSTANT
    }

    pub fn is_rune(&self) -> bool {
        self.spell_type == SPELL_RUNE
    }
}

/// Lua-facing `Spell(type)` builder — newtype wrapper around `Rc<RefCell<PendingSpell>>`
/// to satisfy Rust's orphan rule. The `on_cast_fn` field holds the Lua callback
/// captured via `__newindex` (the `function spell.onCastSpell(creature, variant)`
/// pattern from `data/scripts/spells/`).
#[derive(Clone)]
pub struct SpellBuilder {
    pub spell: Rc<RefCell<PendingSpell>>,
    pub on_cast_fn: Rc<RefCell<Option<RegistryKey>>>,
}

/// Register the `Spell` metatable + constructor.
pub fn register_spell_metatable(lua: &Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<SpellBuilder>(|_registry| {})?;
    lua.register_userdata_type::<PendingSpell>(|_registry| {})?;

    // `Spell(type)` constructor — C++ `luaSpellCreate` (`luascript.cpp:15775`).
    let spell_new = lua.create_function(|_, spell_type: i32| {
        if matches!(spell_type, SPELL_INSTANT | SPELL_RUNE) {
            Ok(Some(SpellBuilder {
                spell: Rc::new(RefCell::new(PendingSpell::new(spell_type))),
                on_cast_fn: Rc::new(RefCell::new(None)),
            }))
        } else {
            Ok(None::<SpellBuilder>)
        }
    })?;
    lua.globals().set("Spell", spell_new)?;

    Ok(())
}

impl UserData for SpellBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `spell:name(name)` — sets the spell name.
        methods.add_method_mut("name", |_, this, name: String| {
            this.spell.borrow_mut().name = name;
            Ok(true)
        });

        // `spell:words(words)` — sets the spellwords (instant spells only).
        // C++ `InstantSpell::setWords` — `spells.cpp`.
        methods.add_method_mut("words", |_, this, words: String| {
            this.spell.borrow_mut().words = words;
            Ok(true)
        });

        // `spell:level(level)` — sets minimum level.
        methods.add_method_mut("level", |_, this, level: u32| {
            this.spell.borrow_mut().level = level;
            Ok(true)
        });

        // `spell:magicLevel(level)` — sets minimum magic level.
        methods.add_method_mut("magicLevel", |_, this, level: u32| {
            this.spell.borrow_mut().magic_level = level;
            Ok(true)
        });

        // `spell:mana(mana)` — sets mana cost.
        methods.add_method_mut("mana", |_, this, mana: u32| {
            this.spell.borrow_mut().mana = mana;
            Ok(true)
        });

        // `spell:manaPercent(percent)` — sets mana cost as percentage of max mana.
        methods.add_method_mut("manaPercent", |_, this, percent: u32| {
            this.spell.borrow_mut().mana_percent = percent;
            Ok(true)
        });

        // `spell:soul(soul)` — sets soul cost.
        methods.add_method_mut("soul", |_, this, soul: u32| {
            this.spell.borrow_mut().soul = soul;
            Ok(true)
        });

        // `spell:group(group)` — sets spell group (cooldown grouping).
        methods.add_method_mut("group", |_, this, group: u8| {
            this.spell.borrow_mut().group = group;
            Ok(true)
        });

        // `spell:cooldown(ms)` — sets spell cooldown.
        methods.add_method_mut("cooldown", |_, this, ms: u32| {
            this.spell.borrow_mut().cooldown = ms;
            Ok(true)
        });

        // `spell:groupCooldown(ms)` — sets group cooldown.
        methods.add_method_mut("groupCooldown", |_, this, ms: u32| {
            this.spell.borrow_mut().group_cooldown = ms;
            Ok(true)
        });

        // `spell:isPremium(bool)` — sets premium-only flag.
        methods.add_method_mut("isPremium", |_, this, val: bool| {
            this.spell.borrow_mut().is_premium = val;
            Ok(true)
        });

        // `spell:isAggressive(bool)` — sets aggressive flag (PZ lock / PVP).
        methods.add_method_mut("isAggressive", |_, this, val: bool| {
            this.spell.borrow_mut().is_aggressive = val;
            Ok(true)
        });

        // `spell:needTarget(bool)` — sets target requirement.
        methods.add_method_mut("needTarget", |_, this, val: bool| {
            this.spell.borrow_mut().need_target = val;
            Ok(true)
        });

        // `spell:needWeapon(bool)` — sets weapon requirement.
        methods.add_method_mut("needWeapon", |_, this, val: bool| {
            this.spell.borrow_mut().need_weapon = val;
            Ok(true)
        });

        // `spell:needLearn(bool)` — sets learn requirement.
        methods.add_method_mut("needLearn", |_, this, val: bool| {
            this.spell.borrow_mut().need_learn = val;
            Ok(true)
        });

        // `spell:isSelfTarget(bool)` — sets self-target flag.
        methods.add_method_mut("isSelfTarget", |_, this, val: bool| {
            this.spell.borrow_mut().is_self_target = val;
            Ok(true)
        });

        // `spell:hasParams(bool)` — sets whether the spell accepts a parameter
        // (text after the spell words). C++ `InstantSpell::setHasParam` — `spells.h:155`.
        methods.add_method_mut("hasParams", |_, this, val: bool| {
            this.spell.borrow_mut().has_param = val;
            Ok(true)
        });

        // `spell:hasPlayerNameParam(bool)` — sets whether the spell accepts a
        // player name parameter. C++ `InstantSpell::setHasPlayerNameParam` — `spells.h:157`.
        methods.add_method_mut("hasPlayerNameParam", |_, this, val: bool| {
            this.spell.borrow_mut().has_player_name_param = val;
            Ok(true)
        });

        // `spell:vocation(name, ...)` — TFS variadic: `spell:vocation("Knight", "Elite Knight")`.
        // Adds each vocation name to the allowed list.
        methods.add_method_mut("vocation", |_, this, vocs: mlua::MultiValue| {
            let mut b = this.spell.borrow_mut();
            for v in vocs.into_vec() {
                if let Some(s) = v.as_str() {
                    b.vocations.push(s.to_string());
                }
            }
            Ok(true)
        });

        // `spell:id(id)` — for rune spells, sets the rune item id.
        methods.add_method_mut("id", |_, this, id: u16| {
            this.spell.borrow_mut().rune_id = id;
            Ok(true)
        });

        // `spell:charges(charges)` — for rune spells, sets the rune charges.
        methods.add_method_mut("charges", |_, this, charges: u32| {
            this.spell.borrow_mut().charges = charges;
            Ok(true)
        });

        // `spell:onCastSpell(callbackName)` — C++ `luaSpellOnCastSpell`.
        // In TFS, this loads the callback from the script's global scope. We store
        // the function name (or "onCastSpell" if set via `spell.onCastSpell = fn`).
        // The berserk.lua pattern uses `function spell.onCastSpell(creature, variant)`,
        // which is a field assignment, not a method call. We handle both:
        // 1. `spell.onCastSpell = function(...)` → captured by `__newindex` (TODO)
        // 2. `spell:onCastSpell("name")` → stores the name here
        methods.add_method_mut("onCastSpell", |_, this, name: String| {
            this.spell.borrow_mut().on_cast_callback = Some(name);
            Ok(true)
        });

        // `spell:register()` — C++ `luaSpellRegister` (`luascript.cpp:15875`).
        // Pushes a snapshot into the `_pending_spells` global table, and if a
        // callback function was captured via `__newindex`, stores it in
        // `_pending_spell_callbacks` at the same index.
        methods.add_method("register", |lua, this, ()| {
            let globals = lua.globals();
            let pending: mlua::Table = globals.get("_pending_spells")?;
            let len = pending.len()?;
            let idx = len + 1;
            let snapshot = this.spell.borrow().clone();
            pending.set(idx, snapshot)?;

            // If a callback function was captured via `__newindex`, store it
            // in the parallel `_pending_spell_callbacks` table at the same index.
            let callback_key = this.on_cast_fn.borrow_mut().take();
            if let Some(key) = callback_key {
                let callbacks: mlua::Table = globals.get("_pending_spell_callbacks")?;
                let func: mlua::Function = lua.registry_value(&key)?;
                callbacks.set(idx, func)?;
            }

            Ok(true)
        });

        // `__newindex` — captures `spell.onCastSpell = function(creature, variant)`
        // (the `data/scripts/spells/` pattern). C++ TFS uses `loadCallback()` which
        // reads the function from the Lua stack; we capture it directly via the
        // metamethod and store it as a `RegistryKey` for later retrieval at
        // `register()` time.
        methods.add_meta_method(
            mlua::MetaMethod::NewIndex,
            |lua, this, (key, value): (String, Value)| {
                if key == "onCastSpell" && let Value::Function(func) = value {
                    let registry_key = lua.create_registry_value(func)?;
                    *this.on_cast_fn.borrow_mut() = Some(registry_key);
                }
                Ok(())
            },
        );
    }
}

// `PendingSpell` must be UserData so it can be stored in the `_pending_spells` Lua table.
impl UserData for PendingSpell {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_constructor_creates_instant_builder() {
        let lua = Lua::new();
        register_spell_metatable(&lua).expect("registration must succeed");
        crate::combat_enums::register_combat_enums(&lua).expect("enum registration must succeed");
        lua.globals()
            .set("_pending_spells", lua.create_table().unwrap())
            .unwrap();
        lua.globals()
            .set("_pending_spell_callbacks", lua.create_table().unwrap())
            .unwrap();

        let result: mlua::AnyUserData = lua
            .load(
                r#"
                local spell = Spell(SPELL_INSTANT)
                spell:manaPercent(80)
                spell:level(35)
                spell:isAggressive(true)
                spell:isPremium(true)
                spell:name("Berserk")
                spell:vocation("Knight", "Elite Knight")
                spell:words("ex,ori")
                return spell
            "#,
            )
            .eval()
            .expect("spell setup must succeed");
        let s_ref = result
            .borrow::<SpellBuilder>()
            .expect("must be SpellBuilder");
        let s = s_ref.spell.borrow();
        assert!(s.is_instant());
        assert_eq!(s.name, "Berserk");
        assert_eq!(s.words, "ex,ori");
        assert_eq!(s.level, 35);
        assert_eq!(s.mana_percent, 80);
        assert!(s.is_aggressive);
        assert!(s.is_premium);
        assert_eq!(s.vocations, vec!["Knight", "Elite Knight"]);
    }

    #[test]
    fn spell_register_pushes_to_pending_table() {
        let lua = Lua::new();
        register_spell_metatable(&lua).expect("registration must succeed");
        crate::combat_enums::register_combat_enums(&lua).expect("enum registration must succeed");
        lua.globals()
            .set("_pending_spells", lua.create_table().unwrap())
            .unwrap();
        lua.globals()
            .set("_pending_spell_callbacks", lua.create_table().unwrap())
            .unwrap();

        lua.load(
            r#"
            local spell = Spell(SPELL_INSTANT)
            spell:name("Berserk")
            spell:words("ex,ori")
            spell:level(35)
            spell:register()
        "#,
        )
        .exec()
        .expect("spell register must succeed");

        let pending: mlua::Table = lua.globals().get("_pending_spells").unwrap();
        assert_eq!(pending.len().unwrap(), 1);
    }

    #[test]
    fn spell_newindex_captures_on_cast_spell_callback() {
        // PC-3a: `function spell.onCastSpell(creature, variant)` is a field
        // assignment captured by `__newindex`. After `:register()`, the callback
        // must appear in `_pending_spell_callbacks` at the same index as the
        // spell in `_pending_spells`.
        let lua = Lua::new();
        register_spell_metatable(&lua).expect("registration must succeed");
        crate::combat_enums::register_combat_enums(&lua).expect("enum registration must succeed");
        lua.globals()
            .set("_pending_spells", lua.create_table().unwrap())
            .unwrap();
        lua.globals()
            .set("_pending_spell_callbacks", lua.create_table().unwrap())
            .unwrap();

        lua.load(
            r#"
            local spell = Spell(SPELL_INSTANT)
            spell:name("Test Spell")
            spell:words("ex,ori")

            function spell.onCastSpell(creature, variant)
                return true
            end

            spell:register()
        "#,
        )
        .exec()
        .expect("spell with onCastSpell must register");

        let pending: mlua::Table = lua.globals().get("_pending_spells").unwrap();
        assert_eq!(pending.len().unwrap(), 1, "one spell registered");

        let callbacks: mlua::Table = lua.globals().get("_pending_spell_callbacks").unwrap();
        assert_eq!(callbacks.len().unwrap(), 1, "one callback captured");

        // The callback must be a callable function (call it to verify).
        let func: mlua::Function = callbacks.get(1).unwrap();
        let result: bool = func.call(()).expect("callback must be callable");
        assert!(result, "callback must return true");
    }
}
