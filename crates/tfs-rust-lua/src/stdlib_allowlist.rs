//! Game-VM stdlib allowlist (VM hardening pillar 1).
//!
//! `tasks/tools-actions/vm-hardening.md`. mlua `Lua::new()` loads `ALL_SAFE`,
//! which includes `io`, `os`, and `package` — any data-pack file can then
//! `os.execute`, `os.remove`, or `package.loadlib`. The shipped VM loads only
//! `string`/`table`/`math`/`bit`/`jit` plus a private `os` (stolen `time` /
//! `date` / `clock`, then replaced) and registers `tfs.appendLog` as the sole
//! script write path, rooted at `data/logs/`.
//!
//! LuaJIT has no `StdLib::COROUTINE` flag — `coroutine` is in the base library.
//! `dofile` stays for the TFS `global.lua` load chain; `load` / `loadstring` /
//! `loadfile` / `require` / `package` drop (unused in the data pack).
//! `data/migrations/*.lua` are C++ leftover tooling and are not loaded here.

use mlua::{Function, Lua, LuaOptions, StdLib, Table, Value};
use std::cell::RefCell;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

/// Stdlib flags for the game VM.
///
/// `OS` is loaded only so LuaJIT's `os.time` / `os.date` / `os.clock` stay
/// exact (strftime, `*t` tables, `os.time{year=…}`). [`install_os_time_shim`]
/// then replaces `os` with those three functions. `JIT` stays so
/// `luaInstructionBudget = 0` can still `jit.on()`. LuaJIT has no
/// `StdLib::COROUTINE` — `coroutine` lives in the base library.
fn game_stdlib() -> StdLib {
    StdLib::STRING | StdLib::TABLE | StdLib::MATH | StdLib::BIT | StdLib::JIT | StdLib::OS
}

const DENIED_GLOBALS: &[&str] = &[
    "io",
    "package",
    "require",
    "loadstring",
    "loadfile",
    "load",
    "module",
];

const DEFAULT_LOG_ROOT: &str = "data/logs";
const MAX_KIND_LEN: usize = 255;

thread_local! {
    static LOG_ROOT: RefCell<PathBuf> = const { RefCell::new(PathBuf::new()) };
}

fn log_root() -> PathBuf {
    LOG_ROOT.with(|root| {
        let mut borrowed = root.borrow_mut();
        if borrowed.as_os_str().is_empty() {
            *borrowed = PathBuf::from(DEFAULT_LOG_ROOT);
        }
        borrowed.clone()
    })
}

/// Override the `tfs.appendLog` root (tests). Returns the previous root.
#[cfg(test)]
pub(crate) fn set_log_root(path: PathBuf) -> PathBuf {
    LOG_ROOT.with(|root| {
        let mut borrowed = root.borrow_mut();
        if borrowed.as_os_str().is_empty() {
            *borrowed = PathBuf::from(DEFAULT_LOG_ROOT);
        }
        std::mem::replace(&mut *borrowed, path)
    })
}

/// Build the allowlisted game Lua state (no `io` / `package` / `loadstring`).
pub(crate) fn create_allowlisted_lua() -> mlua::Result<Lua> {
    let lua = Lua::new_with(game_stdlib(), LuaOptions::default())?;
    install_os_time_shim(&lua)?;
    install_lua51_table_compat(&lua)?;
    strip_denied_globals(&lua)?;
    register_tfs_append_log(&lua)?;
    Ok(lua)
}

/// LuaJIT is 5.1: no `table.pack` / `table.unpack`. EventCallback `__call`
/// (`data/scripts/lib/event_callbacks.lua`) uses `table.pack(...)`.
fn install_lua51_table_compat(lua: &Lua) -> mlua::Result<()> {
    lua.load(
        r#"
        if table.pack == nil then
            function table.pack(...)
                return { n = select('#', ...), ... }
            end
        end
        if table.unpack == nil then
            table.unpack = unpack
        end
        "#,
    )
    .set_name("lua51_table_compat")
    .exec()
}

fn install_os_time_shim(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    let os: Table = globals.get("os")?;
    let shim = lua.create_table()?;
    for name in ["time", "date", "clock"] {
        let f: Function = os.get(name)?;
        shim.set(name, f)?;
    }
    globals.set("os", shim)?;
    Ok(())
}

fn strip_denied_globals(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    for name in DENIED_GLOBALS {
        globals.set(*name, Value::Nil)?;
    }
    Ok(())
}

fn register_tfs_append_log(lua: &Lua) -> mlua::Result<()> {
    let tfs = lua.create_table()?;
    tfs.set(
        "appendLog",
        lua.create_function(|_, (kind, text): (String, String)| Ok(append_log(&kind, &text)))?,
    )?;
    lua.globals().set("tfs", tfs)?;
    Ok(())
}

fn append_log(kind: &str, text: &str) -> bool {
    let path = match resolve_log_path(kind) {
        Ok(p) => p,
        Err(reason) => {
            tracing::warn!(kind, reason, "tfs.appendLog rejected kind");
            return false;
        }
    };
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!(path = %path.display(), error = %e, "tfs.appendLog create_dir failed");
        return false;
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(text.as_bytes()) {
                tracing::warn!(path = %path.display(), error = %e, "tfs.appendLog write failed");
                return false;
            }
            true
        }
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "tfs.appendLog open failed");
            false
        }
    }
}

