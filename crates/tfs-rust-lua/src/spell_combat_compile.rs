//! Boot-time compile of simple spell/rune scripts into native combat specs.
//!
//! Pack surface: TFS revscript `Spell` / `Combat` under `data/scripts/spells/`.
//! C++ reference: `spells.cpp` `Spell::castSpell` / `CombatSpell`; `combat.cpp`
//! `Combat::doCombat` / `doAreaCombat`.

use std::collections::HashMap;
use std::path::Path;

use crate::combat_scripts::collect_lua_files;
use crate::lua_mutation::ConditionApplySpec;
use crate::userdata::combat::AreaCombat;

/// One compiled spell or rune whose `onCastSpell` is pure `combat:execute`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledNativeSpellCombat {
    pub key: String,
    pub need_direction: bool,
    pub combat_type: i32,
    pub effect: i32,
    pub distance_effect: i32,
    pub block_shield: bool,
    pub block_armor: bool,
    pub aggressive: bool,
    pub dispel_type: i32,
    pub create_item: i32,
    pub no_damage: bool,
    pub area: Option<AreaCombat>,
    pub damage: CompiledSpellDamage,
    pub conditions: Vec<ConditionApplySpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledSpellDamage {
    None,
    LevelMagic {
        base: i32,
        variation: i32,
        pvp_half: bool,
        /// `computeHealing` instead of `computeDamage` — positive magnitudes.
        healing: bool,
    },
    Skill {
        base: i32,
        variation: i32,
        limit_min: bool,
        limit_max: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandlerKind {
    Instant,
    Rune,
}

/// Scan `data/scripts/spells/**/*.lua` and return compilable combat handlers.
pub fn compile_native_spell_combats(data_dir: &Path) -> Vec<CompiledNativeSpellCombat> {
    let spells_dir = data_dir.join("scripts/spells");
    let areas_path = spells_dir.join("areas.lua");
    let areas = std::fs::read_to_string(&areas_path)
        .ok()
        .map(|content| parse_areas_lua(&content))
        .unwrap_or_default();

    let mut lua_files = Vec::new();
    collect_lua_files(&spells_dir, &mut lua_files);
    lua_files.sort();

    let mut out = Vec::new();
    for path in lua_files {
        if should_skip_spell_file(&path) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        out.extend(parse_spell_file(&content, &areas));
    }
    out
}

fn should_skip_spell_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };
    name == "areas.lua" || name.starts_with('#')
}

fn parse_areas_lua(content: &str) -> HashMap<String, AreaCombat> {
    let mut out = HashMap::new();
    let mut rest = content;
    while let Some(eq_idx) = rest.find('=') {
        let head = rest[..eq_idx].trim();
        let Some(name) = head
            .split_whitespace()
            .last()
            .filter(|n| n.starts_with("AREA"))
        else {
            rest = &rest[eq_idx + 1..];
            continue;
        };
        let after_eq = rest[eq_idx + 1..].trim_start();
        let Some(table) = extract_braced_table(after_eq) else {
            rest = &rest[eq_idx + 1..];
            continue;
        };
        if let Some(matrix) = parse_matrix_table(table) {
            out.insert(name.to_string(), AreaCombat::from_matrix(matrix));
        }
        rest = &after_eq[table.len()..];
    }
    out
}

fn extract_braced_table(s: &str) -> Option<&str> {
    let s = s.trim_start();
    if !s.starts_with('{') {
        return None;
    }
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_matrix_table(table: &str) -> Option<Vec<Vec<u8>>> {
    let inner = table.trim().trim_start_matches('{').trim_end_matches('}');
    let mut rows = Vec::new();
    let mut rest = inner;
    while let Some(row_start) = rest.find('{') {
        let row_slice = &rest[row_start..];
        let row_table = extract_braced_table(row_slice)?;
        let row_inner = row_table
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}');
        let row: Vec<u8> = row_inner
            .split(',')
            .filter_map(|cell| cell.trim().parse().ok())
            .collect();
        if row.is_empty() {
            return None;
        }
        rows.push(row);
        rest = &row_slice[row_table.len()..];
    }
    (!rows.is_empty()).then_some(rows)
}

