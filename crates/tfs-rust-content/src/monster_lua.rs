//! Lua-as-data writer/parser for monster defs (`data/monster/<slug>.lua`).
//!
//! Domain: TFS `monsters.cpp` `Monsters::loadMonster` (same `MonsterType` /
//! `MonsterSpellNode` / loot / summons surface as XML). Schema is Lua-as-data
//! (`return { schema = 1, … }`), not TFS `Game.createMonsterType`.
//! Plan: `tasks/monsters-lua-plan.md`.

use std::collections::HashMap;
use std::path::Path;

use mlua::{LuaSerdeExt, Table, Value};
use serde::Deserialize;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::warn;

use crate::data_lua::{load_data_table_str, require_schema, sandboxed_data_lua};
use crate::items::ItemDatabase;
use crate::monsters::{
    LootBlock, MAX_LOOTCHANCE, MonsterDefenses, MonsterOutfit, MonsterSpellNode, MonsterType,
    MonsterTypeFlags, SummonBlock, parse_monster_file, parse_monster_index,
};

/// Expected `schema` version for monster Lua defs.
pub const MONSTERS_SCHEMA: u32 = 1;

/// Lowercase index name with spaces turned into `_` (`"Red Butterfly"` → `red_butterfly`).
pub fn monster_lua_slug(index_name: &str) -> String {
    index_name
        .chars()
        .map(|c| {
            if c == ' ' {
                '_'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

/// Emit a schema-1 monster Lua file. `index_name` is the spawn/index key
/// (`monsters.xml` `name`), not `MonsterType.name` (display / file attr).
pub fn emit_monster_lua(
    index_name: &str,
    mtype: &MonsterType,
    items: Option<&ItemDatabase>,
) -> String {
    let mut out = String::new();
    out.push_str("-- Generated from XML. Source: ");
    out.push_str(&xml_source_label(&mtype.filename));
    out.push('\n');
    out.push_str("return {\n");
    emit_kv_u32(&mut out, 1, "schema", MONSTERS_SCHEMA);
    emit_kv_str(&mut out, 1, "name", index_name);
    if !mtype.name.eq_ignore_ascii_case(index_name) {
        emit_kv_str(&mut out, 1, "title", &mtype.name);
    }
    emit_kv_str(&mut out, 1, "description", &mtype.name_description);
    emit_kv_str(&mut out, 1, "race", &mtype.race);
    emit_kv_u32(&mut out, 1, "experience", mtype.experience);
    emit_kv_u32(&mut out, 1, "speed", mtype.speed);
    emit_kv_u32(&mut out, 1, "mana_cost", mtype.mana_cost);
    emit_kv_u32(&mut out, 1, "health", mtype.health_now);
    emit_kv_u32(&mut out, 1, "max_health", mtype.health_max);
    emit_outfit(&mut out, &mtype.outfit);
    emit_change_target(&mut out, &mtype.flags);
    emit_target_strategy(&mut out, &mtype.flags);
    if mtype.flags.lose_target_percent != 0 {
        out.push_str("  lose_target = { chance = ");
        out.push_str(&mtype.flags.lose_target_percent.to_string());
        out.push_str(" },\n");
    }
    emit_flags(&mut out, &mtype.flags);
    emit_attacks(&mut out, &mtype.attack_spells);
    emit_defenses(&mut out, &mtype.defenses);
    emit_immunities(&mut out, &mtype.defenses);
    emit_voices(&mut out, &mtype.talk_texts);
    emit_summons(&mut out, mtype.max_summons, &mtype.summons);
    emit_loot(&mut out, &mtype.loot, items);
    out.push_str("}\n");
    out
}

/// Parse a schema-1 monster Lua source into [`MonsterType`].
///
/// `MonsterType.name` is `title` when present, otherwise Lua `name`. The spawn
/// lookup key is Lua `name` (callers insert into the DB).
pub fn parse_monster_lua(src: &str, file: &str, items: &ItemDatabase) -> Result<MonsterType> {
    let lua = sandboxed_data_lua()?;
    let root = load_data_table_str(&lua, src, file)?;
    require_schema(&root, MONSTERS_SCHEMA)?;

    let def: MonsterDef =
        lua.from_value(Value::Table(root.clone()))
            .map_err(|e| TfsRustError::Content {
                file: file.to_string(),
                message: format!("deserialize monster failed: {e}"),
            })?;
    if def.name.is_empty() {
        return Err(TfsRustError::Content {
            file: file.to_string(),
            message: "monster lua missing 'name'".to_string(),
        });
    }

    let (max_summons, summons) = match root.get::<Value>("summons") {
        Ok(Value::Table(t)) => parse_summons_table(&t, file)?,
        _ => (0, Vec::new()),
    };

    let display_name = def.title.clone().unwrap_or_else(|| def.name.clone());
    let mut flags = flags_from_def(&def);
    if flags.can_push_creatures {
        flags.pushable = false;
    }

    let loot = loot_from_defs(def.loot.unwrap_or_default(), items, file)?;

    Ok(MonsterType {
        name: display_name,
        filename: file.to_string(),
        name_description: def.description.unwrap_or_default(),
        race: def.race.unwrap_or_default(),
        experience: def.experience.unwrap_or(0),
        speed: def.speed.unwrap_or(0),
        health_now: def.health.unwrap_or(0),
        health_max: def.max_health.unwrap_or(0),
        outfit: outfit_from_def(def.outfit),
        flags,
        mana_cost: def.mana_cost.unwrap_or(0),
        loot,
        attack_spells: spells_from_defs(def.attacks.unwrap_or_default(), "attack"),
        defenses: defenses_from_def(def.defenses, def.immunities),
        max_summons,
        summons,
        talk_texts: voices_from_defs(def.voices.unwrap_or_default()),
    })
}

/// Parse `monsters.xml` + per-file XML and write `data/monster/<slug>.lua`.
/// Overwrites existing files (including `dragon.lua`). Does not delete XML.
pub fn export_monsters_lua(
    monster_dir: &Path,
    out_dir: &Path,
    items: &ItemDatabase,
) -> Result<usize> {
    let index_path = monster_dir.join("monsters.xml");
    let entries = parse_monster_index(&index_path)?;
    std::fs::create_dir_all(out_dir).map_err(|e| TfsRustError::Content {
        file: out_dir.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;

    let mut written = 0usize;
    for entry in entries {
        let slug = monster_lua_slug(&entry.index_name);
        if slug.is_empty() {
            continue;
        }
        let xml_path = monster_dir.join(&entry.file);
        let mtype = parse_monster_file(&xml_path, items)?;
        let lua = emit_monster_lua(&entry.index_name, &mtype, Some(items));
        let out_path = out_dir.join(format!("{slug}.lua"));
        std::fs::write(&out_path, lua).map_err(|e| TfsRustError::Content {
            file: out_path.to_string_lossy().into_owned(),
            message: e.to_string(),
        })?;
        written += 1;
    }
    Ok(written)
}

// --- serde defs (Lua keys) -------------------------------------------------

#[derive(Debug, Deserialize)]
struct MonsterDef {
    name: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    race: Option<String>,
    #[serde(default)]
    experience: Option<u32>,
    #[serde(default)]
    speed: Option<u32>,
    #[serde(default)]
    mana_cost: Option<u32>,
    #[serde(default)]
    health: Option<u32>,
    #[serde(default)]
    max_health: Option<u32>,
    #[serde(default)]
    outfit: Option<OutfitDef>,
    #[serde(default)]
    change_target: Option<ChangeTargetDef>,
    #[serde(default)]
    target_strategy: Option<TargetStrategyDef>,
    #[serde(default)]
    lose_target: Option<LoseTargetDef>,
    #[serde(default)]
    flags: Option<FlagsDef>,
    #[serde(default)]
    attacks: Option<Vec<SpellDef>>,
    #[serde(default)]
    defenses: Option<DefensesDef>,
    #[serde(default)]
    immunities: Option<ImmunitiesDef>,
    #[serde(default)]
    voices: Option<Vec<VoiceDef>>,
    #[serde(default)]
    loot: Option<Vec<LootDef>>,
}

#[derive(Debug, Deserialize)]
struct OutfitDef {
    #[serde(default)]
    look_type: Option<i32>,
    #[serde(default)]
    look_head: Option<i32>,
    #[serde(default)]
    look_body: Option<i32>,
    #[serde(default)]
    look_legs: Option<i32>,
    #[serde(default)]
    look_feet: Option<i32>,
    #[serde(default)]
    look_addons: Option<i32>,
    #[serde(default)]
    look_type_ex: Option<i32>,
    #[serde(default)]
    look_mount: Option<i32>,
    #[serde(default)]
    corpse: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct ChangeTargetDef {
    #[serde(default)]
    chance: Option<i32>,
    #[serde(default)]
    interval: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TargetStrategyDef {
    #[serde(default)]
    nearest: Option<u8>,
    #[serde(default)]
    weakest: Option<u8>,
    #[serde(default)]
    most_damage: Option<u8>,
    #[serde(default)]
    random: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct LoseTargetDef {
    #[serde(default)]
    chance: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct FlagsDef {
    #[serde(default)]
    hostile: Option<bool>,
    #[serde(default)]
    summonable: Option<bool>,
    #[serde(default)]
    illusionable: Option<bool>,
    #[serde(default)]
    pushable: Option<bool>,
    #[serde(default)]
    convinceable: Option<bool>,
    #[serde(default)]
    can_push_items: Option<bool>,
    #[serde(default)]
    can_push_creatures: Option<bool>,
    #[serde(default)]
    challengeable: Option<bool>,
    #[serde(default)]
    target_distance: Option<i32>,
    #[serde(default)]
    run_health: Option<i32>,
    #[serde(default)]
    static_attack: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct DefensesDef {
    #[serde(default)]
    armor: Option<i32>,
    #[serde(default)]
    defense: Option<i32>,
    #[serde(default)]
    spells: Option<Vec<SpellDef>>,
}

#[derive(Debug, Default, Deserialize)]
struct ImmunitiesDef {
    #[serde(default)]
    fire: Option<bool>,
    #[serde(default)]
    energy: Option<bool>,
    #[serde(default)]
    poison: Option<bool>,
    #[serde(default)]
    physical: Option<bool>,
    #[serde(default)]
    outfit: Option<bool>,
    #[serde(default)]
    life_drain: Option<bool>,
    #[serde(default)]
    paralyze: Option<bool>,
    #[serde(default)]
    invisible: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct VoiceDef {
    text: String,
    #[serde(default)]
    yell: bool,
}

#[derive(Debug, Deserialize)]
struct LootDef {
    id: u32,
    chance: i32,
    #[serde(default)]
    count_max: Option<i32>,
    #[serde(default)]
    sub_type: Option<i32>,
    #[serde(default)]
    action_id: Option<i32>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    child: Option<Vec<LootDef>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LuaAtom {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
}

#[derive(Debug, Deserialize)]
struct SpellDef {
    name: String,
    #[serde(default)]
    delay: Option<i32>,
    #[serde(default)]
    min: Option<i32>,
    #[serde(default)]
    max: Option<i32>,
    #[serde(default)]
    range: Option<i32>,
    #[serde(default)]
    radius: Option<i32>,
    #[serde(default)]
    length: Option<i32>,
    #[serde(default)]
    spread: Option<i32>,
    #[serde(default)]
    skill: Option<i32>,
    #[serde(default)]
    attack: Option<i32>,
    #[serde(default)]
    duration: Option<i32>,
    #[serde(default)]
    speed: Option<i32>,
    #[serde(default)]
    cycle: Option<i32>,
    #[serde(default)]
    item: Option<LuaAtom>,
    #[serde(default)]
    monster: Option<String>,
    #[serde(default)]
    target: Option<bool>,
    #[serde(default)]
    poison_cycles: Option<i32>,
    #[serde(default)]
    skill_factor: Option<i32>,
    #[serde(default)]
    skill_next_level: Option<i32>,
    #[serde(default)]
    skill_add_count: Option<i32>,
    #[serde(default)]
    speed_variation: Option<i32>,
    #[serde(default)]
    min_cycle: Option<i32>,
    #[serde(default)]
    drunkness: Option<i32>,
    #[serde(default)]
    effect: Option<String>,
    #[serde(default)]
    shoot: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, LuaAtom>,
}

impl LuaAtom {
    fn as_attr_string(&self) -> String {
        match self {
            LuaAtom::Bool(b) => if *b { "1" } else { "0" }.to_string(),
            LuaAtom::I64(n) => n.to_string(),
            LuaAtom::F64(n) => {
                if n.fract() == 0.0 {
                    (*n as i64).to_string()
                } else {
                    n.to_string()
                }
            }
            LuaAtom::String(s) => s.clone(),
        }
    }
}

fn insert_i32(attrs: &mut HashMap<String, String>, xml: &str, v: Option<i32>) {
    if let Some(n) = v {
        attrs.insert(xml.to_string(), n.to_string());
    }
}

fn spell_to_node(def: SpellDef, element: &str) -> MonsterSpellNode {
    let mut attributes = HashMap::new();
    attributes.insert("name".to_string(), def.name);
    insert_i32(&mut attributes, "delay", def.delay);
    insert_i32(&mut attributes, "min", def.min);
    insert_i32(&mut attributes, "max", def.max);
    insert_i32(&mut attributes, "range", def.range);
    insert_i32(&mut attributes, "radius", def.radius);
    insert_i32(&mut attributes, "length", def.length);
    insert_i32(&mut attributes, "spread", def.spread);
    insert_i32(&mut attributes, "skill", def.skill);
    insert_i32(&mut attributes, "attack", def.attack);
    insert_i32(&mut attributes, "duration", def.duration);
    insert_i32(&mut attributes, "speed", def.speed);
    insert_i32(&mut attributes, "cycle", def.cycle);
    insert_i32(&mut attributes, "poisoncycles", def.poison_cycles);
    insert_i32(&mut attributes, "skillfactor", def.skill_factor);
    insert_i32(&mut attributes, "skillnextlevel", def.skill_next_level);
    insert_i32(&mut attributes, "skilladdcount", def.skill_add_count);
    insert_i32(&mut attributes, "speedvariation", def.speed_variation);
    insert_i32(&mut attributes, "mincycle", def.min_cycle);
    insert_i32(&mut attributes, "drunkness", def.drunkness);
    if let Some(item) = def.item {
        attributes.insert("item".to_string(), item.as_attr_string());
    }
    if let Some(monster) = def.monster {
        attributes.insert("monster".to_string(), monster);
    }
    if let Some(target) = def.target {
        attributes.insert(
            "target".to_string(),
            if target { "1" } else { "0" }.to_string(),
        );
    }
    for (k, v) in def.extra {
        if k == "name" {
            continue;
        }
        attributes.insert(lua_spell_key_to_xml(&k), v.as_attr_string());
    }
    let mut attribute_children = Vec::new();
    if let Some(effect) = def.effect {
        attribute_children.push(("areaeffect".to_string(), effect));
    }
    if let Some(shoot) = def.shoot {
        attribute_children.push(("shooteffect".to_string(), shoot));
    }
    MonsterSpellNode {
        element: element.to_string(),
        attributes,
        attribute_children,
    }
}

fn lua_spell_key_to_xml(key: &str) -> String {
    match key {
        "poison_cycles" => "poisoncycles",
        "skill_factor" => "skillfactor",
        "skill_next_level" => "skillnextlevel",
        "skill_add_count" => "skilladdcount",
        "speed_variation" => "speedvariation",
        "min_cycle" => "mincycle",
        other => other,
    }
    .to_string()
}

fn spells_from_defs(defs: Vec<SpellDef>, element: &str) -> Vec<MonsterSpellNode> {
    defs.into_iter()
        .map(|d| spell_to_node(d, element))
        .collect()
}

fn outfit_from_def(def: Option<OutfitDef>) -> MonsterOutfit {
    let Some(d) = def else {
        return MonsterOutfit::default();
    };
    MonsterOutfit {
        look_type: d.look_type.unwrap_or(136),
        look_head: d.look_head.unwrap_or(0),
        look_body: d.look_body.unwrap_or(0),
        look_legs: d.look_legs.unwrap_or(0),
        look_feet: d.look_feet.unwrap_or(0),
        look_addons: d.look_addons.unwrap_or(0),
        look_type_ex: d.look_type_ex.unwrap_or(0),
        look_mount: d.look_mount.unwrap_or(0),
        corpse_id: d.corpse.unwrap_or(0),
    }
}

fn flags_from_def(def: &MonsterDef) -> MonsterTypeFlags {
    let mut flags = MonsterTypeFlags::default();
    if let Some(f) = &def.flags {
        if let Some(v) = f.hostile {
            flags.is_hostile = v;
        }
        if let Some(v) = f.summonable {
            flags.summonable = v;
        }
        if let Some(v) = f.illusionable {
            flags.illusionable = v;
        }
        if let Some(v) = f.pushable {
            flags.pushable = v;
        }
        if let Some(v) = f.convinceable {
            flags.convinceable = v;
        }
        if let Some(v) = f.can_push_items {
            flags.can_push_items = v;
        }
        if let Some(v) = f.can_push_creatures {
            flags.can_push_creatures = v;
        }
        if let Some(v) = f.challengeable {
            flags.is_challengeable = v;
        }
        if let Some(mut v) = f.target_distance {
            if v < 1 {
                v = 1;
            }
            flags.target_distance = v;
        }
        if let Some(v) = f.run_health {
            flags.run_away_health = v;
        }
        if let Some(mut v) = f.static_attack {
            if v > 100 {
                v = 100;
            }
            flags.static_attack_chance = v;
        }
    }
    if let Some(c) = &def.change_target {
        if let Some(mut chance) = c.chance {
            if chance > 100 {
                chance = 100;
            }
            flags.change_target_chance = chance;
        }
        if let Some(interval) = c.interval {
            flags.change_target_speed = interval;
        }
    }
    if let Some(s) = &def.target_strategy {
        if let Some(v) = s.nearest {
            flags.strategy_nearest = v;
        }
        if let Some(v) = s.weakest {
            flags.strategy_health = v;
        }
        if let Some(v) = s.most_damage {
            flags.strategy_damage = v;
        }
        if let Some(v) = s.random {
            flags.strategy_random = v;
        }
    }
    if let Some(l) = &def.lose_target
        && let Some(c) = l.chance
    {
        flags.lose_target_percent = c;
    }
    flags
}

fn defenses_from_def(def: Option<DefensesDef>, imm: Option<ImmunitiesDef>) -> MonsterDefenses {
    let mut out = MonsterDefenses {
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
    if let Some(d) = def {
        out.armor = d.armor;
        out.defense = d.defense;
        out.spells = spells_from_defs(d.spells.unwrap_or_default(), "defense");
    }
    if let Some(i) = imm {
        out.immunity_fire = i.fire.unwrap_or(false);
        out.immunity_energy = i.energy.unwrap_or(false);
        out.immunity_poison = i.poison.unwrap_or(false);
        out.immunity_physical = i.physical.unwrap_or(false);
        out.immunity_outfit = i.outfit.unwrap_or(false);
        out.immunity_life_drain = i.life_drain.unwrap_or(false);
        out.immunity_paralyze = i.paralyze.unwrap_or(false);
        out.see_invisible = i.invisible.unwrap_or(false);
    }
    out
}

fn voices_from_defs(voices: Vec<VoiceDef>) -> Vec<String> {
    voices
        .into_iter()
        .map(|v| {
            let mut text = v.text;
            if v.yell && !text.starts_with("#y ") && !text.starts_with("#Y ") {
                text.insert_str(0, "#y ");
            }
            text
        })
        .collect()
}

fn loot_from_defs(defs: Vec<LootDef>, items: &ItemDatabase, file: &str) -> Result<Vec<LootBlock>> {
    let mut out = Vec::new();
    for d in defs {
        if let Some(block) = loot_from_def(d, items, file)? {
            out.push(block);
        }
    }
    Ok(out)
}

fn loot_from_def(def: LootDef, items: &ItemDatabase, file: &str) -> Result<Option<LootBlock>> {
    if def.id == 0 || def.id > u16::MAX as u32 {
        return Ok(None);
    }
    let id_u16 = def.id as u16;
    if items
        .items
        .get(&id_u16)
        .map(|t| t.name.is_empty())
        .unwrap_or(true)
    {
        warn!(
            target: "tfs_rust_content",
            file = %file,
            id = def.id,
            "unknown loot item id (skipping entry)"
        );
        return Ok(None);
    }

    let mut chance = def.chance;
    if chance > MAX_LOOTCHANCE {
        warn!(
            target: "tfs_rust_content",
            file = %file,
            chance,
            "loot chance above MAX_LOOTCHANCE (capped)"
        );
        chance = MAX_LOOTCHANCE;
    }

    let countmax = def.count_max.unwrap_or(1).max(1);
    let sub_type = def
        .sub_type
        .unwrap_or_else(|| items.charges_default(id_u16));
    let action_id = def.action_id.unwrap_or(0);
    let text = def.text.unwrap_or_default();

    let mut child_loot = Vec::new();
    if items.is_container(id_u16)
        && let Some(children) = def.child
    {
        for c in children {
            if let Some(block) = loot_from_def(c, items, file)? {
                child_loot.push(block);
            }
        }
    }

    Ok(Some(LootBlock {
        id: def.id,
        countmax,
        chance,
        sub_type,
        action_id,
        text,
        child_loot,
    }))
}

fn parse_summons_table(t: &Table, file: &str) -> Result<(u32, Vec<SummonBlock>)> {
    let max_summons: u32 = match t.get::<Value>("max") {
        Ok(Value::Integer(n)) => (n.max(0) as u32).min(100),
        Ok(Value::Number(n)) => (n.max(0.0) as u32).min(100),
        _ => 0,
    };
    let len = t.raw_len();
    let mut summons = Vec::new();
    for i in 1..=len {
        let row: Table = t.get(i).map_err(|e| TfsRustError::Content {
            file: file.to_string(),
            message: format!("summons[{i}]: {e}"),
        })?;
        let name: String = row.get("name").map_err(|e| TfsRustError::Content {
            file: file.to_string(),
            message: format!("summons[{i}] missing name: {e}"),
        })?;
        if name.is_empty() {
            warn!(file, "monster summon missing name");
            continue;
        }
        let delay = table_i32(&row, "delay").unwrap_or(1).clamp(1, 100);
        let max = table_u32(&row, "max").unwrap_or(max_summons);
        let force = table_bool(&row, "force").unwrap_or(false);
        let mut chance = table_i32(&row, "chance").unwrap_or(100);
        if chance > 100 {
            chance = 100;
        }
        summons.push(SummonBlock {
            name,
            delay,
            max,
            force,
            chance,
        });
    }
    Ok((max_summons, summons))
}

fn table_i32(t: &Table, key: &str) -> Option<i32> {
    match t.get::<Value>(key) {
        Ok(Value::Integer(n)) => Some(n as i32),
        Ok(Value::Number(n)) => Some(n as i32),
        _ => None,
    }
}

fn table_u32(t: &Table, key: &str) -> Option<u32> {
    match t.get::<Value>(key) {
        Ok(Value::Integer(n)) if n >= 0 => Some(n as u32),
        Ok(Value::Number(n)) if n >= 0.0 => Some(n as u32),
        _ => None,
    }
}

fn table_bool(t: &Table, key: &str) -> Option<bool> {
    match t.get::<Value>(key) {
        Ok(Value::Boolean(b)) => Some(b),
        Ok(Value::Integer(n)) => Some(n != 0),
        _ => None,
    }
}

// --- writer ---------------------------------------------------------------

fn xml_source_label(filename: &str) -> String {
    let normalized = filename.replace('\\', "/");
    if let Some(idx) = normalized.rfind("monsters/") {
        return normalized[idx..].to_string();
    }
    Path::new(filename)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(filename)
        .to_string()
}

fn lua_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push('\\');
                out.push_str(&(c as u32).to_string());
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn emit_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn emit_kv_str(out: &mut String, depth: usize, key: &str, value: &str) {
    emit_indent(out, depth);
    out.push_str(key);
    out.push_str(" = ");
    out.push_str(&lua_string(value));
    out.push_str(",\n");
}

fn emit_kv_u32(out: &mut String, depth: usize, key: &str, value: u32) {
    emit_indent(out, depth);
    out.push_str(key);
    out.push_str(" = ");
    out.push_str(&value.to_string());
    out.push_str(",\n");
}

fn emit_kv_i32(out: &mut String, depth: usize, key: &str, value: i32) {
    emit_indent(out, depth);
    out.push_str(key);
    out.push_str(" = ");
    out.push_str(&value.to_string());
    out.push_str(",\n");
}

fn emit_kv_bool(out: &mut String, depth: usize, key: &str, value: bool) {
    emit_indent(out, depth);
    out.push_str(key);
    out.push_str(" = ");
    out.push_str(if value { "true" } else { "false" });
    out.push_str(",\n");
}

fn emit_outfit(out: &mut String, o: &MonsterOutfit) {
    out.push_str("  outfit = {\n");
    emit_kv_i32(out, 2, "look_type", o.look_type);
    emit_kv_i32(out, 2, "look_head", o.look_head);
    emit_kv_i32(out, 2, "look_body", o.look_body);
    emit_kv_i32(out, 2, "look_legs", o.look_legs);
    emit_kv_i32(out, 2, "look_feet", o.look_feet);
    if o.look_addons != 0 {
        emit_kv_i32(out, 2, "look_addons", o.look_addons);
    }
    if o.look_type_ex != 0 {
        emit_kv_i32(out, 2, "look_type_ex", o.look_type_ex);
    }
    if o.look_mount != 0 {
        emit_kv_i32(out, 2, "look_mount", o.look_mount);
    }
    emit_kv_u32(out, 2, "corpse", u32::from(o.corpse_id));
    out.push_str("  },\n");
}

fn emit_change_target(out: &mut String, flags: &MonsterTypeFlags) {
    if flags.change_target_chance == 0 && flags.change_target_speed == 0 {
        return;
    }
    out.push_str("  change_target = { chance = ");
    out.push_str(&flags.change_target_chance.to_string());
    if flags.change_target_speed != 0 {
        out.push_str(", interval = ");
        out.push_str(&flags.change_target_speed.to_string());
    }
    out.push_str(" },\n");
}

fn emit_target_strategy(out: &mut String, flags: &MonsterTypeFlags) {
    out.push_str("  target_strategy = { nearest = ");
    out.push_str(&flags.strategy_nearest.to_string());
    out.push_str(", weakest = ");
    out.push_str(&flags.strategy_health.to_string());
    out.push_str(", most_damage = ");
    out.push_str(&flags.strategy_damage.to_string());
    out.push_str(", random = ");
    out.push_str(&flags.strategy_random.to_string());
    out.push_str(" },\n");
}

fn emit_flags(out: &mut String, flags: &MonsterTypeFlags) {
    out.push_str("  flags = {\n");
    emit_kv_bool(out, 2, "hostile", flags.is_hostile);
    emit_kv_bool(out, 2, "summonable", flags.summonable);
    emit_kv_bool(out, 2, "illusionable", flags.illusionable);
    emit_kv_bool(out, 2, "pushable", flags.pushable);
    emit_kv_bool(out, 2, "convinceable", flags.convinceable);
    emit_kv_bool(out, 2, "can_push_items", flags.can_push_items);
    emit_kv_bool(out, 2, "can_push_creatures", flags.can_push_creatures);
    if !flags.is_challengeable {
        emit_kv_bool(out, 2, "challengeable", false);
    }
    emit_kv_i32(out, 2, "target_distance", flags.target_distance);
    emit_kv_i32(out, 2, "run_health", flags.run_away_health);
    if flags.static_attack_chance != 95 {
        emit_kv_u32(out, 2, "static_attack", flags.static_attack_chance);
    }
    out.push_str("  },\n");
}

const SPELL_XML_NUM: &[(&str, &str)] = &[
    ("delay", "delay"),
    ("min", "min"),
    ("max", "max"),
    ("range", "range"),
    ("radius", "radius"),
    ("length", "length"),
    ("spread", "spread"),
    ("skill", "skill"),
    ("attack", "attack"),
    ("duration", "duration"),
    ("speed", "speed"),
    ("speedvariation", "speed_variation"),
    ("cycle", "cycle"),
    ("mincycle", "min_cycle"),
    ("item", "item"),
    ("monster", "monster"),
    ("poisoncycles", "poison_cycles"),
    ("skillfactor", "skill_factor"),
    ("skillnextlevel", "skill_next_level"),
    ("skilladdcount", "skill_add_count"),
    ("drunkness", "drunkness"),
];

fn known_spell_xml_keys() -> &'static [&'static str] {
    &[
        "name",
        "delay",
        "min",
        "max",
        "range",
        "radius",
        "length",
        "spread",
        "skill",
        "attack",
        "duration",
        "speed",
        "speedvariation",
        "cycle",
        "mincycle",
        "item",
        "monster",
        "target",
        "poisoncycles",
        "skillfactor",
        "skillnextlevel",
        "skilladdcount",
        "drunkness",
        "drunkenness",
    ]
}

fn emit_attr_value(out: &mut String, raw: &str) {
    if let Ok(n) = raw.parse::<i64>() {
        out.push_str(&n.to_string());
    } else {
        out.push_str(&lua_string(raw));
    }
}

fn emit_spell(out: &mut String, node: &MonsterSpellNode, depth: usize) {
    emit_indent(out, depth);
    out.push_str("{\n");
    let name = node
        .attributes
        .get("name")
        .map(String::as_str)
        .unwrap_or("");
    emit_kv_str(out, depth + 1, "name", name);
    for (xml, lua) in SPELL_XML_NUM {
        if let Some(v) = node.attributes.get(*xml) {
            emit_indent(out, depth + 1);
            out.push_str(lua);
            out.push_str(" = ");
            emit_attr_value(out, v);
            out.push_str(",\n");
        }
    }
    if !node.attributes.contains_key("drunkness")
        && let Some(v) = node.attributes.get("drunkenness")
    {
        emit_indent(out, depth + 1);
        out.push_str("drunkness = ");
        emit_attr_value(out, v);
        out.push_str(",\n");
    }
    if let Some(v) = node.attributes.get("target") {
        let is_true = v == "1" || v.eq_ignore_ascii_case("true");
        emit_kv_bool(out, depth + 1, "target", is_true);
    }
    let known = known_spell_xml_keys();
    let mut extras: Vec<(&String, &String)> = node
        .attributes
        .iter()
        .filter(|(k, _)| !known.iter().any(|x| x.eq_ignore_ascii_case(k)))
        .collect();
    extras.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in extras {
        emit_indent(out, depth + 1);
        out.push_str(k);
        out.push_str(" = ");
        emit_attr_value(out, v);
        out.push_str(",\n");
    }
    for (k, v) in &node.attribute_children {
        let lua_key = if k.eq_ignore_ascii_case("areaeffect") {
            "effect"
        } else if k.eq_ignore_ascii_case("shooteffect") {
            "shoot"
        } else {
            k.as_str()
        };
        emit_kv_str(out, depth + 1, lua_key, v);
    }
    emit_indent(out, depth);
    out.push_str("},\n");
}

fn emit_attacks(out: &mut String, spells: &[MonsterSpellNode]) {
    if spells.is_empty() {
        return;
    }
    out.push_str("  attacks = {\n");
    for s in spells {
        emit_spell(out, s, 2);
    }
    out.push_str("  },\n");
}

fn emit_defenses(out: &mut String, d: &MonsterDefenses) {
    if d.armor.is_none() && d.defense.is_none() && d.spells.is_empty() {
        return;
    }
    out.push_str("  defenses = {\n");
    if let Some(a) = d.armor {
        emit_kv_i32(out, 2, "armor", a);
    }
    if let Some(def) = d.defense {
        emit_kv_i32(out, 2, "defense", def);
    }
    if !d.spells.is_empty() {
        out.push_str("    spells = {\n");
        for s in &d.spells {
            emit_spell(out, s, 3);
        }
        out.push_str("    },\n");
    }
    out.push_str("  },\n");
}

fn emit_immunities(out: &mut String, d: &MonsterDefenses) {
    // XML always has `<immunities>`; emit all 8 keys (true and false). Order matches XML:
    // fire, energy, poison, physical, outfit, lifedrain, paralyze, invisible.
    let keys = [
        ("fire", d.immunity_fire),
        ("energy", d.immunity_energy),
        ("poison", d.immunity_poison),
        ("physical", d.immunity_physical),
        ("outfit", d.immunity_outfit),
        ("life_drain", d.immunity_life_drain),
        ("paralyze", d.immunity_paralyze),
        ("invisible", d.see_invisible),
    ];
    out.push_str("  immunities = {\n");
    for (k, v) in keys {
        emit_kv_bool(out, 2, k, v);
    }
    out.push_str("  },\n");
}

fn emit_voices(out: &mut String, texts: &[String]) {
    if texts.is_empty() {
        return;
    }
    out.push_str("  voices = {\n");
    for t in texts {
        let (text, yell) = if let Some(rest) = t.strip_prefix("#y ") {
            (rest, true)
        } else if let Some(rest) = t.strip_prefix("#Y ") {
            (rest, true)
        } else {
            (t.as_str(), false)
        };
        emit_indent(out, 2);
        out.push_str("{ text = ");
        out.push_str(&lua_string(text));
        out.push_str(", yell = ");
        out.push_str(if yell { "true" } else { "false" });
        out.push_str(" },\n");
    }
    out.push_str("  },\n");
}

fn emit_summons(out: &mut String, max_summons: u32, summons: &[SummonBlock]) {
    if summons.is_empty() && max_summons == 0 {
        return;
    }
    if summons.is_empty() {
        return;
    }
    out.push_str("  summons = {\n");
    emit_kv_u32(out, 2, "max", max_summons);
    for s in summons {
        emit_indent(out, 2);
        out.push_str("{ name = ");
        out.push_str(&lua_string(&s.name));
        out.push_str(", delay = ");
        out.push_str(&s.delay.to_string());
        out.push_str(", max = ");
        out.push_str(&s.max.to_string());
        if s.force {
            out.push_str(", force = true");
        }
        out.push_str(" },\n");
    }
    out.push_str("  },\n");
}

fn emit_loot_block(
    out: &mut String,
    block: &LootBlock,
    depth: usize,
    items: Option<&ItemDatabase>,
) {
    emit_indent(out, depth);
    out.push_str("{ id = ");
    out.push_str(&block.id.to_string());
    out.push_str(", chance = ");
    out.push_str(&block.chance.to_string());
    if block.countmax != 1 {
        out.push_str(", count_max = ");
        out.push_str(&block.countmax.to_string());
    }
    if block.sub_type != 0 {
        // XML omitted subtype uses charges_default; 0 is the usual default.
        if items
            .map(|db| db.charges_default(block.id as u16) != block.sub_type)
            .unwrap_or(true)
        {
            out.push_str(", sub_type = ");
            out.push_str(&block.sub_type.to_string());
        }
    }
    if block.action_id != 0 {
        out.push_str(", action_id = ");
        out.push_str(&block.action_id.to_string());
    }
    if !block.text.is_empty() {
        out.push_str(", text = ");
        out.push_str(&lua_string(&block.text));
    }
    if !block.child_loot.is_empty() {
        out.push_str(", child = {\n");
        for c in &block.child_loot {
            emit_loot_block(out, c, depth + 1, items);
        }
        emit_indent(out, depth);
        out.push('}');
    }
    out.push_str(" },");
    if let Some(db) = items
        && let Some(it) = db.items.get(&(block.id as u16))
        && !it.name.is_empty()
    {
        out.push_str(" -- ");
        out.push_str(&it.name);
    }
    out.push('\n');
}

fn emit_loot(out: &mut String, loot: &[LootBlock], items: Option<&ItemDatabase>) {
    if loot.is_empty() {
        return;
    }
    out.push_str("  loot = {\n");
    for b in loot {
        emit_loot_block(out, b, 2, items);
    }
    out.push_str("  },\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monsters::{parse_monster_file, parse_monster_index, parse_monster_xml};
    use std::path::PathBuf;

    fn data_monster() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/monster")
    }

    fn repo_items() -> ItemDatabase {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        ItemDatabase::load(
            &root.join("data/items/items.otb"),
            &root.join("data/items/items.xml"),
        )
        .unwrap_or(ItemDatabase {
            items: HashMap::new(),
            client_to_server: HashMap::new(),
        })
    }

    fn spell_attr<'a>(n: &'a MonsterSpellNode, k: &str) -> Option<&'a str> {
        n.attributes.get(k).map(String::as_str)
    }

    fn spell_child<'a>(n: &'a MonsterSpellNode, k: &str) -> Option<&'a str> {
        n.attribute_children
            .iter()
            .find(|(a, _)| a.eq_ignore_ascii_case(k))
            .map(|(_, v)| v.as_str())
    }

    fn assert_spell_shape(a: &MonsterSpellNode, b: &MonsterSpellNode) {
        assert_eq!(spell_attr(a, "name"), spell_attr(b, "name"));
        assert_eq!(spell_attr(a, "delay"), spell_attr(b, "delay"));
        assert_eq!(spell_attr(a, "min"), spell_attr(b, "min"));
        assert_eq!(spell_attr(a, "max"), spell_attr(b, "max"));
        assert_eq!(spell_attr(a, "radius"), spell_attr(b, "radius"));
        assert_eq!(spell_attr(a, "length"), spell_attr(b, "length"));
        assert_eq!(spell_attr(a, "spread"), spell_attr(b, "spread"));
        assert_eq!(spell_attr(a, "target"), spell_attr(b, "target"));
        assert_eq!(spell_child(a, "areaeffect"), spell_child(b, "areaeffect"));
        assert_eq!(spell_child(a, "shooteffect"), spell_child(b, "shooteffect"));
    }

    #[test]
    fn slug_red_butterfly_and_dragon() {
        assert_eq!(monster_lua_slug("Red Butterfly"), "red_butterfly");
        assert_eq!(monster_lua_slug("dragon"), "dragon");
        assert_eq!(monster_lua_slug("Dragon"), "dragon");
    }

    #[test]
    fn emit_parse_dragon_xml() {
        let path = data_monster().join("monsters/dragon.xml");
        if !path.is_file() {
            return;
        }
        let items = repo_items();
        let xml = std::fs::read_to_string(&path).expect("dragon.xml");
        let xml_type = parse_monster_xml(&xml, "monsters/dragon.xml", &items).expect("xml");
        assert_eq!(xml_type.flags.strategy_nearest, 70);
        assert_eq!(xml_type.flags.strategy_health, 10);
        assert_eq!(xml_type.flags.strategy_damage, 10);
        assert_eq!(xml_type.flags.strategy_random, 10);
        assert_eq!(xml_type.flags.lose_target_percent, 5);
        assert_eq!(xml_type.speed, 45);
        assert_eq!(xml_type.outfit.corpse_id, 2844);
        assert_eq!(xml_type.flags.run_away_health, 300);
        assert_eq!(xml_type.attack_spells.len(), 3);
        assert_eq!(
            spell_attr(&xml_type.attack_spells[0], "name"),
            Some("melee")
        );
        assert_eq!(spell_attr(&xml_type.attack_spells[0], "skill"), Some("55"));
        assert_eq!(spell_attr(&xml_type.attack_spells[0], "attack"), Some("42"));
        assert_eq!(spell_attr(&xml_type.attack_spells[1], "name"), Some("fire"));
        assert_eq!(spell_attr(&xml_type.attack_spells[1], "delay"), Some("9"));
        assert_eq!(spell_attr(&xml_type.attack_spells[2], "delay"), Some("7"));
        assert!(!xml_type.loot.is_empty());

        let lua = emit_monster_lua("Dragon", &xml_type, Some(&items));
        assert!(lua.contains("name = \"Dragon\""));
        assert!(!lua.contains("title ="));
        let lua_type = parse_monster_lua(&lua, "dragon.lua", &items).expect("lua");
        assert_eq!(lua_type.name, "Dragon");
        assert_eq!(lua_type.speed, xml_type.speed);
        assert_eq!(
            lua_type.flags.target_distance,
            xml_type.flags.target_distance
        );
        assert_eq!(lua_type.flags.strategy_nearest, 70);
        assert_eq!(lua_type.flags.lose_target_percent, 5);
        assert_eq!(lua_type.attack_spells.len(), xml_type.attack_spells.len());
        assert_spell_shape(&lua_type.attack_spells[1], &xml_type.attack_spells[1]);
        assert_spell_shape(&lua_type.attack_spells[2], &xml_type.attack_spells[2]);
        assert_eq!(lua_type.loot.len(), xml_type.loot.len());
        assert_eq!(lua_type.talk_texts[0], "#y GROOAAARRR");
        assert!(lua_type.defenses.immunity_fire);
        assert!(!lua_type.defenses.immunity_energy);
        assert!(lua_type.defenses.immunity_paralyze);
        assert!(!lua_type.defenses.immunity_outfit);
        assert!(lua.contains("paralyze = true"));
        assert!(lua.contains("outfit = false"));
    }

    #[test]
    fn amazon_emits_all_false_immunities() {
        let path = data_monster().join("monsters/amazon.xml");
        if !path.is_file() {
            return;
        }
        let items = repo_items();
        let xml = std::fs::read_to_string(&path).expect("xml");
        let xml_type = parse_monster_xml(&xml, "monsters/amazon.xml", &items).expect("xml");
        assert!(!xml_type.defenses.immunity_fire);
        assert!(!xml_type.defenses.immunity_energy);
        assert!(!xml_type.defenses.immunity_poison);
        assert!(!xml_type.defenses.immunity_physical);
        assert!(!xml_type.defenses.immunity_outfit);
        assert!(!xml_type.defenses.immunity_life_drain);
        assert!(!xml_type.defenses.immunity_paralyze);
        assert!(!xml_type.defenses.see_invisible);
        let lua = emit_monster_lua("Amazon", &xml_type, Some(&items));
        assert!(lua.contains("immunities ="));
        assert!(lua.contains("paralyze = false"));
        let lua_type = parse_monster_lua(&lua, "amazon.lua", &items).expect("lua");
        assert!(!lua_type.defenses.immunity_paralyze);
        assert!(!lua_type.defenses.immunity_outfit);
    }

    #[test]
    fn ancient_scarab_paralyze_and_outfit_round_trip() {
        let path = data_monster().join("monsters/ancient scarab.xml");
        if !path.is_file() {
            return;
        }
        let items = repo_items();
        let xml = std::fs::read_to_string(&path).expect("xml");
        let xml_type = parse_monster_xml(&xml, "monsters/ancient scarab.xml", &items).expect("xml");
        assert!(xml_type.defenses.immunity_paralyze);
        assert!(xml_type.defenses.immunity_outfit);
        let lua = emit_monster_lua("Ancient Scarab", &xml_type, Some(&items));
        assert!(lua.contains("paralyze = true"));
        assert!(lua.contains("outfit = true"));
        let lua_type = parse_monster_lua(&lua, "ancient_scarab.lua", &items).expect("lua");
        assert!(lua_type.defenses.immunity_paralyze);
        assert!(lua_type.defenses.immunity_outfit);
        assert!(lua_type.defenses.immunity_life_drain);
        assert!(lua_type.defenses.see_invisible);
    }

    #[test]
    fn red_butterfly_name_and_title() {
        let path = data_monster().join("monsters/red butterfly.xml");
        if !path.is_file() {
            return;
        }
        let items = repo_items();
        let xml = std::fs::read_to_string(&path).expect("xml");
        let xml_type = parse_monster_xml(&xml, "monsters/red butterfly.xml", &items).expect("xml");
        assert_eq!(xml_type.name, "Butterfly");
        let lua = emit_monster_lua("Red Butterfly", &xml_type, Some(&items));
        assert!(lua.contains("name = \"Red Butterfly\""));
        assert!(lua.contains("title = \"Butterfly\""));
        let lua_type = parse_monster_lua(&lua, "red_butterfly.lua", &items).expect("lua");
        assert_eq!(lua_type.name, "Butterfly");
    }

    #[test]
    fn giant_spider_summons_round_trip() {
        let path = data_monster().join("monsters/giant spider.xml");
        if !path.is_file() {
            return;
        }
        let items = repo_items();
        let xml = std::fs::read_to_string(&path).expect("xml");
        let xml_type = parse_monster_xml(&xml, "monsters/giant spider.xml", &items).expect("xml");
        assert_eq!(xml_type.max_summons, 2);
        assert_eq!(xml_type.summons[0].delay, 10);
        assert_eq!(xml_type.summons[0].max, 2);
        let lua = emit_monster_lua("Giant Spider", &xml_type, Some(&items));
        let lua_type = parse_monster_lua(&lua, "giant_spider.lua", &items).expect("lua");
        assert_eq!(lua_type.max_summons, 2);
        assert_eq!(lua_type.summons.len(), 1);
        assert_eq!(lua_type.summons[0].name, "Poison Spider");
        assert_eq!(lua_type.summons[0].delay, 10);
        assert_eq!(lua_type.summons[0].max, 2);
    }

    #[test]
    fn warlock_spells_round_trip() {
        let path = data_monster().join("monsters/warlock.xml");
        if !path.is_file() {
            return;
        }
        let items = repo_items();
        let xml = std::fs::read_to_string(&path).expect("xml");
        let xml_type = parse_monster_xml(&xml, "monsters/warlock.xml", &items).expect("xml");
        let lua = emit_monster_lua("Warlock", &xml_type, Some(&items));
        let lua_type = parse_monster_lua(&lua, "warlock.lua", &items).expect("lua");
        assert_eq!(lua_type.attack_spells.len(), xml_type.attack_spells.len());
        let names: Vec<_> = lua_type
            .attack_spells
            .iter()
            .map(|s| spell_attr(s, "name").unwrap_or(""))
            .collect();
        assert_eq!(
            names,
            vec![
                "melee",
                "energy",
                "firefield",
                "firefield",
                "fire",
                "speed",
                "manadrain",
                "physical",
            ]
        );
        assert_eq!(spell_attr(&lua_type.attack_spells[1], "delay"), Some("8"));
        assert_eq!(
            spell_attr(&lua_type.attack_spells[5], "duration"),
            Some("40000")
        );
        assert_eq!(
            spell_attr(&lua_type.attack_spells[5], "speedvariation"),
            Some("20")
        );
        assert_eq!(
            spell_child(&lua_type.attack_spells[1], "areaeffect"),
            Some("energy")
        );
        assert_eq!(lua_type.defenses.spells.len(), 2);
        assert_eq!(
            spell_attr(&lua_type.defenses.spells[0], "name"),
            Some("invisible")
        );
        for (a, b) in lua_type
            .attack_spells
            .iter()
            .zip(xml_type.attack_spells.iter())
        {
            assert_spell_shape(a, b);
        }
    }

    #[test]
    fn all_xml_monsters_round_trip_through_lua() {
        let dir = data_monster();
        let index = dir.join("monsters.xml");
        if !index.is_file() {
            return;
        }
        let items = repo_items();
        let entries = parse_monster_index(&index).expect("index");
        assert_eq!(entries.len(), 157, "stock 772 pack size");
        for entry in entries {
            let xml_type = parse_monster_file(&dir.join(&entry.file), &items)
                .unwrap_or_else(|e| panic!("{} xml: {e}", entry.index_name));
            let lua = emit_monster_lua(&entry.index_name, &xml_type, Some(&items));
            let slug = monster_lua_slug(&entry.index_name);
            let lua_type = parse_monster_lua(&lua, &format!("{slug}.lua"), &items)
                .unwrap_or_else(|e| panic!("{} lua: {e}", entry.index_name));
            assert_eq!(lua_type.name, xml_type.name, "{}", entry.index_name);
            assert_eq!(lua_type.speed, xml_type.speed, "{}", entry.index_name);
            assert_eq!(
                lua_type.flags.lose_target_percent, xml_type.flags.lose_target_percent,
                "{}",
                entry.index_name
            );
            assert_eq!(
                lua_type.flags.strategy_nearest, xml_type.flags.strategy_nearest,
                "{}",
                entry.index_name
            );
            assert_eq!(
                lua_type.attack_spells.len(),
                xml_type.attack_spells.len(),
                "{}",
                entry.index_name
            );
            assert_eq!(lua_type.summons, xml_type.summons, "{}", entry.index_name);
            assert_eq!(
                lua_type.max_summons, xml_type.max_summons,
                "{}",
                entry.index_name
            );
            assert_eq!(
                lua_type.loot.len(),
                xml_type.loot.len(),
                "{}",
                entry.index_name
            );
            assert_eq!(
                lua_type.talk_texts, xml_type.talk_texts,
                "{}",
                entry.index_name
            );
            assert_eq!(
                lua_type.defenses.immunity_paralyze, xml_type.defenses.immunity_paralyze,
                "{}",
                entry.index_name
            );
            assert_eq!(
                lua_type.defenses.immunity_outfit, xml_type.defenses.immunity_outfit,
                "{}",
                entry.index_name
            );
            assert_eq!(
                lua_type.defenses.immunity_life_drain, xml_type.defenses.immunity_life_drain,
                "{}",
                entry.index_name
            );
            assert_eq!(
                lua_type.defenses.see_invisible, xml_type.defenses.see_invisible,
                "{}",
                entry.index_name
            );
            for (a, b) in lua_type
                .attack_spells
                .iter()
                .zip(xml_type.attack_spells.iter())
            {
                assert_spell_shape(a, b);
            }
        }
    }

    #[test]
    fn dragon_xml_lua_round_trip_compare() {
        let path = data_monster().join("monsters/dragon.xml");
        if !path.is_file() {
            return;
        }
        let items = repo_items();
        let xml = std::fs::read_to_string(&path).expect("xml");
        let xml_type = parse_monster_xml(&xml, "monsters/dragon.xml", &items).expect("xml");
        let lua = emit_monster_lua("Dragon", &xml_type, Some(&items));
        let lua_type = parse_monster_lua(&lua, "dragon.lua", &items).expect("lua");
        assert_eq!(lua_type.name, xml_type.name);
        assert_eq!(lua_type.speed, xml_type.speed);
        assert_eq!(
            lua_type.flags.target_distance,
            xml_type.flags.target_distance
        );
        assert_eq!(
            lua_type.flags.strategy_nearest,
            xml_type.flags.strategy_nearest
        );
        assert_eq!(
            lua_type.flags.strategy_health,
            xml_type.flags.strategy_health
        );
        assert_eq!(
            lua_type.flags.strategy_damage,
            xml_type.flags.strategy_damage
        );
        assert_eq!(
            lua_type.flags.strategy_random,
            xml_type.flags.strategy_random
        );
        assert_eq!(
            lua_type.flags.lose_target_percent,
            xml_type.flags.lose_target_percent
        );
        assert_eq!(lua_type.attack_spells.len(), xml_type.attack_spells.len());
        assert_eq!(
            spell_attr(&lua_type.attack_spells[1], "delay"),
            spell_attr(&xml_type.attack_spells[1], "delay")
        );
        assert_eq!(lua_type.loot.len(), xml_type.loot.len());
    }
}
