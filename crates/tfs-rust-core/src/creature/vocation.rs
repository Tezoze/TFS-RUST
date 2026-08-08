//! Level and vocation stat progression (TFS `Player::getReqExperience`, vocation gains).
// C++ reference: `player.cpp`, `vocation.cpp`; 772 base speed — `gameserver/src/player.h` `updateBaseSpeed`.
//
// 772 per-vocation `AddLevel` for HP/mana/cap — `crplayer.cc:1050-1093` `TPlayer::SetProfession`.
// Level-1 vitals floor (HP=150, Mana=0, Cap=400 oz) — `runtime/mon/human.mon` race data
// (`Skills = { (HitPoints, 150, 0, 150, …), (Mana, 0, 0, 0, …), (CarryStrength, 400, 0, 400, …) }`).
// Runtime capacity is centi-oz (`Player::capacity = 40000`, `iologindata.cpp` `cap * 100`);
// TFS loads vocation `gainCap` as XML oz × 100 (`vocation.cpp`).

use tfs_rust_content::vocations::VocationDef;

/// Total experience required to **reach** `level` (level >= 2). Matches the
/// 772 `TSkillLevel::GetExpForLevel` polynomial (`crskill.cc:352`) — same curve
/// as `combat::math::experience_for_level`'s `DeltaPoly` native path with
/// `Delta = 100`. Kept as a pure function (no `MechanicsProfile`/`FormulaHooks`
/// needed) for level-up checks in `player.rs`/`stats.rs` that run without a
/// profile in scope. The hookable Tier-2 path is `combat::math::experience_for_level`.
// C++ reference: `Player::getExpForLevel` / `crskill.cc:352` `TSkillLevel::GetExpForLevel`.
pub fn total_experience_for_level(level: u32) -> u64 {
    crate::combat::math::experience_for_level_poly(level as i64, 100).max(0) as u64
}

/// Experience needed to go from `level` to `level + 1`.
pub fn experience_to_next_level(level: i32) -> u64 {
    if level < 1 {
        return 0;
    }
    let next = total_experience_for_level(level as u32 + 1);
    let cur = total_experience_for_level(level as u32);
    next.saturating_sub(cur)
}

/// `Copy` hot-path snapshot of the vocation combat block — cached on `Player`
/// at login so level-up/regen/speed reads don't thread `&VocationRegistry`
/// through every hot-path call. Built once from `VocationDef` via
/// [`VocationProfile::from_def`].
///
/// C++ analogue: the `AddLevel`/`Max` fields on `TSkill` for `SKILL_HITPOINTS`/
/// `SKILL_MANA`/`SKILL_CARRY_STRENGTH` + vocation `baseSpeed`/`attackSpeed`/
/// `manaMultiplier`/`soulMax`/formula multipliers.
#[derive(Debug, Clone, Copy, Default)]
pub struct VocationProfile {
    pub id: i32,
    /// `fromvocation` — base vocation id; `!= id` means promoted (`crplayer.cc:344`).
    pub from_vocation: i32,
    /// `basespeed` — vocation GoStrength floor (`gameserver/src/player.h` `updateBaseSpeed`).
    pub base_speed: i32,
    /// `gainhp` — HP gain per level (`crplayer.cc:1051` `AddLevel`).
    pub gain_hp: i32,
    /// `gainmana` — mana gain per level (`crplayer.cc:1052` `AddLevel`).
    pub gain_mana: i32,
    /// `gaincap` per level in **centi-oz** (TFS `vocation.cpp` `gainCap = xml * 100`;
    /// `crplayer.cc:1053` `AddLevel` is oz on the skill — we match TFS internal units).
    pub gain_cap: i32,
    /// Level-1 HP floor (`human.mon` `HitPoints` `Actual=150`).
    pub base_hp: i32,
    /// Level-1 mana floor (`human.mon` `Mana` `Actual=0`).
    pub base_mana: i32,
    /// Level-1 capacity floor in **centi-oz** (`human.mon` CarryStrength 400 oz → 40000).
    pub base_cap: i32,
    /// `attackspeed` — melee attack cadence in ms (TFS `Vocation::attackSpeed`).
    pub attack_speed_ms: u32,
    /// `manamultiplier` — mana spell-cost multiplier (TFS `Vocation::manaMultiplier`).
    pub mana_multiplier: f32,
    /// `soulmax` — max soul points (`crplayer.cc:130` `Soul->Max`).
    pub soul_max: i32,
    /// `gainsoulticks` — rounds between soul regen ticks (`crplayer.cc:137`).
    pub gain_soul_ticks: u32,
    /// Vocation `<formula>` block — damage/defense/armor multipliers.
    pub formula: VocationFormulaProfile,
    /// `SKILL_FIST..SKILL_FISHING` multipliers (indices 0..6).
    pub skill_multipliers: [f32; 7],
    /// `allowPvp` — vocation may initiate PvP (`vocations.xml`; 772 `PROFESSION_NONE` ⇒ false).
    pub allow_pvp: bool,
}

