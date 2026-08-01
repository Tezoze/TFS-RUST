//! PC-5 — skill tries `Increase` / magic-level advance.
//!
//! Domain: TFS per-level `tries` storage (`Player::addSkillTries` / `addManaSpent`).
//! Outcomes: 772 `TSkillProbe::Increase` (`crskill.cc:386`) — same leveling curve via
//! [`crate::combat::math::req_skill_tries`] with profile `skillTuning` + vocation multipliers.
//!
//! Call sites multiply by `config.rateSkill` / `rateMagic` before invoking these helpers
//! (TFS `data/events/scripts/player.lua` `onGainSkillTries`). Curve knobs stay in
//! `skillTuning`; rates stay in `config.lua`.
//!
//! C++ reference:
//! - `TSkillProbe::Increase` / `ProbeValue` — `crskill.cc:386`, `:535`.
//! - `TSkillProbe::GetExpForLevel` — `crskill.cc:472-496`.
//! - TFS `Vocation::getReqSkillTries` / `getReqMana` — `vocation.cpp:139-154`.

use crate::combat::math::req_skill_tries;
use crate::creature::Player;
use crate::formulas::{FormulaHooks, MechanicsProfile};
use crate::player::combat::SkillNr;

impl SkillNr {
    /// Index into `skill_multipliers` / `skill_tries.skill_base` (0..6).
    #[inline]
    pub fn try_index(self) -> usize {
        match self {
            SkillNr::Fist => 0,
            SkillNr::Club => 1,
            SkillNr::Sword => 2,
            SkillNr::Axe => 3,
            SkillNr::Distance => 4,
            SkillNr::Shielding => 5,
            SkillNr::Fishing => 6,
        }
    }

    #[inline]
    pub fn tries(self, skills: &crate::creature::PlayerSkills) -> u64 {
        match self {
            SkillNr::Fist => skills.fist_tries,
            SkillNr::Club => skills.club_tries,
            SkillNr::Sword => skills.sword_tries,
            SkillNr::Axe => skills.axe_tries,
            SkillNr::Distance => skills.dist_tries,
            SkillNr::Shielding => skills.shielding_tries,
            SkillNr::Fishing => skills.fishing_tries,
        }
    }

    #[inline]
    pub fn set_tries(self, skills: &mut crate::creature::PlayerSkills, tries: u64) {
        match self {
            SkillNr::Fist => skills.fist_tries = tries,
            SkillNr::Club => skills.club_tries = tries,
            SkillNr::Sword => skills.sword_tries = tries,
            SkillNr::Axe => skills.axe_tries = tries,
            SkillNr::Distance => skills.dist_tries = tries,
            SkillNr::Shielding => skills.shielding_tries = tries,
            SkillNr::Fishing => skills.fishing_tries = tries,
        }
    }

    #[inline]
    pub fn set_level(self, skills: &mut crate::creature::PlayerSkills, level: i32) {
        match self {
            SkillNr::Fist => skills.fist = level,
            SkillNr::Club => skills.club = level,
            SkillNr::Sword => skills.sword = level,
            SkillNr::Axe => skills.axe = level,
            SkillNr::Distance => skills.dist = level,
            SkillNr::Shielding => skills.shielding = level,
            SkillNr::Fishing => skills.fishing = level,
        }
    }
}

impl Player {
    /// TFS `Player::getPercentLevel` — `player.cpp:1914-1925`.
    #[inline]
    pub fn percent_level(count: u64, next_level_count: u64) -> u8 {
        if next_level_count == 0 {
            return 0;
        }
        let result = (count * 100) / next_level_count;
        if result > 100 {
            0
        } else {
            result as u8
        }
    }

    /// Percent toward next combat-skill level (TFS `skills[skill].percent` after `addSkillAdvance`).
    pub fn skill_percent(
        &self,
        skill: SkillNr,
        profile: &MechanicsProfile,
        hooks: &FormulaHooks,
    ) -> u8 {
        let idx = skill.try_index();
        let need = req_skill_tries(
            hooks,
            idx as i32,
            skill.level(&self.skills) + 1,
            profile.skill_tries.skill_base[idx],
            f64::from(self.vocation_profile.skill_multipliers[idx]),
            profile.skill_tries.min_level[idx],
        );
        Self::percent_level(skill.tries(&self.skills), need)
    }

