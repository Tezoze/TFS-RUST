//! Login / gameworld account checks — `IOLoginData::loginserverAuthentication` / `gameworldAuthentication`.
// C++ reference: `src/iologindata.cpp`.

use sqlx::Row;
use tfs_rust_common::error::{Result, TfsRustError};

use crate::password::{hash_bcrypt_async, needs_upgrade, verify_password, PasswordHashConfig};
use crate::pool::DbPool;

async fn load_character_names(pool: &DbPool, account_id: i32) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT name FROM players WHERE account_id = ? AND deletion = 0 ORDER BY name ASC",
    )
    .bind(account_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| TfsRustError::Database(e.to_string()))?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let name: String = r
            .try_get("name")
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        out.push(name);
    }
    Ok(out)
}

/// Load account by `name` (1098 / TFS 1.4.2), verify password, upgrade legacy SHA1 to bcrypt.
async fn authenticate_account_password(
    pool: &DbPool,
    hash_cfg: &PasswordHashConfig,
    account_name: &str,
    password: &str,
) -> Result<Option<i32>> {
    let row = sqlx::query("SELECT id, password FROM accounts WHERE name = ?")
        .bind(account_name)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    verify_loaded_account(pool, hash_cfg, row, password).await
}

/// Load account by numeric `id` (7.72 account number), verify password, upgrade legacy SHA1.
//
// C++ reference: `gameserver/src/iologindata.cpp` `IOLoginData::loginserverAuthentication` /
// `gameworldAuthentication` — both `SELECT ... FROM accounts WHERE id = {accountNumber}`. The 7.72
// "account number" is the `accounts.id` primary key, not a separate column.
async fn authenticate_account_password_by_number(
    pool: &DbPool,
    hash_cfg: &PasswordHashConfig,
    account_number: u32,
    password: &str,
) -> Result<Option<i32>> {
    let account_number = i32::try_from(account_number).map_err(|_| {
        TfsRustError::Database(format!("account number out of range: {account_number}"))
    })?;
    let row = sqlx::query("SELECT id, password FROM accounts WHERE id = ?")
        .bind(account_number)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    verify_loaded_account(pool, hash_cfg, row, password).await
}

/// Shared tail of the name/number auth paths: verify password against the loaded row and
/// transparently re-hash legacy SHA1 stores to bcrypt on success.
async fn verify_loaded_account(
    pool: &DbPool,
    hash_cfg: &PasswordHashConfig,
    row: Option<sqlx::mysql::MySqlRow>,
    password: &str,
) -> Result<Option<i32>> {
    let Some(row) = row else {
        return Ok(None);
    };
    let account_id: i32 = row
        .try_get("id")
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    let stored: String = row
        .try_get("password")
        .map_err(|e| TfsRustError::Database(e.to_string()))?;

    if !verify_password(password, &stored, *hash_cfg).await? {
        return Ok(None);
    }

    if needs_upgrade(&stored) {
        let upgraded = hash_bcrypt_async(password, hash_cfg.bcrypt_cost).await?;
        sqlx::query("UPDATE accounts SET password = ? WHERE id = ?")
            .bind(&upgraded)
            .bind(account_id)
            .execute(pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
    }

    Ok(Some(account_id))
}

/// Load premium expiry + character names for an authenticated account id.
async fn load_account_premium(pool: &DbPool, account_id: i32) -> Result<i64> {
    let row = sqlx::query("SELECT premium_ends_at FROM accounts WHERE id = ?")
        .bind(account_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    Ok(row
        .map(|r| {
            r.try_get::<Option<u32>, _>("premium_ends_at")
                .map_err(|e| TfsRustError::Database(e.to_string()))
                .map(|v| v.map(i64::from).unwrap_or(0))
        })
        .transpose()?
        .unwrap_or(0))
}

/// Build the login-server result tuple `(account_id, characters, premium_ends_at)` for an
/// already-authenticated account id. Shared between the name- and number-keyed entry points.
async fn loginserver_result(
    pool: &DbPool,
    account_id: i32,
) -> Result<(u32, Vec<String>, i64)> {
    let premium_ends_at = load_account_premium(pool, account_id).await?;
    let chars = load_character_names(pool, account_id).await?;
    let id_u32 = u32::try_from(account_id)
        .map_err(|_| TfsRustError::Database(format!("account id out of range: {account_id}")))?;
    Ok((id_u32, chars, premium_ends_at))
}

/// Confirm a character belongs to an authenticated account; returns the account id as `u32`.
async fn gameworld_result(
    pool: &DbPool,
    account_id: i32,
    character_name: &str,
) -> Result<Option<u32>> {
    let pid: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM players WHERE name = ? AND account_id = ? AND deletion = 0 LIMIT 1",
    )
    .bind(character_name)
    .bind(account_id)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| TfsRustError::Database(e.to_string()))?;
    if pid.is_none() {
        return Ok(None);
    }
    u32::try_from(account_id)
        .map(Some)
        .map_err(|_| TfsRustError::Database(format!("account id out of range: {account_id}")))
}

