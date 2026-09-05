//! In-memory kill statistics + flush.
//! Pack: TFS `kill_statistics`; `/deathlist` is deaths, not this table.
//! Corpus: `AddKillStatistics` / `WriteKillStatistics` / `InitKillStatistics` — `crmain.cc`.

use std::collections::HashMap;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use tfs_rust_db::death::{DeathStore, KillStatisticDelta};

/// Corpus `RaceData[].Name == "human"` for players.
pub const HUMAN_RACE: &str = "human";
/// Env / field / DoT attacker with no creature (`crmain.cc` fire/poison/energy).
pub const ENV_KILL_RACE: &str = "(fire/poison/energy)";

const KILL_STAT_NAME_MAX: usize = 35;

/// RAM counters: `(times_killed / killed, players_killed / killed_by)`.
#[derive(Debug, Default)]
pub struct KillStatistics {
    counts: HashMap<String, (u32, u32)>,
}

impl KillStatistics {
    /// `AddKillStatistics(attacker_race, defender_race)` — `crmain.cc:1168-1174`.
    ///
    /// Monster vs monster (neither `"human"`): no increment.
    pub fn add(&mut self, attacker_race: &str, defender_race: &str) {
        let attacker = truncate_name(attacker_race);
        let defender = truncate_name(defender_race);
        if attacker == HUMAN_RACE {
            self.inc_killed(&defender);
        }
        if defender == HUMAN_RACE {
            self.inc_killed_by(&attacker);
        }
    }

    fn inc_killed(&mut self, race: &str) {
        self.counts.entry(race.to_string()).or_default().0 += 1;
    }

    fn inc_killed_by(&mut self, race: &str) {
        self.counts.entry(race.to_string()).or_default().1 += 1;
    }

    /// Times this race was killed (SQL `killed`).
    pub fn killed(&self, race: &str) -> u32 {
        self.counts
            .get(truncate_name(race).as_str())
            .map(|c| c.0)
            .unwrap_or(0)
    }

    /// Players killed by this race (SQL `killed_by`).
    pub fn killed_by(&self, race: &str) -> u32 {
        self.counts
            .get(truncate_name(race).as_str())
            .map(|c| c.1)
            .unwrap_or(0)
    }

    /// Drain nonzero rows and zero RAM (`InitKillStatistics` after write).
    pub fn drain_nonzero(&mut self) -> Vec<KillStatisticDelta> {
        let taken = std::mem::take(&mut self.counts);
        taken
            .into_iter()
            .filter(|(_, (killed, killed_by))| *killed != 0 || *killed_by != 0)
            .map(|(name, (killed, killed_by))| KillStatisticDelta {
                name,
                killed_by,
                killed,
            })
            .collect()
    }
}

fn truncate_name(name: &str) -> String {
    name.chars().take(KILL_STAT_NAME_MAX).collect()
}

/// Player → `"human"`; monster/NPC → `base.name`; missing attacker → env token.
pub fn race_name_for_kind(kind: &CreatureKind) -> String {
    match kind {
        CreatureKind::Player(_) => HUMAN_RACE.to_string(),
        CreatureKind::Monster(m) => truncate_name(&m.base.name),
        CreatureKind::Npc(n) => truncate_name(&n.base.name),
    }
}

impl GameWorld {
    /// Snapshot RAM and fire-and-forget upsert (`tick_other_minute_jobs` minute 55).
    pub(crate) fn spawn_kill_statistics_flush(&mut self) {
        let rows = self.kill_stats.drain_nonzero();
        if rows.is_empty() {
            return;
        }
        let time = unix_u32();
        let db = self.db.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(e) = DeathStore::new(&db)
                    .upsert_kill_statistics(&rows, time)
                    .await
                {
                    tracing::error!(?e, "kill statistics flush failed");
                }
            });
        }
    }

    /// Drain RAM for an awaited shutdown / daily-save flush (game thread only).
    pub(crate) fn take_kill_statistics_flush(&mut self) -> Vec<KillStatisticDelta> {
        self.kill_stats.drain_nonzero()
    }
}

fn unix_u32() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().min(u64::from(u32::MAX)) as u32)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_kills_rat_increments_killed() {
        let mut ks = KillStatistics::default();
        ks.add(HUMAN_RACE, "rat");
        assert_eq!(ks.killed("rat"), 1);
        assert_eq!(ks.killed_by("rat"), 0);
        assert_eq!(ks.killed(HUMAN_RACE), 0);
        assert_eq!(ks.killed_by(HUMAN_RACE), 0);
    }

    #[test]
    fn rat_kills_human_increments_killed_by() {
        let mut ks = KillStatistics::default();
        ks.add("rat", HUMAN_RACE);
        assert_eq!(ks.killed_by("rat"), 1);
        assert_eq!(ks.killed("rat"), 0);
    }

    #[test]
    fn env_kills_human_increments_env_killed_by() {
        let mut ks = KillStatistics::default();
        ks.add(ENV_KILL_RACE, HUMAN_RACE);
        assert_eq!(ks.killed_by(ENV_KILL_RACE), 1);
        assert_eq!(ks.killed(ENV_KILL_RACE), 0);
    }

    #[test]
    fn rat_kills_dragon_no_change() {
        let mut ks = KillStatistics::default();
        ks.add("rat", "dragon");
        assert!(ks.drain_nonzero().is_empty());
    }

    #[test]
    fn human_vs_human_both_counters_on_human() {
        let mut ks = KillStatistics::default();
        ks.add(HUMAN_RACE, HUMAN_RACE);
        assert_eq!(ks.killed(HUMAN_RACE), 1);
        assert_eq!(ks.killed_by(HUMAN_RACE), 1);
    }

    #[test]
    fn drain_zeros_ram() {
        let mut ks = KillStatistics::default();
        ks.add(HUMAN_RACE, "rat");
        ks.add("rat", HUMAN_RACE);
        let rows = ks.drain_nonzero();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "rat");
        assert_eq!(rows[0].killed, 1);
        assert_eq!(rows[0].killed_by, 1);
        assert_eq!(ks.killed("rat"), 0);
        assert_eq!(ks.killed_by("rat"), 0);
        assert!(ks.drain_nonzero().is_empty());
    }
}