/// `Copy` mirror of `tfs_rust_content::vocations::VocationFormula`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VocationFormulaProfile {
    pub melee_damage: f32,
    pub dist_damage: f32,
    pub defense: f32,
    pub armor: f32,
}

impl VocationProfile {
    /// Build the hot-path snapshot from the full `VocationDef` (loaded from
    /// `data/vocations.lua`). Called at login and on vocation change.
    pub fn from_def(d: &VocationDef) -> Self {
        // Content (`vocations.lua` / XML) stores capacity in oz; TFS multiplies by 100
        // at load (`vocation.cpp` `gainCap`). Keep `VocationDef` in oz for data-pack
        // parity; convert here so `Player.capacity` / level-up stay in centi-oz.
        Self {
            id: d.id as i32,
            from_vocation: d.from_vocation as i32,
            base_speed: d.base_speed,
            gain_hp: d.gain_hp,
            gain_mana: d.gain_mana,
            gain_cap: d.gain_cap.saturating_mul(100),
            base_hp: d.base_hp,
            base_mana: d.base_mana,
            base_cap: d.base_cap.saturating_mul(100),
            attack_speed_ms: d.attack_speed_ms,
            mana_multiplier: d.mana_multiplier,
            soul_max: d.soul_max,
            gain_soul_ticks: d.gain_soul_ticks,
            formula: VocationFormulaProfile {
                melee_damage: d.formula.melee_damage,
                dist_damage: d.formula.dist_damage,
                defense: d.formula.defense,
                armor: d.formula.armor,
            },
            skill_multipliers: d.skill_multipliers,
            allow_pvp: d.allow_pvp,
        }
    }

    /// Fallback profile for vocation id 0 ("None") when the registry is absent
    /// (test harness). Matches the shipped `data/vocations.lua` vocation 0,
    /// with capacity fields in centi-oz (TFS internal units).
    pub fn none_vocation() -> Self {
        Self {
            id: 0,
            from_vocation: 0,
            base_speed: 70,
            gain_hp: 5,
            gain_mana: 5,
            gain_cap: 1000,
            base_hp: 150,
            base_mana: 0,
            base_cap: 40000,
            attack_speed_ms: 2000,
            mana_multiplier: 4.0,
            soul_max: 100,
            gain_soul_ticks: 120,
            formula: VocationFormulaProfile {
                melee_damage: 1.0,
                dist_damage: 1.0,
                defense: 1.0,
                armor: 1.0,
            },
            skill_multipliers: [1.5, 2.0, 2.0, 2.0, 2.0, 1.5, 1.1],
            allow_pvp: false,
        }
    }

    /// Per-level resource gains `(hp, mana, cap)` — `Vocation::getHealthGain`/
    /// `getManaGain`/`getCapGain` (`vocation.cpp`).
    pub fn per_level_gains(&self) -> (i32, i32, i32) {
        (self.gain_hp, self.gain_mana, self.gain_cap)
    }

    /// Recompute max health / mana / cap for current level (called on level-up).
    ///
    /// HP/mana: `base + gain * (level - 1)` from vocation/race floor.
    /// Cap: same shape in **centi-oz** (TFS `capacity += getCapGain()` with
    /// `getCapGain()` already ×100 from XML oz).
    pub fn recalculate_vitals(&self, level: i32) -> (i32, i32, i32) {
        let l = level.max(1);
        let max_health = self.base_hp + self.gain_hp * (l - 1);
        let max_mana = self.base_mana + self.gain_mana * (l - 1);
        let cap = self.base_cap + self.gain_cap * (l - 1);
        (max_health, max_mana, cap)
    }
}

use crate::formulas::StepSpeedModel;

