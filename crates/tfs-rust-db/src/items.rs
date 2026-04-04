use crate::pool::DbPool;
use tfs_rust_common::error::{Result, TfsRustError};

pub struct ItemStore<'a> {
    pool: &'a DbPool,
}

impl<'a> ItemStore<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn load_items(&self, player_id: u32) -> Result<()> {
        let sql = "SELECT pid, sid, itemtype, count, attributes FROM player_items WHERE player_id = ? ORDER BY sid ASC";
        let _rows = sqlx::query(sql)
            .bind(player_id)
            .fetch_all(self.pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn save_items(&self, player_id: u32) -> Result<()> {
        let sql_delete = "DELETE FROM player_items WHERE player_id = ?";
        sqlx::query(sql_delete)
            .bind(player_id)
            .execute(self.pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        Ok(())
    }
}
