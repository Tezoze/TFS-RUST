//! Compile simple AID-keyed movement scripts into native relocate specs.
//!
//! Pack surface: TFS revscript `MoveEvent():aid(N)` under `data/scripts/movements/`.
//! C++ reference: `movement.cpp` `MoveEvents::registerLuaEvent`, `executeStep`.

use std::path::Path;

use crate::combat_scripts::collect_lua_files;
use crate::move_events::MoveEventKind;

const AID_MIN: u16 = 3000;
const AID_MAX: u16 = 3123;

/// One compiled handler for an action-id keyed move event in the 3000–3123 range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledAidMoveEntry {
    pub kind: MoveEventKind,
    pub aid: u16,
    pub gate: AidMoveGate,
    pub reloc: AidMoveRelocSpec,
    pub effect: Option<AidMoveEffectSpec>,
    pub set_town: Option<String>,
}

/// Optional player gate extracted from the callback body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AidMoveGate {
    /// No player predicate (typical unconditional `onAddItem` tile paths).
    None,
    /// `creature:isPlayer()` without further checks.
    IsPlayer,
    /// `creature:isPlayer()` and `getLevel() < N`.
    PlayerLevelBelow { level: u32 },
    /// `creature:isPlayer()` and `not isPremium()`.
    PlayerNotPremium,
    /// Vocation if/else with two destinations (optional leading `isPlayer`).
    VocationBranch {
        is_player: bool,
        vocation_ids: Vec<u8>,
    },
}

/// Relocate source position passed to `doRelocate(from, to)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocFrom {
    ItemPosition,
    Absolute { x: u16, y: u16, z: u8 },
}

/// Destination position for `doRelocate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelocTo {
    Absolute { x: u16, y: u16, z: u8 },
    /// `{x = item:getPosition().x +/- N, y = Y, z = Z}` (or `tileitem` variant).
    ItemXOffset { dx: i16, y: u16, z: u8 },
    /// `{x = item:getPosition().x +/- N, y = item:getPosition().y +/- M, z = Z}`.
    ItemRelative { dx: i16, dy: i16, z: u8 },
}

/// One or two `doRelocate` arms (vocation portals use if/else).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AidMoveRelocSpec {
    Single { from: RelocFrom, to: RelocTo },
    VocationBranch {
        from: RelocFrom,
        then_to: RelocTo,
        else_to: RelocTo,
    },
}

/// Optional `Game.sendMagicEffect(pos, id)` side effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AidMoveEffectSpec {
    pub position: EffectPosition,
    pub effect_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectPosition {
    Absolute { x: u16, y: u16, z: u8 },
    ItemPosition,
    ItemXOffset { dx: i16, y: u16, z: u8 },
    ItemRelative { dx: i16, dy: i16, z: u8 },
}

/// Scan `data/scripts/movements/**/*.lua` and return compilable AID handlers.
pub fn compile_aid_move_handlers(data_dir: &Path) -> Vec<CompiledAidMoveEntry> {
    let movements_dir = data_dir.join("scripts/movements");
    let mut lua_files = Vec::new();
    collect_lua_files(&movements_dir, &mut lua_files);
    lua_files.sort();

    let mut out = Vec::new();
    for path in lua_files {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        out.extend(parse_movement_file(&content));
    }
    out
}

fn parse_movement_file(content: &str) -> Vec<CompiledAidMoveEntry> {
    split_move_event_blocks(content)
        .into_iter()
        .filter_map(|block| parse_move_event_block(&block))
        .collect()
}

fn split_move_event_blocks(content: &str) -> Vec<String> {
    const START: &str = "local moveevent = MoveEvent()";
    let mut blocks = Vec::new();
    let mut rest = content;
    while let Some(start_idx) = rest.find(START) {
        let after_start = start_idx + START.len();
        let tail = &rest[after_start..];
        let end_idx = tail
            .find(START)
            .map(|i| after_start + i)
            .unwrap_or(rest.len());
        blocks.push(rest[..end_idx].to_string());
        rest = &rest[end_idx..];
    }
    blocks
}

fn parse_move_event_block(block: &str) -> Option<CompiledAidMoveEntry> {
    if has_skip_complexity(block) {
        return None;
    }

    let aid = parse_aid(block)?;
    let (kind, callback_field) = parse_kind(block)?;
    let body = extract_callback_body(block, callback_field)?;

    let gate = parse_gate(&body, kind);
    let reloc = parse_reloc_spec(&body, &gate)?;
    let effect = parse_magic_effect(&body);
    let set_town = parse_set_town(&body);

    Some(CompiledAidMoveEntry {
        kind,
        aid,
        gate,
        reloc,
        effect,
        set_town,
    })
}

