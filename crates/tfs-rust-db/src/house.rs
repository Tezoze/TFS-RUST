//! House persistence: `tile_store` (house tile item blobs) and `house_lists` (access lists).
// C++ reference: src/iomapserialize.cpp IOMapSerialize::{loadHouseItems,saveHouseItems,loadHouseInfo,saveHouseInfo}

use crate::pool::DbPool;
use sqlx::FromRow;
use tfs_rust_common::error::{Result, TfsRustError};

/// One row from `tile_store` (`house_id` + serialized tile stack blob in `data`).
#[derive(Debug, Clone, FromRow)]
pub struct TileStoreRow {
    pub house_id: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, FromRow)]
pub struct HouseListRow {
    pub house_id: u32,
    pub listid: u32,
    pub list: String,
}

pub struct HouseStore<'a> {
    pool: &'a DbPool,
}

impl<'a> HouseStore<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// C++ `IOMapSerialize::loadHouseItems` uses `SELECT data FROM tile_store` (blobs only).
    /// We also expose `house_id` for multi-row saves matching `saveHouseItems`.
    pub async fn load_tile_store(&self) -> Result<Vec<TileStoreRow>> {
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query_as::<_, TileStoreRow>("SELECT house_id, data FROM tile_store")
                        .fetch_all(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }

    /// Matches `IOMapSerialize::saveHouseItems`: delete all tile rows, then insert fresh blobs.
    pub async fn replace_all_tile_store(&self, rows: &[TileStoreRow]) -> Result<()> {
        let mut tx = self
            .pool
            .inner()
            .begin()
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        sqlx::query("DELETE FROM tile_store")
            .execute(&mut *tx)
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        for row in rows {
            sqlx::query("INSERT INTO tile_store (house_id, data) VALUES (?, ?)")
                .bind(row.house_id)
                .bind(&row.data)
                .execute(&mut *tx)
                .await
                .map_err(|e| TfsRustError::Database(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    /// C++ `IOMapSerialize::loadHouseInfo` — guest/subowner/door lists (`list` column, not `data`).
    pub async fn load_house_lists(&self) -> Result<Vec<HouseListRow>> {
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query_as::<_, HouseListRow>(
                        "SELECT house_id, listid, list FROM house_lists",
                    )
                    .fetch_all(&pool)
                    .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }

    /// C++ `IOMapSerialize::saveHouseInfo` — replace all access list rows.
    pub async fn replace_all_house_lists(&self, rows: &[HouseListRow]) -> Result<()> {
        let mut tx = self
            .pool
            .inner()
            .begin()
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        sqlx::query("DELETE FROM house_lists")
            .execute(&mut *tx)
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        for row in rows {
            sqlx::query("INSERT INTO house_lists (house_id, listid, list) VALUES (?, ?, ?)")
                .bind(row.house_id)
                .bind(row.listid)
                .bind(&row.list)
                .execute(&mut *tx)
                .await
                .map_err(|e| TfsRustError::Database(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }
}