    /// Percent toward next magic level (TFS `magLevelPercent` after `addManaSpent`).
    pub fn magic_percent(&self, profile: &MechanicsProfile, hooks: &FormulaHooks) -> u8 {
        let need = req_skill_tries(
            hooks,
            -1,
            self.skills.maglevel + 1,
            profile.skill_tries.magic_skill_base,
            f64::from(self.vocation_profile.mana_multiplier),
            profile.skill_tries.magic_min_level,
        );
        Self::percent_level(self.skills.manaspent, need)
    }

    /// C++ `TSkillProbe::Increase(Amount)` — per-level tries model (`crskill.cc:386`).
    ///
    /// Adds `amount` tries; while tries ≥ `req_skill_tries` for `level+1`, subtracts the
    /// requirement and increments the skill level. Returns how many skill levels were gained.
    pub fn skill_increase(
        &mut self,
        skill: SkillNr,
        amount: u64,
        profile: &MechanicsProfile,
        hooks: &FormulaHooks,
    ) -> u32 {
        if amount == 0 {
            return 0;
        }
        let idx = skill.try_index();
        let skill_base = profile.skill_tries.skill_base[idx];
        let min_level = profile.skill_tries.min_level[idx];
        let multiplier = f64::from(self.vocation_profile.skill_multipliers[idx]);
        let skill_code = idx as i32;

        let mut tries = skill.tries(&self.skills).saturating_add(amount);
        let mut level = skill.level(&self.skills);
        let mut levels_gained = 0u32;

        loop {
            let need = req_skill_tries(
                hooks,
                skill_code,
                level + 1,
                skill_base,
                multiplier,
                min_level,
            );
            if need == 0 || tries < need {
                break;
            }
            tries -= need;
            level += 1;
            levels_gained += 1;
        }

        skill.set_tries(&mut self.skills, tries);
        skill.set_level(&mut self.skills, level);
        levels_gained
    }

    /// TFS `Player::addManaSpent` — accumulates toward magic level (`vocation.cpp:149-154`).
    ///
    /// Uses `skill_base=1600`, `min_level=0`, `multiplier=mana_multiplier`.
    /// Returns how many magic levels were gained.
    pub fn magic_increase(
        &mut self,
        amount: u64,
        profile: &MechanicsProfile,
        hooks: &FormulaHooks,
    ) -> u32 {
        if amount == 0 {
            return 0;
        }
        let skill_base = profile.skill_tries.magic_skill_base;
        let min_level = profile.skill_tries.magic_min_level;
        let multiplier = f64::from(self.vocation_profile.mana_multiplier);

        let mut tries = self.skills.manaspent.saturating_add(amount);
        let mut level = self.skills.maglevel;
        let mut levels_gained = 0u32;

        loop {
            let need = req_skill_tries(hooks, -1, level + 1, skill_base, multiplier, min_level);
            if need == 0 || tries < need {
                break;
            }
            tries -= need;
            level += 1;
            levels_gained += 1;
        }

        self.skills.manaspent = tries;
        self.skills.maglevel = level;
        levels_gained
    }