/// Stored `Creature::baseSpeed` (GoStrength) before `GetSpeed = 2*base+80`.
///
/// - **1098** — TFS `vocation->getBaseSpeed() + 2*(level-1)` (`src/player.h` `updateBaseSpeed`).
/// - **772** — decompile `TSkillAdd::Advance` with `AddLevel=1` (`crskill.cc:667`,
///   `human.mon:27` GoStrength `AddLevel=1`): level 1 starts at `Act=base_speed`,
///   each of the `level-1` level-ups adds `AddLevel` → `base_speed + (level-1)`.
///   TVP `gameserver/src/player.h:1102` uses `base + (level>1?level:0)` (off-by-one
///   vs the decompile); we follow the decompile as 772 mechanics authority.
///
/// Reads `base_speed` from the cached [`VocationProfile`] snapshot — no
/// `&VocationRegistry` borrow needed in hot paths.
///
/// `set_max_speed` mirrors TFS `Player::updateBaseSpeed()`: when the player's
/// group has `PlayerFlag_SetMaxSpeed`, base speed is pinned to `PLAYER_MAX_SPEED`
/// (1500) instead of the vocation-derived value (`src/player.h`).
pub fn base_walk_speed(
    model: StepSpeedModel,
    profile: &VocationProfile,
    level: i32,
    set_max_speed: bool,
) -> i32 {
    if set_max_speed {
        return 1500;
    }
    let voc_base = profile.base_speed;
    let l = level.max(1);
    match model {
        // 772 decompile: `Act + (level-1)*AddLevel` with AddLevel=1 (`crskill.cc:667`).
        StepSpeedModel::LinearGo => voc_base + (l - 1),
        StepSpeedModel::TfsLog => (voc_base + 2 * (l - 1)).clamp(10, 1500),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::StepSpeedModel;

    #[test]
    fn base_walk_speed_matches_decompile_advance() {
        // Decompile `TSkillAdd::Advance` (`crskill.cc:667`): Act = base + (level-1)*AddLevel,
        // AddLevel=1 from `human.mon:27`. base_speed=220 (test fixture), level 8 → 220+7=227,
        // GetSpeed = 2*227+80 = 534.
        let profile = VocationProfile {
            base_speed: 220,
            ..VocationProfile::none_vocation()
        };
        assert_eq!(
            base_walk_speed(StepSpeedModel::LinearGo, &profile, 8, false),
            227
        );
        assert_eq!(
            crate::formulas::linear_go_effective_speed(base_walk_speed(
                StepSpeedModel::LinearGo,
                &profile,
                8,
                false
            )),
            534
        );
        // Shipped vocations.lua base_speed=70, level 8 → 70+7=77, GetSpeed=234.
        let shipped = VocationProfile::none_vocation();
        assert_eq!(
            base_walk_speed(StepSpeedModel::LinearGo, &shipped, 8, false),
            77
        );
        // Level 1 → base unchanged (no level-ups yet).
        assert_eq!(
            base_walk_speed(StepSpeedModel::LinearGo, &shipped, 1, false),
            70
        );
        // TFS 1098: 220 + 2*7 = 234
        assert_eq!(
            base_walk_speed(StepSpeedModel::TfsLog, &profile, 8, false),
            234
        );
        // GM max speed flag pins to 1500 regardless of vocation/level.
        assert_eq!(
            base_walk_speed(StepSpeedModel::LinearGo, &profile, 8, true),
            1500
        );
        assert_eq!(
            base_walk_speed(StepSpeedModel::TfsLog, &shipped, 1, true),
            1500
        );
    }

    #[test]
    fn recalculate_vitals_uses_vocation_floor_and_gains() {
        // Knight: base_hp=150, gain_hp=15, level 8 → 150 + 15*7 = 255.
        // Cap in centi-oz: base 40000 + 2500*7 = 57500 (400 + 25*7 oz).
        let knight = VocationProfile {
            id: 4,
            base_speed: 70,
            gain_hp: 15,
            gain_mana: 5,
            gain_cap: 2500,
            base_hp: 150,
            base_mana: 0,
            base_cap: 40000,
            ..VocationProfile::none_vocation()
        };
        let (hp, mana, cap) = knight.recalculate_vitals(8);
        assert_eq!(hp, 255);
        assert_eq!(mana, 35);
        assert_eq!(cap, 57500);

        // Level 1 → floor values.
        let (hp1, mana1, cap1) = knight.recalculate_vitals(1);
        assert_eq!((hp1, mana1, cap1), (150, 0, 40000));
    }

    #[test]
    fn from_def_converts_capacity_oz_to_centi_oz() {
        let def = VocationDef {
            id: 4,
            client_id: 1,
            name: "Knight".into(),
            description: "a knight".into(),
            from_vocation: 4,
            gain_cap: 25,
            gain_hp: 15,
            gain_mana: 5,
            gain_hp_ticks: 6,
            gain_hp_amount: 1,
            gain_mana_ticks: 6,
            gain_mana_amount: 2,
            mana_multiplier: 3.0,
            attack_speed_ms: 2000,
            base_speed: 70,
            soul_max: 100,
            gain_soul_ticks: 120,
            allow_pvp: false,
            base_hp: 150,
            base_mana: 0,
            base_cap: 400,
            formula: Default::default(),
            skill_multipliers: [1.1; 7],
        };
        let p = VocationProfile::from_def(&def);
        assert_eq!(p.base_cap, 40000);
        assert_eq!(p.gain_cap, 2500);
        assert_eq!(p.recalculate_vitals(8).2, 57500);
    }
}
