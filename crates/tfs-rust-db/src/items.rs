//! Item rows for `player_items`, `player_depotitems`, `player_inboxitems`, `player_storeinboxitems`.
// C++ reference: src/iologindata.cpp IOLoginData::loadItems / saveItems

use crate::pool::DbPool;
use sqlx::MySql;
use sqlx::Transaction;
use tfs_rust_common::error::{Result, TfsRustError};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ItemRecord {
    pub pid: i32,
    pub sid: i32,
    pub itemtype: u16,
    pub count: i16,
    pub attributes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTable {
    /// Equipment and inventory (`player_items`).
    Inventory,
    /// Depot chests (`player_depotitems`).
    Depot,
    /// Inbox (`player_inboxitems`).
    Inbox,
    /// Store inbox (`player_storeinboxitems`).
    StoreInbox,
}

impl ItemTable {
    fn delete_sql(self) -> &'static str {
        match self {
            ItemTable::Inventory => "DELETE FROM player_items WHERE player_id = ?",
            ItemTable::Depot => "DELETE FROM player_depotitems WHERE player_id = ?",
            ItemTable::Inbox => "DELETE FROM player_inboxitems WHERE player_id = ?",
            ItemTable::StoreInbox => "DELETE FROM player_storeinboxitems WHERE player_id = ?",
        }
    }

    fn insert_sql(self) -> &'static str {
        match self {
            ItemTable::Inventory => {
                "INSERT INTO player_items (player_id, pid, sid, itemtype, count, attributes) VALUES (?, ?, ?, ?, ?, ?)"
            }
            ItemTable::Depot => {
                "INSERT INTO player_depotitems (player_id, pid, sid, itemtype, count, attributes) VALUES (?, ?, ?, ?, ?, ?)"
            }
            ItemTable::Inbox => {
                "INSERT INTO player_inboxitems (player_id, pid, sid, itemtype, count, attributes) VALUES (?, ?, ?, ?, ?, ?)"
            }
            ItemTable::StoreInbox => {
                "INSERT INTO player_storeinboxitems (player_id, pid, sid, itemtype, count, attributes) VALUES (?, ?, ?, ?, ?, ?)"
            }
        }
    }
}

pub struct ItemStore<'a> {
    pool: &'a DbPool,
}

impl<'a> ItemStore<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// C++ loads with `ORDER BY sid DESC` for tree reconstruction; order is irrelevant when
    /// rebuilding sid-keyed maps.
    pub async fn load_items(&self, player_id: i32, table: ItemTable) -> Result<Vec<ItemRecord>> {
        const Q_INV: &str = "SELECT pid, sid, itemtype, count, attributes FROM player_items WHERE player_id = ? ORDER BY sid ASC";
        const Q_DEPOT: &str = "SELECT pid, sid, itemtype, count, attributes FROM player_depotitems WHERE player_id = ? ORDER BY sid ASC";
        const Q_INBOX: &str = "SELECT pid, sid, itemtype, count, attributes FROM player_inboxitems WHERE player_id = ? ORDER BY sid ASC";
        const Q_STORE: &str = "SELECT pid, sid, itemtype, count, attributes FROM player_storeinboxitems WHERE player_id = ? ORDER BY sid ASC";
        let sql = match table {
            ItemTable::Inventory => Q_INV,
            ItemTable::Depot => Q_DEPOT,
            ItemTable::Inbox => Q_INBOX,
            ItemTable::StoreInbox => Q_STORE,
        };
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query_as::<_, ItemRecord>(sql)
                        .bind(player_id)
                        .fetch_all(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }

    pub async fn save_items(
        &self,
        player_id: i32,
        table: ItemTable,
        items: &[ItemRecord],
    ) -> Result<()> {
        let mut tx = self
            .pool
            .inner()
            .begin()
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Self::save_items_tx(&mut tx, player_id, table, items).await?;
        tx.commit()
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    /// Save items inside an existing transaction (used by `PlayerStore::save_player`).
    pub async fn save_items_tx(
        tx: &mut Transaction<'_, MySql>,
        player_id: i32,
        table: ItemTable,
        items: &[ItemRecord],
    ) -> Result<()> {
        sqlx::query(table.delete_sql())
            .bind(player_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        let ins = table.insert_sql();
        for item in items {
            sqlx::query(ins)
                .bind(player_id)
                .bind(item.pid)
                .bind(item.sid)
                .bind(item.itemtype)
                .bind(item.count)
                .bind(&item.attributes)
                .execute(&mut **tx)
                .await
                .map_err(|e| TfsRustError::Database(e.to_string()))?;
        }
        Ok(())
    }
}