fn has_skip_complexity(block: &str) -> bool {
    const SKIP: &[&str] = &[
        "onStepOut",
        "getStorageValue",
        "setStorageValue",
        "getStorage",
        "setStorage",
        ":transform(",
        "createMonster",
        "doTargetCombat",
        ":addDamage",
        "clearField",
        "doCreateItem",
        "doRemoveItem",
        "doSetItemActionId",
    ];
    if SKIP.iter().any(|kw| block.contains(kw)) {
        return true;
    }
    // StepOut-only: has onStepOut but neither onStepIn nor onAddItem.
    block.contains("onStepOut")
        && !block.contains("onStepIn")
        && !block.contains("onAddItem")
}

fn parse_aid(block: &str) -> Option<u16> {
    let marker = ":aid(";
    let start = block.find(marker)? + marker.len();
    let tail = block[start..].trim_start();
    let end = tail.find(')')?;
    let raw = tail[..end].trim();
    let aid: u16 = raw.parse().ok()?;
    (AID_MIN..=AID_MAX).contains(&aid).then_some(aid)
}

fn parse_kind(block: &str) -> Option<(MoveEventKind, &'static str)> {
    let tile_item = block.contains(":tileItem(true)");
    if block.contains("function moveevent.onStepIn(") {
        Some((MoveEventKind::StepIn, "onStepIn"))
    } else if block.contains("function moveevent.onAddItem(") {
        let kind = MoveEventKind::AddItem.with_tile_item(tile_item);
        Some((kind, "onAddItem"))
    } else {
        None
    }
}

fn extract_callback_body(block: &str, field: &str) -> Option<String> {
    let head = format!("function moveevent.{field}(");
    let start = block.find(&head)?;
    let register_pos = block.find("moveevent:").unwrap_or(block.len());
    let chunk = &block[start..register_pos];
    let end_idx = chunk.rfind("\nend")?;
    Some(chunk[..end_idx + "\nend".len()].to_string())
}

fn parse_gate(body: &str, kind: MoveEventKind) -> AidMoveGate {
    if !matches!(kind, MoveEventKind::StepIn) {
        return AidMoveGate::None;
    }

    let cond = extract_top_if_condition(body);
    let Some(cond) = cond else {
        return AidMoveGate::None;
    };

    let is_player = cond.contains("creature:isPlayer()");
    let level_below = parse_level_below(&cond);
    let not_premium = cond.contains("isPremium()") && cond.contains("not ");
    let vocation_ids = parse_vocation_ids(&cond);

    if let Some(ids) = vocation_ids {
        return AidMoveGate::VocationBranch {
            is_player,
            vocation_ids: ids,
        };
    }
    if is_player && not_premium {
        return AidMoveGate::PlayerNotPremium;
    }
    if is_player {
        if let Some(level) = level_below {
            return AidMoveGate::PlayerLevelBelow { level };
        }
        return AidMoveGate::IsPlayer;
    }
    AidMoveGate::None
}

