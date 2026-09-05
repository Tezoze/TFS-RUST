//! SpellNr ↔ display name — `InitSpells` `Spell->Comment` (`magic.cc:4416+`).
//!
//! NPC `TeachSpell` / `SpellKnown` use SpellNr; TFS `player_spells` stores names.
//! Cast gate is `config.lua` `learnSpells`, not per-script `needLearn`.
//!
//! - Pack: `Player::hasLearnedInstantSpell` / `learnInstantSpell`.
//! - Corpus: `TPlayer::SpellKnown` / `LearnSpell` — `crplayer.cc:1130-1150`.

use tfs_rust_content::spells::{InstantSpellDef, SpellRegistry};

/// Corpus `CreateSpell` Comment, sorted by SpellNr.
const SPELL_NR_NAMES: &[(i32, &str)] = &[
    (1, "Light Healing"),
    (2, "Intense Healing"),
    (3, "Ultimate Healing"),
    (4, "Intense Healing Rune"),
    (5, "Ultimate Healing Rune"),
    (6, "Haste"),
    (7, "Light Magic Missile"),
    (8, "Heavy Magic Missile"),
    (9, "Summon Creature"),
    (10, "Light"),
    (11, "Great Light"),
    (12, "Convince Creature"),
    (13, "Energy Wave"),
    (14, "Chameleon"),
    (15, "Fireball"),
    (16, "Great Fireball"),
    (17, "Firebomb"),
    (18, "Explosion"),
    (19, "Fire Wave"),
    (20, "Find Person"),
    (21, "Sudden Death"),
    (22, "Energy Beam"),
    (23, "Great Energy Beam"),
    (24, "Ultimate Explosion"),
    (25, "Fire Field"),
    (26, "Poison Field"),
    (27, "Energy Field"),
    (28, "Fire Wall"),
    (29, "Antidote"),
    (30, "Destroy Field"),
    (31, "Antidote Rune"),
    (32, "Poison Wall"),
    (33, "Energy Wall"),
    (34, "Get Item"),
    (35, "Get Item"),
    (37, "Move"),
    (38, "Creature Illusion"),
    (39, "Strong Haste"),
    (40, "Get Experience"),
    (41, "Change Data"),
    (42, "Food"),
    (44, "Magic Shield"),
    (45, "Invisible"),
    (46, "Get Skill Experience"),
    (47, "Teleport to Friend"),
    (48, "Poisoned Arrow"),
    (49, "Explosive Arrow"),
    (50, "Soulfire"),
    (51, "Conjure Arrow"),
    (52, "Retrieve Friend"),
    (53, "Summon Wild Creature"),
    (54, "Paralyze"),
    (55, "Energybomb"),
    (56, "Poison Storm"),
    (57, "Banish Account"),
    (58, "Get Position"),
    (60, "Temple Teleport"),
    (61, "Delete Account"),
    (62, "Set Namerule"),
    (63, "Create Gold"),
    (64, "Change Profession or Sex"),
    (65, "Entry in Criminal Record"),
    (66, "Namelock"),
    (67, "Kick Player"),
    (68, "Delete Character"),
    (69, "Banish IP Address"),
    (70, "Banish Character"),
    (71, "Invite Guests"),
    (72, "Invite Subowners"),
    (73, "Kick Guest"),
    (74, "Edit Door"),
    (75, "Ultimate Light"),
    (76, "Magic Rope"),
    (77, "Envenom"),
    (78, "Desintegrate"),
    (79, "Conjure Bolt"),
    (80, "Berserk"),
    (81, "Levitate"),
    (82, "Mass Healing"),
    (83, "Animate Dead"),
    (84, "Heal Friend"),
    (85, "Undead Legion"),
    (86, "Magic Wall"),
    (87, "Force Strike"),
    (88, "Energy Strike"),
    (89, "Flame Strike"),
    (90, "Cancel Invisibility"),
    (91, "Poisonbomb"),
    (92, "Enchant Staff"),
    (93, "Challenge"),
    (94, "Wild Growth"),
    (95, "Power Bolt"),
    (96, "Get Quest Value"),
    (97, "Set Quest Value"),
    (98, "Desintegrate Spell"),
    (99, "Levitate Gamemaster"),
    (100, "Clear Quest Values"),
    (101, "Kill All Creatures"),
    (102, "Start Monsterraid"),
];

/// Pack `spell:name()` when it differs from `InitSpells` Comment.
const SPELL_NAME_ALIASES: &[(&str, &str)] = &[
    ("Invisible", "Invisibility"),
    ("Power Bolt", "Conjure Power Bolt"),
];

pub fn spell_name_for_nr(nr: i32) -> Option<&'static str> {
    SPELL_NR_NAMES
        .binary_search_by_key(&nr, |p| p.0)
        .ok()
        .map(|i| SPELL_NR_NAMES[i].1)
}

fn names_equiv(a: &str, b: &str) -> bool {
    if a.eq_ignore_ascii_case(b) {
        return true;
    }
    SPELL_NAME_ALIASES.iter().any(|(corpus, pack)| {
        (corpus.eq_ignore_ascii_case(a) && pack.eq_ignore_ascii_case(b))
            || (pack.eq_ignore_ascii_case(a) && corpus.eq_ignore_ascii_case(b))
    })
}

fn spell_nr_for_name(name: &str) -> Option<i32> {
    SPELL_NR_NAMES
        .iter()
        .find(|(_, n)| names_equiv(n, name))
        .map(|(nr, _)| *nr)
}

