//! Per-invocation Lua instruction budget (VM hardening pillar 4).
//!
//! `tasks/tools-actions/vm-hardening.md`. Game simulation is single-threaded
//! (`TFS-threading`); a `while true do end` in any `data/scripts/**` callback
//! would otherwise hang ticks, packets, and saves until `kill -9`.
//!
//! mlua 0.12 `Lua::set_hook` + `HookTriggers::every_nth_instruction` on LuaJIT.
//! Compiled traces do **not** fire count hooks, so this module calls `jit.off()`
//! while the budget is enabled (same as mlua's own `test_limit_execution_instructions`).
//! Set the budget to `0` to restore LuaJIT.
//!
//! The budget is **per Rust→Lua entry** (re-armed at each `with_lua_instruction_budget`
//! outermost call). Nested Lua→Rust→Lua (userdata, combat callbacks from a spell)
//! shares the outer budget.
//!
//! Aborting a script does **not** roll back mutations. Mutations apply immediately
//! (`TFS-lua-boundaries` Mutation Path), so a killed callback leaves partial effects
//! — failure isolation, not atomicity.

use mlua::{HookTriggers, Lua};
use std::cell::Cell;

/// Default VM-instruction budget per script invocation.
///
/// Sized from a 500-creature × 20-item loot-style loop (~10× headroom), guarded
/// by `default_budget_covers_heavy_loot_loop_with_headroom`. Override from
/// `config.lua` via `luaInstructionBudget`. `0` disables the hook.
pub const DEFAULT_LUA_INSTRUCTION_BUDGET: u32 = 10_000_000;

/// Error text raised when the count hook fires. Kept stable for tests and logs.
pub const INSTRUCTION_BUDGET_EXCEEDED: &str = "script exceeded instruction budget";

