//! Account password hashing and verification (bcrypt default, legacy SHA1 upgrade on login).
//!
//! C++ reference: `src/iologindata.cpp` `loginserverAuthentication` / `gameworldAuthentication`,
//! `src/tools.cpp` `transformToSHA1`.
//! Design reference: otland/forgottenserver#2148 (self-describing stored hashes).

use sha1::{Digest, Sha1};
use tfs_rust_common::error::{Result, TfsRustError};

/// bcrypt work factor and legacy SHA1 toggle (`config.lua`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasswordHashConfig {
    pub legacy_sha1_enabled: bool,
    pub bcrypt_cost: u32,
}

impl Default for PasswordHashConfig {
    fn default() -> Self {
        Self {
            legacy_sha1_enabled: true,
            bcrypt_cost: 12,
        }
    }
}

impl PasswordHashConfig {
    pub fn new(legacy_sha1_enabled: bool, bcrypt_cost: u32) -> Result<Self> {
        if !(4..=31).contains(&bcrypt_cost) {
            return Err(TfsRustError::Config(format!(
                "passwordHashCost must be between 4 and 31, got {bcrypt_cost}"
            )));
        }
        Ok(Self {
            legacy_sha1_enabled,
            bcrypt_cost,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredPasswordFormat {
    Sha1Legacy,
    Bcrypt,
    Unknown,
}

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

fn is_sha1_hex(stored: &str) -> bool {
    stored.len() == 40
        && stored
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn is_bcrypt(stored: &str) -> bool {
    stored.starts_with("$2a$") || stored.starts_with("$2b$") || stored.starts_with("$2y$")
}

pub fn detect_format(stored: &str) -> StoredPasswordFormat {
    if is_sha1_hex(stored) {
        StoredPasswordFormat::Sha1Legacy
    } else if is_bcrypt(stored) {
        StoredPasswordFormat::Bcrypt
    } else {
        StoredPasswordFormat::Unknown
    }
}

pub fn needs_upgrade(stored: &str) -> bool {
    detect_format(stored) == StoredPasswordFormat::Sha1Legacy
}

/// Hash a plaintext password with bcrypt (`$2b$…`, PHP `password_verify` compatible).
pub fn hash_bcrypt(plaintext: &str, cost: u32) -> Result<String> {
    bcrypt::hash(plaintext, cost).map_err(|e| TfsRustError::Database(format!("bcrypt hash: {e}")))
}

/// Async bcrypt hash — runs on the blocking thread pool (cost 12 ≈ hundreds of ms).
pub async fn hash_bcrypt_async(plaintext: &str, cost: u32) -> Result<String> {
    let plaintext = plaintext.to_owned();
    tokio::task::spawn_blocking(move || hash_bcrypt(&plaintext, cost))
        .await
        .map_err(|e| TfsRustError::Database(format!("password hash task: {e}")))?
}

/// Synchronous verify — run on `spawn_blocking` when bcrypt is involved.
pub fn verify_password_sync(plaintext: &str, stored: &str, cfg: &PasswordHashConfig) -> bool {
    match detect_format(stored) {
        StoredPasswordFormat::Sha1Legacy => {
            if !cfg.legacy_sha1_enabled {
                return false;
            }
            sha1_password_hex(plaintext) == stored
        }
        StoredPasswordFormat::Bcrypt => bcrypt::verify(plaintext, stored).unwrap_or_default(),
        StoredPasswordFormat::Unknown => false,
    }
}

/// Async verify — bcrypt runs on the blocking thread pool.
pub async fn verify_password(
    plaintext: &str,
    stored: &str,
    cfg: PasswordHashConfig,
) -> Result<bool> {
    let plaintext = plaintext.to_owned();
    let stored = stored.to_owned();
    tokio::task::spawn_blocking(move || verify_password_sync(&plaintext, &stored, &cfg))
        .await
        .map_err(|e| TfsRustError::Database(format!("password verify task: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cfg() -> PasswordHashConfig {
        PasswordHashConfig::default()
    }

    #[test]
    fn sha1_password_hex_matches_seed_script() {
        assert_eq!(
            sha1_password_hex("1"),
            "356a192b7913b04c54574d18c28d46e6395428ab"
        );
    }

    #[test]
    fn detect_format_classifies_hashes() {
        assert_eq!(
            detect_format("356a192b7913b04c54574d18c28d46e6395428ab"),
            StoredPasswordFormat::Sha1Legacy
        );
        assert_eq!(
            detect_format("$2b$04$abcdefghijklmnopqrstuuOQJLWEqGqF6eJ5Z6Q5Q5Q5Q5Q5Q5Q5"),
            StoredPasswordFormat::Bcrypt
        );
        assert_eq!(detect_format("not-a-hash"), StoredPasswordFormat::Unknown);
        assert_eq!(
            detect_format("356A192B7913B04C54574D18C28D46E6395428AB"),
            StoredPasswordFormat::Unknown
        );
    }

    #[test]
    fn verify_password_sync_sha1_match_and_mismatch() {
        let cfg = test_cfg();
        let stored = sha1_password_hex("secret");
        assert!(verify_password_sync("secret", &stored, &cfg));
        assert!(!verify_password_sync("wrong", &stored, &cfg));
    }

    #[test]
    fn verify_password_sync_rejects_sha1_when_legacy_disabled() {
        let cfg = PasswordHashConfig {
            legacy_sha1_enabled: false,
            bcrypt_cost: 12,
        };
        let stored = sha1_password_hex("secret");
        assert!(!verify_password_sync("secret", &stored, &cfg));
    }

    #[test]
    fn hash_bcrypt_round_trip() {
        let cfg = PasswordHashConfig::new(true, 4).expect("valid cost");
        let stored = hash_bcrypt("hunter2", cfg.bcrypt_cost).expect("hash");
        assert!(stored.starts_with("$2"));
        assert!(verify_password_sync("hunter2", &stored, &cfg));
        assert!(!verify_password_sync("wrong", &stored, &cfg));
    }

    #[test]
    fn needs_upgrade_only_for_sha1() {
        assert!(needs_upgrade("356a192b7913b04c54574d18c28d46e6395428ab"));
        let stored = hash_bcrypt("x", 4).expect("hash");
        assert!(!needs_upgrade(&stored));
    }

    #[test]
    fn password_hash_config_rejects_invalid_cost() {
        assert!(PasswordHashConfig::new(true, 3).is_err());
        assert!(PasswordHashConfig::new(true, 32).is_err());
    }
}
