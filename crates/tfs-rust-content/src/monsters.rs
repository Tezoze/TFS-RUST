//! Monster definitions from `data/monster/` (index + per-file XML).
// C++ reference: `src/monsters.cpp` `Monsters::loadMonster`, `loadLootItem`, `deserializeSpell` (attacks/defenses parsed as spell nodes).

use quick_xml::Reader;
use quick_xml::events::Event;
use roxmltree::Document;
use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::{info, warn};

use crate::items::ItemDatabase;

/// Same cap as TFS `MAX_LOOTCHANCE` (`src/monsters.h`).
pub const MAX_LOOTCHANCE: i32 = 100_000;

#[derive(Debug, Clone)]
pub struct LootBlock {
    pub id: u32,
    pub countmax: i32,
    pub chance: i32,
    pub sub_type: i32,
    pub action_id: i32,
    pub text: String,
    pub child_loot: Vec<LootBlock>,
}

#[derive(Debug, Clone)]
pub struct MonsterSpellNode {
    /// Element local name, e.g. `attack`, `defense`, `melee`.
    pub element: String,
    pub attributes: HashMap<String, String>,
    /// Nested `<attribute key="..." value="..."/>` pairs.
    pub attribute_children: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct MonsterDefenses {
    pub armor: Option<i32>,
    pub defense: Option<i32>,
    pub spells: Vec<MonsterSpellNode>,
    /// `<immunity poison="1"/>` — `crmain.cc:548` `RaceData[Race].NoPoison`.
    pub immunity_poison: bool,
    /// `<immunity fire="1"/>` — `crmain.cc:549` `RaceData[Race].NoBurning`.
    pub immunity_fire: bool,
    /// `<immunity energy="1"/>` — `crmain.cc:550` `RaceData[Race].NoEnergy`.
    pub immunity_energy: bool,
    /// `<immunity lifedrain="1"/>` — `crmain.cc:619` `RaceData[Race].NoLifeDrain`. PC-3 (M3′):
    /// non-physical immunity for `DAMAGE_LIFEDRAIN`; `Damage(LIFEDRAIN)` emits `EFFECT_BLOCK_HIT`
    /// and returns 0.
    pub immunity_life_drain: bool,
    /// `<immunity invisible="1"/>` — `crmain.cc:1493` `RaceData[Race].SeeInvisible`.
    pub see_invisible: bool,
    /// `<immunity physical="1"/>` — `crmain.cc:615` `RaceData[Race].NoHit`. Physical damage
    /// immunity: `Damage(PHYSICAL)` emits `EFFECT_BLOCK_HIT` and returns 0.
    pub immunity_physical: bool,
    /// XML `paralyze`; 772 `NoParalyze` (`crmain.cc:1515` / `magic.cc` speed impact).
    pub immunity_paralyze: bool,
    /// XML `outfit`; TFS/TVP surface — no 772 `RaceData` twin; store for the data pack.
    pub immunity_outfit: bool,
}

/// Monster `<look>` block — C++ `MonsterType` look fields (`monsters.cpp` `loadMonster`).
#[derive(Debug, Clone)]
pub struct MonsterOutfit {
    pub look_type: i32,
    pub look_head: i32,
    pub look_body: i32,
    pub look_legs: i32,
    pub look_feet: i32,
    pub look_addons: i32,
    pub look_type_ex: i32,
    pub look_mount: i32,
    /// TVP `<look corpse="…">` — race corpse item id (`crmain.cc:204`).
    pub corpse_id: u16,
}

impl Default for MonsterOutfit {
    fn default() -> Self {
        Self {
            look_type: 136,
            look_head: 0,
            look_body: 0,
            look_legs: 0,
            look_feet: 0,
            look_addons: 0,
            look_type_ex: 0,
            look_mount: 0,
            corpse_id: 0,
        }
    }
}

/// AI/movement flags from `<flags>` — C++ `MonsterType` (`monsters.h`).
#[derive(Debug, Clone, Copy)]
pub struct MonsterTypeFlags {
    /// `<flag targetdistance=…>` — default 1 (`monsters.h`).
    pub target_distance: i32,
    /// `<flag runonhealth=…>` — default 0.
    pub run_away_health: i32,
    /// `<flag staticattack=…>` — default 95.
    pub static_attack_chance: u32,
    pub can_push_creatures: bool,
    pub can_push_items: bool,
    /// `<flag pushable=…>` — default true (`monsters.h`); forced false when `can_push_creatures`.
    pub pushable: bool,
    /// `<flag hostile=…>` — default true for wild monsters.
    pub is_hostile: bool,
    /// `<flag illusionable=…>` — default false (`monsters.h`).
    pub illusionable: bool,
    /// `<flag challengeable=…>` — default true (`monsters.h`).
    pub is_challengeable: bool,
    /// `<flag summonable=…>` — default false (`monsters.h`).
    pub summonable: bool,
    /// `<flag convinceable=…>` — default false (`monsters.h`).
    pub convinceable: bool,
    /// `<targetchange interval/speed=…>` — default 0 (`monsters.h`).
    pub change_target_speed: u32,
    /// `<targetchange chance=…>` — default 0.
    pub change_target_chance: i32,
    /// `<losetarget chance=…>` — 772 `RaceData.LoseTarget` (`crmain.cc:1244`, `:1473`).
    pub lose_target_percent: u8,
    /// `<targetstrategy nearest=…>` — `Strategy[0]` (`crmain.cc:1245`, default 100).
    pub strategy_nearest: u8,
    /// `<targetstrategy weakest=…>` — `Strategy[1]`.
    pub strategy_health: u8,
    /// `<targetstrategy mostdamage=…>` — `Strategy[2]`.
    pub strategy_damage: u8,
    /// `<targetstrategy random=…>` — residual `Strategy[3]` (never compared; leftover bucket).
    pub strategy_random: u8,
}

impl Default for MonsterTypeFlags {
    fn default() -> Self {
        Self {
            target_distance: 1,
            run_away_health: 0,
            static_attack_chance: 95,
            can_push_creatures: false,
            can_push_items: false,
            pushable: true,
            is_hostile: true,
            illusionable: false,
            is_challengeable: true,
            summonable: false,
            convinceable: false,
            change_target_speed: 0,
            change_target_chance: 0,
            lose_target_percent: 0,
            strategy_nearest: 100,
            strategy_health: 0,
            strategy_damage: 0,
            strategy_random: 0,
        }
    }
}

/// One `<summon>` entry under `<summons>` — TVP/TFS XML domain; CASTING maps to 772
/// `IMPACT_SUMMON` (`crnonpl.cc:2647`, `magic.cc` `TSummonImpact`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummonBlock {
    pub name: String,
    /// Cast gate modulus — XML `delay` (default 1); 772 `SpellData.Delay` / `rand() % Delay`.
    pub delay: i32,
    /// Cap for `SummonedCreatures < Maximum` — XML `max` (defaults to parent `maxSummons`).
    pub max: u32,
    /// Passed to place-creature when set — XML `force`.
    pub force: bool,
    /// XML `chance` (default 100); unused by 772 CASTING (delay modulus only).
    pub chance: i32,
}

