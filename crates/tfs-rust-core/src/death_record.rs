//! Player death row + kill statistics snapshot/persist.
//! Pack: TFS `player_deaths` / `kill_statistics`; `/deathlist`.
//! Corpus: `TPlayer::RecordDeath` — `crplayer.cc`; `AddKillStatistics` / `WriteKillStatistics` — `crmain.cc`.

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::kill_statistics::{ENV_KILL_RACE, race_name_for_kind};
use tfs_rust_common::enums::CombatType;
use tfs_rust_db::death::{DeathStore, PlayerDeathRow};

const KILLED_BY_MAX: usize = 255;
const MOSTDAMAGE_BY_MAX: usize = 100;

/// Last-hit or most-damage actor used by the death-row builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeathActor {
    pub name: String,
    pub is_player: bool,
}

/// Corpus env remarks (`CharacterDeathOrder` / `RecordDeath`) mapped to TFS `killed_by`.
pub fn env_killed_by(ctype: CombatType) -> &'static str {
    match ctype {
        CombatType::Earth | CombatType::PoisonPeriodic => "poison",
        CombatType::Fire | CombatType::FirePeriodic => "fire",
        CombatType::Energy | CombatType::EnergyPeriodic => "energy",
        _ => "a hit",
    }
}

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Pure TFS `player_deaths` builder — one row (not two corpus rows).
pub fn build_player_death_row(
    player_id: i32,
    time: i64,
    level: i32,
    last_hit: Option<&DeathActor>,
    last_damage_type: CombatType,
    most_damage: Option<&DeathActor>,
    unjustified: i8,
    mostdamage_unjustified: i8,
) -> PlayerDeathRow {
    let (killed_by, is_player) = match last_hit {
        Some(actor) => (
            truncate(&actor.name, KILLED_BY_MAX),
            i8::from(actor.is_player),
        ),
        None => (env_killed_by(last_damage_type).to_string(), 0),
    };
    let (mostdamage_by, mostdamage_is_player) = match most_damage {
        Some(actor) => (
            truncate(&actor.name, MOSTDAMAGE_BY_MAX),
            i8::from(actor.is_player),
        ),
        None => (String::new(), 0),
    };
    PlayerDeathRow {
        player_id,
        time,
        level,
        killed_by,
        is_player,
        mostdamage_by,
        mostdamage_is_player,
        unjustified,
        mostdamage_unjustified,
    }
}

impl GameWorld {
    /// Snapshot kill-stat races + player death row, then persist (VIP-style spawn).
    ///
    /// Call from [`Self::apply_creature_death`] **before** skill/exp loss. Does **not**
    /// call `RecordMurder` (already in `player_on_pvp_death_marks`).
    pub(crate) fn record_lethal_outcome(&mut self, victim: CreatureId) {
        let Some((
            last_hit,
            last_damage_type,
            defender_race,
            is_player,
            old_level,
            guid,
            damage_map,
        )) = self.creatures.get(victim).map(|kind| {
            let (old_level, guid) = match kind {
                CreatureKind::Player(p) => (p.level, p.guid),
                _ => (0, 0),
            };
            (
                kind.base().last_hit_by,
                kind.base().last_damage_type,
                race_name_for_kind(kind),
                matches!(kind, CreatureKind::Player(_)),
                old_level,
                guid,
                kind.base().damage_map.clone(),
            )
        })
        else {
            return;
        };

        let attacker_race = last_hit
            .and_then(|id| self.creatures.get(id).map(race_name_for_kind))
            .unwrap_or_else(|| ENV_KILL_RACE.to_string());
        let last_hit_actor = last_hit.and_then(|id| {
            self.creatures.get(id).map(|k| DeathActor {
                name: k.base().name.clone(),
                is_player: matches!(k, CreatureKind::Player(_)),
            })
        });

        let window = self.mechanics.profile.exp_attribution_rounds;
        let most_id = damage_map.most_dangerous(self.round_nr, window);
        let most_damage_id_actor = most_id.and_then(|id| {
            if Some(id) == last_hit {
                return None;
            }
            let k = self.creatures.get(id)?;
            if !matches!(k, CreatureKind::Player(_)) {
                return None;
            }
            Some((
                id,
                DeathActor {
                    name: k.base().name.clone(),
                    is_player: true,
                },
            ))
        });

        self.kill_stats.add(&attacker_race, &defender_race);

        if !is_player {
            return;
        }
        // Corpus `GetPlayer == NULL` abort (`crplayer.cc` RecordDeath): last-hit player
        // already gone from SlotMap — skip persist, kill stats already applied.
        if last_hit.is_some() && last_hit_actor.is_none() {
            return;
        }

        let unjustified = match (last_hit, last_hit_actor.as_ref()) {
            (Some(id), Some(a)) if a.is_player => {
                i8::from(!self.player_is_attack_justified(id, victim))
            }
            _ => 0,
        };
        let mostdamage_unjustified = match most_damage_id_actor.as_ref() {
            Some((id, _)) => i8::from(!self.player_is_attack_justified(*id, victim)),
            None => 0,
        };
        let Ok(player_id) = i32::try_from(guid) else {
            return;
        };
        let time = unix_i64();
        let row = build_player_death_row(
            player_id,
            time,
            old_level,
            last_hit_actor.as_ref(),
            last_damage_type,
            most_damage_id_actor.as_ref().map(|(_, a)| a),
            unjustified,
            mostdamage_unjustified,
        );
        self.persist_player_death(row);
    }

