//! House persistence: `tile_store` (house tile item blobs) and `house_lists` (access lists).
// C++ reference: src/iomapserialize.cpp IOMapSerialize::{loadHouseItems,saveHouseItems,loadHouseInfo,saveHouseInfo}

use std::collections::HashMap;

use crate::pool::DbPool;
use sqlx::{FromRow, MySql, QueryBuilder};
use tfs_rust_common::error::{Result, TfsRustError};

const UPSERT_CHUNK: usize = 50;
const TILE_STORE_INSERT_CHUNK: usize = 32;
const NAME_IN_CHUNK: usize = 200;

/// TFS `schema.sql` signed `int` → domain `u32` (same as `PlayerRecord::id`).
fn as_u32(v: i32) -> u32 {
    u32::try_from(v).unwrap_or(0)
}

fn as_i32(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// sqlx `push_tuples` emits ` ((?), (?)) ` for `IN` lists. `INSERT … VALUES` needs
/// `(?), (?)` — the extra wrap is MariaDB 1241 ("Operand should contain 1 column(s)").
fn push_insert_values<T, F>(qb: &mut QueryBuilder<MySql>, rows: &[T], mut bind_row: F)
where
    F: FnMut(sqlx::query_builder::Separated<'_, MySql, &'static str>, &T),
{
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        qb.push("(");
        bind_row(qb.separated(", "), row);
        qb.push(")");
    }
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
    /// JOIN `players.name` for `owner` (boot load; auction SELECT uses NULL).
    pub owner_player_name: Option<String>,
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

        for chunk in rows.chunks(TILE_STORE_INSERT_CHUNK) {
            let mut qb =
                QueryBuilder::<MySql>::new("INSERT INTO tile_store (house_id, data) VALUES ");
            push_insert_values(&mut qb, chunk, |mut b, row| {
                b.push_bind(row.house_id).push_bind(&row.data);
            });
            qb.build()
                .execute(&mut *tx)
                .await
                .map_err(|e| TfsRustError::Database(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    /// C++ `IOMapSerialize::loadHouseInfo` — owners from `houses` table + player names.
    pub async fn load_house_owners(&self) -> Result<Vec<HouseOwnerRow>> {
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query_as::<_, HouseOwnerRow>(
                        "SELECT h.id, h.owner, h.paid, h.warnings, h.name, h.rent, h.town_id, \
                         h.bid, h.bid_end, h.last_bid, h.highest_bidder, h.size, h.beds, \
                         p.name AS owner_player_name \
                         FROM houses h \
                         LEFT JOIN players p ON p.id = h.owner AND p.deletion = 0",
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

    /// Bulk `getGuidByName` for access-list parse (lowercase name keys).
    pub async fn guid_by_names(&self, names: &[String]) -> Result<HashMap<String, u32>> {
        let mut out = HashMap::new();
        if names.is_empty() {
            return Ok(out);
        }
        for chunk in names.chunks(NAME_IN_CHUNK) {
            let chunk = chunk.to_vec();
            let rows: Vec<(i32, String)> = self
                .pool
                .execute_with_retry(|| {
                    let pool = self.pool.inner().clone();
                    let chunk = chunk.clone();
                    async move {
                        let mut qb = QueryBuilder::<MySql>::new(
                            "SELECT id, name FROM players WHERE deletion = 0 AND LOWER(name) IN (",
                        );
                        let mut sep = qb.separated(", ");
                        for n in &chunk {
                            sep.push_bind(n);
                        }
                        qb.push(")");
                        qb.build_query_as().fetch_all(&pool).await
                    }
                })
                .await
                .map_err(|e| TfsRustError::Database(e.to_string()))?;
            for (id, name) in rows {
                out.insert(name.to_ascii_lowercase(), as_u32(id));
            }
        }
        Ok(out)
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
        if rows.is_empty() {
            return Ok(());
        }
        for chunk in rows.chunks(UPSERT_CHUNK) {
            let chunk = chunk.to_vec();
            self.pool
                .execute_with_retry(|| {
                    let pool = self.pool.inner().clone();
                    let chunk = chunk.clone();
                    async move {
                        let mut qb = QueryBuilder::<MySql>::new(
                            "INSERT INTO houses (id, owner, paid, warnings, name, rent, town_id, \
                             bid, bid_end, last_bid, highest_bidder, size, beds) VALUES ",
                        );
                        push_insert_values(&mut qb, &chunk, |mut b, row| {
                            b.push_bind(as_i32(row.id))
                                .push_bind(as_i32(row.owner))
                                .push_bind(row.paid)
                                .push_bind(row.warnings)
                                .push_bind(&row.name)
                                .push_bind(row.rent)
                                .push_bind(row.town_id)
                                .push_bind(row.bid)
                                .push_bind(row.bid_end)
                                .push_bind(row.last_bid)
                                .push_bind(row.highest_bidder)
                                .push_bind(row.size)
                                .push_bind(row.beds);
                        });
                        qb.push(
                            " ON DUPLICATE KEY UPDATE \
                             owner = VALUES(owner), paid = VALUES(paid), warnings = VALUES(warnings), \
                             name = VALUES(name), rent = VALUES(rent), town_id = VALUES(town_id), \
                             bid = VALUES(bid), bid_end = VALUES(bid_end), last_bid = VALUES(last_bid), \
                             highest_bidder = VALUES(highest_bidder), size = VALUES(size), beds = VALUES(beds)",
                        );
                        qb.build().execute(&pool).await
                    }
                })
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
                        "SELECT id, owner, paid, warnings, name, rent, town_id, bid, bid_end, last_bid, highest_bidder, size, beds, \
                         CAST(NULL AS CHAR) AS owner_player_name \
                         FROM houses \
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