fn resolve_log_path(kind: &str) -> Result<PathBuf, &'static str> {
    if kind.is_empty() || kind.len() > MAX_KIND_LEN || kind.contains('\0') {
        return Err("invalid log kind");
    }
    let mut relative = PathBuf::new();
    for component in Path::new(kind).components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_str().ok_or("invalid log kind")?;
                if !is_safe_kind_component(part) {
                    return Err("invalid log kind");
                }
                relative.push(part);
            }
            _ => return Err("log kind must be a relative path"),
        }
    }
    if relative.as_os_str().is_empty() {
        return Err("invalid log kind");
    }
    Ok(log_root().join(relative))
}

fn is_safe_kind_component(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '-' | '_'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct RestoreLogRoot(PathBuf);
    impl Drop for RestoreLogRoot {
        fn drop(&mut self) {
            set_log_root(std::mem::take(&mut self.0));
        }
    }

    fn temp_log_root() -> (PathBuf, RestoreLogRoot) {
        let dir = std::env::temp_dir().join(format!(
            "tfs-lua-logs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp log root");
        let prev = set_log_root(dir.clone());
        (dir, RestoreLogRoot(prev))
    }

    fn eval_bool(lua: &Lua, chunk: &str) -> bool {
        lua.load(chunk).eval().expect(chunk)
    }

    #[test]
    fn denied_stdlib_is_nil() {
        let lua = create_allowlisted_lua().expect("vm");
        for name in DENIED_GLOBALS {
            assert!(
                eval_bool(&lua, &format!("return {name} == nil")),
                "{name} must be nil on the game VM"
            );
        }
        assert!(eval_bool(&lua, "return os.execute == nil"));
        assert!(eval_bool(&lua, "return os.remove == nil"));
        assert!(eval_bool(&lua, "return os.getenv == nil"));
        assert!(eval_bool(&lua, "return os.rename == nil"));
        assert!(eval_bool(&lua, "return os.exit == nil"));
    }

    #[test]
    fn os_time_date_clock_remain() {
        let lua = create_allowlisted_lua().expect("vm");
        assert!(eval_bool(&lua, "return type(os.time) == 'function'"));
        assert!(eval_bool(&lua, "return type(os.date) == 'function'"));
        assert!(eval_bool(&lua, "return type(os.clock) == 'function'"));
        assert!(eval_bool(&lua, "return os.time() > 0"));
        assert!(eval_bool(&lua, "return type(os.date('%Y')) == 'string'"));
        assert!(eval_bool(&lua, "return type(os.date('*t')) == 'table'"));
        assert!(eval_bool(
            &lua,
            "return os.time{year=1970, month=1, day=2} > 0"
        ));
        assert!(eval_bool(&lua, "return os.clock() >= 0"));
        assert!(eval_bool(&lua, "return type(jit) == 'table'"));
        assert!(eval_bool(&lua, "return type(dofile) == 'function'"));
        assert!(eval_bool(&lua, "return type(coroutine) == 'table'"));
        assert!(eval_bool(&lua, "return type(table.pack) == 'function'"));
        assert!(eval_bool(&lua, "return table.pack(1, 2).n == 2"));
        assert!(eval_bool(&lua, "return type(table.unpack) == 'function'"));
    }

    #[test]
    fn append_log_writes_under_root() {
        let (dir, _restore) = temp_log_root();
        let lua = create_allowlisted_lua().expect("vm");
        let ok: bool = lua
            .load(r#"return tfs.appendLog("God commands.log", "[line]\n")"#)
            .eval()
            .expect("appendLog");
        assert!(ok);
        let written = fs::read_to_string(dir.join("God commands.log")).expect("read log");
        assert_eq!(written, "[line]\n");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn append_log_creates_kind_subdir() {
        let (dir, _restore) = temp_log_root();
        let lua = create_allowlisted_lua().expect("vm");
        let ok: bool = lua
            .load(r#"return tfs.appendLog("bugs/Tester report.txt", "comment\n")"#)
            .eval()
            .expect("appendLog");
        assert!(ok);
        let written =
            fs::read_to_string(dir.join("bugs").join("Tester report.txt")).expect("read report");
        assert_eq!(written, "comment\n");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn append_log_rejects_path_escape() {
        let (dir, _restore) = temp_log_root();
        for kind in [
            "../escape.log",
            "/tmp/abs.log",
            "ok/../../escape.log",
            "",
            "bad\0name.log",
        ] {
            let ok = append_log(kind, "nope");
            assert!(!ok, "kind {kind:?} must be rejected");
        }
        assert!(
            fs::read_dir(&dir).expect("dir").next().is_none(),
            "rejected kinds must not create files under the log root"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn data_pack_runtime_sites_use_append_log() {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let functions = fs::read_to_string(data.join("scripts/functions.lua")).expect("functions");
        assert!(
            functions.contains("tfs.appendLog"),
            "logCommand must use tfs.appendLog"
        );
        assert!(
            !functions.contains("io.open") && !functions.contains("io.write"),
            "functions.lua must not use io.*"
        );
        let bug =
            fs::read_to_string(data.join("scripts/eventcallbacks/player/default_onReportBug.lua"))
                .expect("onReportBug");
        assert!(
            bug.contains("tfs.appendLog"),
            "onReportBug must use tfs.appendLog"
        );
        assert!(
            !bug.contains("io.open") && !bug.contains("io.write"),
            "default_onReportBug.lua must not use io.*"
        );
    }
}
