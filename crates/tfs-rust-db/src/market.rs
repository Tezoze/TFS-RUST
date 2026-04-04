use crate::pool::DbPool;
use tfs_rust_common::error::{Result, TfsRustError};

pub struct MarketStore<'a> {
    pool: &'a DbPool,
}

impl<'a> MarketStore<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn fetch_offers(&self) -> Result<()> {
        let sql =
            "SELECT id, player_id, amount, price, created, anonymous, itemtype FROM market_offers";
        let _rows = sqlx::query(sql)
            .fetch_all(self.pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn create_offer(
        &self,
        player_id: u32,
        itemtype: u16,
        amount: u16,
        price: u64,
    ) -> Result<()> {
        let sql = "INSERT INTO market_offers (player_id, itemtype, amount, price, created, anonymous) VALUES (?, ?, ?, ?, ?, ?)";
        sqlx::query(sql)
            .bind(player_id)
            .bind(itemtype)
            .bind(amount)
            .bind(price)
            .bind(0)
            .bind(0)
            .execute(self.pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }
}