fn pack_alias_for_corpus(corpus: &str) -> Option<&'static str> {
    SPELL_NAME_ALIASES
        .iter()
        .find(|(c, _)| c.eq_ignore_ascii_case(corpus))
        .map(|(_, pack)| *pack)
}

/// Display / persist name: pack `spell:name()` when registered, else Comment.
pub fn teach_spell_persist_key_for_registry(nr: i32, registry: &SpellRegistry) -> String {
    let Some(corpus) = spell_name_for_nr(nr) else {
        return nr.to_string();
    };
    if registry.instant_by_name.contains_key(corpus) {
        return corpus.to_string();
    }
    if let Some(pack) = pack_alias_for_corpus(corpus) {
        if registry.instant_by_name.contains_key(pack) {
            return pack.to_string();
        }
    }
    corpus.to_string()
}

pub fn teach_spell_persist_key(nr: i32) -> String {
    spell_name_for_nr(nr)
        .map(str::to_string)
        .unwrap_or_else(|| nr.to_string())
}

/// `CheckSpellbook` only wraps `CastSpell`/`RuneSpell` (ex/ut/ad).
/// House `al*` / GM `om*` and level-0 instants skip SpellKnown.
pub fn spoken_spell_requires_learn(spell: &InstantSpellDef) -> bool {
    if spell.level == 0 {
        return false;
    }
    let glued: String = spell
        .words
        .chars()
        .filter(|c| *c != ',' && !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    !(glued.starts_with("al") || glued.starts_with("om"))
}

pub fn persist_knows_spell_name(spells: &[String], name: &str) -> bool {
    let nr = spell_nr_for_name(name).map(|n| n.to_string());
    spells
        .iter()
        .any(|s| names_equiv(s, name) || nr.as_ref().is_some_and(|k| s == k))
}

pub fn persist_knows_spell(spells: &[String], spell: &InstantSpellDef) -> bool {
    persist_knows_spell_name(spells, &spell.name)
}

pub fn persist_knows_spell_nr(spells: &[String], nr: i32) -> bool {
    let key = nr.to_string();
    let name = spell_name_for_nr(nr);
    spells
        .iter()
        .any(|s| s == &key || name.is_some_and(|n| names_equiv(s, n)))
}

pub fn spell_level_for_nr(registry: &SpellRegistry, nr: i32) -> i32 {
    let Some(corpus) = spell_name_for_nr(nr) else {
        return 0;
    };
    if let Some(d) = registry.instant_by_name.get(corpus) {
        return d.level as i32;
    }
    if let Some(pack) = pack_alias_for_corpus(corpus) {
        if let Some(d) = registry.instant_by_name.get(pack) {
            return d.level as i32;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duria_types_map_to_comments() {
        assert_eq!(spell_name_for_nr(20), Some("Find Person"));
        assert_eq!(spell_name_for_nr(10), Some("Light"));
        assert_eq!(spell_name_for_nr(1), Some("Light Healing"));
        assert_eq!(spell_name_for_nr(29), Some("Antidote"));
        assert_eq!(spell_name_for_nr(11), Some("Great Light"));
        assert_eq!(spell_name_for_nr(80), Some("Berserk"));
    }

    #[test]
    fn house_words_skip_learn() {
        let house = InstantSpellDef {
            words: "aleta sio".into(),
            level: 0,
            ..Default::default()
        };
        assert!(!spoken_spell_requires_learn(&house));
        let haste = InstantSpellDef {
            words: "ut,ani, hur".into(),
            level: 14,
            name: "Haste".into(),
            ..Default::default()
        };
        assert!(spoken_spell_requires_learn(&haste));
    }

    #[test]
    fn persist_matches_name_or_nr() {
        let spell = InstantSpellDef {
            name: "Find Person".into(),
            ..Default::default()
        };
        assert!(persist_knows_spell(&["20".into()], &spell));
        assert!(persist_knows_spell(&["find person".into()], &spell));
        assert!(!persist_knows_spell(&["Light".into()], &spell));
        assert!(persist_knows_spell_nr(&["Find Person".into()], 20));
        assert_eq!(teach_spell_persist_key(20), "Find Person");
    }

    #[test]
    fn pack_aliases_match_corpus_comments() {
        let invis = InstantSpellDef {
            name: "Invisibility".into(),
            ..Default::default()
        };
        assert!(persist_knows_spell(&["Invisible".into()], &invis));
        assert!(persist_knows_spell(&["45".into()], &invis));
        assert!(persist_knows_spell_nr(&["Invisibility".into()], 45));
        assert!(persist_knows_spell_name(
            &["Invisible".into()],
            "Invisibility"
        ));

        let bolt = InstantSpellDef {
            name: "Conjure Power Bolt".into(),
            ..Default::default()
        };
        assert!(persist_knows_spell(&["Power Bolt".into()], &bolt));
        assert!(persist_knows_spell(&["95".into()], &bolt));

        let mut registry = SpellRegistry::default();
        registry.instant_by_name.insert(
            "Invisibility".into(),
            InstantSpellDef {
                name: "Invisibility".into(),
                level: 35,
                ..Default::default()
            },
        );
        assert_eq!(spell_level_for_nr(&registry, 45), 35);
        assert_eq!(
            teach_spell_persist_key_for_registry(45, &registry),
            "Invisibility"
        );
    }
}
