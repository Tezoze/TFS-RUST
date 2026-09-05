//! Player death row + kill statistics SQL.
//! Pack: TFS `player_deaths` / `kill_statistics`; `/deathlist`.
//! Corpus: `TPlayer::RecordDeath` — `crplayer.cc`; `WriteKillStatistics` — `crmain.cc`.

use crate::pool::DbPool;
use tfs_rust_common::error::{Result, TfsRustError};

/// One `player_deaths` insert — TFS column shape (`data/migrations/12.lua`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDeathRow {
    pub player_id: i32,
    pub time: i64,
    pub level: i32,
    pub killed_by: String,
    pub is_player: i8,
    pub mostdamage_by: String,
    pub mostdamage_is_player: i8,
    pub unjustified: i8,
    pub mostdamage_unjustified: i8,
}

/// One `kill_statistics` upsert-add delta after a RAM drain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillStatisticDelta {
    pub name: String,
    /// Players killed by this race (corpus `KilledPlayers` → TFS `killed_by`).
    pub killed_by: u32,
    /// Times this race was killed (corpus `KilledCreatures` → TFS `killed`).
    pub killed: u32,
}

pub struct DeathStore<'a> {
    pool: &'a DbPool,
}

impl<'a> DeathStore<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// TFS `player_deaths` insert (`IOLoginData` death row / `/deathlist`).
    pub async fn insert_player_death(&self, row: &PlayerDeathRow) -> Result<()> {
        let player_id = row.player_id;
        let time = row.time;
        let level = row.level;
        let killed_by = row.killed_by.clone();
        let is_player = row.is_player;
        let mostdamage_by = row.mostdamage_by.clone();
        let mostdamage_is_player = row.mostdamage_is_player;
        let unjustified = row.unjustified;
        let mostdamage_unjustified = row.mostdamage_unjustified;
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let killed_by = killed_by.clone();
                let mostdamage_by = mostdamage_by.clone();
                async move {
                    sqlx::query(
                        r#"INSERT INTO player_deaths
                          (player_id, time, level, killed_by, is_player,
                           mostdamage_by, mostdamage_is_player, unjustified, mostdamage_unjustified)
                        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                    )
                    .bind(player_id)
                    .bind(time)
                    .bind(level)
                    .bind(&killed_by)
                    .bind(is_player)
                    .bind(&mostdamage_by)
                    .bind(mostdamage_is_player)
                    .bind(unjustified)
                    .bind(mostdamage_unjustified)
                    .execute(&pool)
                    .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    /// Upsert-add kill counters then caller zeros RAM (`WriteKillStatistics` + `InitKillStatistics`).
    pub async fn upsert_kill_statistics(
        &self,
        rows: &[KillStatisticDelta],
        time: u32,
    ) -> Result<()> {
        for row in rows {
            let name = row.name.clone();
            let killed_by = row.killed_by;
            let killed = row.killed;
            self.pool
                .execute_with_retry(|| {
                    let pool = self.pool.inner().clone();
                    let name = name.clone();
                    async move {
                        sqlx::query(
                            r#"INSERT INTO kill_statistics (name, killed_by, killed, time)
                            VALUES (?, ?, ?, ?)
                            ON DUPLICATE KEY UPDATE
                              killed_by = killed_by + VALUES(killed_by),
                              killed = killed + VALUES(killed),
                              time = VALUES(time)"#,
                        )
                        .bind(&name)
                        .bind(killed_by)
                        .bind(killed)
                        .bind(time)
                        .execute(&pool)
                        .await
                    }
                })
                .await
                .map_err(|e| TfsRustError::Database(e.to_string()))?;
        }
        Ok(())
    }
}