    fn persist_player_death(&self, row: PlayerDeathRow) {
        let db = self.db.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(e) = DeathStore::new(&db).insert_player_death(&row).await {
                    tracing::error!(?e, player_id = row.player_id, "player_deaths insert failed");
                }
            });
        }
    }
}

fn unix_i64() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::CreatureKind;
    use crate::sim_harness::{minimal_world, test_player};
    use tfs_rust_common::Position;

    #[test]
    fn env_physical_is_a_hit() {
        let row = build_player_death_row(1, 0, 8, None, CombatType::Physical, None, 0, 0);
        assert_eq!(row.killed_by, "a hit");
        assert_eq!(row.is_player, 0);
    }

    #[test]
    fn env_earth_is_poison() {
        let row = build_player_death_row(1, 0, 8, None, CombatType::Earth, None, 0, 0);
        assert_eq!(row.killed_by, "poison");
        assert_eq!(row.is_player, 0);
    }

    #[test]
    fn monster_name_is_not_player() {
        let actor = DeathActor {
            name: "rat".into(),
            is_player: false,
        };
        let row = build_player_death_row(1, 0, 8, Some(&actor), CombatType::Physical, None, 0, 0);
        assert_eq!(row.killed_by, "rat");
        assert_eq!(row.is_player, 0);
    }

    #[test]
    fn player_name_is_player() {
        let actor = DeathActor {
            name: "Alice".into(),
            is_player: true,
        };
        let row = build_player_death_row(1, 0, 8, Some(&actor), CombatType::Physical, None, 0, 0);
        assert_eq!(row.killed_by, "Alice");
        assert_eq!(row.is_player, 1);
    }

    #[test]
    fn mostdamage_filled_when_different_player() {
        let last = DeathActor {
            name: "Alice".into(),
            is_player: true,
        };
        let most = DeathActor {
            name: "Bob".into(),
            is_player: true,
        };
        let row = build_player_death_row(
            1,
            0,
            20,
            Some(&last),
            CombatType::Physical,
            Some(&most),
            1,
            0,
        );
        assert_eq!(row.killed_by, "Alice");
        assert_eq!(row.is_player, 1);
        assert_eq!(row.mostdamage_by, "Bob");
        assert_eq!(row.mostdamage_is_player, 1);
        assert_eq!(row.unjustified, 1);
        assert_eq!(row.mostdamage_unjustified, 0);
    }

    #[test]
    fn build_player_save_data_copies_soul_timer() {
        let mut world = minimal_world();
        let mut p = test_player("Soul", Position::new(100, 100, 7));
        p.soul_cycle = 2;
        p.soul_count = 7;
        p.soul_max_count = 120;
        let cid = world.creatures.insert(CreatureKind::Player(p));
        let data = world.build_player_save_data(cid).expect("persist baseline");
        assert_eq!(data.player.soul_cycle, 2);
        assert_eq!(data.player.soul_count, 7);
        assert_eq!(data.player.soul_max_count, 120);
    }
}