fn parse_spell_file(content: &str, areas: &HashMap<String, AreaCombat>) -> Vec<CompiledNativeSpellCombat> {
    split_handler_blocks(content)
        .into_iter()
        .filter_map(|(kind, block)| parse_handler_block(content, kind, &block, areas))
        .collect()
}

fn split_handler_blocks(content: &str) -> Vec<(HandlerKind, String)> {
    const INSTANT: &str = "local spell = Spell(";
    const RUNE: &str = "local rune = Spell(";
    let mut blocks = Vec::new();
    let mut rest = content;
    loop {
        let instant_idx = rest.find(INSTANT);
        let rune_idx = rest.find(RUNE);
        let (kind, marker, start_idx) = match (instant_idx, rune_idx) {
            (Some(i), Some(j)) if i <= j => (HandlerKind::Instant, INSTANT, i),
            (Some(_), Some(j)) => (HandlerKind::Rune, RUNE, j),
            (Some(i), None) => (HandlerKind::Instant, INSTANT, i),
            (None, Some(j)) => (HandlerKind::Rune, RUNE, j),
            (None, None) => break,
        };
        let after = start_idx + marker.len();
        let tail = &rest[after..];
        let end_rel = next_handler_start(tail).unwrap_or(tail.len());
        let block = rest[..start_idx + marker.len() + end_rel].to_string();
        blocks.push((kind, block));
        rest = &rest[start_idx + marker.len() + end_rel..];
    }
    blocks
}

fn next_handler_start(s: &str) -> Option<usize> {
    let instant = s.find("local spell = Spell(");
    let rune = s.find("local rune = Spell(");
    match (instant, rune) {
        (Some(i), Some(j)) => Some(i.min(j)),
        (Some(i), None) => Some(i),
        (None, Some(j)) => Some(j),
        (None, None) => None,
    }
}

fn parse_handler_block(
    file_content: &str,
    kind: HandlerKind,
    block: &str,
    areas: &HashMap<String, AreaCombat>,
) -> Option<CompiledNativeSpellCombat> {
    let cast_body = extract_on_cast_body(block, kind)?;
    let combat_var = extract_execute_combat_var(&cast_body)?;
    if !is_pure_execute(&cast_body, &combat_var) {
        return None;
    }

    if !file_content.contains(&format!("local {combat_var} = Combat()")) {
        return None;
    }
    if has_target_callback(file_content, &combat_var) {
        return None;
    }

    let key = match kind {
        HandlerKind::Instant => parse_instant_key(block)?,
        HandlerKind::Rune => parse_rune_key(block)?,
    };

    let need_direction = matches!(kind, HandlerKind::Instant)
        && block.contains("spell:needDirection(true)");

    let mut compiled = ParsedCombatBlock {
        combat_type: 0,
        effect: 0,
        distance_effect: 0,
        block_shield: false,
        block_armor: false,
        aggressive: false,
        dispel_type: 0,
        create_item: 0,
        no_damage: false,
        area: None,
        damage: CompiledSpellDamage::None,
        conditions: Vec::new(),
    };

    parse_combat_setup(file_content, &combat_var, areas, &mut compiled)?;

    Some(CompiledNativeSpellCombat {
        key,
        need_direction,
        combat_type: compiled.combat_type,
        effect: compiled.effect,
        distance_effect: compiled.distance_effect,
        block_shield: compiled.block_shield,
        block_armor: compiled.block_armor,
        aggressive: compiled.aggressive,
        dispel_type: compiled.dispel_type,
        create_item: compiled.create_item,
        no_damage: compiled.no_damage,
        area: compiled.area,
        damage: compiled.damage,
        conditions: compiled.conditions,
    })
}

