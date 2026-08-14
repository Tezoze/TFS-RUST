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
    /// Whether the spell accepts a parameter (text after the spell words).
    /// C++ `InstantSpell::hasParam` — `spells.h:155`. Set via `spell:hasParams(true)`.
    pub has_param: bool,
    /// Whether the spell accepts a player name parameter.
    /// C++ `InstantSpell::hasPlayerNameParam` — `spells.h:157`.
    /// Set via `spell:hasPlayerNameParam(true)`.
    pub has_player_name_param: bool,
    /// Whether the spell needs a direction (beam/wave spells).
    /// C++ `InstantSpell::needDirection` — `spells.h:160`.
    pub need_direction: bool,
    /// Max Chebyshev range (`-1` = unlimited). C++ `Spell::range` — `spells.h:294`.
    pub range: i32,
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
    /// Magic level required to use the rune.
    /// C++ `RuneSpell::runeMagicLevel` — set via `rune:runeMagicLevel(n)`.
    pub rune_magic_level: u32,
    /// Allow casting at a distance from the target.
    /// C++ `RuneSpell::allowFarUse` — `spells.h:290`.
    pub allow_far_use: bool,
    /// Blocked by walls between caster and target.
    /// C++ `RuneSpell::blockWalls` — `spells.h:291`.
    pub block_walls: bool,
    /// Check floor difference between caster and target.
    /// C++ `RuneSpell::checkFloor` — `spells.h:292`.
    pub check_floor: bool,
    /// Blocked by solid items on target tile.
    /// C++ `RuneSpell::blockSolid` — `spells.h:293`.
    pub block_solid: bool,
    /// Blocked by creatures on target tile.
    /// C++ `RuneSpell::blockCreature` — `spells.h:294`.
    pub block_creature: bool,
    /// Triggers PZ lock on aggressive use.
    /// C++ `RuneSpell::isPzLock` — `spells.h:295`.
    pub is_pz_lock: bool,
    /// Whether the rune's cooldown counts as a spell cooldown.
    /// C++ `RuneSpell::cooldownSpellTime` — `spells.h:296`.
    /// Default **false**: 772 runes only set `EarliestMultiuseTime`; set true to also
    /// bump `EarliestSpellTime` (TFS-style shared spell exhaust).
    pub cooldown_spell_time: bool,
    /// Max Chebyshev range for `playerRuneSpellCheck` (`-1` = unlimited).
    /// C++ `Spell::range` — `spells.h:294` / `spells.cpp:719`.
    pub range: i32,
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
    ///
    /// **Note:** This does an exact match on the comma-separated registered form
    /// (e.g. `"ex,ori"`). For matching player-typed text like `"exori"` or
    /// `"exevo vis lux"`, use [`get_instant_spell`] instead.
    pub fn get_instant_by_words(&self, words: &str) -> Option<&InstantSpellDef> {
        self.instant_by_words.get(&words.to_ascii_lowercase())
    }

    /// Look up an instant spell by player-typed text.
    ///
    /// C++ reference: `Spells::getInstantSpell` — `spells.cpp:223-251`.
    /// Trims + collapses extra spaces, strips quoted parameters, then iterates
    /// all instant spells calling [`compare_spell_words`] for syllable-by-syllable
    /// matching.
    ///
    /// Returns the matched spell and the extracted parameter (text after the
    /// spell words), if any.
    pub fn get_instant_spell(&self, text: &str) -> Option<(&InstantSpellDef, String)> {
        let cleaned = remove_extra_spaces(text.trim());
        if cleaned.is_empty() {
            return None;
        }

        // Strip quoted parameter — C++ `getInstantSpell` `spells.cpp:229-237`.
        let (constructed_words, quoted_param) = match cleaned.find('"') {
            Some(idx) => {
                // Don't allow `exura"` — must be `exura "param"`.
                let before = &cleaned[..idx];
                if !before.ends_with(' ') {
                    return None;
                }
                (&cleaned[..idx - 1], Some(&cleaned[idx..]))
            }
            None => (cleaned.as_str(), None),
        };

        for spell in self.instant_by_words.values() {
            let support_param = spell.has_param || spell.has_player_name_param;
            if compare_spell_words(&spell.words, constructed_words, support_param) {
                // Extract the parameter — C++ `playerSaySpell` `spells.cpp:44-68`.
                let param = extract_param(&spell.words, &cleaned, quoted_param);
                return Some((spell, param));
            }
        }
        None
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

/// Compare player-typed text against registered spell words.
///
/// C++ reference: `compareSpellWords` — `tools.cpp:386-411`.
///
/// Splits `spell_words` by `,` into syllables, then matches each syllable
/// against the front of `given` (case-insensitive). Leading spaces in the
/// registered syllable (e.g. `" vis"`) require a space in the input; if the
/// syllable has no leading space but the input does, the space is consumed.
///
/// If `support_param` is false, any remaining text after all syllables match
/// causes the match to fail. If true, remaining text is the spell parameter.
pub fn compare_spell_words(spell_words: &str, given: &str, support_param: bool) -> bool {
    let mut remaining = given;
    for syllable in spell_words.split(',') {
        // C++ `tools.cpp:393-395`: if the input has a leading space but the
        // registered syllable doesn't, consume the space from the input.
        if remaining.starts_with(' ') && !syllable.starts_with(' ') {
            remaining = &remaining[1..];
        }

        // Compare the first N chars of `remaining` against the syllable.
        let syllable_len = syllable.len();
        if remaining.len() < syllable_len {
            return false;
        }
        if !remaining[..syllable_len].eq_ignore_ascii_case(syllable) {
            return false;
        }
        remaining = &remaining[syllable_len..];
    }

    // C++ `tools.cpp:405-408`: remaining text without param support → no match.
    if !remaining.is_empty() && !support_param {
        return false;
    }

    true
}

/// Merge comma-separated spell words into a single string (no commas).
///
/// C++ reference: `mergeSpellWords` — `tools.cpp:413-421`.
/// `"ex,evo, vis, lux"` → `"exevo vis lux"`.
pub fn merge_spell_words(words: &str) -> String {
    words.split(',').collect()
}

/// Count whitespace characters in a string.
///
/// C++ reference: `countSpaces` — `tools.cpp:375-384`.
fn count_spaces(s: &str) -> usize {
    s.chars().filter(|c| c.is_whitespace()).count()
}

/// Collapse consecutive spaces into single spaces.
///
/// C++ reference: `removeExtraSpaces` — `tools.cpp:428-434`.
fn remove_extra_spaces(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else {
            result.push(c);
            prev_space = false;
        }
    }
    result
}

