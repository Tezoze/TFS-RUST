//! Spell definitions loaded from `data/scripts/spells/**/*.lua` via the TFS Lua
//! `Spell(SPELL_INSTANT | SPELL_RUNE)` API.
//!
//! PC-2b: the Lua `Spell` userdata accumulates config fields during script loading
//! (`:words`, `:level`, `:mana`, `:vocation`, `:register`). The `:register()` call
//! pushes a `PendingSpell` into the Lua runtime's pending buffer, which is drained
//! into a `SpellRegistry` after all spell scripts load.
//!
//! C++ reference: `spells.cpp` `Spells::load`, `spells.h:108-380` `InstantSpell` /
//! `RuneSpell`, `luascript.cpp:3095-3137` `luaSpellCreate` / `luaSpellRegister`.

use std::collections::HashMap;

/// Spell type — mirrors TFS `SpellType_t` (`enums.h:75-76`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpellType {
    #[default]
    Undefined,
    Instant,
    Rune,
}

/// An instant spell definition (word-triggered: "ex,ori" → berserk).
/// C++ `InstantSpell` — `spells.h:108-260`.
#[derive(Debug, Clone, Default)]
pub struct InstantSpellDef {
    /// Spell name (e.g. "Berserk").
    pub name: String,
    /// Spellwords (e.g. "ex,ori") — the `Say` text that triggers this spell.
    pub words: String,
    /// Minimum level to cast.
    pub level: u32,
    /// Minimum magic level to cast.
    pub magic_level: u32,
    /// Mana cost (absolute). `0` if `mana_percent` is used instead.
    pub mana: u32,
    /// Mana cost as a percentage of max mana (e.g. 80 = 80%). `0` if `mana` is used.
    pub mana_percent: u32,
    /// Soul cost.
    pub soul: u32,
    /// Spell group for cooldown grouping (TFS `SpellGroup_t`).
    pub group: u8,
    /// Cooldown in ms.
    pub cooldown: u32,
    /// Group cooldown in ms.
    pub group_cooldown: u32,
    /// Premium-only flag.
    pub is_premium: bool,
    /// Aggressive spell (triggers PZ lock / PVP).
    pub is_aggressive: bool,
    /// Vocation names that can cast (e.g. `["Knight", "Elite Knight"]`).
    pub vocations: Vec<String>,
    /// Whether the spell needs a target creature.
    pub need_target: bool,
    /// Whether the spell needs a weapon equipped.
    pub need_weapon: bool,
    /// Whether the spell needs to be learned first.
    pub need_learn: bool,
    /// Whether the spell is self-target only.
    pub is_self_target: bool,
    /// Lua callback registry key for `onCastSpell`.
    pub on_cast_callback: Option<String>,
}

/// A rune spell definition (item-use-triggered: use rune item → cast).
/// C++ `RuneSpell` — `spells.h:270-380`.
#[derive(Debug, Clone, Default)]
pub struct RuneSpellDef {
    /// Spell name.
    pub name: String,
    /// Rune item id (e.g. 3152 = Sudden Death rune).
    pub rune_id: u16,
    /// Rune charges.
    pub charges: u32,
    /// Minimum level to use.
    pub level: u32,
    /// Minimum magic level to use.
    pub magic_level: u32,
    /// Mana cost to cast (conjuring runes costs mana; using them usually doesn't).
    pub mana: u32,
    /// Spell group.
    pub group: u8,
    /// Cooldown in ms.
    pub cooldown: u32,
    /// Group cooldown in ms.
    pub group_cooldown: u32,
    /// Aggressive spell.
    pub is_aggressive: bool,
    /// Vocation names that can use.
    pub vocations: Vec<String>,
    /// Whether the rune needs a target.
    pub need_target: bool,
    /// Lua callback registry key for `onCastSpell`.
    pub on_cast_callback: Option<String>,
}

/// Union of spell definition types.
#[derive(Debug, Clone)]
pub enum SpellDef {
    Instant(InstantSpellDef),
    Rune(RuneSpellDef),
}

impl SpellDef {
    pub fn name(&self) -> &str {
        match self {
            SpellDef::Instant(s) => &s.name,
            SpellDef::Rune(s) => &s.name,
        }
    }

    pub fn is_instant(&self) -> bool {
        matches!(self, SpellDef::Instant(_))
    }
}

/// Registry of all spell definitions loaded from Lua scripts.
/// Mirrors `VocationRegistry` — keyed by spellwords for instant spells, by item id
/// for rune spells.
#[derive(Debug, Clone, Default)]
pub struct SpellRegistry {
    /// Instant spells keyed by spellwords (lowercased for case-insensitive lookup).
    pub instant_by_words: HashMap<String, InstantSpellDef>,
    /// Instant spells keyed by name.
    pub instant_by_name: HashMap<String, InstantSpellDef>,
    /// Rune spells keyed by rune item id.
    pub runes_by_id: HashMap<u16, RuneSpellDef>,
    /// Rune spells keyed by name.
    pub runes_by_name: HashMap<String, RuneSpellDef>,
}

impl SpellRegistry {
    /// Look up an instant spell by its spellwords (case-insensitive).
    pub fn get_instant_by_words(&self, words: &str) -> Option<&InstantSpellDef> {
        self.instant_by_words.get(&words.to_ascii_lowercase())
    }

    /// Look up an instant spell by name.
    pub fn get_instant_by_name(&self, name: &str) -> Option<&InstantSpellDef> {
        self.instant_by_name.get(name)
    }

    /// Look up a rune spell by item id.
    pub fn get_rune(&self, item_id: u16) -> Option<&RuneSpellDef> {
        self.runes_by_id.get(&item_id)
    }

    /// Total number of registered spells.
    pub fn len(&self) -> usize {
        self.instant_by_words.len() + self.runes_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_registry_lookup_by_words() {
        let mut reg = SpellRegistry::default();
        let spell = InstantSpellDef {
            name: "Berserk".to_string(),
            words: "ex,ori".to_string(),
            level: 35,
            mana_percent: 80,
            is_aggressive: true,
            is_premium: true,
            vocations: vec!["Knight".to_string(), "Elite Knight".to_string()],
            ..Default::default()
        };
        reg.instant_by_words
            .insert("ex,ori".to_string(), spell.clone());
        reg.instant_by_name.insert("Berserk".to_string(), spell);

        let s = reg.get_instant_by_words("ex,ori").expect("spell found");
        assert_eq!(s.level, 35);
        assert_eq!(s.mana_percent, 80);
        assert!(s.is_aggressive);
        assert!(s.is_premium);
        assert_eq!(s.vocations.len(), 2);

        // Case-insensitive lookup.
        assert!(reg.get_instant_by_words("EX,ORI").is_some());
    }
}
