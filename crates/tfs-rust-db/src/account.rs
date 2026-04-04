//! Login / gameworld account checks — `IOLoginData::loginserverAuthentication` / `gameworldAuthentication`.
// C++ reference: `src/iologindata.cpp`.

use sha1::{Digest, Sha1};
use sqlx::Row;
use tfs_rust_common::error::{Result, TfsRustError};

use crate::pool::DbPool;

/// Lowercase hex SHA1 (40 chars), matching `transformToSHA1` in `src/tools.cpp`.
pub fn sha1_password_hex(password: &str) -> String {
    let mut h = Sha1::new();
    h.update(password.as_bytes());
    let r = h.finalize();
    let mut s = String::with_capacity(40);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for b in r {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

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

/// Returns `None` if account missing or password wrong. `(account_id, characters, premium_ends_at)`.
pub async fn loginserver_authentication(
    pool: &DbPool,
    account_name: &str,
    password: &str,
) -> Result<Option<(u32, Vec<String>, i64)>> {
    let row = sqlx::query("SELECT id, password, premium_ends_at FROM accounts WHERE name = ?")
        .bind(account_name)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    let Some(row) = row else {
        return Ok(None);
    };
    // TFS `accounts.id` is typically signed `INT`, not `INT UNSIGNED`.
    let id: i32 = row
        .try_get("id")
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    let db_pass: String = row
        .try_get("password")
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    // TFS `accounts.premium_ends_at` is usually `INT UNSIGNED` (unix time), not `BIGINT`.
    let premium_ends_at: i64 = row
        .try_get::<Option<u32>, _>("premium_ends_at")
        .map_err(|e| TfsRustError::Database(e.to_string()))?
        .map(i64::from)
        .unwrap_or(0);

    if sha1_password_hex(password) != db_pass {
        return Ok(None);
    }

    let chars = load_character_names(pool, id).await?;
    let id_u32 = u32::try_from(id).map_err(|_| {
        TfsRustError::Database(format!("account id out of range: {id}"))
    })?;
    Ok(Some((id_u32, chars, premium_ends_at)))
}

/// Validate account password + that character belongs to account. Returns account id or None.
pub async fn gameworld_authentication(
    pool: &DbPool,
    account_name: &str,
    password: &str,
    character_name: &str,
) -> Result<Option<u32>> {
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
    let db_pass: String = row
        .try_get("password")
        .map_err(|e| TfsRustError::Database(e.to_string()))?;
    if sha1_password_hex(password) != db_pass {
        return Ok(None);
    }

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