/// Returns `None` if account missing or password wrong. `(account_id, characters, premium_ends_at)`.
///
/// 1098 / TFS 1.4.2 account-**name** login (`src/iologindata.cpp` `loginserverAuthentication`).
pub async fn loginserver_authentication(
    pool: &DbPool,
    hash_cfg: &PasswordHashConfig,
    account_name: &str,
    password: &str,
) -> Result<Option<(u32, Vec<String>, i64)>> {
    let Some(account_id) =
        authenticate_account_password(pool, hash_cfg, account_name, password).await?
    else {
        return Ok(None);
    };
    Ok(Some(loginserver_result(pool, account_id).await?))
}

/// 7.72 account-**number** login.
//
// C++ reference: `gameserver/src/iologindata.cpp` `IOLoginData::loginserverAuthentication(uint32_t
// accountNumber, ...)` — `SELECT id, password, type, premium_ends_at FROM accounts WHERE id = ?`.
pub async fn loginserver_authentication_by_number(
    pool: &DbPool,
    hash_cfg: &PasswordHashConfig,
    account_number: u32,
    password: &str,
) -> Result<Option<(u32, Vec<String>, i64)>> {
    let Some(account_id) =
        authenticate_account_password_by_number(pool, hash_cfg, account_number, password).await?
    else {
        return Ok(None);
    };
    Ok(Some(loginserver_result(pool, account_id).await?))
}

/// Validate account password + that character belongs to account. Returns account id or None.
///
/// 1098 / TFS 1.4.2 account-**name** game login (`src/iologindata.cpp` `gameworldAuthentication`).
pub async fn gameworld_authentication(
    pool: &DbPool,
    hash_cfg: &PasswordHashConfig,
    account_name: &str,
    password: &str,
    character_name: &str,
) -> Result<Option<u32>> {
    let Some(account_id) =
        authenticate_account_password(pool, hash_cfg, account_name, password).await?
    else {
        return Ok(None);
    };
    gameworld_result(pool, account_id, character_name).await
}

/// 7.72 account-**number** game login.
//
// C++ reference: `gameserver/src/iologindata.cpp` `IOLoginData::gameworldAuthentication(uint32_t
// accountNumber, const std::string& password, std::string& characterName)` —
// `SELECT id, password FROM accounts WHERE id = ?` then character-ownership check.
pub async fn gameworld_authentication_by_number(
    pool: &DbPool,
    hash_cfg: &PasswordHashConfig,
    account_number: u32,
    password: &str,
    character_name: &str,
) -> Result<Option<u32>> {
    let Some(account_id) =
        authenticate_account_password_by_number(pool, hash_cfg, account_number, password).await?
    else {
        return Ok(None);
    };
    gameworld_result(pool, account_id, character_name).await
}
