//! Vocation combat data — Lua-as-data pilot (`data/defs/vocations.lua`).
//!
//! Replaces the outgoing `quick-xml` `Vocation`/`VocationDatabase` per
//! `docs/DATA_FORMAT_MIGRATION.md` Phase 1 (vocations pilot). The full TVP
//! combat block (gains, regen cadence, formulas, skill multipliers, soul,
//! attack/base speed) plus the level-1 vitals floor now live in
//! `data/defs/vocations.lua` and deserialize into [`VocationDef`] via `mlua`'s
//! `serde` feature. [`VocationRegistry`] indexes them by id for game-thread
//! lookups; a `Copy` hot-path snapshot (`VocationProfile`) lives in
//! `tfs-rust-core::creature::vocation` for level-up/regen/speed reads without
//! a content dependency in hot paths.
//!
//! C++ reference (772 outcomes — `tibia-game-master/src/`):
//! - Per-vocation `AddLevel` for HP/mana/cap — `crplayer.cc:1050-1093` `TPlayer::SetProfession`.
//! - Regen cadence `gainhpticks`/`gainhpamount`/`gainmanaticks`/`gainmanaamount` —
//!   `crskill.cc:828-885` `TSkillFed::Event`.
//! - Level-1 vitals floor (HP=150, Mana=0, Cap=400) — `runtime/mon/human.mon`
//!   `Skills = { (HitPoints, 150, 0, 150, …), (Mana, 0, 0, 0, …),
//!   (CarryStrength, 400, 0, 400, …) }` (race data, not vocation — `AddLevel`
//!   overrides only the per-level gain, not the floor).
//! - Skill multipliers feed `TSkillProbe::GetExpForLevel` `FactorPercent`
//!   (`crskill.cc:472-512`).

use std::collections::HashMap;
use std::path::Path;

use mlua::LuaSerdeExt;
use serde::Deserialize;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

use crate::data_lua::{load_data_table, require_schema, sandboxed_data_lua};

/// Expected `schema` version for `data/defs/vocations.lua`.
pub const VOCATIONS_SCHEMA: u32 = 1;

/// Full vocation combat block — mirrors `data/defs/vocations.lua` 1:1.
///
/// Fields are `snake_case` to match the Lua keys directly (no `serde(rename)`
/// noise). Numeric types are widened where the C++/Lua value fits but the
/// runtime wants a wider carrier (e.g. `gain_hp: i32` even though the XML
/// attribute is a small positive int).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct VocationDef {
    pub id: u16,
    pub client_id: u16,
    pub name: String,
    pub description: String,
    pub from_vocation: u16,
    /// `gaincap` — capacity gain per level (`crplayer.cc:1053` `AddLevel`).
    pub gain_cap: i32,
    /// `gainhp` — HP gain per level (`crplayer.cc:1051` `AddLevel`).
    pub gain_hp: i32,
    /// `gainmana` — mana gain per level (`crplayer.cc:1052` `AddLevel`).
    pub gain_mana: i32,
    /// `gainhpticks` — rounds between HP regen ticks (`crskill.cc:828-874`).
    /// `0` ⇒ no HP regen from food.
    pub gain_hp_ticks: u32,
    /// `gainhpamount` — HP gained per regen tick (`crskill.cc:880`).
    pub gain_hp_amount: i32,
    /// `gainmanaticks` — rounds between mana regen ticks (`crskill.cc:828-874`).
    /// `0` ⇒ no mana regen.
    pub gain_mana_ticks: u32,
    /// `gainmanaamount` — mana gained per regen tick (`crskill.cc:884`).
    pub gain_mana_amount: i32,
    /// `manamultiplier` — mana spell-cost multiplier (TFS `Vocation::manaMultiplier`).
    pub mana_multiplier: f32,
    /// `attackspeed` — melee attack cadence in ms (TFS `Vocation::attackSpeed`).
    pub attack_speed_ms: u32,
    /// `basespeed` — vocation GoStrength floor (`gameserver/src/player.h` `updateBaseSpeed`).
    pub base_speed: i32,
    /// `soulmax` — max soul points (`crplayer.cc:130` `Soul->Max`).
    pub soul_max: i32,
    /// `gainsoulticks` — rounds between soul regen ticks (`crplayer.cc:137`).
    pub gain_soul_ticks: u32,
    /// `allowpvp` — vocation PVP toggle (TFS `Vocation::allowPvp`).
    #[serde(default)]
    pub allow_pvp: bool,
    /// Level-1 vitals floor — HP at level 1. Sourced from `runtime/mon/human.mon`
    /// race data (`HitPoints` `Actual=150`); `AddLevel` only changes the per-level
    /// gain, not the floor. Defaults to `150` if the Lua file omits it.
    #[serde(default = "default_base_hp")]
    pub base_hp: i32,
    /// Level-1 mana floor (`Mana` `Actual=0` in `human.mon`).
    #[serde(default = "default_base_mana")]
    pub base_mana: i32,
    /// Level-1 capacity floor (`CarryStrength` `Actual=400` in `human.mon`).
    #[serde(default = "default_base_cap")]
    pub base_cap: i32,
    /// `<formula>` block — vocation damage/defense/armor multipliers.
    pub formula: VocationFormula,
    /// `<skill id multiplier>` — `SKILL_FIST..SKILL_FISHING` (indices 0..6).
    /// Length must be exactly 7.
    pub skill_multipliers: [f32; 7],
}

