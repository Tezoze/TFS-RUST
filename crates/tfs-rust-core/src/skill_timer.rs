//! 772 `TSkill::Process` — `(Cycle, Count, MaxCount)` timer for spell conditions.
//!
//! C++ reference: `crskill.cc:176-193` `TSkill::Process`; `SetTimer` call sites
//! `magic.cc:2288,2308,2336,2349,3431,3501,3511,3516,4250`.

use tfs_rust_common::enums::ConditionType;
use tfs_rust_lua::ConditionApplySpec;

/// Pack strong-haste `CONDITION_PARAM_SPEED` (`strong_haste.lua`). Corpus uses 70;
/// we only switch the timer triple, not the speed delta.
pub const STRONG_HASTE_SPEED_MIN: i32 = 60;

/// 772 `TSkill` Cycle / Count / MaxCount (`crskill.cc:176-193`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkillTimer {
    pub cycle: i32,
    pub count: i32,
    pub max_count: i32,
}

/// One `ProcessSkills` step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerStep {
    Idle,
    Event,
    Expired,
}

impl SkillTimer {
    pub fn from_triple(t: SkillTimerTriple) -> Self {
        Self {
            cycle: t.cycle,
            count: t.count,
            max_count: t.max_count,
        }
    }

    /// `TSkill::Process` (`crskill.cc:176-193`).
    ///
    /// `Cycle == 0` at entry → Expired (CheckState / remove). Last Event leaves
    /// `Cycle == 0` in the list for one more tick (L11).
    pub fn process(&mut self) -> TimerStep {
        if self.cycle == 0 {
            return TimerStep::Expired;
        }
        if self.count <= 0 {
            self.count = self.max_count;
            let range = if self.cycle < 0 { 1 } else { -1 };
            self.cycle += range;
            TimerStep::Event
        } else {
            self.count -= 1;
            TimerStep::Idle
        }
    }

    /// ProcessSkills ticks until `Expired` (inclusive of the expiry tick).
    ///
    /// `remaining = count + cycle * (max_count + 1) - max_count + 1` when MaxCount > 0.
    /// When MaxCount is 0 the timer is a simple Cycle countdown (Infight / legacy blobs).
    pub fn remaining_process_ticks(&self) -> i32 {
        if self.cycle <= 0 {
            return 0;
        }
        if self.max_count <= 0 {
            return self.cycle;
        }
        self.count + self.cycle * (self.max_count + 1) - self.max_count + 1
    }
}

/// Profile triple loaded from `772.lua` `skillTimers`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTimerTriple {
    pub cycle: i32,
    pub count: i32,
    pub max_count: i32,
}

impl SkillTimerTriple {
    pub const fn new(cycle: i32, count: i32, max_count: i32) -> Self {
        Self {
            cycle,
            count,
            max_count,
        }
    }
}

/// Corpus `SetTimer` literals (`magic.cc` haste/paralyze/light/invis/manashield).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTimers {
    pub haste: SkillTimerTriple,
    pub strong_haste: SkillTimerTriple,
    pub paralyze: SkillTimerTriple,
    pub mana_shield: SkillTimerTriple,
    pub invisible: SkillTimerTriple,
    /// Utevo lux Duration (`Enlight(..., 6, 500)`).
    pub light_6: i32,
    /// Gran lux Duration (`..., 8, 1000`). Pack great_light level 7 aliases to 8.
    pub light_8: i32,
    /// Vis lux Duration (`..., 9, 2000`).
    pub light_9: i32,
}

impl SkillTimers {
    /// `magic.cc:2288,2308,2336,2349,3431,3501,4250`.
    pub const fn classic_772() -> Self {
        Self {
            haste: SkillTimerTriple::new(3, 10, 10),
            strong_haste: SkillTimerTriple::new(2, 10, 10),
            paralyze: SkillTimerTriple::new(1, 10, 10),
            mana_shield: SkillTimerTriple::new(1, 200, 200),
            invisible: SkillTimerTriple::new(1, 200, 200),
            light_6: 500,
            light_8: 1000,
            light_9: 2000,
        }
    }

    pub fn light_duration(&self, radius: u8) -> i32 {
        match map_light_radius(radius) {
            8 => self.light_8,
            9 => self.light_9,
            _ => self.light_6,
        }
    }
}

/// Pack `great_light.lua` uses level 7; corpus gran lux is Radius 8.
pub fn map_light_radius(radius: u8) -> u8 {
    if radius == 7 { 8 } else { radius }
}

/// `SetTimer(SKILL_LIGHT, Radius, Duration/Radius, Duration/Radius)` (`magic.cc:2336`).
pub fn light_timer(radius: u8, timers: &SkillTimers) -> SkillTimer {
    let radius = map_light_radius(radius).max(1);
    let duration = timers.light_duration(radius);
    let r = i32::from(radius);
    let count = duration / r;
    SkillTimer {
        cycle: r,
        count,
        max_count: count,
    }
}

/// True for conditions that use [`SkillTimer::process`] (not Infight / Outfit / YellTicks).
pub fn uses_skill_timer_process(ctype: ConditionType) -> bool {
    matches!(
        ctype,
        ConditionType::Fire
            | ConditionType::Energy
            | ConditionType::Haste
            | ConditionType::Paralyze
            | ConditionType::Light
            | ConditionType::Invisible
            | ConditionType::ManaShield
            | ConditionType::Drunk
    )
}

