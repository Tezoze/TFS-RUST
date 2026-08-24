//! Boot-time DB ops that TFS ran from `data/globalevents/scripts/startup.lua`.
//!
//! Pack surface: TFS `globalevent.cpp` `GLOBALEVENT_STARTUP` → Lua `onStartup`.
//! House auctions (`bid_end` / bank pay) are later-era and are **not** ported here.
// C++ reference: TFS `data/globalevents/scripts/startup.lua` (ops, not 772 combat).

use sqlx::Row;

use crate::pool::DbPool;
use tfs_rust_common::error::{Result, TfsRustError};

/// Town row written into `towns` from in-memory OTBM data.
#[derive(Debug, Clone)]
pub struct TownInsert {
    pub id: u32,
    pub name: String,
    pub posx: i32,
    pub posy: i32,
    pub posz: i32,
}

/// Truncate the MEMORY `players_online` table (stale rows from a crash).
pub async fn truncate_players_online(pool: &DbPool) -> Result<()> {
    sqlx::query("TRUNCATE TABLE `players_online`")
        .execute(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    Ok(())
}

/// Insert a logged-in player into `players_online`.
pub async fn insert_player_online(pool: &DbPool, player_id: u32) -> Result<()> {
    sqlx::query("INSERT IGNORE INTO `players_online` (`player_id`) VALUES (?)")
        .bind(player_id as i32)
        .execute(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    Ok(())
}

/// Remove a player from `players_online` on logout.
pub async fn delete_player_online(pool: &DbPool, player_id: u32) -> Result<()> {
    sqlx::query("DELETE FROM `players_online` WHERE `player_id` = ?")
        .bind(player_id as i32)
        .execute(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    Ok(())
}

/// Expire pending guild wars, deleted characters, IP bans, and account bans.
///
/// Does **not** settle house auctions.
pub async fn expire_ops_rows(pool: &DbPool, now_unix: i64) -> Result<()> {
    sqlx::query("DELETE FROM `guild_wars` WHERE `status` = 0")
        .execute(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;

    sqlx::query("DELETE FROM `players` WHERE `deletion` != 0 AND `deletion` < ?")
        .bind(now_unix)
        .execute(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;

    sqlx::query("DELETE FROM `ip_bans` WHERE `expires_at` != 0 AND `expires_at` <= ?")
        .bind(now_unix)
        .execute(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;

    let expired = sqlx::query(
        "SELECT `account_id`, `reason`, `banned_at`, `expires_at`, `banned_by` \
         FROM `account_bans` WHERE `expires_at` != 0 AND `expires_at` <= ?",
    )
    .bind(now_unix)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| TfsRustError::Database(e.to_string()))?;

    for row in expired {
        let account_id: i32 = row.try_get("account_id").unwrap_or(0);
        let reason: String = row.try_get("reason").unwrap_or_default();
        let banned_at: i64 = row.try_get("banned_at").unwrap_or(0);
        let expires_at: i64 = row.try_get("expires_at").unwrap_or(0);
        let banned_by: i32 = row.try_get("banned_by").unwrap_or(0);

        sqlx::query(
            "INSERT INTO `account_ban_history` \
             (`account_id`, `reason`, `banned_at`, `expired_at`, `banned_by`) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(account_id)
        .bind(&reason)
        .bind(banned_at)
        .bind(expires_at)
        .bind(banned_by)
        .execute(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;

        sqlx::query("DELETE FROM `account_bans` WHERE `account_id` = ?")
            .bind(account_id)
            .execute(pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
    }
    Ok(())
}

/// Replace `towns` with the OTBM town list (website / tools read this table).
pub async fn replace_towns(pool: &DbPool, towns: &[TownInsert]) -> Result<()> {
    sqlx::query("TRUNCATE TABLE `towns`")
        .execute(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    for town in towns {
        sqlx::query(
            "INSERT INTO `towns` (`id`, `name`, `posx`, `posy`, `posz`) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(town.id as i32)
        .bind(&town.name)
        .bind(town.posx)
        .bind(town.posy)
        .bind(town.posz)
        .execute(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    }
    Ok(())
}

/// `server_config.players_record` — TFS `Game::loadPlayersRecord`.
pub async fn load_players_record(pool: &DbPool) -> Result<u32> {
    let row = sqlx::query("SELECT `value` FROM `server_config` WHERE `config` = 'players_record'")
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    let Some(row) = row else {
        return Ok(0);
    };
    let value: String = row.try_get("value").unwrap_or_default();
    Ok(value.parse().unwrap_or(0))
}

/// Persist a new players-online record.
pub async fn save_players_record(pool: &DbPool, record: u32) -> Result<()> {
    sqlx::query(
        "INSERT INTO `server_config` (`config`, `value`) VALUES ('players_record', ?) \
         ON DUPLICATE KEY UPDATE `value` = VALUES(`value`)",
    )
    .bind(record.to_string())
    .execute(pool.inner())
    .await
    .map_err(|e| TfsRustError::Database(e.to_string()))?;
    Ok(())
}

/// Full startup ops minus house auctions.
pub async fn run_startup_ops(pool: &DbPool, towns: &[TownInsert], now_unix: i64) -> Result<()> {
    truncate_players_online(pool).await?;
    expire_ops_rows(pool, now_unix).await?;
    replace_towns(pool, towns).await?;
    Ok(())
}