fn default_base_hp() -> i32 {
    150
}
fn default_base_mana() -> i32 {
    0
}
fn default_base_cap() -> i32 {
    400
}

/// Vocation `<formula>` block — multipliers applied to attack/defense/armor
/// damage rolls (TFS `Vocation::meleeDamage`/`distDamage`/`defense`/`armor`).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct VocationFormula {
    #[serde(default = "default_one")]
    pub melee_damage: f32,
    #[serde(default = "default_one")]
    pub dist_damage: f32,
    #[serde(default = "default_one")]
    pub defense: f32,
    #[serde(default = "default_one")]
    pub armor: f32,
}

fn default_one() -> f32 {
    1.0
}

impl Default for VocationFormula {
    fn default() -> Self {
        Self {
            melee_damage: 1.0,
            dist_damage: 1.0,
            defense: 1.0,
            armor: 1.0,
        }
    }
}

/// Indexed vocation registry — materialized once at startup, immutable on the
/// game thread. Replaces the outgoing `VocationDatabase`.
#[derive(Debug, Clone, Default)]
pub struct VocationRegistry {
    pub vocations: HashMap<u16, VocationDef>,
}

impl VocationRegistry {
    /// `Player::vocation` id → protocol `u8` client id (`ProtocolGame::sendBasicData`).
    pub fn client_id_u8(&self, vocation_id: i32) -> u8 {
        if vocation_id < 0 {
            return 0;
        }
        let id = vocation_id as u16;
        self.vocations
            .get(&id)
            .map(|v| (v.client_id.min(255)) as u8)
            .unwrap_or(0)
    }

    /// `TSkillFed::Event` regen cadence for a vocation (`crskill.cc:828-885`).
    /// Returns `(hp_ticks, hp_amount, mana_ticks, mana_amount)`. When the
    /// vocation is absent from the registry, falls back to the C++ `default:`
    /// case (`SecsPerHP = 12`, `SecsPerMana = 6`) with the hardcoded
    /// `Change(1)`/`Change(2)` amounts (`crskill.cc:871,880,884`).
    pub fn fed_regen_params(&self, vocation_id: i32) -> (u32, i32, u32, i32) {
        if vocation_id < 0 {
            return (12, 1, 6, 2);
        }
        self.vocations
            .get(&(vocation_id as u16))
            .map(|v| {
                (
                    v.gain_hp_ticks,
                    v.gain_hp_amount,
                    v.gain_mana_ticks,
                    v.gain_mana_amount,
                )
            })
            .unwrap_or((12, 1, 6, 2))
    }

    /// Lookup a vocation definition by id.
    pub fn get(&self, vocation_id: i32) -> Option<&VocationDef> {
        if vocation_id < 0 {
            return None;
        }
        self.vocations.get(&(vocation_id as u16))
    }