/// Extract the spell parameter from the input text.
///
/// C++ reference: `playerSaySpell` — `spells.cpp:44-68`.
///
/// The parameter is the text after the spell words. The spell word length
/// (in the input) is `mergeSpellWords(words).len() + countSpaces(words)`.
/// The given length (where the param starts) is `instantLen + 1 - countSpaces`.
fn extract_param(spell_words: &str, cleaned: &str, quoted_param: Option<&str>) -> String {
    if let Some(qp) = quoted_param {
        // Strip quotes from the quoted parameter.
        return qp.replace('"', "").trim_start().to_string();
    }

    let merged_len = merge_spell_words(spell_words).len();
    let spaces = count_spaces(spell_words);
    let instant_len = merged_len + spaces;
    let given_len = instant_len + 1 - spaces;

    if given_len < cleaned.len() {
        let param = &cleaned[given_len..];
        param.trim_start().to_string()
    } else {
        String::new()
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

    // --- compare_spell_words tests ---

    #[test]
    fn compare_two_syllable_no_space() {
        // "ex,ori" → player types "exori" (first two syllables glued)
        assert!(compare_spell_words("ex,ori", "exori", false));
        assert!(compare_spell_words("ex,ori", "EXORI", false));
    }

    #[test]
    fn compare_four_syllable_with_spaces() {
        // "ex,evo, vis, lux" → player types "exevo vis lux"
        assert!(compare_spell_words(
            "ex,evo, vis, lux",
            "exevo vis lux",
            false
        ));
        assert!(compare_spell_words(
            "ex,evo, vis, lux",
            "EXEVO VIS LUX",
            false
        ));
    }

    #[test]
    fn compare_extra_spaces_collapsed() {
        // Extra spaces are collapsed by `remove_extra_spaces` in `get_instant_spell`
        // before `compare_spell_words` is called. Test the collapsed form here.
        assert!(compare_spell_words(
            "ex,evo, vis, lux",
            "exevo vis lux",
            false
        ));
    }

    #[test]
    fn compare_trailing_space_ok() {
        // Trailing spaces are trimmed by `get_instant_spell` before calling
        // `compare_spell_words`. Test the trimmed form here.
        assert!(compare_spell_words("ex,ori", "exori", false));
    }

    #[test]
    fn compare_extra_text_no_param_fails() {
        assert!(!compare_spell_words("ex,ori", "exori blah", false));
    }

    #[test]
    fn compare_extra_text_with_param_ok() {
        assert!(compare_spell_words("ex,ori", "exori blah", true));
    }

    #[test]
    fn compare_wrong_syllable_fails() {
        assert!(!compare_spell_words("ex,ori", "exura", false));
    }

    #[test]
    fn compare_too_short_fails() {
        assert!(!compare_spell_words("ex,evo, vis, lux", "exevo", false));
    }

    // --- get_instant_spell tests ---

    fn make_registry() -> SpellRegistry {
        let mut reg = SpellRegistry::default();
        reg.instant_by_words.insert(
            "ex,ori".to_string(),
            InstantSpellDef {
                name: "Berserk".to_string(),
                words: "ex,ori".to_string(),
                ..Default::default()
            },
        );
        reg.instant_by_words.insert(
            "ex,evo, vis, lux".to_string(),
            InstantSpellDef {
                name: "Energy Beam".to_string(),
                words: "ex,evo, vis, lux".to_string(),
                ..Default::default()
            },
        );
        reg.instant_by_words.insert(
            "ex,iva".to_string(),
            InstantSpellDef {
                name: "Find Person".to_string(),
                words: "ex,iva".to_string(),
                has_param: true,
                ..Default::default()
            },
        );
        reg
    }

    #[test]
    fn get_instant_spell_matches_glued_syllables() {
        let reg = make_registry();
        let (spell, param) = reg.get_instant_spell("exori").expect("match");
        assert_eq!(spell.name, "Berserk");
        assert!(param.is_empty());
    }

    #[test]
    fn get_instant_spell_matches_space_separated() {
        let reg = make_registry();
        let (spell, param) = reg.get_instant_spell("exevo vis lux").expect("match");
        assert_eq!(spell.name, "Energy Beam");
        assert!(param.is_empty());
    }

    #[test]
    fn get_instant_spell_case_insensitive() {
        let reg = make_registry();
        let (spell, _) = reg.get_instant_spell("EXORI").expect("match");
        assert_eq!(spell.name, "Berserk");
    }

    #[test]
    fn get_instant_spell_extra_spaces() {
        let reg = make_registry();
        let (spell, _) = reg.get_instant_spell("exevo  vis  lux").expect("match");
        assert_eq!(spell.name, "Energy Beam");
    }

    #[test]
    fn get_instant_spell_with_param() {
        let reg = make_registry();
        let (spell, param) = reg.get_instant_spell("exiva PlayerName").expect("match");
        assert_eq!(spell.name, "Find Person");
        assert_eq!(param, "PlayerName");
    }

    #[test]
    fn get_instant_spell_no_match() {
        let reg = make_registry();
        assert!(reg.get_instant_spell("hello world").is_none());
    }

    #[test]
    fn get_instant_spell_no_param_rejected() {
        let reg = make_registry();
        // "exori blah" — Berserk has no param, so extra text rejects
        assert!(reg.get_instant_spell("exori blah").is_none());
    }

    #[test]
    fn get_instant_spell_with_quoted_param() {
        let reg = make_registry();
        let (spell, param) = reg
            .get_instant_spell("exiva \"Player Name\"")
            .expect("match");
        assert_eq!(spell.name, "Find Person");
        assert_eq!(param, "Player Name");
    }

    #[test]
    fn get_instant_spell_quoted_without_space_rejected() {
        let reg = make_registry();
        // "exiva\"" without space before quote — rejected by C++ `spells.cpp:232`
        assert!(reg.get_instant_spell("exiva\"").is_none());
    }

    // --- merge_spell_words tests ---

    #[test]
    fn merge_spell_words_removes_commas() {
        assert_eq!(merge_spell_words("ex,ori"), "exori");
        assert_eq!(merge_spell_words("ex,evo, vis, lux"), "exevo vis lux");
    }
}
