//! `config.lua` loaded via mlua (TFS-style globals).
// C++ reference: `configmanager.cpp` `ConfigManager::load`.

use mlua::{Lua, Value};
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetConfig {
    pub ip: String,
    pub bind_only_global_address: bool,
    pub login_port: u16,
    pub game_port: u16,
    pub status_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbConfig {
    pub host: String,
    pub user: String,
    pub pass: String,
    pub database: String,
    pub port: u16,
    pub sock: String,
}

fn is_missing_config_key(err: &TfsRustError) -> bool {
    match err {
        TfsRustError::Config(msg) => msg.starts_with("missing "),
        _ => false,
    }
}

fn get_string_or(cfg: &ConfigManager, key: &str, default: &str) -> Result<String> {
    match cfg.get_string(key) {
        Ok(v) => Ok(v),
        Err(e) if is_missing_config_key(&e) => Ok(default.to_string()),
        Err(e) => Err(e),
    }
}

fn get_i64_or(cfg: &ConfigManager, key: &str, default: i64) -> Result<i64> {
    match cfg.get_i64(key) {
        Ok(v) => Ok(v),
        Err(e) if is_missing_config_key(&e) => Ok(default),
        Err(e) => Err(e),
    }
}

pub(crate) fn get_bool_or(cfg: &ConfigManager, key: &str, default: bool) -> Result<bool> {
    match cfg.get_bool(key) {
        Ok(v) => Ok(v),
        Err(e) if is_missing_config_key(&e) => Ok(default),
        Err(e) => Err(e),
    }
}

fn checked_port(value: i64, key: &str) -> Result<u16> {
    if !(1..=u16::MAX as i64).contains(&value) {
        return Err(TfsRustError::Config(format!(
            "key `{key}` port out of range: {value}"
        )));
    }
    Ok(value as u16)
}

pub struct ConfigManager {
    lua: Lua,
}

impl ConfigManager {
    pub fn load(path: &Path) -> Result<Self> {
        let lua = Lua::new();
        let chunk = std::fs::read_to_string(path)
            .map_err(|e| TfsRustError::Config(format!("read {}: {e}", path.display())))?;
        lua.load(chunk)
            .set_name(path.display().to_string())
            .exec()
            .map_err(|e| TfsRustError::Lua(format!("execute {}: {e}", path.display())))?;
        info!(file = %path.display(), "loaded config.lua");
        Ok(Self { lua })
    }

    pub fn get_string(&self, key: &str) -> Result<String> {
        let v: Value = self
            .lua
            .globals()
            .get(key)
            .map_err(|e| TfsRustError::Config(format!("lua get {key}: {e}")))?;
        match v {
            Value::String(s) => Ok(s.to_string_lossy()),
            Value::Integer(i) => Ok(i.to_string()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Boolean(b) => Ok(b.to_string()),
            Value::Nil => Err(TfsRustError::Config(format!("missing string key `{key}`"))),
            _ => Err(TfsRustError::Config(format!(
                "key `{key}` is not convertible to string"
            ))),
        }
    }

    pub fn get_f64(&self, key: &str) -> Result<f64> {
        let v: Value = self
            .lua
            .globals()
            .get(key)
            .map_err(|e| TfsRustError::Config(format!("lua get {key}: {e}")))?;
        match v {
            Value::Number(n) => Ok(n),
            Value::Integer(i) => Ok(i as f64),
            Value::Nil => Err(TfsRustError::Config(format!("missing number key `{key}`"))),
            _ => Err(TfsRustError::Config(format!("key `{key}` is not a number"))),
        }
    }

    pub fn get_i64(&self, key: &str) -> Result<i64> {
        let v: Value = self
            .lua
            .globals()
            .get(key)
            .map_err(|e| TfsRustError::Config(format!("lua get {key}: {e}")))?;
        match v {
            Value::Integer(i) => Ok(i),
            Value::Number(n) => Ok(n as i64),
            Value::Nil => Err(TfsRustError::Config(format!("missing integer key `{key}`"))),
            _ => Err(TfsRustError::Config(format!(
                "key `{key}` is not an integer"
            ))),
        }
    }

    pub fn get_bool(&self, key: &str) -> Result<bool> {
        let v: Value = self
            .lua
            .globals()
            .get(key)
            .map_err(|e| TfsRustError::Config(format!("lua get {key}: {e}")))?;
        match v {
            Value::Boolean(b) => Ok(b),
            Value::Nil => Err(TfsRustError::Config(format!("missing bool key `{key}`"))),
            _ => Err(TfsRustError::Config(format!(
                "key `{key}` is not a boolean"
            ))),
        }
    }

    /// Fail fast at startup if any required global is absent.
    pub fn require_keys(&self, keys: &[&str]) -> Result<()> {
        for key in keys {
            let v: Value = self
                .lua
                .globals()
                .get(*key)
                .map_err(|e| TfsRustError::Config(format!("lua get {key}: {e}")))?;
            if v.is_nil() {
                return Err(TfsRustError::Config(format!(
                    "required config key `{key}` is missing"
                )));
            }
        }
        Ok(())
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }
}

impl NetConfig {
    // C++ ref: src/configmanager.cpp:168-194
    pub fn from_config(cfg: &ConfigManager) -> Result<Self> {
        let ip = get_string_or(cfg, "ip", "127.0.0.1")?;
        let bind_only_global_address = get_bool_or(cfg, "bindOnlyGlobalAddress", false)?;
        let login_port = checked_port(get_i64_or(cfg, "loginProtocolPort", 7171)?, "loginProtocolPort")?;
        let game_port = checked_port(get_i64_or(cfg, "gameProtocolPort", 7172)?, "gameProtocolPort")?;
        let status_port = checked_port(get_i64_or(cfg, "statusProtocolPort", 7171)?, "statusProtocolPort")?;

        Ok(Self {
            ip,
            bind_only_global_address,
            login_port,
            game_port,
            status_port,
        })
    }
}

impl DbConfig {
    // C++ ref: src/configmanager.cpp:178-184
    pub fn from_config(cfg: &ConfigManager) -> Result<Self> {
        let host = get_string_or(cfg, "mysqlHost", "127.0.0.1")?;
        let user = get_string_or(cfg, "mysqlUser", "forgottenserver")?;
        let pass = get_string_or(cfg, "mysqlPass", "")?;
        let database = get_string_or(cfg, "mysqlDatabase", "forgottenserver")?;
        let port = checked_port(get_i64_or(cfg, "mysqlPort", 3306)?, "mysqlPort")?;
        let sock = get_string_or(cfg, "mysqlSock", "")?;

        Ok(Self {
            host,
            user,
            pass,
            database,
            port,
            sock,
        })
    }
}

/// SQLx pool size / timeouts. C++ ref: `configmanager.cpp` `mysqlConnectionPool*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MysqlPoolConfig {
    pub min_connections: u32,
    pub max_connections: u32,
    /// Close idle pool connections after this many seconds; `0` = no idle limit (`None` in SQLx).
    pub idle_timeout_secs: u64,
    /// Max seconds to wait for a free connection; `0` = use 30s (safe default for busy pools).
    pub acquire_timeout_secs: u64,
}

impl MysqlPoolConfig {
    pub fn from_config(cfg: &ConfigManager) -> Result<Self> {
        let min = get_i64_or(cfg, "mysqlConnectionPoolSize", 1)?;
        let max = get_i64_or(cfg, "mysqlConnectionMaxPoolSize", 5)?;
        let idle = get_i64_or(cfg, "mysqlConnectionTimeout", 300)?;
        let acquire = get_i64_or(cfg, "mysqlConnectionAcquireTimeout", 10)?;
        if min < 0 {
            return Err(TfsRustError::Config(
                "mysqlConnectionPoolSize must be >= 0".into(),
            ));
        }
        if max < 1 {
            return Err(TfsRustError::Config(
                "mysqlConnectionMaxPoolSize must be >= 1".into(),
            ));
        }
        if max > 10_000 {
            return Err(TfsRustError::Config(
                "mysqlConnectionMaxPoolSize too large (max 10000)".into(),
            ));
        }
        if idle < 0 {
            return Err(TfsRustError::Config(
                "mysqlConnectionTimeout must be >= 0".into(),
            ));
        }
        if acquire < 0 {
            return Err(TfsRustError::Config(
                "mysqlConnectionAcquireTimeout must be >= 0".into(),
            ));
        }
        let min_u = min as u32;
        let max_u = max as u32;
        if min_u > max_u {
            return Err(TfsRustError::Config(
                "mysqlConnectionPoolSize must be <= mysqlConnectionMaxPoolSize".into(),
            ));
        }
        Ok(Self {
            min_connections: min_u,
            max_connections: max_u,
            idle_timeout_secs: idle as u64,
            acquire_timeout_secs: acquire as u64,
        })
    }

    /// Build options for [`tfs_rust_db::DbPool::connect`].
    pub fn to_db_pool_options(&self) -> tfs_rust_db::DbPoolConnectOptions {
        use std::time::Duration;
        tfs_rust_db::DbPoolConnectOptions {
            min_connections: self.min_connections,
            max_connections: self.max_connections,
            idle_timeout: if self.idle_timeout_secs == 0 {
                None
            } else {
                Some(Duration::from_secs(self.idle_timeout_secs))
            },
            acquire_timeout: if self.acquire_timeout_secs == 0 {
                Duration::from_secs(30)
            } else {
                Duration::from_secs(self.acquire_timeout_secs)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_from_lua(lua_source: &str) -> ConfigManager {
        let lua = Lua::new();
        lua.load(lua_source).exec().expect("config chunk should load");
        ConfigManager { lua }
    }

    #[test]
    fn net_config_defaults_when_keys_missing() {
        let cfg = config_from_lua("");
        let net = NetConfig::from_config(&cfg).expect("defaults should be applied");

        assert_eq!(net.ip, "127.0.0.1");
        assert!(!net.bind_only_global_address);
        assert_eq!(net.login_port, 7171);
        assert_eq!(net.game_port, 7172);
        assert_eq!(net.status_port, 7171);
    }

    #[test]
    fn net_config_reads_all_keys() {
        let cfg = config_from_lua(
            r#"
            ip = "10.20.30.40"
            bindOnlyGlobalAddress = true
            loginProtocolPort = 9001
            gameProtocolPort = 9002
            statusProtocolPort = 9003
            "#,
        );
        let net = NetConfig::from_config(&cfg).expect("net config should parse");

        assert_eq!(net.ip, "10.20.30.40");
        assert!(net.bind_only_global_address);
        assert_eq!(net.login_port, 9001);
        assert_eq!(net.game_port, 9002);
        assert_eq!(net.status_port, 9003);
    }

    #[test]
    fn net_config_rejects_out_of_range_port_values() {
        let cfg_zero = config_from_lua("loginProtocolPort = 0");
        let err = NetConfig::from_config(&cfg_zero).expect_err("port 0 must fail");
        assert!(matches!(err, TfsRustError::Config(_)));

        let cfg_too_high = config_from_lua("gameProtocolPort = 70000");
        let err = NetConfig::from_config(&cfg_too_high).expect_err("port > 65535 must fail");
        assert!(matches!(err, TfsRustError::Config(_)));
    }

    #[test]
    fn db_config_defaults_when_keys_missing() {
        let cfg = config_from_lua("");
        let db = DbConfig::from_config(&cfg).expect("defaults should be applied");

        assert_eq!(db.host, "127.0.0.1");
        assert_eq!(db.user, "forgottenserver");
        assert_eq!(db.pass, "");
        assert_eq!(db.database, "forgottenserver");
        assert_eq!(db.port, 3306);
        assert_eq!(db.sock, "");
    }

    #[test]
    fn db_config_reads_all_keys() {
        let cfg = config_from_lua(
            r#"
            mysqlHost = "db.internal"
            mysqlUser = "otuser"
            mysqlPass = "secret"
            mysqlDatabase = "otdb"
            mysqlPort = 43306
            mysqlSock = "/tmp/mysql.sock"
            "#,
        );
        let db = DbConfig::from_config(&cfg).expect("db config should parse");

        assert_eq!(db.host, "db.internal");
        assert_eq!(db.user, "otuser");
        assert_eq!(db.pass, "secret");
        assert_eq!(db.database, "otdb");
        assert_eq!(db.port, 43306);
        assert_eq!(db.sock, "/tmp/mysql.sock");
    }

    #[test]
    fn db_config_rejects_out_of_range_port_values() {
        let cfg_zero = config_from_lua("mysqlPort = 0");
        let err = DbConfig::from_config(&cfg_zero).expect_err("port 0 must fail");
        assert!(matches!(err, TfsRustError::Config(_)));

        let cfg_too_high = config_from_lua("mysqlPort = 70000");
        let err = DbConfig::from_config(&cfg_too_high).expect_err("port > 65535 must fail");
        assert!(matches!(err, TfsRustError::Config(_)));
    }

    #[test]
    fn mysql_pool_config_defaults_when_keys_missing() {
        let cfg = config_from_lua("");
        let p = MysqlPoolConfig::from_config(&cfg).expect("pool defaults");
        assert_eq!(p.min_connections, 1);
        assert_eq!(p.max_connections, 5);
        assert_eq!(p.idle_timeout_secs, 300);
        assert_eq!(p.acquire_timeout_secs, 10);
    }

    #[test]
    fn mysql_pool_config_reads_keys() {
        let cfg = config_from_lua(
            r#"
            mysqlConnectionPoolSize = 3
            mysqlConnectionMaxPoolSize = 15
            mysqlConnectionTimeout = 120
            mysqlConnectionAcquireTimeout = 5
            "#,
        );
        let p = MysqlPoolConfig::from_config(&cfg).expect("pool");
        assert_eq!(p.min_connections, 3);
        assert_eq!(p.max_connections, 15);
        assert_eq!(p.idle_timeout_secs, 120);
        assert_eq!(p.acquire_timeout_secs, 5);
    }

    #[test]
    fn mysql_pool_config_rejects_min_greater_than_max() {
        let cfg = config_from_lua(
            r#"
            mysqlConnectionPoolSize = 10
            mysqlConnectionMaxPoolSize = 2
            "#,
        );
        let err = MysqlPoolConfig::from_config(&cfg).expect_err("min > max");
        assert!(matches!(err, TfsRustError::Config(_)));
    }
}
