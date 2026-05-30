//! Login / gameworld account checks — `IOLoginData::loginserverAuthentication` / `gameworldAuthentication`.
// C++ reference: `src/iologindata.cpp`.

use sqlx::Row;
use tfs_rust_common::error::{Result, TfsRustError};

use crate::password::{hash_bcrypt, needs_upgrade, verify_password, PasswordHashConfig};
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

/// Load account, verify password, upgrade legacy SHA1 to bcrypt on success.
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
        let upgraded = hash_bcrypt(password, hash_cfg.bcrypt_cost)?;
        sqlx::query("UPDATE accounts SET password = ? WHERE id = ?")
            .bind(&upgraded)
            .bind(account_id)
            .execute(pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
    }

    Ok(Some(account_id))
}

/// Returns `None` if account missing or password wrong. `(account_id, characters, premium_ends_at)`.
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

    let row = sqlx::query("SELECT premium_ends_at FROM accounts WHERE id = ?")
        .bind(account_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    let premium_ends_at: i64 = row
        .map(|r| {
            r.try_get::<Option<u32>, _>("premium_ends_at")
                .map_err(|e| TfsRustError::Database(e.to_string()))
                .map(|v| v.map(i64::from).unwrap_or(0))
        })
        .transpose()?
        .unwrap_or(0);

    let chars = load_character_names(pool, account_id).await?;
    let id_u32 = u32::try_from(account_id).map_err(|_| {
        TfsRustError::Database(format!("account id out of range: {account_id}"))
    })?;
    Ok(Some((id_u32, chars, premium_ends_at)))
}

/// Validate account password + that character belongs to account. Returns account id or None.
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
