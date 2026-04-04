use crate::pool::DbPool;
use async_trait::async_trait;
use tfs_rust_common::error::{Result, TfsRustError};

#[async_trait]
pub trait PlayerRepository {
    async fn load_player(&self, name: &str) -> Result<()>;
    async fn save_player(&self, player_id: u32) -> Result<()>;
}

pub struct PlayerStore<'a> {
    pool: &'a DbPool,
}

impl<'a> PlayerStore<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl<'a> PlayerRepository for PlayerStore<'a> {
    async fn load_player(&self, name: &str) -> Result<()> {
        let sql = "SELECT id, account_id, group_id, level, voc, health, healthmax, experience, lookbody, lookfeet, lookhead, looklegs, looktype, lookaddons, maglevel, mana, manamax, manaspent, soul, town_id, posx, posy, posz, conditions, cap, sex, lastlogin, lastpost, lastlogout, lastip, save, skull, skulltime, lastlogout, blessings, onlinetime, deletion, balance, offlinetraining_time, offlinetraining_skill, stamina, skill_fist, skill_fist_tries, skill_club, skill_club_tries, skill_sword, skill_sword_tries, skill_axe, skill_axe_tries, skill_dist, skill_dist_tries, skill_shielding, skill_shielding_tries, skill_fishing, skill_fishing_tries FROM players WHERE name = ?";
        let _player_row = sqlx::query(sql)
            .bind(name)
            .fetch_optional(self.pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        Ok(())
    }

    async fn save_player(&self, player_id: u32) -> Result<()> {
        let sql = "UPDATE players SET level = ?, health = ? WHERE id = ?";
        sqlx::query(sql)
            .bind(1) // stub
            .bind(100) // stub
            .bind(player_id)
            .execute(self.pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }
}
