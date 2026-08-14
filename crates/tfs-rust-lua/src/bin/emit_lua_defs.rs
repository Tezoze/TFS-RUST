//! Emit LuaLS `.d.lua` stubs from a live `LuaRuntime`.
//!
//! Usage (from repo root):
//! ```text
//! cargo run -p tfs-rust-lua --bin emit-lua-defs
//! cargo run -p tfs-rust-lua --bin emit-lua-defs -- --check
//! cargo run -p tfs-rust-lua --bin emit-lua-defs -- --out lua-defs
//! ```
//!
//! VM hardening pillar 5 — `tasks/tools-actions/vm-hardening.md`.

use std::path::PathBuf;
use std::process::ExitCode;

use tfs_rust_lua::lua_defs::{
    check_lua_defs, default_lua_defs_dir, snapshot_lua_defs, write_lua_defs,
};
use tfs_rust_lua::LuaRuntime;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("emit-lua-defs: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    let mut check = false;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--check" => check = true,
            "--out" => {
                i += 1;
                let path = args
                    .get(i)
                    .ok_or("--out requires a directory")?;
                out = Some(PathBuf::from(path));
            }
            "-h" | "--help" => {
                return Ok(
                    "emit-lua-defs [--check] [--out DIR]\n\
                     Write LuaLS stubs from the live engine VM (pillar 5)."
                        .into(),
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }

    let dir = out.unwrap_or_else(default_lua_defs_dir);
    let runtime = LuaRuntime::new().map_err(|e| e.to_string())?;
    let snapshot = snapshot_lua_defs(&runtime).map_err(|e| e.to_string())?;
    if check {
        check_lua_defs(&dir, &snapshot)?;
        Ok(format!("{} is current", dir.display()))
    } else {
        write_lua_defs(&dir, &snapshot).map_err(|e| e.to_string())?;
        Ok(format!(
            "wrote engine.d.lua, constants.d.lua, globals.d.lua to {}",
            dir.display()
        ))
    }
}