struct ParsedCombatBlock {
    combat_type: i32,
    effect: i32,
    distance_effect: i32,
    block_shield: bool,
    block_armor: bool,
    aggressive: bool,
    dispel_type: i32,
    create_item: i32,
    no_damage: bool,
    area: Option<AreaCombat>,
    damage: CompiledSpellDamage,
    conditions: Vec<ConditionApplySpec>,
}

fn extract_on_cast_body(block: &str, kind: HandlerKind) -> Option<String> {
    let field = match kind {
        HandlerKind::Instant => "spell.onCastSpell",
        HandlerKind::Rune => "rune.onCastSpell",
    };
    let head = format!("function {field}(");
    let start = block.find(&head)?;
    let register_pos = block.find(":register()").unwrap_or(block.len());
    let chunk = &block[start..register_pos];
    let end_idx = chunk.rfind("\nend")?;
    Some(chunk[..end_idx + "\nend".len()].to_string())
}

fn extract_execute_combat_var(cast_body: &str) -> Option<String> {
    let marker = ":execute(";
    let idx = cast_body.find("return ")?;
    let tail = &cast_body[idx..];
    let var_end = tail.find(marker)?;
    let var_part = tail[..var_end].trim();
    let var = var_part.strip_prefix("return")?.trim();
    (!var.is_empty() && var.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')).then(|| var.to_string())
}

fn normalize_lua_stmt(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn is_pure_execute(cast_body: &str, combat_var: &str) -> bool {
    let inner = cast_body
        .strip_prefix("function spell.onCastSpell(creature, variant)")
        .or_else(|| cast_body.strip_prefix("function rune.onCastSpell(creature, variant)"))
        .and_then(|s| s.strip_suffix("end"))
        .unwrap_or(cast_body);
    let norm = normalize_lua_stmt(inner);
    let a = format!("return{combat_var}:execute(creature,variant)");
    let b = format!("return{combat_var}:execute(creature,variant,false)");
    norm == a || norm == b
}

fn has_target_callback(content: &str, var: &str) -> bool {
    content.contains(&format!("{var}:setCallback(CALLBACK_PARAM_TARGETTILE"))
        || content.contains(&format!("{var}:setCallback(CALLBACK_PARAM_TARGETCREATURE"))
}

fn combat_var_line(content: &str, var: &str, suffix: &str) -> bool {
    content.lines().any(|line| line.trim().starts_with(&format!("{var}:{suffix}")))
}

fn parse_combat_setup(
    file_content: &str,
    var: &str,
    areas: &HashMap<String, AreaCombat>,
    out: &mut ParsedCombatBlock,
) -> Option<()> {
    for line in file_content.lines() {
        let line = line.trim();
        if !line.starts_with(&format!("{var}:")) {
            continue;
        }
        if let Some((key, val)) = parse_set_parameter_line(line) {
            apply_combat_param(out, key, val);
        }
    }

    if combat_var_line(file_content, var, "setArea(") {
        out.area = parse_set_area_from_file(file_content, var, areas);
    }

    out.conditions = parse_conditions_from_file(file_content, var);
    out.damage = parse_damage_from_file(file_content, var)?;
    Some(())
}

fn parse_set_parameter_line(line: &str) -> Option<(i32, i32)> {
    if !line.contains(":setParameter(") {
        return None;
    }
    let start = line.find(":setParameter(")? + ":setParameter(".len();
    let tail = &line[start..];
    let end = tail.find(')')?;
    let args = tail[..end].split(',').map(str::trim).collect::<Vec<_>>();
    if args.len() != 2 {
        return None;
    }
    Some((resolve_lua_i32(args[0])?, resolve_lua_i32(args[1])?))
}

fn parse_set_area_from_file(
    content: &str,
    var: &str,
    areas: &HashMap<String, AreaCombat>,
) -> Option<AreaCombat> {
    let marker = format!("{var}:setArea(createCombatArea(");
    let start = content.find(&marker)? + marker.len();
    let tail = &content[start..];
    let end = tail.find("))")?;
    let args = tail[..end].split(',').map(str::trim).collect::<Vec<_>>();
    let primary = areas.get(*args.first()?)?.clone();
    if args.len() >= 2 {
        let ext = areas.get(args[1])?;
        Some(primary.with_ext_area(ext.matrix.clone()))
    } else {
        Some(primary)
    }
}

fn parse_conditions_from_file(content: &str, combat_var: &str) -> Vec<ConditionApplySpec> {
    let mut specs = Vec::new();
    let mut cond_vars: HashMap<String, ConditionApplySpec> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if let Some((var, spec)) = parse_condition_decl(line) {
            cond_vars.insert(var, spec);
            continue;
        }
        if let Some((var, key, val)) = parse_condition_set_parameter(line) {
            if let Some(spec) = cond_vars.get_mut(&var) {
                apply_condition_param(spec, key, val);
            }
            continue;
        }
        if line.starts_with(&format!("{combat_var}:addCondition(")) {
            if let Some(var) = parse_combat_add_condition(line) {
                if let Some(spec) = cond_vars.remove(&var) {
                    specs.push(spec);
                }
            }
        }
    }
    specs
}

fn parse_damage_from_file(content: &str, var: &str) -> Option<CompiledSpellDamage> {
    if !content.contains(&format!("{var}:setCallback(")) {
        return Some(CompiledSpellDamage::None);
    }
    for line in content.lines() {
        let line = line.trim();
        if !line.starts_with(&format!("{var}:setCallback(")) {
            continue;
        }
        let start = line.find("setCallback(")? + "setCallback(".len();
        let tail = &line[start..];
        let end = tail.find(')')?;
        let args = tail[..end].split(',').map(str::trim).collect::<Vec<_>>();
        if args.len() != 2 {
            return None;
        }
        let cb_kind = args[0];
        let func_name = args[1].trim_matches('"').trim_matches('\'');
        let body = extract_function_body(content, func_name)?;
        if cb_kind.contains("LEVELMAGICVALUE") {
            return parse_level_magic_damage(&body);
        }
        if cb_kind.contains("SKILLVALUE") {
            return parse_skill_damage(&body);
        }
        return None;
    }
    Some(CompiledSpellDamage::None)
}

fn parse_instant_key(block: &str) -> Option<String> {
    let words = extract_quoted_arg(block, "spell:words(")?;
    Some(words.to_ascii_lowercase())
}

fn parse_rune_key(block: &str) -> Option<String> {
    let marker = "rune:runeId(";
    let start = block.find(marker)? + marker.len();
    let tail = block[start..].trim_start();
    let end = tail.find(')')?;
    let id = tail[..end].trim();
    Some(format!("rune:{id}"))
}

fn extract_quoted_arg(s: &str, prefix: &str) -> Option<String> {
    let start = s.find(prefix)? + prefix.len();
    let tail = s[start..].trim_start();
    let quote = tail.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &tail[1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

fn apply_combat_param(out: &mut ParsedCombatBlock, key: i32, value: i32) {
    match key {
        0 => out.combat_type = value,
        1 => out.effect = value,
        2 => out.distance_effect = value,
        3 => out.block_shield = value != 0,
        4 => out.block_armor = value != 0,
        6 => out.create_item = value,
        7 => out.aggressive = value != 0,
        8 => out.dispel_type = value,
        10 => out.no_damage = value != 0,
        _ => {}
    }
}

fn parse_condition_decl(line: &str) -> Option<(String, ConditionApplySpec)> {
    let marker = "local ";
    let cond_marker = " = Condition(";
    if !line.starts_with(marker) || !line.contains(cond_marker) {
        return None;
    }
    let after_local = line.strip_prefix(marker)?;
    let name_end = after_local.find(cond_marker)?;
    let var = after_local[..name_end].trim().to_string();
    let args_start = name_end + cond_marker.len();
    let args_end = after_local.rfind(')')?;
    let args = after_local[args_start..args_end]
        .split(',')
        .map(str::trim)
        .collect::<Vec<_>>();
    let ctype = resolve_lua_i32(args.first()?)?;
    let cond_id = args
        .get(1)
        .and_then(|a| resolve_lua_i32(a))
        .unwrap_or(-1);
    Some((
        var,
        ConditionApplySpec {
            ctype,
            cond_id,
            ..Default::default()
        },
    ))
}

fn parse_condition_set_parameter(line: &str) -> Option<(String, i32, i32)> {
    if !line.contains(":setParameter(") {
        return None;
    }
    let colon = line.find(':')?;
    let var = line[..colon].trim().to_string();
    let start = line.find(":setParameter(")? + ":setParameter(".len();
    let tail = &line[start..];
    let end = tail.find(')')?;
    let args = tail[..end].split(',').map(str::trim).collect::<Vec<_>>();
    if args.len() != 2 {
        return None;
    }
    Some((var, resolve_lua_i32(args[0])?, resolve_lua_i32(args[1])?))
}

fn apply_condition_param(spec: &mut ConditionApplySpec, key: i32, value: i32) {
    match key {
        2 => spec.ticks = value,
        4 => spec.health_gain = value,
        5 => spec.health_ticks = value,
        6 => spec.mana_gain = value,
        7 => spec.mana_ticks = value,
        9 => spec.speed = value,
        10 => spec.light_level = value,
        11 => spec.light_color = value,
        45 => spec.sub_id = value as u32,
        56 => spec.cycle = value,
        58 => spec.count = value,
        59 => spec.max_count = value,
        _ => {}
    }
}

fn parse_combat_add_condition(line: &str) -> Option<String> {
    let marker = ":addCondition(";
    if !line.contains(marker) {
        return None;
    }
    let start = line.find(marker)? + marker.len();
    let tail = line[start..].trim();
    let end = tail.find(')')?;
    let var = tail[..end].trim();
    (!var.is_empty()).then(|| var.to_string())
}

fn extract_function_body(content: &str, name: &str) -> Option<String> {
    let head = format!("function {name}(");
    let start = content.find(&head)?;
    let tail = &content[start..];
    let end_idx = tail.rfind("\nend")?;
    Some(tail[..end_idx + "\nend".len()].to_string())
}

fn parse_level_magic_damage(body: &str) -> Option<CompiledSpellDamage> {
    let healing = body.contains("computeHealing(");
    let marker = if healing {
        "computeHealing("
    } else {
        "computeDamage("
    };
    let idx = body.find(marker)? + marker.len();
    let tail = &body[idx..];
    let end = tail.find(')')?;
    let args: Vec<&str> = tail[..end].split(',').map(str::trim).collect();
    if args.len() < 2 {
        return None;
    }
    let base: i32 = args[0].parse().ok()?;
    let variation: i32 = args[1].parse().ok()?;
    let pvp_half = args
        .get(2)
        .is_some_and(|a| *a == "true");
    Some(CompiledSpellDamage::LevelMagic {
        base,
        variation,
        pvp_half,
        healing,
    })
}

fn parse_skill_damage(body: &str) -> Option<CompiledSpellDamage> {
    let marker = "computeSkillDamage(";
    let idx = body.find(marker)? + marker.len();
    let tail = &body[idx..];
    let end = tail.find(')')?;
    let args: Vec<&str> = tail[..end].split(',').map(str::trim).collect();
    if args.len() < 3 {
        return None;
    }
    let base: i32 = args[0].parse().ok()?;
    let variation: i32 = args[1].parse().ok()?;
    let limit_min = args.get(3).is_some_and(|a| *a == "true");
    let limit_max = args.get(4).is_some_and(|a| *a == "true");
    Some(CompiledSpellDamage::Skill {
        base,
        variation,
        limit_min,
        limit_max,
    })
}

fn resolve_lua_i32(token: &str) -> Option<i32> {
    let token = token.trim();
    match token {
        "true" => Some(1),
        "false" => Some(0),
        _ if token.chars().all(|c| c.is_ascii_digit() || c == '-') => token.parse().ok(),
        _ => enum_name_to_i32(token),
    }
}

fn enum_name_to_i32(name: &str) -> Option<i32> {
    Some(match name {
        // COMBAT_PARAM_* keys handled via numeric path; COMBAT types:
        "COMBAT_NONE" => 0,
        "COMBAT_PHYSICALDAMAGE" => 1 << 0,
        "COMBAT_ENERGYDAMAGE" => 1 << 1,
        "COMBAT_EARTHDAMAGE" => 1 << 2,
        "COMBAT_FIREDAMAGE" => 1 << 3,
        "COMBAT_UNDEFINEDDAMAGE" => 1 << 4,
        "COMBAT_LIFEDRAIN" => 1 << 5,
        "COMBAT_MANADRAIN" => 1 << 6,
        "COMBAT_HEALING" => 1 << 7,
        // CONST_ME_*
        "CONST_ME_NONE" => 0,
        "CONST_ME_DRAWBLOOD" => 1,
        "CONST_ME_LOSEENERGY" => 2,
        "CONST_ME_POFF" => 3,
        "CONST_ME_BLOCKHIT" => 4,
        "CONST_ME_EXPLOSIONAREA" => 5,
        "CONST_ME_EXPLOSIONHIT" => 6,
        "CONST_ME_FIREAREA" => 7,
        "CONST_ME_YELLOW_RINGS" => 8,
        "CONST_ME_GREEN_RINGS" => 9,
        "CONST_ME_HITAREA" => 10,
        "CONST_ME_TELEPORT" => 11,
        "CONST_ME_ENERGYHIT" => 12,
        "CONST_ME_MAGIC_BLUE" => 13,
        "CONST_ME_MAGIC_RED" => 14,
        "CONST_ME_MAGIC_GREEN" => 15,
        "CONST_ME_HITBYFIRE" => 16,
        "CONST_ME_HITBYPOISON" => 17,
        "CONST_ME_MORTAREA" => 18,
        "CONST_ME_POISONAREA" => 21,
        // CONST_ANI_*
        "CONST_ANI_NONE" => 0,
        "CONST_ANI_SPEAR" => 1,
        "CONST_ANI_BOLT" => 2,
        "CONST_ANI_ARROW" => 3,
        "CONST_ANI_FIRE" => 4,
        "CONST_ANI_ENERGY" => 5,
        "CONST_ANI_POISONARROW" => 6,
        // CONDITION_*
        "CONDITION_NONE" => 0,
        "CONDITION_POISON" => 1 << 0,
        "CONDITION_FIRE" => 1 << 1,
        "CONDITION_ENERGY" => 1 << 2,
        "CONDITION_BLEEDING" => 1 << 3,
        "CONDITION_HASTE" => 1 << 4,
        "CONDITION_PARALYZE" => 1 << 5,
        "CONDITION_OUTFIT" => 1 << 6,
        "CONDITION_INVISIBLE" => 1 << 7,
        "CONDITION_LIGHT" => 1 << 8,
        "CONDITION_MANASHIELD" => 1 << 9,
        "CONDITION_INFIGHT" => 1 << 10,
        "CONDITION_DRUNK" => 1 << 11,
        "CONDITION_MUTED" => 1 << 14,
        "CONDITION_CHANNELMUTEDTICKS" => 1 << 15,
        "CONDITION_YELLTICKS" => 1 << 16,
        "CONDITION_ATTRIBUTES" => 1 << 17,
        // CONDITION_PARAM_* / CONDITIONID_*
        "CONDITION_PARAM_OWNER" => 1,
        "CONDITION_PARAM_TICKS" => 2,
        "CONDITION_PARAM_SPEED" => 9,
        "CONDITION_PARAM_CYCLE" => 56,
        "CONDITION_PARAM_COUNT" => 58,
        "CONDITION_PARAM_MAX_COUNT" => 59,
        "CONDITIONID_DEFAULT" => -1,
        // ITEM_* (CREATEITEM)
        "ITEM_POISONFIELD_PVP" => 1490,
        "ITEM_FIREFIELD_PVP_FULL" => 1487,
        "ITEM_ENERGYFIELD_PVP" => 1491,
        "ITEM_MAGICWALL" => 1497,
        "ITEM_WILDGROWTH" => 1499,
        // COMBAT_PARAM_* (when used as values — rare)
        "COMBAT_PARAM_TYPE" => 0,
        "COMBAT_PARAM_EFFECT" => 1,
        "COMBAT_PARAM_DISTANCEEFFECT" => 2,
        "COMBAT_PARAM_BLOCKSHIELD" => 3,
        "COMBAT_PARAM_BLOCKARMOR" => 4,
        "COMBAT_PARAM_CREATEITEM" => 6,
        "COMBAT_PARAM_AGGRESSIVE" => 7,
        "COMBAT_PARAM_DISPEL" => 8,
        "COMBAT_PARAM_NODAMAGE" => 10,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn data_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    #[test]
    fn energy_strike_compiles_level_magic() {
        let compiled = compile_native_spell_combats(&data_dir());
        let entry = compiled
            .iter()
            .find(|e| e.key == "ex,ori, vis")
            .expect("energy strike");
        assert!(entry.need_direction);
        assert_eq!(entry.combat_type, 1 << 1);
        assert_eq!(entry.effect, 11);
        assert!(entry.aggressive);
        assert_eq!(
            entry.damage,
            CompiledSpellDamage::LevelMagic {
                base: 45,
                variation: 10,
                pvp_half: false,
                healing: false,
            }
        );
    }

    #[test]
    fn berserk_compiles_skill_and_area() {
        let compiled = compile_native_spell_combats(&data_dir());
        let entry = compiled
            .iter()
            .find(|e| e.key == "ex,ori")
            .expect("berserk");
        assert!(entry.area.is_some());
        assert_eq!(
            entry.damage,
            CompiledSpellDamage::Skill {
                base: 80,
                variation: 20,
                limit_min: false,
                limit_max: true,
            }
        );
        assert!(entry.block_armor);
        assert!(!entry.block_shield);
    }

    #[test]
    fn antidote_compiles_dispel_no_damage() {
        let compiled = compile_native_spell_combats(&data_dir());
        let entry = compiled
            .iter()
            .find(|e| e.key == "ex,ana, pox")
            .expect("antidote");
        assert_eq!(entry.dispel_type, 1);
        assert!(!entry.aggressive);
        assert_eq!(entry.damage, CompiledSpellDamage::None);
        assert!(!entry.no_damage);
    }

    #[test]
    fn cancel_invisibility_skipped() {
        let compiled = compile_native_spell_combats(&data_dir());
        assert!(
            !compiled.iter().any(|e| e.key == "ex,ana, ina"),
            "cancel invisibility uses TARGETCREATURE callback"
        );
    }

    #[test]
    fn fireball_rune_compiles_rune_key() {
        let compiled = compile_native_spell_combats(&data_dir());
        let entry = compiled
            .iter()
            .find(|e| e.key == "rune:2302")
            .expect("fireball rune");
        assert!(!entry.need_direction);
        assert_eq!(
            entry.damage,
            CompiledSpellDamage::LevelMagic {
                base: 20,
                variation: 5,
                pvp_half: true,
                healing: false,
            }
        );
        assert_eq!(entry.distance_effect, 4);
    }

    #[test]
    fn areas_lua_parses_matrix() {
        let path = data_dir().join("scripts/spells/areas.lua");
        let content = std::fs::read_to_string(path).expect("areas.lua");
        let areas = parse_areas_lua(&content);
        assert!(areas.contains_key("AREA_SQUARE1X1"));
        assert!(areas.contains_key("AREA_CIRCLE2X2"));
    }

    #[test]
    fn compile_pack_has_many_native_handlers() {
        let compiled = compile_native_spell_combats(&data_dir());
        assert!(
            compiled.len() >= 35,
            "expected most pure combat:execute handlers, got {}",
            compiled.len()
        );
    }
}