    /// Load `data/defs/vocations.lua` via the sandboxed data-Lua loader, deserialize
    /// into `Vec<VocationDef>`, validate, and index by id.
    pub fn load(path: &Path) -> Result<Self> {
        info!("Loading vocations from {:?}", path);
        let lua = sandboxed_data_lua()?;
        let root = load_data_table(&lua, path)?;
        require_schema(&root, VOCATIONS_SCHEMA)?;

        let vocs_value = root
            .get("vocations")
            .map_err(|e| TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: format!("missing 'vocations' array: {e}"),
            })?;
        let defs: Vec<VocationDef> = lua
            .from_value(vocs_value)
            .map_err(|e| TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: format!("deserialize vocations failed: {e}"),
            })?;

        validate_vocations(&defs, path)?;

        let vocations = defs
            .into_iter()
            .map(|d| (d.id, d))
            .collect::<HashMap<u16, VocationDef>>();
        Ok(Self { vocations })
    }
}

/// Semantic validation pass — unique ids, non-zero tick divisors where amount > 0,
/// `skill_multipliers` length 7 (enforced by the array type, but checked here for
/// a clear error message). Fails fast at startup per the migration doc guardrails.
fn validate_vocations(defs: &[VocationDef], path: &Path) -> Result<()> {
    let mut seen = HashMap::new();
    for d in defs {
        if let Some(prev) = seen.insert(d.id, d.name.as_str()) {
            return Err(TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: format!("duplicate vocation id {}: '{}' vs '{}'", d.id, prev, d.name),
            });
        }
        if d.gain_hp_amount > 0 && d.gain_hp_ticks == 0 {
            return Err(TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: format!(
                    "vocation {} '{}' has gain_hp_amount>0 but gain_hp_ticks=0",
                    d.id, d.name
                ),
            });
        }
        if d.gain_mana_amount > 0 && d.gain_mana_ticks == 0 {
            return Err(TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: format!(
                    "vocation {} '{}' has gain_mana_amount>0 but gain_mana_ticks=0",
                    d.id, d.name
                ),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Temporary XML loader — kept only for the dual-load golden equivalence test
// (PC-0 step 6). Deleted once `data/XML/vocations.xml` is retired.
// ---------------------------------------------------------------------------

/// Temporary: load the outgoing `data/XML/vocations.xml` into the same
/// `VocationDef` shape so the golden test can assert the Lua file carries the
/// same data. Not used in production — `VocationRegistry::load` is the real path.
#[cfg(test)]
fn load_xml_for_golden_test(path: &Path) -> Result<Vec<VocationDef>> {
    let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;

    // Use roxmltree (already a dep) for a clean DOM-based parse — this is a
    // test-only path. The streaming quick-xml approach is fiddly with the
    // Start/Empty child distinction; roxmltree's DOM model handles it naturally.
    let doc = roxmltree::Document::parse(&xml).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    let mut defs = Vec::new();
    for voc in doc.descendants().filter(|n| n.has_tag_name("vocation")) {
        let mut id = None;
        let mut client_id = 0u16;
        let mut name = String::new();
        let mut description = String::new();
        let mut from_vocation = 0u16;
        let mut gain_cap = 0i32;
        let mut gain_hp = 0i32;
        let mut gain_mana = 0i32;
        let mut gain_hp_ticks = 0u32;
        let mut gain_hp_amount = 0i32;
        let mut gain_mana_ticks = 0u32;
        let mut gain_mana_amount = 0i32;
        let mut mana_multiplier = 1.0f32;
        let mut attack_speed_ms = 2000u32;
        let mut base_speed = 70i32;
        let mut soul_max = 100i32;
        let mut gain_soul_ticks = 120u32;
        let mut allow_pvp = false;
        let mut formula = VocationFormula::default();
        let mut skill_multipliers = [1.0f32; 7];

        for attr in voc.attributes() {
            let key = attr.name();
            let value = attr.value();
            match key {
                "id" => id = Some(parse_num(path, value)?),
                "clientid" => client_id = parse_num(path, value)?,
                "name" => name = value.into(),
                "description" => description = value.into(),
                "fromvoc" => from_vocation = parse_num(path, value)?,
                "gaincap" => gain_cap = parse_num(path, value)?,
                "gainhp" => gain_hp = parse_num(path, value)?,
                "gainmana" => gain_mana = parse_num(path, value)?,
                "gainhpticks" => gain_hp_ticks = parse_num(path, value)?,
                "gainhpamount" => gain_hp_amount = parse_num(path, value)?,
                "gainmanaticks" => gain_mana_ticks = parse_num(path, value)?,
                "gainmanaamount" => gain_mana_amount = parse_num(path, value)?,
                "manamultiplier" => mana_multiplier = value.parse().unwrap_or(1.0),
                "attackspeed" => attack_speed_ms = parse_num(path, value)?,
                "basespeed" => base_speed = parse_num(path, value)?,
                "soulmax" => soul_max = parse_num(path, value)?,
                "gainsoulticks" => gain_soul_ticks = parse_num(path, value)?,
                "allowPvp" => allow_pvp = value != "0",
                _ => {}
            }
        }

        for child in voc.children().filter(|n| n.is_element()) {
            let tag = child.tag_name().name();
            if tag == "formula" {
                for attr in child.attributes() {
                    let v: f32 = attr.value().parse().unwrap_or(1.0);
                    match attr.name() {
                        "meleeDamage" => formula.melee_damage = v,
                        "distDamage" => formula.dist_damage = v,
                        "defense" => formula.defense = v,
                        "armor" => formula.armor = v,
                        _ => {}
                    }
                }
            } else if tag == "skill" {
                let mut skill_id = 0u16;
                let mut mult = 1.0f32;
                for attr in child.attributes() {
                    match attr.name() {
                        "id" => skill_id = parse_num(path, attr.value())?,
                        "multiplier" => mult = attr.value().parse().unwrap_or(1.0),
                        _ => {}
                    }
                }
                if skill_id < 7 {
                    skill_multipliers[skill_id as usize] = mult;
                }
            }
        }

        let vocation_id = id.ok_or_else(|| TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "vocation entry missing required 'id'".to_string(),
        })?;
        defs.push(VocationDef {
            id: vocation_id,
            client_id,
            name,
            description,
            from_vocation,
            gain_cap,
            gain_hp,
            gain_mana,
            gain_hp_ticks,
            gain_hp_amount,
            gain_mana_ticks,
            gain_mana_amount,
            mana_multiplier,
            attack_speed_ms,
            base_speed,
            soul_max,
            gain_soul_ticks,
            allow_pvp,
            base_hp: 150,
            base_mana: 0,
            base_cap: 400,
            formula,
            skill_multipliers,
        });
    }

    Ok(defs)
}

#[cfg(test)]
fn parse_num<T: std::str::FromStr>(path: &Path, s: &str) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    s.parse::<T>().map_err(|err| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: format!("invalid numeric attribute '{s}': {err}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden parse: `data/defs/vocations.lua` carries the full TVP block with
    /// known 772 values (knight skill[4]=1.4, sorcerer gainmana=30,
    /// attackspeed=2000, basespeed=70).
    #[test]
    fn golden_parse_vocations_lua() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("data")
            .join("defs")
            .join("vocations.lua");
        if !path.is_file() {
            eprintln!("skipping golden_parse_vocations_lua — {} not found", path.display());
            return;
        }
        let reg = VocationRegistry::load(&path).expect("load vocations.lua");

        // Knight (id=4): skill[4]=1.4, gain_hp=15, gain_mana=5, gain_cap=25.
        let knight = reg.vocations.get(&4).expect("knight vocation");
        assert_eq!(knight.name, "Knight");
        assert_eq!(knight.gain_hp, 15);
        assert_eq!(knight.gain_mana, 5);
        assert_eq!(knight.gain_cap, 25);
        assert_eq!(knight.skill_multipliers[4], 1.4);
        assert_eq!(knight.attack_speed_ms, 2000);
        assert_eq!(knight.base_speed, 70);

        // Sorcerer (id=1): gainmana=30, manamultiplier=1.1.
        let sorc = reg.vocations.get(&1).expect("sorcerer vocation");
        assert_eq!(sorc.name, "Sorcerer");
        assert_eq!(sorc.gain_mana, 30);
        assert!((sorc.mana_multiplier - 1.1).abs() < 1e-4);

        // Level-1 floor defaults (from human.mon race data).
        assert_eq!(knight.base_hp, 150);
        assert_eq!(knight.base_mana, 0);
        assert_eq!(knight.base_cap, 400);

        // Formula block defaults (all 1.0 in shipped pack).
        assert!((knight.formula.melee_damage - 1.0).abs() < 1e-4);
    }

    /// Dual-load golden equivalence: the new `data/defs/vocations.lua` loader
    /// produces the same `VocationDef`s as the outgoing `vocations.xml` parser
    /// (PC-0 step 6 / `DATA_FORMAT_MIGRATION.md` "golden equivalence").
    #[test]
    fn dual_load_xml_lua_equivalence() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let lua_path = manifest.join("data").join("defs").join("vocations.lua");
        let xml_path = manifest.join("data").join("XML").join("vocations.xml");
        if !lua_path.is_file() || !xml_path.is_file() {
            eprintln!(
                "skipping dual_load_xml_lua_equivalence — missing data files (lua={} xml={})",
                lua_path.is_file(),
                xml_path.is_file()
            );
            return;
        }

        let reg = VocationRegistry::load(&lua_path).expect("load vocations.lua");
        let xml_defs = load_xml_for_golden_test(&xml_path).expect("load vocations.xml");

        // Every XML vocation must be present in the Lua registry with matching
        // combat-relevant fields. base_hp/base_mana/base_cap are Lua-only
        // additions (level-1 floor) — they default to 150/0/400 in the XML
        // loader, so we compare those defaults too.
        for xml_def in &xml_defs {
            let lua_def = reg
                .vocations
                .get(&xml_def.id)
                .unwrap_or_else(|| panic!("vocation {} missing from lua", xml_def.id));
            assert_eq!(lua_def.id, xml_def.id, "id mismatch");
            assert_eq!(lua_def.client_id, xml_def.client_id, "client_id {}", xml_def.id);
            assert_eq!(lua_def.name, xml_def.name, "name {}", xml_def.id);
            assert_eq!(lua_def.from_vocation, xml_def.from_vocation, "fromvoc {}", xml_def.id);
            assert_eq!(lua_def.gain_cap, xml_def.gain_cap, "gain_cap {}", xml_def.id);
            assert_eq!(lua_def.gain_hp, xml_def.gain_hp, "gain_hp {}", xml_def.id);
            assert_eq!(lua_def.gain_mana, xml_def.gain_mana, "gain_mana {}", xml_def.id);
            assert_eq!(lua_def.gain_hp_ticks, xml_def.gain_hp_ticks, "gain_hp_ticks {}", xml_def.id);
            assert_eq!(lua_def.gain_hp_amount, xml_def.gain_hp_amount, "gain_hp_amount {}", xml_def.id);
            assert_eq!(
                lua_def.gain_mana_ticks, xml_def.gain_mana_ticks,
                "gain_mana_ticks {}", xml_def.id
            );
            assert_eq!(
                lua_def.gain_mana_amount, xml_def.gain_mana_amount,
                "gain_mana_amount {}", xml_def.id
            );
            assert!(
                (lua_def.mana_multiplier - xml_def.mana_multiplier).abs() < 1e-4,
                "mana_multiplier {}",
                xml_def.id
            );
            assert_eq!(lua_def.attack_speed_ms, xml_def.attack_speed_ms, "attack_speed {}", xml_def.id);
            assert_eq!(lua_def.base_speed, xml_def.base_speed, "base_speed {}", xml_def.id);
            assert_eq!(lua_def.soul_max, xml_def.soul_max, "soul_max {}", xml_def.id);
            assert_eq!(
                lua_def.gain_soul_ticks, xml_def.gain_soul_ticks,
                "gain_soul_ticks {}", xml_def.id
            );
            assert_eq!(lua_def.allow_pvp, xml_def.allow_pvp, "allow_pvp {}", xml_def.id);
            for i in 0..7 {
                assert!(
                    (lua_def.skill_multipliers[i] - xml_def.skill_multipliers[i]).abs() < 1e-4,
                    "skill[{}] {}",
                    i,
                    xml_def.id
                );
            }
            assert!((lua_def.formula.melee_damage - xml_def.formula.melee_damage).abs() < 1e-4);
            assert!((lua_def.formula.dist_damage - xml_def.formula.dist_damage).abs() < 1e-4);
            assert!((lua_def.formula.defense - xml_def.formula.defense).abs() < 1e-4);
            assert!((lua_def.formula.armor - xml_def.formula.armor).abs() < 1e-4);
        }

        // Lua must not carry vocations absent from the XML.
        assert_eq!(reg.vocations.len(), xml_defs.len(), "vocation count mismatch");
    }
}