#[derive(Debug, Clone)]
pub struct MonsterType {
    pub name: String,
    pub filename: String,
    pub name_description: String,
    pub race: String,
    pub experience: u32,
    pub speed: u32,
    pub health_now: u32,
    pub health_max: u32,
    pub outfit: MonsterOutfit,
    pub flags: MonsterTypeFlags,
    /// XML `manacost=` — summon/convince mana (`monsters.h` `manaCost`).
    pub mana_cost: u32,
    pub loot: Vec<LootBlock>,
    pub attack_spells: Vec<MonsterSpellNode>,
    pub defenses: MonsterDefenses,
    /// `<summons maxSummons=…>` — cap 100 (`monsters.cpp`).
    pub max_summons: u32,
    /// `<summons><summon …/></summons>` — merged into CASTING as `SpellImpact::Summon`.
    pub summons: Vec<SummonBlock>,
    /// `<voices><voice sentence="…"/></voices>` — 772 `RaceData.Talk` list (`crnonpl.cc:2442`).
    /// Empty when the monster has no `<voices>` block; the idle talk gate still draws `rand()%50`
    /// + `random(1, Talks)` for RNG parity but emits no packet (matches C++ `Talks == 0` return).
    ///   Each entry may carry a `#y `/`#Y ` prefix (decompile yell marker, `crnonpl.cc:2450`) — the
    ///   idle talk path strips it and switches to `TALKTYPE_MONSTER_YELL` on hit.
    pub talk_texts: Vec<String>,
}

impl MonsterType {
    /// Blood family for on-hit effect + splash, derived from the XML `race` attribute.
    /// See [`tfs_rust_common::enums::BloodType::from_race_str`].
    pub fn blood_type(&self) -> tfs_rust_common::enums::BloodType {
        tfs_rust_common::enums::BloodType::from_race_str(&self.race)
    }
}

pub struct MonsterDatabase {
    pub monsters: HashMap<String, MonsterType>,
}