/// Arm haste / paralyze / light / invis / manashield from corpus triples.
/// Returns `None` for types that keep pack `ceil(ms/1000)` duration.
pub fn arm_from_spec(
    ctype: ConditionType,
    spec: &ConditionApplySpec,
    timers: &SkillTimers,
) -> Option<SkillTimer> {
    match ctype {
        ConditionType::Haste => Some(SkillTimer::from_triple(
            if spec.speed >= STRONG_HASTE_SPEED_MIN {
                timers.strong_haste
            } else {
                timers.haste
            },
        )),
        ConditionType::Paralyze => Some(SkillTimer::from_triple(timers.paralyze)),
        ConditionType::ManaShield => Some(SkillTimer::from_triple(timers.mana_shield)),
        ConditionType::Invisible => Some(SkillTimer::from_triple(timers.invisible)),
        ConditionType::Light => {
            let level = spec.light_level.clamp(0, 255) as u8;
            Some(light_timer(level, timers))
        }
        _ => None,
    }
}

/// Rebuild Cycle/Count/MaxCount from a TFS blob remaining-tick count.
pub fn reconstruct_from_remaining(remaining: i32, triple: SkillTimerTriple) -> SkillTimer {
    if remaining <= 0 {
        return SkillTimer {
            cycle: 0,
            count: 0,
            max_count: triple.max_count,
        };
    }
    let period = (triple.max_count + 1).max(1);
    let cycle = ((remaining - 1) / period).max(1);
    SkillTimer {
        cycle,
        count: triple.max_count,
        max_count: triple.max_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_until_expired(mut t: SkillTimer) -> (i32, i32) {
        let mut last_event = 0;
        let mut expired = 0;
        for tick in 1..=10_000 {
            match t.process() {
                TimerStep::Event => last_event = tick,
                TimerStep::Expired => {
                    expired = tick;
                    break;
                }
                TimerStep::Idle => {}
            }
        }
        (last_event, expired)
    }

    #[test]
    fn haste_lasts_33_ticks() {
        let t = SkillTimer::from_triple(SkillTimers::classic_772().haste);
        let (last_event, expired) = run_until_expired(t);
        assert_eq!(last_event, 33);
        assert_eq!(expired, 34);
    }

    #[test]
    fn strong_haste_lasts_22_ticks() {
        let t = SkillTimer::from_triple(SkillTimers::classic_772().strong_haste);
        let (last_event, expired) = run_until_expired(t);
        assert_eq!(last_event, 22);
        assert_eq!(expired, 23);
    }

    #[test]
    fn paralyze_rune_11_ticks() {
        let t = SkillTimer::from_triple(SkillTimers::classic_772().paralyze);
        let (last_event, expired) = run_until_expired(t);
        assert_eq!(last_event, 11);
        assert_eq!(expired, 12);
    }

    #[test]
    fn manashield_expires_at_202() {
        let t = SkillTimer::from_triple(SkillTimers::classic_772().mana_shield);
        let (last_event, expired) = run_until_expired(t);
        assert_eq!(last_event, 201);
        assert_eq!(expired, 202);
    }

    #[test]
    fn invisibility_last_event_at_201() {
        let t = SkillTimer::from_triple(SkillTimers::classic_772().invisible);
        let (last_event, expired) = run_until_expired(t);
        assert_eq!(last_event, 201);
        assert_eq!(expired, 202);
    }

    #[test]
    fn utevo_lux_shrinks_every_84_total_504() {
        let timers = SkillTimers::classic_772();
        let mut t = light_timer(6, &timers);
        assert_eq!(t.cycle, 6);
        assert_eq!(t.count, 83);
        let mut shrinks = Vec::new();
        for tick in 1..=600 {
            match t.process() {
                TimerStep::Event => shrinks.push((tick, t.cycle)),
                TimerStep::Expired => {
                    assert_eq!(shrinks.last().copied(), Some((504, 0)));
                    assert_eq!(tick, 505);
                    return;
                }
                TimerStep::Idle => {}
            }
        }
        panic!("utevo lux did not expire; shrinks={shrinks:?}");
    }

    #[test]
    fn drunk_event_every_duration_plus_one() {
        let mut t = SkillTimer {
            cycle: 1,
            count: 120,
            max_count: 120,
        };
        let (last_event, expired) = run_until_expired(t);
        assert_eq!(last_event, 121);
        assert_eq!(expired, 122);
        t = SkillTimer {
            cycle: 1,
            count: 120,
            max_count: 120,
        };
        for _ in 0..120 {
            assert_eq!(t.process(), TimerStep::Idle);
        }
        assert_eq!(t.process(), TimerStep::Event);
        assert_eq!(t.cycle, 0);
    }

    #[test]
    fn remaining_process_ticks_fresh_haste_is_34() {
        let t = SkillTimer::from_triple(SkillTimers::classic_772().haste);
        assert_eq!(t.remaining_process_ticks(), 34);
    }

    #[test]
    fn pack_great_light_level_7_aliases_radius_8() {
        let timers = SkillTimers::classic_772();
        let t = light_timer(7, &timers);
        assert_eq!(t.cycle, 8);
        assert_eq!(t.count, 1000 / 8);
        assert_eq!(t.max_count, 125);
    }
}
