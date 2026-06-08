//! 772 `AdvanceGame` staggered subsystem counters.
//!
//! C++ reference: `tibia-game-master/src/main.cc` `AdvanceGame` (~312–449).

/// Which subsystems fired after accumulating one beat delay.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Subsystem772Fired {
    pub creatures: bool,
    pub cron: bool,
    pub skills: bool,
    pub other: bool,
}

/// Independent ~1000 ms counters with staggered first-fire thresholds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SubsystemCounters772 {
    creature_time: u64,
    cron_time: u64,
    skill_time: u64,
    other_time: u64,
}

/// 772 `CreatureTimeCounter` first-fire threshold.
const CREATURE_THRESHOLD: u64 = 1750;
/// 772 `CronTimeCounter` first-fire threshold.
const CRON_THRESHOLD: u64 = 1500;
/// 772 `SkillTimeCounter` first-fire threshold.
const SKILL_THRESHOLD: u64 = 1250;
/// 772 `OtherTimeCounter` first-fire threshold.
const OTHER_THRESHOLD: u64 = 1000;
/// All counters subtract this on fire (CipSoft ~1000 ms period).
const RESET_MS: u64 = 1000;

impl SubsystemCounters772 {
    /// Accumulate `delay_ms` and return which subsystems should run this beat.
    pub fn accumulate(&mut self, delay_ms: u64) -> Subsystem772Fired {
        self.creature_time = self.creature_time.saturating_add(delay_ms);
        self.cron_time = self.cron_time.saturating_add(delay_ms);
        self.skill_time = self.skill_time.saturating_add(delay_ms);
        self.other_time = self.other_time.saturating_add(delay_ms);

        let mut fired = Subsystem772Fired::default();

        if self.creature_time >= CREATURE_THRESHOLD {
            self.creature_time -= RESET_MS;
            fired.creatures = true;
        }
        if self.cron_time >= CRON_THRESHOLD {
            self.cron_time -= RESET_MS;
            fired.cron = true;
        }
        if self.skill_time >= SKILL_THRESHOLD {
            self.skill_time -= RESET_MS;
            fired.skills = true;
        }
        if self.other_time >= OTHER_THRESHOLD {
            self.other_time -= RESET_MS;
            fired.other = true;
        }

        fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_fire_at_beat_driven_thresholds() {
        let mut counters = SubsystemCounters772::default();

        // 1000 ms — only `other` (threshold 1000).
        let f = counters.accumulate(1000);
        assert!(!f.creatures);
        assert!(!f.cron);
        assert!(!f.skills);
        assert!(f.other);

        // +250 ms → 1250 total on skill counter since reset → skills fire.
        let f = counters.accumulate(250);
        assert!(!f.creatures);
        assert!(!f.cron);
        assert!(f.skills);
        assert!(!f.other);

        // +250 ms → 1500 on cron counter → cron fires.
        let f = counters.accumulate(250);
        assert!(!f.creatures);
        assert!(f.cron);
        assert!(!f.skills);
        assert!(!f.other);

        // +250 ms → 1750 on creature counter → creatures fire.
        let f = counters.accumulate(250);
        assert!(f.creatures);
        assert!(!f.cron);
        assert!(!f.skills);
        assert!(!f.other);
    }

    #[test]
    fn counters_can_fire_multiple_subsystems_one_beat() {
        let mut counters = SubsystemCounters772::default();
        // Large lag step — all thresholds crossed in one AdvanceGame call.
        let f = counters.accumulate(2000);
        assert!(f.creatures);
        assert!(f.cron);
        assert!(f.skills);
        assert!(f.other);
    }
}
