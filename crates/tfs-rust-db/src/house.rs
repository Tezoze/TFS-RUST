//! House persistence: `tile_store` (house tile item blobs) and `house_lists` (access lists).
// C++ reference: src/iomapserialize.cpp IOMapSerialize::{loadHouseItems,saveHouseItems,loadHouseInfo,saveHouseInfo}

use crate::pool::DbPool;
use sqlx::FromRow;
use tfs_rust_common::error::{Result, TfsRustError};

/// TFS `schema.sql` signed `int` → domain `u32` (same as `PlayerRecord::id`).
fn as_u32(v: i32) -> u32 {
    u32::try_from(v).unwrap_or(0)
}

fn as_i32(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// One row from `tile_store` (`house_id` + serialized tile stack blob in `data`).
/// SQL: `house_id int` (signed).
#[derive(Debug, Clone, FromRow)]
pub struct TileStoreRow {
    pub house_id: i32,
    pub data: Vec<u8>,
}

impl TileStoreRow {
    pub fn new(house_id: u32, data: Vec<u8>) -> Self {
        Self {
            house_id: as_i32(house_id),
            data,
        }
    }

    pub fn house_id_u32(&self) -> u32 {
        as_u32(self.house_id)
    }
}

/// SQL: `house_id int`, `listid int` (signed).
#[derive(Debug, Clone, FromRow)]
pub struct HouseListRow {
    pub house_id: i32,
    pub listid: i32,
    pub list: String,
}

impl HouseListRow {
    pub fn new(house_id: u32, listid: u32, list: String) -> Self {
        Self {
            house_id: as_i32(house_id),
            listid: as_i32(listid),
            list,
        }
    }

    pub fn house_id_u32(&self) -> u32 {
        as_u32(self.house_id)
    }

    pub fn listid_u32(&self) -> u32 {
        as_u32(self.listid)
    }
}

/// Full `houses` row — C++ `IOMapSerialize::loadHouseInfo` + MyAAC bid columns.
/// SQL types match TFS `schema.sql` (`id`/`owner` signed `int`; `paid` unsigned).
#[derive(Debug, Clone, FromRow)]
pub struct HouseOwnerRow {
    pub id: i32,
    pub owner: i32,
    pub paid: u32,
    pub warnings: i32,
    pub name: String,
    pub rent: i32,
    pub town_id: i32,
    pub bid: i32,
    pub bid_end: i32,
    pub last_bid: i32,
    pub highest_bidder: i32,
    pub size: i32,
    pub beds: i32,
}

impl HouseOwnerRow {
    pub fn id_u32(&self) -> u32 {
        as_u32(self.id)
    }

    pub fn owner_u32(&self) -> u32 {
        as_u32(self.owner)
    }
}

/// Values written by `saveHouseInfo` / boot XML sync.
#[derive(Debug, Clone)]
pub struct HouseInfoUpsert {
    pub id: u32,
    pub owner: u32,
    pub paid: u32,
    pub warnings: i32,
    pub name: String,
    pub rent: i32,
    pub town_id: i32,
    pub bid: i32,
    pub bid_end: i32,
    pub last_bid: i32,
    pub highest_bidder: i32,
    pub size: i32,
    pub beds: i32,
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

    /// C++ `IOMapSerialize::loadHouseInfo` — owners from `houses` table.
    pub async fn load_house_owners(&self) -> Result<Vec<HouseOwnerRow>> {
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query_as::<_, HouseOwnerRow>(
                        "SELECT id, owner, paid, warnings, name, rent, town_id, bid, bid_end, last_bid, highest_bidder, size, beds FROM houses",
                    )
                        .fetch_all(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }

    /// C++ `IOLoginData::getGuidByName` — lowercase-insensitive name → player id.
    pub async fn guid_by_name(&self, name: &str) -> Result<Option<u32>> {
        let name = name.to_string();
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let name = name.clone();
                async move {
                    let row: Option<(i32,)> = sqlx::query_as(
                        "SELECT id FROM players WHERE LOWER(name) = ? AND deletion = 0 LIMIT 1",
                    )
                    .bind(&name)
                    .fetch_optional(&pool)
                    .await?;
                    Ok(row.map(|r| as_u32(r.0)))
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }

    /// C++ `IOLoginData::getNameByGuid` — house `ownerName` at load / look text.
    pub async fn name_by_guid(&self, guid: u32) -> Result<Option<String>> {
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    let row: Option<(String,)> = sqlx::query_as(
                        "SELECT name FROM players WHERE id = ? AND deletion = 0 LIMIT 1",
                    )
                    .bind(as_i32(guid))
                    .fetch_optional(&pool)
                    .await?;
                    Ok(row.map(|r| r.0))
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
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

    /// C++ `IOMapSerialize::saveHouseInfo` — upsert `houses` metadata so MyAAC lists stay in sync.
    pub async fn upsert_house_info(&self, rows: &[HouseInfoUpsert]) -> Result<()> {
        for row in rows {
            sqlx::query(
                "INSERT INTO houses (id, owner, paid, warnings, name, rent, town_id, bid, bid_end, last_bid, highest_bidder, size, beds)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON DUPLICATE KEY UPDATE
                   owner = VALUES(owner), paid = VALUES(paid), warnings = VALUES(warnings),
                   name = VALUES(name), rent = VALUES(rent), town_id = VALUES(town_id),
                   bid = VALUES(bid), bid_end = VALUES(bid_end), last_bid = VALUES(last_bid),
                   highest_bidder = VALUES(highest_bidder), size = VALUES(size), beds = VALUES(beds)",
            )
            .bind(as_i32(row.id))
            .bind(as_i32(row.owner))
            .bind(row.paid)
            .bind(row.warnings)
            .bind(&row.name)
            .bind(row.rent)
            .bind(row.town_id)
            .bind(row.bid)
            .bind(row.bid_end)
            .bind(row.last_bid)
            .bind(row.highest_bidder)
            .bind(row.size)
            .bind(row.beds)
            .execute(self.pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Due auctions written by MyAAC (`owner = 0`, `bid_end` elapsed).
    pub async fn settle_auction_candidates(&self, now: u32) -> Result<Vec<HouseOwnerRow>> {
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query_as::<_, HouseOwnerRow>(
                        "SELECT id, owner, paid, warnings, name, rent, town_id, bid, bid_end, last_bid, highest_bidder, size, beds
                         FROM houses
                         WHERE owner = 0 AND highest_bidder != 0 AND bid_end != 0 AND bid_end < ?",
                    )
                    .bind(as_i32(now))
                    .fetch_all(&pool)
                    .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }

    pub async fn update_house_owner_state(
        &self,
        id: u32,
        owner: u32,
        paid: u32,
        warnings: i32,
        bid: i32,
        bid_end: i32,
        last_bid: i32,
        highest_bidder: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE houses SET owner = ?, paid = ?, warnings = ?, bid = ?, bid_end = ?, last_bid = ?, highest_bidder = ? WHERE id = ?",
        )
        .bind(as_i32(owner))
        .bind(paid)
        .bind(warnings)
        .bind(bid)
        .bind(bid_end)
        .bind(last_bid)
        .bind(highest_bidder)
        .bind(as_i32(id))
        .execute(self.pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }
}