impl MonsterDatabase {
    pub fn load_dir(dir: &Path, items: &ItemDatabase) -> Result<Self> {
        info!("Loading monsters from {:?}", dir);
        let index_path = dir.join("monsters.xml");
        let mut files = parse_monster_index(&index_path)?;

        if files.is_empty() {
            let monsters_dir = dir.join("monsters");
            if monsters_dir.exists() {
                for entry in
                    std::fs::read_dir(&monsters_dir).map_err(|e| TfsRustError::Content {
                        file: monsters_dir.to_string_lossy().into_owned(),
                        message: e.to_string(),
                    })?
                {
                    let entry = entry.map_err(|e| TfsRustError::Content {
                        file: monsters_dir.to_string_lossy().into_owned(),
                        message: e.to_string(),
                    })?;
                    if entry.path().extension().and_then(|ext| ext.to_str()) == Some("xml") {
                        let file = format!("monsters/{}", entry.file_name().to_string_lossy());
                        let stem = entry
                            .path()
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .replace('_', " ");
                        files.push(MonsterIndexEntry {
                            index_name: stem,
                            file,
                        });
                    }
                }
            }
        }

        let mut monsters = HashMap::new();
        for entry in files {
            let monster_path = dir.join(&entry.file);
            let monster = parse_monster_file(&monster_path, items)?;
            // C++ `Monsters::loadMonster(file, monsterName)` — map key is index `name`, not file attr.
            monsters.insert(entry.index_name.to_lowercase(), monster);
        }

        Ok(Self { monsters })
    }