thread_local! {
    static INSTRUCTION_BUDGET: Cell<u32> = const { Cell::new(DEFAULT_LUA_INSTRUCTION_BUDGET) };
    static HOOK_DEPTH: Cell<u32> = const { Cell::new(0) };
    static JIT_OFF_FOR_HOOKS: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn set_thread_instruction_budget(budget: u32) -> u32 {
    INSTRUCTION_BUDGET.with(|c| {
        let prev = c.get();
        c.set(budget);
        prev
    })
}

pub(crate) fn restore_luajit(lua: &Lua) {
    let _ = lua.load("if jit then jit.on() end").exec();
    JIT_OFF_FOR_HOOKS.with(|c| c.set(false));
}

fn disable_luajit_for_hooks(lua: &Lua) -> mlua::Result<()> {
    if JIT_OFF_FOR_HOOKS.with(Cell::get) {
        return Ok(());
    }
    // LuaJIT does not call count hooks from compiled traces (mlua 0.12
    // `tests/hooks.rs` `test_limit_execution_instructions`).
    lua.load("if jit then jit.off() end").exec()?;
    JIT_OFF_FOR_HOOKS.with(|c| c.set(true));
    Ok(())
}

/// Run `f` with the thread-local instruction budget armed on `lua`.
///
/// Re-entrant: nested calls share the outer budget and must not `remove_hook`
/// on the way out (combat `setCallback` bodies run inside `onCastSpell`).
pub(crate) fn with_lua_instruction_budget<T>(
    lua: &Lua,
    f: impl FnOnce() -> mlua::Result<T>,
) -> mlua::Result<T> {
    let budget = INSTRUCTION_BUDGET.with(Cell::get);
    if budget == 0 {
        return f();
    }

    let depth = HOOK_DEPTH.with(|d| {
        let n = d.get();
        d.set(n + 1);
        n
    });
    let outermost = depth == 0;

    struct Guard<'a> {
        lua: &'a Lua,
        outermost: bool,
    }
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            HOOK_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
            if self.outermost {
                self.lua.remove_hook();
            }
        }
    }

    if outermost {
        if let Err(e) = disable_luajit_for_hooks(lua) {
            HOOK_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
            return Err(e);
        }
        if let Err(e) = lua.set_hook(
            HookTriggers::new().every_nth_instruction(budget),
            |_lua, _debug| Err(mlua::Error::runtime(INSTRUCTION_BUDGET_EXCEEDED)),
        ) {
            HOOK_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
            return Err(e);
        }
    }

    let _guard = Guard { lua, outermost };
    f()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    struct RestoreBudget(u32);
    impl Drop for RestoreBudget {
        fn drop(&mut self) {
            set_thread_instruction_budget(self.0);
        }
    }

    fn restore_on_drop() -> RestoreBudget {
        RestoreBudget(INSTRUCTION_BUDGET.with(Cell::get))
    }

    fn count_instructions(chunk: &str, step: u32) -> u64 {
        let lua = Lua::new();
        lua.load("if jit then jit.off() end")
            .exec()
            .expect("jit.off");
        let count = std::rc::Rc::new(Cell::new(0u64));
        let hook_count = count.clone();
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(step),
            move |_lua, _debug| {
                hook_count.set(hook_count.get() + u64::from(step));
                Ok(mlua::VmState::Continue)
            },
        )
        .expect("set_hook");
        lua.load(chunk).exec().expect("chunk");
        lua.remove_hook();
        count.get()
    }

    /// Loot-style nested tables: 500 creatures × 20 drops, then a sum pass.
    const HEAVY_LOOT_LOOP: &str = r#"
        local t = {}
        for c = 1, 500 do
            for i = 1, 20 do
                t[#t + 1] = { id = i, count = c % 10 }
            end
        end
        local n = 0
        for _, item in ipairs(t) do
            n = n + (item.count or 0)
        end
        return n
    "#;

    #[test]
    fn infinite_loop_is_aborted() {
        let _restore = restore_on_drop();
        set_thread_instruction_budget(10_000);
        let lua = Lua::new();
        let start = Instant::now();
        let err = with_lua_instruction_budget(&lua, || lua.load("while true do end").exec())
            .expect_err("runaway loop must error, not hang the thread");
        let msg = err.to_string();
        assert!(
            msg.contains(INSTRUCTION_BUDGET_EXCEEDED),
            "expected instruction-budget error, got: {msg}"
        );
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "abort took {:?}; budget too large for a hang-protection test",
            start.elapsed()
        );
    }

    #[test]
    fn abort_does_not_roll_back_prior_lua_side_effects() {
        let _restore = restore_on_drop();
        set_thread_instruction_budget(10_000);
        let lua = Lua::new();
        let err = with_lua_instruction_budget(&lua, || lua.load("x = 1; while true do end").exec())
            .expect_err("must abort");
        assert!(err.to_string().contains(INSTRUCTION_BUDGET_EXCEEDED));
        let x: i64 = lua.globals().get("x").expect("x set before abort");
        assert_eq!(x, 1, "abort is failure isolation, not atomicity");
    }

    #[test]
    fn budget_resets_per_invocation() {
        let _restore = restore_on_drop();
        set_thread_instruction_budget(50_000);
        let lua = Lua::new();
        let chunk = "local n = 0; for i = 1, 1000 do n = n + 1 end; return n";
        with_lua_instruction_budget(&lua, || lua.load(chunk).exec()).expect("first call");
        with_lua_instruction_budget(&lua, || lua.load(chunk).exec())
            .expect("second call must get a fresh budget, not a leftover lifetime counter");
    }

    #[test]
    fn budget_zero_disables_hook() {
        let _restore = restore_on_drop();
        set_thread_instruction_budget(0);
        let lua = Lua::new();
        restore_luajit(&lua);
        with_lua_instruction_budget(&lua, || {
            lua.load("local n = 0; for i = 1, 100000 do n = n + 1 end")
                .exec()
        })
        .expect("budget 0 must not arm the count hook");
    }

    #[test]
    fn nested_calls_share_outer_budget() {
        let _restore = restore_on_drop();
        set_thread_instruction_budget(10_000);
        let lua = Lua::new();
        let err = with_lua_instruction_budget(&lua, || {
            with_lua_instruction_budget(&lua, || lua.load("while true do end").exec())
        })
        .expect_err("inner runaway still hits the outer hook");
        assert!(err.to_string().contains(INSTRUCTION_BUDGET_EXCEEDED));
    }

    #[test]
    fn default_budget_covers_heavy_loot_loop_with_headroom() {
        let counted = count_instructions(HEAVY_LOOT_LOOP, 1_000);
        assert!(
            u64::from(DEFAULT_LUA_INSTRUCTION_BUDGET) >= counted.saturating_mul(10),
            "DEFAULT_LUA_INSTRUCTION_BUDGET ({}) is less than 10× measured loot-loop instructions ({counted})",
            DEFAULT_LUA_INSTRUCTION_BUDGET
        );
    }

    #[test]
    fn legitimate_loot_loop_succeeds_under_default_budget() {
        let _restore = restore_on_drop();
        set_thread_instruction_budget(DEFAULT_LUA_INSTRUCTION_BUDGET);
        let lua = Lua::new();
        let n: i64 = with_lua_instruction_budget(&lua, || lua.load(HEAVY_LOOT_LOOP).eval())
            .expect("heavy loot loop must fit the default budget");
        assert!(n > 0);
    }
}