    /// TFS `Player::removeSkillTries` — death skill loss (PC-5 M7).
    ///
    /// Removes `amount` tries from the current level bar, demoting levels when the bar
    /// underflows (never below `min_level`). Returns `true` if the skill level changed.
    pub fn skill_decrease(
        &mut self,
        skill: SkillNr,
        amount: u64,
        profile: &MechanicsProfile,
        hooks: &FormulaHooks,
    ) -> bool {
        if amount == 0 {
            return false;
        }
        let idx = skill.try_index();
        let skill_base = profile.skill_tries.skill_base[idx];
        let min_level = profile.skill_tries.min_level[idx];
        let multiplier = f64::from(self.vocation_profile.skill_multipliers[idx]);
        let skill_code = idx as i32;

        let mut remaining = amount;
        let mut tries = skill.tries(&self.skills);
        let mut level = skill.level(&self.skills);
        let old_level = level;

        if tries >= remaining {
            tries -= remaining;
            skill.set_tries(&mut self.skills, tries);
            return false;
        }
        remaining -= tries;
        tries = 0;

        while remaining > 0 && level > min_level {
            let need = req_skill_tries(hooks, skill_code, level, skill_base, multiplier, min_level);
            level -= 1;
            if remaining > need {
                remaining -= need;
            } else {
                tries = need - remaining;
                remaining = 0;
            }
        }

        skill.set_tries(&mut self.skills, tries);
        skill.set_level(&mut self.skills, level.max(min_level));
        level != old_level
    }

    /// Death magic-level try loss — same shape as [`Self::skill_decrease`].
    pub fn magic_decrease(
        &mut self,
        amount: u64,
        profile: &MechanicsProfile,
        hooks: &FormulaHooks,
    ) -> bool {
        if amount == 0 {
            return false;
        }
        let skill_base = profile.skill_tries.magic_skill_base;
        let min_level = profile.skill_tries.magic_min_level;
        let multiplier = f64::from(self.vocation_profile.mana_multiplier);

        let mut remaining = amount;
        let mut tries = self.skills.manaspent;
        let mut level = self.skills.maglevel;
        let old_level = level;

        if tries >= remaining {
            self.skills.manaspent = tries - remaining;
            return false;
        }
        remaining -= tries;
        tries = 0;

        while remaining > 0 && level > min_level {
            let need = req_skill_tries(hooks, -1, level, skill_base, multiplier, min_level);
            level -= 1;
            if remaining > need {
                remaining -= need;
            } else {
                tries = need - remaining;
                remaining = 0;
            }
        }

        self.skills.manaspent = tries;
        self.skills.maglevel = level.max(min_level);
        level != old_level
    }

    /// Cumulative tries invested in `skill` at the current level (sum of reqs + current bar).
    pub fn skill_total_tries(
        &self,
        skill: SkillNr,
        profile: &MechanicsProfile,
        hooks: &FormulaHooks,
    ) -> u64 {
        let idx = skill.try_index();
        let skill_base = profile.skill_tries.skill_base[idx];
        let min_level = profile.skill_tries.min_level[idx];
        let multiplier = f64::from(self.vocation_profile.skill_multipliers[idx]);
        let skill_code = idx as i32;
        let level = skill.level(&self.skills);
        let mut sum = skill.tries(&self.skills);
        let mut l = min_level + 1;
        while l <= level {
            sum = sum.saturating_add(req_skill_tries(
                hooks,
                skill_code,
                l,
                skill_base,
                multiplier,
                min_level,
            ));
            l += 1;
        }
        sum
    }