    /// Lookup by index name (case-insensitive) — C++ `Monsters::getMonsterType`.
    pub fn get_by_name(&self, name: &str) -> Option<&MonsterType> {
        self.monsters.get(&name.to_lowercase())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MonsterIndexEntry {
    /// `monsters.xml` `<monster name="...">` — spawn lookup key in C++.
    pub(crate) index_name: String,
    pub(crate) file: String,
}

pub(crate) fn parse_monster_index(path: &Path) -> Result<Vec<MonsterIndexEntry>> {
    let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut entries = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"monster" => {
                let mut index_name = String::new();
                let mut file = String::new();
                for attr in e.attributes() {
                    let attr = attr.map_err(|err| TfsRustError::Content {
                        file: path.to_string_lossy().into_owned(),
                        message: err.to_string(),
                    })?;
                    if attr.key.as_ref() == b"name" {
                        index_name = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                    } else if attr.key.as_ref() == b"file" {
                        file = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                    }
                }
                if !index_name.is_empty() && !file.is_empty() {
                    entries.push(MonsterIndexEntry { index_name, file });
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(TfsRustError::Content {
                    file: path.to_string_lossy().into_owned(),
                    message: err.to_string(),
                });
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(entries)
}

fn find_monster_element<'a, 'input>(
    doc: &'a Document<'input>,
) -> Option<roxmltree::Node<'a, 'input>> {
    doc.root_element()
        .children()
        .find(|n| n.is_element() && n.has_tag_name("monster"))
        .or_else(|| {
            doc.descendants()
                .find(|n| n.is_element() && n.has_tag_name("monster"))
        })
}

fn parse_spell_node(node: roxmltree::Node<'_, '_>) -> MonsterSpellNode {
    let element = node.tag_name().name().to_string();
    let mut attributes = HashMap::new();
    for a in node.attributes() {
        attributes.insert(a.name().to_string(), a.value().to_string());
    }
    let mut attribute_children = Vec::new();
    for c in node.children().filter(|n| n.is_element()) {
        if c.tag_name().name().eq_ignore_ascii_case("attribute") {
            let k = c.attribute("key").unwrap_or("").to_string();
            let v = c.attribute("value").unwrap_or("").to_string();
            attribute_children.push((k, v));
        }
    }
    MonsterSpellNode {
        element,
        attributes,
        attribute_children,
    }
}

fn load_loot_item(
    node: roxmltree::Node<'_, '_>,
    items: &ItemDatabase,
    file: &str,
) -> Result<Option<LootBlock>> {
    let id_u32 = if let Some(s) = node.attribute("id") {
        let raw: i32 = s.parse().unwrap_or(0);
        if raw <= 0 || raw > u16::MAX as i32 {
            return Ok(None);
        }
        let id_u16 = raw as u16;
        if items
            .items
            .get(&id_u16)
            .map(|t| t.name.is_empty())
            .unwrap_or(true)
        {
            warn!(
                target: "tfs_rust_content",
                file = %file,
                id = raw,
                "unknown loot item id (skipping entry)"
            );
            return Ok(None);
        }
        raw as u32
    } else if let Some(name) = node.attribute("name") {
        match items.item_id_by_exact_name(name, file) {
            Ok(id) => id as u32,
            Err(e) => {
                warn!(target: "tfs_rust_content", file = %file, "{}", e);
                return Ok(None);
            }
        }
    } else {
        return Ok(None);
    };

    let id_u16 = id_u32 as u16;

    let countmax = node
        .attribute("countmax")
        .and_then(|a| a.parse::<i32>().ok())
        .map(|c| c.max(1))
        .unwrap_or(1);

    let chance = if let Some(a) = node
        .attribute("chance")
        .or_else(|| node.attribute("chance1"))
    {
        let loot_chance: i32 = a.parse().unwrap_or(MAX_LOOTCHANCE);
        if loot_chance > MAX_LOOTCHANCE {
            warn!(
                target: "tfs_rust_content",
                file = %file,
                chance = loot_chance,
                "loot chance above MAX_LOOTCHANCE (capped)"
            );
        }
        loot_chance.min(MAX_LOOTCHANCE)
    } else {
        MAX_LOOTCHANCE
    };

    let sub_type = if let Some(a) = node.attribute("subtype") {
        a.parse().unwrap_or(0)
    } else {
        items.charges_default(id_u16)
    };

    let action_id = node
        .attribute("actionId")
        .and_then(|a| a.parse().ok())
        .unwrap_or(0);

    let text = node.attribute("text").unwrap_or("").to_string();

    let mut child_loot = Vec::new();
    if items.is_container(id_u16) {
        let inside = node
            .children()
            .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("inside"));
        let iter: Box<dyn Iterator<Item = roxmltree::Node<'_, '_>>> = if let Some(ins) = inside {
            Box::new(ins.children().filter(|n| n.is_element()))
        } else {
            Box::new(node.children().filter(|n| n.is_element()))
        };
        for sub in iter {
            if sub.tag_name().name().eq_ignore_ascii_case("item")
                && let Some(child) = load_loot_item(sub, items, file)?
            {
                child_loot.push(child);
            }
        }
    }

    Ok(Some(LootBlock {
        id: id_u32,
        countmax,
        chance,
        sub_type,
        action_id,
        text,
        child_loot,
    }))
}

fn parse_loot_section(
    loot_el: roxmltree::Node<'_, '_>,
    items: &ItemDatabase,
    file: &str,
) -> Result<Vec<LootBlock>> {
    let mut out = Vec::new();
    for child in loot_el.children().filter(|n| n.is_element()) {
        if child.tag_name().name().eq_ignore_ascii_case("item")
            && let Some(block) = load_loot_item(child, items, file)?
        {
            out.push(block);
        }
    }
    Ok(out)
}

pub(crate) fn parse_monster_file(path: &Path, items: &ItemDatabase) -> Result<MonsterType> {
    let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    let file_str = path.to_string_lossy().into_owned();
    parse_monster_xml(&xml, &file_str, items)
}

pub(crate) fn parse_monster_xml(
    xml: &str,
    file_str: &str,
    items: &ItemDatabase,
) -> Result<MonsterType> {
    let doc = Document::parse(xml).map_err(|e| TfsRustError::Content {
        file: file_str.to_string(),
        message: e.to_string(),
    })?;

    let monster = find_monster_element(&doc).ok_or_else(|| TfsRustError::Content {
        file: file_str.to_string(),
        message: "missing root <monster>".to_string(),
    })?;

    let mut name = String::new();
    let mut name_description = String::new();
    let mut race = String::new();
    let mut experience = 0u32;
    let mut speed = 0u32;
    let mut health_now = 0u32;
    let mut health_max = 0u32;

    if let Some(a) = monster.attribute("name") {
        name = a.to_string();
    }
    if let Some(a) = monster.attribute("nameDescription") {
        name_description = a.to_string();
    }
    if let Some(a) = monster.attribute("race") {
        race = a.to_string();
    }
    if let Some(a) = monster.attribute("experience") {
        experience = a.parse().unwrap_or(0);
    }
    if let Some(a) = monster.attribute("speed") {
        speed = a.parse().unwrap_or(0);
    }
    let mut mana_cost = 0u32;
    if let Some(a) = monster.attribute("manacost") {
        mana_cost = a.parse().unwrap_or(0);
    }

    for child in monster.children().filter(|n| n.is_element()) {
        let tag = child.tag_name().name();
        if tag.eq_ignore_ascii_case("health") {
            if let Some(a) = child.attribute("now") {
                health_now = a.parse().unwrap_or(0);
            }
            if let Some(a) = child.attribute("max") {
                health_max = a.parse().unwrap_or(0);
            }
        }
    }

    if name.is_empty() {
        return Err(TfsRustError::Content {
            file: file_str.to_string(),
            message: "monster file missing root 'monster name'".to_string(),
        });
    }

    let mut outfit = MonsterOutfit::default();
    let mut flags = MonsterTypeFlags::default();
    let mut loot = Vec::new();
    let mut attack_spells = Vec::new();
    let mut defenses = MonsterDefenses {
        armor: None,
        defense: None,
        spells: Vec::new(),
        immunity_poison: false,
        immunity_fire: false,
        immunity_energy: false,
        immunity_life_drain: false,
        see_invisible: false,
        immunity_physical: false,
        immunity_paralyze: false,
        immunity_outfit: false,
    };
    let mut talk_texts = Vec::new();
    let mut max_summons = 0u32;
    let mut summons = Vec::new();

    for child in monster.children().filter(|n| n.is_element()) {
        let tag = child.tag_name().name();
        if tag.eq_ignore_ascii_case("flags") {
            parse_monster_flags(child, &mut flags, file_str);
        } else if tag.eq_ignore_ascii_case("look") {
            outfit.look_type = child
                .attribute("type")
                .and_then(|a| a.parse().ok())
                .unwrap_or(136);
            outfit.look_head = child
                .attribute("head")
                .and_then(|a| a.parse().ok())
                .unwrap_or(0);
            outfit.look_body = child
                .attribute("body")
                .and_then(|a| a.parse().ok())
                .unwrap_or(0);
            outfit.look_legs = child
                .attribute("legs")
                .and_then(|a| a.parse().ok())
                .unwrap_or(0);
            outfit.look_feet = child
                .attribute("feet")
                .and_then(|a| a.parse().ok())
                .unwrap_or(0);
            outfit.look_addons = child
                .attribute("addons")
                .and_then(|a| a.parse().ok())
                .unwrap_or(0);
            outfit.look_type_ex = child
                .attribute("typeex")
                .and_then(|a| a.parse().ok())
                .unwrap_or(0);
            outfit.look_mount = child
                .attribute("mount")
                .and_then(|a| a.parse().ok())
                .unwrap_or(0);
            outfit.corpse_id = child
                .attribute("corpse")
                .and_then(|a| a.parse().ok())
                .unwrap_or(0);
        } else if tag.eq_ignore_ascii_case("loot") {
            loot = parse_loot_section(child, items, file_str)?;
        } else if tag.eq_ignore_ascii_case("attacks") {
            for a in child.children().filter(|n| n.is_element()) {
                attack_spells.push(parse_spell_node(a));
            }
        } else if tag.eq_ignore_ascii_case("defenses") {
            defenses.armor = child.attribute("armor").and_then(|a| a.parse().ok());
            defenses.defense = child.attribute("defense").and_then(|a| a.parse().ok());
            for d in child.children().filter(|n| n.is_element()) {
                defenses.spells.push(parse_spell_node(d));
            }
        } else if tag.eq_ignore_ascii_case("immunities") {
            for imm in child.children().filter(|n| n.is_element()) {
                if imm.tag_name().name().eq_ignore_ascii_case("immunity") {
                    if imm
                        .attribute("poison")
                        .or_else(|| imm.attribute("earth"))
                        .is_some_and(parse_bool_flag)
                    {
                        defenses.immunity_poison = true;
                    }
                    // C++ `RaceData[Race].NoBurning` — `crmain.cc:549`.
                    if imm.attribute("fire").is_some_and(parse_bool_flag) {
                        defenses.immunity_fire = true;
                    }
                    // C++ `RaceData[Race].NoEnergy` — `crmain.cc:550`.
                    if imm.attribute("energy").is_some_and(parse_bool_flag) {
                        defenses.immunity_energy = true;
                    }
                    // C++ `RaceData[Race].NoLifeDrain` — `crmain.cc:619`. PC-3 (M3′).
                    if imm.attribute("lifedrain").is_some_and(parse_bool_flag) {
                        defenses.immunity_life_drain = true;
                    }
                    // C++ `RaceData[Race].SeeInvisible` — `crmain.cc:1493`.
                    if imm.attribute("invisible").is_some_and(parse_bool_flag) {
                        defenses.see_invisible = true;
                    }
                    // C++ `RaceData[Race].NoHit` — `crmain.cc:615`. Physical damage immunity.
                    if imm.attribute("physical").is_some_and(parse_bool_flag) {
                        defenses.immunity_physical = true;
                    }
                    // C++ `RaceData[Race].NoParalyze` — `crmain.cc:1515` / `magic.cc` speed impact.
                    if imm.attribute("paralyze").is_some_and(parse_bool_flag) {
                        defenses.immunity_paralyze = true;
                    }
                    // TFS/TVP `<immunity outfit>` — no 772 `RaceData` twin; stored for the data pack.
                    if imm.attribute("outfit").is_some_and(parse_bool_flag) {
                        defenses.immunity_outfit = true;
                    }
                }
            }
        } else if tag.eq_ignore_ascii_case("targetchange") {
            parse_target_change(child, &mut flags, file_str);
        } else if tag.eq_ignore_ascii_case("targetstrategy") {
            parse_target_strategy(child, &mut flags);
        } else if tag.eq_ignore_ascii_case("losetarget") {
            parse_lose_target(child, &mut flags);
        } else if tag.eq_ignore_ascii_case("summons") {
            parse_summons(child, &mut max_summons, &mut summons, file_str);
        } else if tag.eq_ignore_ascii_case("voices") {
            // 772 `RaceData.Talk` list — `<voice sentence="…"/>` (`crnonpl.cc:2442`, `crmain.cc:1551`).
            for voice in child.children().filter(|n| n.is_element()) {
                if voice.tag_name().name().eq_ignore_ascii_case("voice")
                    && let Some(sentence) = voice.attribute("sentence")
                    && !sentence.is_empty()
                {
                    let mut text = sentence.to_string();
                    if voice.attribute("yell").is_some_and(parse_bool_flag)
                        && !text.starts_with("#y ")
                        && !text.starts_with("#Y ")
                    {
                        text.insert_str(0, "#y ");
                    }
                    talk_texts.push(text);
                }
            }
        }
    }

    Ok(MonsterType {
        name,
        filename: file_str.to_string(),
        name_description,
        race,
        experience,
        speed,
        health_now,
        health_max,
        outfit,
        flags,
        mana_cost,
        loot,
        attack_spells,
        defenses,
        max_summons,
        summons,
        talk_texts,
    })
}

/// `<summons maxSummons=…><summon name=… delay=… max=…/></summons>` — TVP `monsters.cpp` ~1226.
fn parse_summons(
    node: roxmltree::Node<'_, '_>,
    max_summons: &mut u32,
    summons: &mut Vec<SummonBlock>,
    file: &str,
) {
    if let Some(a) = node.attribute("maxSummons") {
        *max_summons = a.parse::<u32>().unwrap_or(0).min(100);
    } else {
        warn!(file, "monster summons missing maxSummons");
    }

    for summon_node in node.children().filter(|n| n.is_element()) {
        if !summon_node.tag_name().name().eq_ignore_ascii_case("summon") {
            continue;
        }
        let Some(name) = summon_node.attribute("name") else {
            warn!(file, "monster summon missing name");
            continue;
        };
        let mut chance = 100i32;
        let mut delay = 1i32;
        let mut max = *max_summons;
        let mut force = false;

        if let Some(a) = summon_node
            .attribute("speed")
            .or_else(|| summon_node.attribute("interval"))
        {
            // TFS ms tick path — stored only if present; CASTING uses `delay`.
            let _speed = a.parse::<i32>().unwrap_or(1000).max(1);
            let _ = _speed;
        }

        if let Some(a) = summon_node.attribute("chance") {
            chance = a.parse().unwrap_or(100);
            if chance > 100 {
                warn!(file, chance, "summon chance out of bounds, clamping to 100");
                chance = 100;
            }
        } else if let Some(a) = summon_node.attribute("delay") {
            delay = a.parse().unwrap_or(1);
            if delay > 100 {
                warn!(file, delay, "summon delay out of bounds, clamping to 100");
                delay = 100;
            }
            if delay < 1 {
                delay = 1;
            }
        }

        if let Some(a) = summon_node.attribute("max") {
            max = a.parse().unwrap_or(max);
        }
        if let Some(a) = summon_node.attribute("force") {
            force = parse_bool_flag(a);
        }

        summons.push(SummonBlock {
            name: name.to_string(),
            delay,
            max,
            force,
            chance,
        });
    }
}

/// C++ `Monsters::loadMonster` flags block (`monsters.cpp` ~959–1001).
fn parse_monster_flags(node: roxmltree::Node<'_, '_>, flags: &mut MonsterTypeFlags, file: &str) {
    for flag in node.children().filter(|n| n.is_element()) {
        if !flag.tag_name().name().eq_ignore_ascii_case("flag") {
            continue;
        }
        for attr in flag.attributes() {
            let name = attr.name();
            let value = attr.value();
            match name {
                "canpushitems" => flags.can_push_items = parse_bool_flag(value),
                "canpushcreatures" => flags.can_push_creatures = parse_bool_flag(value),
                "pushable" => flags.pushable = parse_bool_flag(value),
                "staticattack" => {
                    let mut v: u32 = value.parse().unwrap_or(flags.static_attack_chance);
                    if v > 100 {
                        warn!(
                            file,
                            staticattack = v,
                            "staticattack greater than 100, clamping to 100"
                        );
                        v = 100;
                    }
                    flags.static_attack_chance = v;
                }
                "targetdistance" => {
                    let mut v: i32 = value.parse().unwrap_or(1);
                    if v < 1 {
                        warn!(file, "targetdistance less than 1, using 1");
                        v = 1;
                    }
                    flags.target_distance = v;
                }
                "runonhealth" => flags.run_away_health = value.parse().unwrap_or(0),
                "hostile" => flags.is_hostile = parse_bool_flag(value),
                "illusionable" => flags.illusionable = parse_bool_flag(value),
                "challengeable" => flags.is_challengeable = parse_bool_flag(value),
                "summonable" => flags.summonable = parse_bool_flag(value),
                "convinceable" => flags.convinceable = parse_bool_flag(value),
                _ => {}
            }
        }
    }
    if flags.can_push_creatures {
        // C++: canPushCreatures forces non-pushable (`monsters.cpp` ~997–1000).
        flags.pushable = false;
    }
}

/// `<targetstrategy nearest= weakest= mostdamage= random= />` — 772 `strategy=(w,w,w,w)`
/// (`crmain.cc:1476-1484`). Bucket 3 is residual; `random` is stored but never compared.
fn parse_target_strategy(node: roxmltree::Node<'_, '_>, flags: &mut MonsterTypeFlags) {
    if let Some(a) = node.attribute("nearest") {
        flags.strategy_nearest = a.parse().unwrap_or(flags.strategy_nearest);
    }
    if let Some(a) = node.attribute("weakest") {
        flags.strategy_health = a.parse().unwrap_or(flags.strategy_health);
    }
    if let Some(a) = node.attribute("mostdamage") {
        flags.strategy_damage = a.parse().unwrap_or(flags.strategy_damage);
    }
    if let Some(a) = node.attribute("random") {
        flags.strategy_random = a.parse().unwrap_or(flags.strategy_random);
    }
}

/// `<losetarget chance=N/>` — 772 `losetarget N` (`crmain.cc:1473`).
fn parse_lose_target(node: roxmltree::Node<'_, '_>, flags: &mut MonsterTypeFlags) {
    let raw = node
        .attribute("chance")
        .or_else(|| node.attribute("value"))
        .or_else(|| node.attribute("percent"));
    if let Some(a) = raw {
        flags.lose_target_percent = a.parse().unwrap_or(0);
    }
}

/// `<targetchange>` — TFS 1.4.2 uses `interval` + `chance` (`src/monsters.cpp` ~1007, warns if
/// interval missing). TVP 7.72 / `gameserver` data often has `chance` only (`gameserver` ~945,
/// leaves `changeTargetSpeed` at 0).
fn parse_target_change(node: roxmltree::Node<'_, '_>, flags: &mut MonsterTypeFlags, file: &str) {
    if let Some(a) = node
        .attribute("speed")
        .or_else(|| node.attribute("interval"))
    {
        flags.change_target_speed = a.parse().unwrap_or(0);
    }
    if let Some(a) = node.attribute("chance") {
        let mut chance: i32 = a.parse().unwrap_or(0);
        if chance > 100 {
            warn!(
                file,
                chance, "targetchange chance out of bounds, clamping to 100"
            );
            chance = 100;
        }
        flags.change_target_chance = chance;
    } else {
        warn!(file, "monster targetchange missing chance");
    }
}

fn parse_bool_flag(value: &str) -> bool {
    value == "1" || value.eq_ignore_ascii_case("true")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_monster_ai_flags() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<monster name="Test" speed="100">
    <health now="20" max="20"/>
    <flags>
        <flag targetdistance="4" runonhealth="5" staticattack="90"
              canpushcreatures="1" canpushitems="1" hostile="1"/>
    </flags>
</monster>"#;
        let items = ItemDatabase {
            items: HashMap::new(),
            client_to_server: HashMap::new(),
        };
        let m = parse_monster_xml(xml, "test.xml", &items).expect("parse");
        assert_eq!(m.flags.target_distance, 4);
        assert_eq!(m.flags.run_away_health, 5);
        assert_eq!(m.flags.static_attack_chance, 90);
        assert!(m.flags.can_push_creatures);
        assert!(m.flags.can_push_items);
        assert!(m.flags.is_hostile);
    }

    #[test]
    fn parses_targetstrategy_and_losetarget() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<monster name="Ferumbras" speed="100">
    <health now="20" max="20"/>
    <targetstrategy nearest="60" weakest="5" mostdamage="30" random="5" />
    <losetarget chance="10" />
</monster>"#;
        let items = ItemDatabase {
            items: HashMap::new(),
            client_to_server: HashMap::new(),
        };
        let m = parse_monster_xml(xml, "ferumbras.xml", &items).expect("parse");
        assert_eq!(m.flags.strategy_nearest, 60);
        assert_eq!(m.flags.strategy_health, 5);
        assert_eq!(m.flags.strategy_damage, 30);
        assert_eq!(m.flags.strategy_random, 5);
        assert_eq!(m.flags.lose_target_percent, 10);
    }

    #[test]
    fn parses_monster_summons_block() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<monster name="Giant Spider" speed="80">
    <health now="1300" max="1300"/>
    <summons maxSummons="2">
        <summon name="Poison Spider" max="2" delay="10" />
    </summons>
</monster>"#;
        let items = ItemDatabase {
            items: HashMap::new(),
            client_to_server: HashMap::new(),
        };
        let m = parse_monster_xml(xml, "giant spider.xml", &items).expect("parse");
        assert_eq!(m.max_summons, 2);
        assert_eq!(m.summons.len(), 1);
        assert_eq!(m.summons[0].name, "Poison Spider");
        assert_eq!(m.summons[0].delay, 10);
        assert_eq!(m.summons[0].max, 2);
    }

