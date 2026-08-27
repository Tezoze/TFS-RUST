//! Lua pack `db` / `result` SQL wait-bridge (game thread waits; sqlx on IO workers).
//!
//! C++ reference: `luascript.cpp` `luaDatabaseStoreQuery` / `luaDatabaseExecute` /
//! `luaDatabaseAsyncExecute`; `database.cpp` `storeQuery` / `executeQuery`.
//!
//! Do not use `tokio::task::block_in_place` — the game loop runs on a `LocalSet`.
//! `tokio::spawn` here is sqlx IO only (never `GameWorld`).

use std::cell::RefCell;

use tfs_rust_db::DbPool;
use tfs_rust_lua::{LuaDbOutcome, LuaDbRequest};

thread_local! {
    static LUA_DB_POOL: RefCell<Option<DbPool>> = const { RefCell::new(None) };
}

/// Bind the sqlx pool for Lua pack SQL (game thread).
pub fn bind_lua_db_pool(pool: DbPool) {
    LUA_DB_POOL.with(|slot| {
        *slot.borrow_mut() = Some(pool);
    });
}

fn empty_outcome(req: &LuaDbRequest) -> LuaDbOutcome {
    match req {
        LuaDbRequest::Execute(_) => LuaDbOutcome::Execute(false),
        LuaDbRequest::StoreQuery(_) => LuaDbOutcome::Rows(Vec::new()),
    }
}

/// Apply a Lua pack SQL request. Errors map to false / empty rows (TFS does not throw).
pub fn apply_lua_db(req: LuaDbRequest) -> LuaDbOutcome {
    let Some(pool) = LUA_DB_POOL.with(|slot| slot.borrow().clone()) else {
        return empty_outcome(&req);
    };
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return empty_outcome(&req);
    };

    let store_query = matches!(req, LuaDbRequest::StoreQuery(_));
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    match req {
        LuaDbRequest::Execute(sql) => {
            handle.spawn(async move {
                let outcome = match pool.lua_execute(&sql).await {
                    Ok(()) => LuaDbOutcome::Execute(true),
                    Err(e) => {
                        tracing::warn!(error = %e, "lua db.query failed");
                        LuaDbOutcome::Execute(false)
                    }
                };
                let _ = tx.send(outcome);
            });
        }
        LuaDbRequest::StoreQuery(sql) => {
            handle.spawn(async move {
                let outcome = match pool.lua_store_query(&sql).await {
                    Ok(rows) => LuaDbOutcome::Rows(rows),
                    Err(e) => {
                        tracing::warn!(error = %e, "lua db.storeQuery failed");
                        LuaDbOutcome::Rows(Vec::new())
                    }
                };
                let _ = tx.send(outcome);
            });
        }
    }

    rx.recv().unwrap_or_else(|_| {
        if store_query {
            LuaDbOutcome::Rows(Vec::new())
        } else {
            LuaDbOutcome::Execute(false)
        }
    })
}