    /// Cumulative mana spent toward current magic level.
    pub fn magic_total_tries(
        &self,
        profile: &MechanicsProfile,
        hooks: &FormulaHooks,
    ) -> u64 {
        let skill_base = profile.skill_tries.magic_skill_base;
        let min_level = profile.skill_tries.magic_min_level;
        let multiplier = f64::from(self.vocation_profile.mana_multiplier);
        let level = self.skills.maglevel;
        let mut sum = self.skills.manaspent;
        let mut l = min_level + 1;
        while l <= level {
            sum = sum.saturating_add(req_skill_tries(
                hooks, -1, l, skill_base, multiplier, min_level,
            ));
            l += 1;
        }
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::vocation::VocationProfile;
    use crate::formulas::Mechanics;

    fn bare_player() -> Player {
        // Minimal stub — only skills/vocation matter for increase tests.
        crate::sim_harness::test_player("SkillTest", tfs_rust_common::Position::new(100, 100, 7))
    }

    #[test]
    fn sword_increase_30_no_level_60_levels() {
        let m = Mechanics::for_version(tfs_rust_common::ProtocolVersion::V772);
        let mut p = bare_player();
        p.skills.sword = 10;
        p.skills.sword_tries = 0;
        // None vocation sword mult = 2.0 → L11 needs 50.
        p.vocation_profile = VocationProfile {
            skill_multipliers: [1.5, 2.0, 2.0, 2.0, 2.0, 1.5, 1.1],
            ..p.vocation_profile
        };
        assert_eq!(p.skill_increase(SkillNr::Sword, 30, &m.profile, &m.hooks), 0);
        assert_eq!(p.skills.sword, 10);
        assert_eq!(p.skills.sword_tries, 30);
        assert_eq!(p.skill_percent(SkillNr::Sword, &m.profile, &m.hooks), 60); // 30/50
        assert_eq!(p.skill_increase(SkillNr::Sword, 30, &m.profile, &m.hooks), 1);
        assert_eq!(p.skills.sword, 11);
        assert_eq!(p.skills.sword_tries, 10); // 60 - 50
    }

    #[test]
    fn percent_level_matches_tfs() {
        assert_eq!(Player::percent_level(0, 100), 0);
        assert_eq!(Player::percent_level(50, 100), 50);
        assert_eq!(Player::percent_level(100, 100), 100);
        // TFS returns 0 when result would exceed 100.
        assert_eq!(Player::percent_level(101, 100), 0);
        assert_eq!(Player::percent_level(1, 0), 0);
    }

    #[test]
    fn magic_increase_1600_levels() {
        let m = Mechanics::for_version(tfs_rust_common::ProtocolVersion::V772);
        let mut p = bare_player();
        p.skills.maglevel = 0;
        p.skills.manaspent = 0;
        p.vocation_profile.mana_multiplier = 3.0;
        assert_eq!(p.magic_increase(1600, &m.profile, &m.hooks), 1);
        assert_eq!(p.skills.maglevel, 1);
        assert_eq!(p.skills.manaspent, 0);
    }

    #[test]
    fn add_experience_advances_current_vitals() {
        let mut p = bare_player();
        p.level = 1;
        p.experience = 0;
        p.vocation_profile.gain_hp = 15;
        p.vocation_profile.gain_mana = 5;
        p.vocation_profile.base_hp = 150;
        p.vocation_profile.base_mana = 0;
        p.vocation_profile.base_cap = 40000;
        p.vocation_profile.gain_cap = 1000;
        p.capacity = 40000;
        let (max_hp, max_mana, _) = p.vocation_profile.recalculate_vitals(1);
        p.base.max_health = max_hp;
        p.base.health = max_hp; // full
        p.max_mana = max_mana;
        p.mana = max_mana;
        // Exp to reach level 2.
        let need = crate::creature::vocation::total_experience_for_level(2);
        let leveled = p.add_experience(need, crate::formulas::StepSpeedModel::LinearGo);
        assert!(leveled);
        assert_eq!(p.level, 2);
        // M13: current HP/mana gain the vocation AddLevel, not clamp-only.
        assert_eq!(p.base.health, max_hp + 15);
        assert_eq!(p.mana, max_mana + 5);
        // Capacity stays in centi-oz (TFS `capacity += getCapGain()` with ×100 units).
        assert_eq!(p.capacity, 41000);
    }

    #[test]
    fn remove_experience_keeps_capacity_in_centi_oz() {
        let mut p = bare_player();
        p.level = 2;
        p.experience = crate::creature::vocation::total_experience_for_level(2);
        p.vocation_profile.base_cap = 40000;
        p.vocation_profile.gain_cap = 2500;
        p.vocation_profile.base_hp = 150;
        p.vocation_profile.gain_hp = 15;
        p.vocation_profile.base_mana = 0;
        p.vocation_profile.gain_mana = 5;
        p.capacity = 42500;
        p.base.max_health = 165;
        p.base.health = 165;
        p.max_mana = 5;
        p.mana = 5;
        // Drop below level-2 threshold → level 1.
        let leveled = p.remove_experience(
            p.experience,
            crate::formulas::StepSpeedModel::LinearGo,
        );
        assert!(leveled);
        assert_eq!(p.level, 1);
        assert_eq!(p.capacity, 40000);
    }
}
