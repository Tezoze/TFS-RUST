use crate::pool::DbPool;
use tfs_rust_common::error::{Result, TfsRustError};

pub struct HouseStore<'a> {
    pool: &'a DbPool,
}

impl<'a> HouseStore<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn load_house_items(&self, house_id: u32) -> Result<()> {
        let sql = "SELECT data FROM house_lists WHERE house_id = ? AND listid = 0";
        let _row = sqlx::query(sql)
            .bind(house_id)
            .fetch_optional(self.pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn save_house_items(&self, house_id: u32, data: &[u8]) -> Result<()> {
        let sql = "REPLACE INTO house_lists (house_id, listid, data) VALUES (?, 0, ?)";
        sqlx::query(sql)
            .bind(house_id)
            .bind(data)
            .execute(self.pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }
}
