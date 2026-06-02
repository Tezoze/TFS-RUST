//! `config.lua` loaded via mlua (TFS-style globals).
// C++ reference: `configmanager.cpp` `ConfigManager::load`.

use mlua::{Lua, Value};
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::{protocol_version_from_i64, ProtocolVersion};
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

    /// C++ `ConfigManager::DEPOT_FREE_LIMIT` — `configmanager.cpp` (default 2000).
    pub fn depot_free_limit(&self) -> Result<u32> {
        let v = get_i64_or(self, "depotFreeLimit", 2000)?;
        Ok(v.max(0) as u32)
    }

    /// C++ `ConfigManager::DEPOT_PREMIUM_LIMIT` — `configmanager.cpp` (default 10000).
    pub fn depot_premium_limit(&self) -> Result<u32> {
        let v = get_i64_or(self, "depotPremiumLimit", 10000)?;
        Ok(v.max(0) as u32)
    }

    /// C++ `ConfigManager::RATE_EXPERIENCE` (`rateExp`, default 1.0).
    pub fn rate_experience(&self) -> Result<f64> {
        match self.get_f64("rateExp") {
            Ok(v) => Ok(v.max(0.0)),
            Err(e) if is_missing_config_key(&e) => Ok(1.0),
            Err(e) => Err(e),
        }
    }

    /// C++ `ConfigManager::DEATH_LOSE_PERCENT` (`deathLosePercent`, default -1).
    /// - `-1` => use default formula.
    /// - `0..100` => fixed percentage loss.
    pub fn death_lose_percent(&self) -> Result<i32> {
        let v = get_i64_or(self, "deathLosePercent", -1)? as i32;
        Ok(v)
    }

    /// Whether level-based experience stages are enabled (`expStages`, default false).
    pub fn exp_stages_enabled(&self) -> Result<bool> {
        get_bool_or(self, "expStages", false)
    }

    /// Effective experience rate for `level`.
    ///
    /// - `expStages = false` → returns `rateExp`.
    /// - `expStages = true`  → uses `experienceStages` table when present; falls back to `rateExp`
    ///   if missing/invalid/no-matching-stage.
    pub fn experience_rate_for_level(&self, level: i32) -> Result<f64> {
        let base_rate = self.rate_experience()?;
        if !self.exp_stages_enabled()? {
            return Ok(base_rate);
        }
        let stages = self.parse_experience_stages()?;
        if stages.is_empty() {
            return Ok(base_rate);
        }
        let l = level.max(1);
        for stage in &stages {
            if l >= stage.min_level && stage.max_level.is_none_or(|max| l <= max) {
                return Ok(stage.multiplier.max(0.0));
            }
        }
        Ok(base_rate)
    }

    fn parse_experience_stages(&self) -> Result<Vec<ExperienceStage>> {
        let v: Value = self
            .lua
            .globals()
            .get("experienceStages")
            .map_err(|e| TfsRustError::Config(format!("lua get experienceStages: {e}")))?;
        let Value::Table(stages_tbl) = v else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for stage_val in stages_tbl.sequence_values::<Value>() {
            let stage_val = stage_val
                .map_err(|e| TfsRustError::Config(format!("experienceStages entry: {e}")))?;
            let Value::Table(stage_tbl) = stage_val else {
                continue;
            };
            let min_level = table_i32_required(&stage_tbl, "minlevel")?;
            let max_level = table_i32_optional(&stage_tbl, "maxlevel")?;
            let multiplier = table_f64_required(&stage_tbl, "multiplier")?;
            out.push(ExperienceStage {
                min_level,
                max_level,
                multiplier,
            });
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ExperienceStage {
    min_level: i32,
    max_level: Option<i32>,
    multiplier: f64,
}

fn table_i32_required(table: &mlua::Table, key: &str) -> Result<i32> {
    let v: Value = table
        .get(key)
        .map_err(|e| TfsRustError::Config(format!("experienceStages[{key}]: {e}")))?;
    match v {
        Value::Integer(i) => Ok(i as i32),
        Value::Number(n) => Ok(n as i32),
        Value::Nil => Err(TfsRustError::Config(format!(
            "experienceStages entry missing `{key}`"
        ))),
        _ => Err(TfsRustError::Config(format!(
            "experienceStages `{key}` must be a number"
        ))),
    }
}

fn table_i32_optional(table: &mlua::Table, key: &str) -> Result<Option<i32>> {
    let v: Value = table
        .get(key)
        .map_err(|e| TfsRustError::Config(format!("experienceStages[{key}]: {e}")))?;
    match v {
        Value::Integer(i) => Ok(Some(i as i32)),
        Value::Number(n) => Ok(Some(n as i32)),
        Value::Nil => Ok(None),
        _ => Err(TfsRustError::Config(format!(
            "experienceStages `{key}` must be a number when present"
        ))),
    }
}

fn table_f64_required(table: &mlua::Table, key: &str) -> Result<f64> {
    let v: Value = table
        .get(key)
        .map_err(|e| TfsRustError::Config(format!("experienceStages[{key}]: {e}")))?;
    match v {
        Value::Integer(i) => Ok(i as f64),
        Value::Number(n) => Ok(n),
        Value::Nil => Err(TfsRustError::Config(format!(
            "experienceStages entry missing `{key}`"
        ))),
        _ => Err(TfsRustError::Config(format!(
            "experienceStages `{key}` must be a number"
        ))),
    }
}

/// Monster despawn / walk-back settings — C++ `configmanager.cpp` ~232–251.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonsterWorldConfig {
    /// C++ `deSpawnRadius` / `Monster::despawnRadius` (default 50).
    pub despawn_radius: i32,
    /// C++ `deSpawnRange` / `Monster::despawnRange` Z delta (default 2).
    pub despawn_z_range: i32,
    /// C++ `walkToSpawnRadius` / `DEFAULT_WALKTOSPAWNRADIUS` (default 15; 0 disables).
    pub walk_to_spawn_radius: i32,
    /// C++ `removeOnDespawn` — remove creature vs teleport to spawn (default true).
    pub remove_on_despawn: bool,
}

impl MonsterWorldConfig {
    /// C++ `configmanager.cpp` defaults when keys are absent.
    pub fn defaults() -> Self {
        Self {
            despawn_radius: 50,
            despawn_z_range: 2,
            walk_to_spawn_radius: 15,
            remove_on_despawn: true,
        }
    }

    pub fn from_config(cfg: &ConfigManager) -> Result<Self> {
        Ok(Self {
            despawn_radius: get_i64_or(cfg, "deSpawnRadius", 50)? as i32,
            despawn_z_range: get_i64_or(cfg, "deSpawnRange", 2)? as i32,
            walk_to_spawn_radius: get_i64_or(cfg, "walkToSpawnRadius", 15)? as i32,
            remove_on_despawn: get_bool_or(cfg, "removeOnDespawn", true)?,
        })
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

/// Password hashing settings (`config.lua` `legacySha1Enabled`, `passwordHashCost`).
pub fn password_hash_config_from(cfg: &ConfigManager) -> Result<tfs_rust_db::PasswordHashConfig> {
    let legacy_sha1_enabled = get_bool_or(cfg, "legacySha1Enabled", true)?;
    let bcrypt_cost = get_i64_or(cfg, "passwordHashCost", 12)?;
    if bcrypt_cost < 0 {
        return Err(TfsRustError::Config(
            "passwordHashCost must be >= 0".into(),
        ));
    }
    tfs_rust_db::PasswordHashConfig::new(legacy_sha1_enabled, bcrypt_cost as u32)
}

/// `config.lua` `clientVersion` (default 1098). C++ ref: OTClient/TFS client protocol id.
pub fn protocol_version_from_config(cfg: &ConfigManager) -> Result<ProtocolVersion> {
    let raw = get_i64_or(cfg, "clientVersion", 1098)?;
    protocol_version_from_i64(raw).map_err(TfsRustError::Config)
}

/// `TFS_PROTOCOL_VERSION` env overrides `config.lua` `clientVersion`.
pub fn resolve_protocol_version(cfg: &ConfigManager) -> Result<ProtocolVersion> {
    if let Ok(env) = std::env::var("TFS_PROTOCOL_VERSION") {
        let raw: i64 = env.parse().map_err(|_| {
            TfsRustError::Config(format!("TFS_PROTOCOL_VERSION invalid: {env:?}"))
        })?;
        return protocol_version_from_i64(raw).map_err(TfsRustError::Config);
    }
    protocol_version_from_config(cfg)
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

    #[test]
    fn password_hash_config_defaults_when_keys_missing() {
        let cfg = config_from_lua("");
        let p = password_hash_config_from(&cfg).expect("defaults");
        assert!(p.legacy_sha1_enabled);
        assert_eq!(p.bcrypt_cost, 12);
    }

    #[test]
    fn password_hash_config_reads_keys() {
        let cfg = config_from_lua(
            r#"
            legacySha1Enabled = false
            passwordHashCost = 10
            "#,
        );
        let p = password_hash_config_from(&cfg).expect("password hash config");
        assert!(!p.legacy_sha1_enabled);
        assert_eq!(p.bcrypt_cost, 10);
    }

    #[test]
    fn password_hash_config_rejects_invalid_cost() {
        let cfg = config_from_lua("passwordHashCost = 3");
        let err = password_hash_config_from(&cfg).expect_err("cost too low");
        assert!(matches!(err, TfsRustError::Config(_)));
    }

    #[test]
    fn rate_experience_defaults_to_one() {
        let cfg = config_from_lua("");
        assert_eq!(cfg.rate_experience().expect("rateExp default"), 1.0);
    }

    #[test]
    fn rate_experience_reads_config_value() {
        let cfg = config_from_lua("rateExp = 3.5");
        assert_eq!(cfg.rate_experience().expect("rateExp"), 3.5);
    }

    #[test]
    fn experience_rate_for_level_uses_flat_rate_when_stages_disabled() {
        let cfg = config_from_lua(
            r#"
            rateExp = 5.5
            expStages = false
            experienceStages = {
                { minlevel = 1, maxlevel = 10, multiplier = 100 },
            }
            "#,
        );
        assert_eq!(cfg.experience_rate_for_level(1).expect("rate"), 5.5);
        assert_eq!(cfg.experience_rate_for_level(500).expect("rate"), 5.5);
    }

    #[test]
    fn experience_rate_for_level_uses_stage_table_when_enabled() {
        let cfg = config_from_lua(
            r#"
            rateExp = 5.5
            expStages = true
            experienceStages = {
                { minlevel = 1, maxlevel = 10, multiplier = 100 },
                { minlevel = 11, maxlevel = 20, multiplier = 20 },
                { minlevel = 21, multiplier = 12 },
            }
            "#,
        );
        assert_eq!(cfg.experience_rate_for_level(1).expect("rate"), 100.0);
        assert_eq!(cfg.experience_rate_for_level(15).expect("rate"), 20.0);
        assert_eq!(cfg.experience_rate_for_level(200).expect("rate"), 12.0);
    }

    #[test]
    fn protocol_version_defaults_when_key_missing() {
        let cfg = config_from_lua("");
        let v = protocol_version_from_config(&cfg).expect("default 1098");
        assert_eq!(v, ProtocolVersion::V1098);
    }

    #[test]
    fn protocol_version_reads_772() {
        let cfg = config_from_lua("clientVersion = 772");
        let v = protocol_version_from_config(&cfg).expect("772");
        assert_eq!(v, ProtocolVersion::V772);
    }

    #[test]
    fn protocol_version_rejects_unsupported() {
        let cfg = config_from_lua("clientVersion = 999");
        let err = protocol_version_from_config(&cfg).expect_err("unsupported");
        assert!(matches!(err, TfsRustError::Config(_)));
    }
}
