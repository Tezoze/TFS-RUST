//! Monster definitions from `data/monster/` (index + per-file XML).
// C++ reference: `src/monsters.cpp` `Monsters::loadMonster`, `loadLootItem`, `deserializeSpell` (attacks/defenses parsed as spell nodes).

use quick_xml::events::Event;
use quick_xml::Reader;
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
    pub loot: Vec<LootBlock>,
    pub attack_spells: Vec<MonsterSpellNode>,
    pub defenses: MonsterDefenses,
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
                        files.push(format!("monsters/{}", entry.file_name().to_string_lossy()));
                    }
                }
            }
        }

        let mut monsters = HashMap::new();
        for file in files {
            let monster_path = dir.join(&file);
            let monster = parse_monster_file(&monster_path, items)?;
            monsters.insert(monster.name.to_lowercase(), monster);
        }

        Ok(Self { monsters })
    }
}

fn parse_monster_index(path: &Path) -> Result<Vec<String>> {
    let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    let mut reader = Reader::from_str(&xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut files = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"monster" => {
                for attr in e.attributes() {
                    let attr = attr.map_err(|err| TfsRustError::Content {
                        file: path.to_string_lossy().into_owned(),
                        message: err.to_string(),
                    })?;
                    if attr.key.as_ref() == b"file" {
                        files.push(String::from_utf8_lossy(attr.value.as_ref()).into_owned());
                    }
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

    Ok(files)
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
            if sub.tag_name().name().eq_ignore_ascii_case("item") {
                if let Some(child) = load_loot_item(sub, items, file)? {
                    child_loot.push(child);
                }
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
        if child.tag_name().name().eq_ignore_ascii_case("item") {
            if let Some(block) = load_loot_item(child, items, file)? {
                out.push(block);
            }
        }
    }
    Ok(out)
}

fn parse_monster_file(path: &Path, items: &ItemDatabase) -> Result<MonsterType> {
    let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    let file_str = path.to_string_lossy().into_owned();

    let doc = Document::parse(&xml).map_err(|e| TfsRustError::Content {
        file: file_str.clone(),
        message: e.to_string(),
    })?;

    let monster = find_monster_element(&doc).ok_or_else(|| TfsRustError::Content {
        file: file_str.clone(),
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
            file: file_str,
            message: "monster file missing root 'monster name'".to_string(),
        });
    }

    let mut loot = Vec::new();
    let mut attack_spells = Vec::new();
    let mut defenses = MonsterDefenses {
        armor: None,
        defense: None,
        spells: Vec::new(),
    };

    for child in monster.children().filter(|n| n.is_element()) {
        let tag = child.tag_name().name();
        if tag.eq_ignore_ascii_case("loot") {
            loot = parse_loot_section(child, items, &file_str)?;
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
        }
    }

    Ok(MonsterType {
        name,
        filename: file_str,
        name_description,
        race,
        experience,
        speed,
        health_now,
        health_max,
        loot,
        attack_spells,
        defenses,
    })
}
