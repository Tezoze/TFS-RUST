//! Unparameterized pack SQL helpers for the TFS Lua `db` / `result` surface.
//! C++ reference: `database.cpp` `storeQuery` / `executeQuery`.

use std::collections::HashMap;

use sqlx::mysql::MySqlRow;
use sqlx::{Column, MySql, QueryBuilder, Row};

use crate::pool::DbPool;

impl DbPool {
    /// `Database::executeQuery` — unparameterized pack SQL.
    ///
    /// sqlx 0.9 `query(&str)` requires `'static` SQL; `QueryBuilder` owns the pack
    /// string then `.build().execute` (same execute path).
    pub async fn lua_execute(&self, sql: &str) -> Result<(), String> {
        let mut conn = self.inner().acquire().await.map_err(|e| e.to_string())?;
        QueryBuilder::<MySql>::new(sql)
            .build()
            .execute(&mut *conn)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// `Database::storeQuery` — fetch all rows as name → nullable string cells.
    pub async fn lua_store_query(
        &self,
        sql: &str,
    ) -> Result<Vec<HashMap<String, Option<String>>>, String> {
        let mut conn = self.inner().acquire().await.map_err(|e| e.to_string())?;
        let rows = QueryBuilder::<MySql>::new(sql)
            .build()
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.iter().map(decode_row).collect())
    }
}

fn decode_row(row: &MySqlRow) -> HashMap<String, Option<String>> {
    let mut map = HashMap::with_capacity(row.len());
    for col in row.columns() {
        let name = col.name();
        map.insert(name.to_string(), decode_cell(row, name));
    }
    map
}

fn decode_cell(row: &MySqlRow, name: &str) -> Option<String> {
    if let Ok(v) = row.try_get::<Option<i64>, _>(name) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<Option<u64>, _>(name) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<Option<i32>, _>(name) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<Option<u32>, _>(name) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<Option<i16>, _>(name) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<Option<u16>, _>(name) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<Option<bool>, _>(name) {
        return v.map(|b| if b { "1".to_string() } else { "0".to_string() });
    }
    if let Ok(v) = row.try_get::<Option<i8>, _>(name) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<Option<u8>, _>(name) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(name) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<Option<String>, _>(name) {
        return v;
    }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(name) {
        return v.map(|b| String::from_utf8_lossy(&b).into_owned());
    }
    None
}