fn extract_top_if_condition(body: &str) -> Option<String> {
    let marker = "if ";
    let start = body.find(marker)? + marker.len();
    let tail = &body[start..];
    let then_idx = tail.find(" then")?;
    Some(normalize_ws(&tail[..then_idx]))
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_level_below(cond: &str) -> Option<u32> {
    let marker = "getLevel() < ";
    let idx = cond.find(marker)? + marker.len();
    let tail = cond[idx..].trim();
    let end = tail
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(tail.len());
    tail[..end].parse().ok()
}

fn parse_vocation_ids(cond: &str) -> Option<Vec<u8>> {
    if !cond.contains("getVocation():getId()") {
        return None;
    }
    let mut ids = Vec::new();
    let mut rest = cond;
    while let Some(idx) = rest.find("getId() ==") {
        let tail = &rest[idx + "getId() ==".len()..];
        let tail = tail.trim_start();
        let end = tail
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(tail.len());
        if end == 0 {
            break;
        }
        if let Ok(id) = tail[..end].parse::<u8>() {
            ids.push(id);
        }
        rest = &tail[end..];
    }
    (!ids.is_empty()).then_some(ids)
}

fn parse_reloc_spec(body: &str, gate: &AidMoveGate) -> Option<AidMoveRelocSpec> {
    let calls = extract_do_relocate_calls(body);
    match gate {
        AidMoveGate::VocationBranch { .. } => {
            if calls.len() < 2 {
                return None;
            }
            let (from0, then_to) = parse_do_relocate(&calls[0])?;
            let (from1, else_to) = parse_do_relocate(&calls[1])?;
            if from0 != from1 {
                return None;
            }
            Some(AidMoveRelocSpec::VocationBranch {
                from: from0,
                then_to,
                else_to,
            })
        }
        _ => {
            let (from, to) = parse_do_relocate(calls.first()?)?;
            Some(AidMoveRelocSpec::Single { from, to })
        }
    }
}

fn extract_do_relocate_calls(body: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut rest = body;
    while let Some(idx) = rest.find("doRelocate(") {
        let start = idx + "doRelocate(".len();
        let tail = &rest[start..];
        if let Some(close) = find_matching_paren(tail) {
            calls.push(tail[..close].to_string());
            rest = &tail[close + 1..];
        } else {
            break;
        }
    }
    calls
}

fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_do_relocate(args: &str) -> Option<(RelocFrom, RelocTo)> {
    let comma = find_top_level_comma(args)?;
    let from_raw = args[..comma].trim();
    let to_raw = args[comma + 1..].trim();
    let from = parse_reloc_from(from_raw)?;
    let to = parse_reloc_to(to_raw)?;
    Some((from, to))
}

fn find_top_level_comma(s: &str) -> Option<usize> {
    let mut depth_paren = 0i32;
    let mut depth_brace = 0i32;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '{' => depth_brace += 1,
            '}' => depth_brace -= 1,
            ',' if depth_paren == 0 && depth_brace == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

fn parse_reloc_from(raw: &str) -> Option<RelocFrom> {
    let raw = raw.trim();
    if raw.contains(":getPosition()") {
        return Some(RelocFrom::ItemPosition);
    }
    parse_position_table(raw).map(|(x, y, z)| RelocFrom::Absolute { x, y, z })
}

fn parse_reloc_to(raw: &str) -> Option<RelocTo> {
    let raw = raw.trim();
    if let Some((x, y, z)) = parse_position_table(raw) {
        return Some(RelocTo::Absolute { x, y, z });
    }
    parse_item_relative_dest(raw)
}

fn parse_position_table(raw: &str) -> Option<(u16, u16, u8)> {
    let raw = raw.trim();
    if !raw.starts_with('{') {
        return None;
    }
    let x = parse_table_field_u16(raw, 'x')?;
    let y = parse_table_field_u16(raw, 'y')?;
    let z = parse_table_field_u8(raw, 'z')?;
    Some((x, y, z))
}

fn parse_table_field_u16(table: &str, field: char) -> Option<u16> {
    let needle = format!("{field} = ");
    let idx = table.find(&needle)? + needle.len();
    let tail = table[idx..].trim_start();
    parse_lua_number_prefix(tail)
}

fn parse_table_field_u8(table: &str, field: char) -> Option<u8> {
    parse_table_field_u16(table, field).and_then(|n| u8::try_from(n).ok())
}

fn parse_lua_number_prefix(s: &str) -> Option<u16> {
    let end = s
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    s[..end].parse().ok()
}

fn parse_item_relative_dest(raw: &str) -> Option<RelocTo> {
    let raw = raw.trim();
    if !raw.starts_with('{') {
        return None;
    }
    let x_expr = extract_table_expr(raw, 'x')?;
    let y_expr = extract_table_expr(raw, 'y')?;
    let z = parse_table_field_u8(raw, 'z')?;

    let (dx, _) = parse_item_axis_expr(&x_expr)?;
    if y_expr.contains(":getPosition().y") {
        let dy = parse_item_axis_expr(&y_expr)
            .map(|(offset, _)| offset)
            .unwrap_or(0);
        return Some(RelocTo::ItemRelative { dx, dy, z });
    }
    if let Some(y) = parse_lua_number_prefix(y_expr.trim()) {
        return Some(RelocTo::ItemXOffset {
            dx,
            y,
            z,
        });
    }
    None
}

fn extract_table_expr(table: &str, field: char) -> Option<String> {
    let needle = format!("{field} = ");
    let idx = table.find(&needle)? + needle.len();
    let tail = table[idx..].trim_start();
    let end = tail.find(',').unwrap_or_else(|| tail.find('}').unwrap_or(tail.len()));
    Some(tail[..end].trim().to_string())
}

fn parse_item_axis_expr(expr: &str) -> Option<(i16, bool)> {
    let expr = expr.trim();
    if !expr.contains(":getPosition().") {
        return None;
    }
    if let Some(idx) = expr.find(".x ") {
        let tail = expr[idx + 3..].trim();
        return parse_axis_offset(tail);
    }
    if let Some(idx) = expr.find(".x") {
        let after_x = &expr[idx + 2..];
        if let Some(rest) = after_x.strip_prefix('+').or_else(|| after_x.strip_prefix('-')) {
            let sign = if after_x.starts_with('-') { -1i16 } else { 1i16 };
            let num: i16 = rest
                .trim()
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok()?;
            return Some((sign * num, true));
        }
    }
    if expr.contains(".y") {
        let plus = expr.find('+').or_else(|| expr.find('-'))?;
        let tail = &expr[plus..];
        let sign = if tail.starts_with('-') { -1i16 } else { 1i16 };
        let num: i16 = tail
            .trim_start_matches(['+', '-'])
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .ok()?;
        return Some((sign * num, true));
    }
    None
}

fn parse_axis_offset(tail: &str) -> Option<(i16, bool)> {
    let tail = tail.trim();
    if let Some(rest) = tail.strip_prefix('+') {
        let n: i16 = rest.trim().parse().ok()?;
        Some((n, true))
    } else if let Some(rest) = tail.strip_prefix('-') {
        let n: i16 = rest.trim().parse().ok()?;
        Some((-n, true))
    } else {
        None
    }
}

fn parse_magic_effect(body: &str) -> Option<AidMoveEffectSpec> {
    let marker = "Game.sendMagicEffect(";
    let start = body.find(marker)? + marker.len();
    let tail = &body[start..];
    let close = find_matching_paren(tail)?;
    let args = &tail[..close];
    let comma = find_top_level_comma(args)?;
    let pos_raw = args[..comma].trim();
    let id_raw = args[comma + 1..].trim();
    let effect_id: u8 = id_raw.trim().parse().ok()?;
    let position = parse_effect_position(pos_raw)?;
    Some(AidMoveEffectSpec {
        position,
        effect_id,
    })
}

fn parse_effect_position(raw: &str) -> Option<EffectPosition> {
    let raw = raw.trim();
    if raw.contains(":getPosition()") && !raw.starts_with('{') {
        return Some(EffectPosition::ItemPosition);
    }
    if let Some((x, y, z)) = parse_position_table(raw) {
        return Some(EffectPosition::Absolute { x, y, z });
    }
    let reloc_to = parse_item_relative_dest(raw)?;
    Some(match reloc_to {
        RelocTo::Absolute { x, y, z } => EffectPosition::Absolute { x, y, z },
        RelocTo::ItemXOffset { dx, y, z } => EffectPosition::ItemXOffset { dx, y, z },
        RelocTo::ItemRelative { dx, dy, z } => EffectPosition::ItemRelative { dx, dy, z },
    })
}

fn parse_set_town(body: &str) -> Option<String> {
    let marker = "setTown(Town(\"";
    let start = body.find(marker)? + marker.len();
    let tail = &body[start..];
    let end = tail.find("\")")?;
    Some(tail[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const LEVEL_2_BRIDGE: &str = r#"local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	if creature:isPlayer() and creature:getPlayer():getLevel() < 2 then
		doRelocate(item:getPosition(),{x = item:getPosition().x - 1, y = 32176, z = 07})
		Game.sendMagicEffect({x = item:getPosition().x - 1, y = 32176, z = 07}, 13)
	end
end

moveevent:aid(3051)
moveevent:register()

local moveevent = MoveEvent()

function moveevent.onAddItem(item, tileitem, position)
	doRelocate(tileitem:getPosition(),{x = tileitem:getPosition().x - 1, y = 32176, z = 07})
	Game.sendMagicEffect({x = tileitem:getPosition().x - 1, y = 32176, z = 07}, 13)
end

moveevent:aid(3051)
moveevent:tileItem(true)
moveevent:register()
"#;

    const PREMIUM_BRIDGE: &str = r#"local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	if creature:isPlayer() and not creature:getPlayer():isPremium() then
		doRelocate(item:getPosition(),{x = item:getPosition().x + 3, y = item:getPosition().y, z = 07})
		Game.sendMagicEffect(item:getPosition(), 13)
	end
end

moveevent:aid(3052)
moveevent:register()

local moveevent = MoveEvent()

function moveevent.onAddItem(item, tileitem, position)
	doRelocate(tileitem:getPosition(),{x = tileitem:getPosition().x + 3, y = tileitem:getPosition().y, z = 07})
	Game.sendMagicEffect(item:getPosition(), 13)
end

moveevent:aid(3052)
moveevent:tileItem(true)
moveevent:register()
"#;

    const DRUID_PORTAL: &str = r#"local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	if creature:isPlayer() and (creature:getPlayer():getVocation():getId() == 2 or creature:getPlayer():getVocation():getId() == 6) then
		doRelocate(item:getPosition(),{x = 32851, y = 32339, z = 06})
	else
		doRelocate(item:getPosition(),{x = 32836, y = 32294, z = 07})
	end
end

moveevent:aid(3116)
moveevent:register()
"#;

    #[test]
    fn level_2_bridge_compiles_step_in_and_add_item() {
        let entries = parse_movement_file(LEVEL_2_BRIDGE);
        assert_eq!(entries.len(), 2);

        let step_in = entries
            .iter()
            .find(|e| e.kind == MoveEventKind::StepIn)
            .expect("step in");
        assert_eq!(step_in.aid, 3051);
        assert_eq!(
            step_in.gate,
            AidMoveGate::PlayerLevelBelow { level: 2 }
        );
        assert_eq!(
            step_in.reloc,
            AidMoveRelocSpec::Single {
                from: RelocFrom::ItemPosition,
                to: RelocTo::ItemXOffset {
                    dx: -1,
                    y: 32176,
                    z: 7
                }
            }
        );
        let effect = step_in.effect.as_ref().expect("effect");
        assert_eq!(effect.effect_id, 13);
        assert_eq!(
            effect.position,
            EffectPosition::ItemXOffset {
                dx: -1,
                y: 32176,
                z: 7
            }
        );

        let add_item = entries
            .iter()
            .find(|e| e.kind == MoveEventKind::AddItemItemTile)
            .expect("tile add item");
        assert_eq!(add_item.aid, 3051);
        assert_eq!(add_item.gate, AidMoveGate::None);
    }

    #[test]
    fn premium_bridge_compiles_not_premium_gate_and_relative_dest() {
        let entries = parse_movement_file(PREMIUM_BRIDGE);
        assert_eq!(entries.len(), 2);

        let step_in = entries
            .iter()
            .find(|e| e.kind == MoveEventKind::StepIn)
            .expect("step in");
        assert_eq!(step_in.aid, 3052);
        assert_eq!(step_in.gate, AidMoveGate::PlayerNotPremium);
        assert_eq!(
            step_in.reloc,
            AidMoveRelocSpec::Single {
                from: RelocFrom::ItemPosition,
                to: RelocTo::ItemRelative {
                    dx: 3,
                    dy: 0,
                    z: 7
                }
            }
        );
        let effect = step_in.effect.as_ref().expect("effect");
        assert_eq!(effect.effect_id, 13);
        assert_eq!(effect.position, EffectPosition::ItemPosition);
    }

    #[test]
    fn vocation_portal_compiles_two_arm_reloc() {
        let entries = parse_movement_file(DRUID_PORTAL);
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.aid, 3116);
        assert_eq!(
            entry.gate,
            AidMoveGate::VocationBranch {
                is_player: true,
                vocation_ids: vec![2, 6]
            }
        );
        assert_eq!(
            entry.reloc,
            AidMoveRelocSpec::VocationBranch {
                from: RelocFrom::ItemPosition,
                then_to: RelocTo::Absolute {
                    x: 32851,
                    y: 32339,
                    z: 6
                },
                else_to: RelocTo::Absolute {
                    x: 32836,
                    y: 32294,
                    z: 7
                }
            }
        );
    }

    #[test]
    fn skips_transform_and_storage_scripts() {
        let block = r#"local moveevent = MoveEvent()
function moveevent.onStepIn(creature, item, position, fromPosition)
	setStorageValue(creature, 1, 1)
end
moveevent:aid(3001)
moveevent:register()
"#;
        assert!(parse_movement_file(block).is_empty());

        let block2 = r#"local moveevent = MoveEvent()
function moveevent.onStepIn(creature, item, position, fromPosition)
	doRelocate(item:getPosition(), {x=1,y=2,z=3})
	item:transform(1234)
end
moveevent:aid(3001)
moveevent:register()
"#;
        assert!(parse_movement_file(block2).is_empty());
    }

    #[test]
    fn compile_data_pack_rookgaard_bridges() {
        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let entries = compile_aid_move_handlers(&data_dir);
        assert!(
            entries.iter().any(|e| e.aid == 3051 && e.kind == MoveEventKind::StepIn),
            "level_2_bridge step in"
        );
        assert!(
            entries.iter().any(|e| e.aid == 3052 && e.kind == MoveEventKind::StepIn),
            "premium_bridge step in"
        );
    }
}