    #[test]
    fn index_name_is_lookup_key_not_file_name_attr() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("monster/monsters.xml").is_file() {
            return;
        }
        let items = ItemDatabase {
            items: HashMap::new(),
            client_to_server: HashMap::new(),
        };
        let db = MonsterDatabase::load_dir(&data.join("monster"), &items).expect("load monsters");
        let red = db
            .monsters
            .get("red butterfly")
            .expect("index key red butterfly");
        assert_eq!(red.name, "Butterfly", "display name comes from file XML");
        assert!(
            !db.monsters.contains_key("butterfly"),
            "file name attr must not be the key"
        );
    }

    #[test]
    fn voice_yell_prefixes_talk_text() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<monster name="Yeller" speed="100">
    <health now="20" max="20"/>
    <voices>
        <voice sentence="GROOAAARRR" yell="1" />
        <voice sentence="hello" yell="0" />
    </voices>
</monster>"#;
        let items = ItemDatabase {
            items: HashMap::new(),
            client_to_server: HashMap::new(),
        };
        let m = parse_monster_xml(xml, "yeller.xml", &items).expect("parse");
        assert_eq!(
            m.talk_texts,
            vec!["#y GROOAAARRR".to_string(), "hello".to_string()]
        );
    }

    fn empty_items() -> ItemDatabase {
        ItemDatabase {
            items: HashMap::new(),
            client_to_server: HashMap::new(),
        }
    }

    /// XML has 8 immunity attrs; Amazon is all-false, Ancient Scarab sets outfit+paralyze.
    #[test]
    fn parses_all_eight_immunity_flags() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/monster");
        let amazon_path = data.join("monsters/amazon.xml");
        let scarab_path = data.join("monsters/ancient scarab.xml");
        let dragon_path = data.join("monsters/dragon.xml");
        if !amazon_path.is_file() || !scarab_path.is_file() || !dragon_path.is_file() {
            return;
        }
        let items = empty_items();
        let amazon = parse_monster_file(&amazon_path, &items).expect("amazon xml");
        assert!(!amazon.defenses.immunity_fire);
        assert!(!amazon.defenses.immunity_energy);
        assert!(!amazon.defenses.immunity_poison);
        assert!(!amazon.defenses.immunity_physical);
        assert!(!amazon.defenses.immunity_outfit);
        assert!(!amazon.defenses.immunity_life_drain);
        assert!(!amazon.defenses.immunity_paralyze);
        assert!(!amazon.defenses.see_invisible);

        let scarab = parse_monster_file(&scarab_path, &items).expect("ancient scarab xml");
        assert!(scarab.defenses.immunity_paralyze);
        assert!(scarab.defenses.immunity_outfit);
        assert!(scarab.defenses.immunity_life_drain);
        assert!(scarab.defenses.see_invisible);

        let dragon = parse_monster_file(&dragon_path, &items).expect("dragon xml");
        assert!(dragon.defenses.immunity_paralyze);
        assert!(
            !dragon.defenses.immunity_outfit,
            "dragon.xml outfit=0"
        );
        assert!(dragon.defenses.immunity_fire);
        assert!(dragon.defenses.see_invisible);
    }
}
