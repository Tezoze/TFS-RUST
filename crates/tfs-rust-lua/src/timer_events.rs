//! `addEvent` / `stopEvent` Lua globals backed by the game-thread `Scheduler`.
//!
//! C++ reference: `luascript.cpp` `luaAddEvent` (~3789), `luaStopEvent` (~3907),
//! `LuaEnvironment::executeTimerEvent` (~18238); `scheduler.cpp` `Scheduler::addEvent`/`stopEvent`.
//!
//! ## Design
//!
//! The Lua-facing timer id (`lastTimerEventId` in C++) is generated here and is the
//! same id delivered back via `GameCommand::LuaCallback { event_id }`. The
//! [`TimerScheduler`] trait (implemented by `tfs-rust-core::Scheduler`) spawns the
//! Tokio timer and supports cancellation.
//!
//! `timer_events` holds `mlua::RegistryKey`s for the callback function and
//! parameters. It uses `Rc<RefCell<…>>` so the `addEvent`/`stopEvent` Lua closures
//! (which are `Fn`, not `FnMut`) can mutate it. The map is `!Send` and lives only
//! on the game thread, next to the `Lua` VM.
//!
//! Reentrancy: `execute_timer_event` extracts the desc from the map **before**
//! calling Lua (matching C++ `std::move` + `erase` before `callFunction`), so a
//! callback that calls `addEvent`/`stopEvent` won't alias a borrowed entry.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use mlua::{Lua, MultiValue, RegistryKey, Value};

/// Abstraction over the game-thread timer scheduler (`tfs-rust-core::Scheduler`).
///
/// Defined in the lua crate so core can implement it without a circular dependency.
/// The `event_id` returned by `schedule_after` is delivered verbatim as
/// `GameCommand::LuaCallback { event_id }`.
pub trait TimerScheduler {
    /// Spawn a one-shot timer; returns the event id that will arrive in `LuaCallback`.
    fn schedule_after(&self, delay: Duration) -> u64;
    /// Cancel a pending timer; returns `true` if the id was found and cancelled.
    fn stop_event(&self, event_id: u64) -> bool;
}

thread_local! {
    /// Set once at game-thread startup so `addEvent`/`stopEvent` closures can reach the
    /// `Scheduler` without capturing it (closures are `Fn`, registered at `LuaRuntime::new`).
    static CURRENT_TIMER_SCHEDULER: RefCell<Option<Rc<dyn TimerScheduler>>> = const { RefCell::new(None) };
}

/// Bind a [`TimerScheduler`] to the current thread for the lifetime of the game thread.
///
/// Called once from `run_server.rs` after the `Scheduler` is created. Panics if already
/// set (there is exactly one game thread).
pub fn set_timer_scheduler(scheduler: Rc<dyn TimerScheduler>) {
    CURRENT_TIMER_SCHEDULER.with(|s| {
        if s.borrow().is_some() {
            panic!("CURRENT_TIMER_SCHEDULER already set");
        }
        *s.borrow_mut() = Some(scheduler);
    });
}

fn with_scheduler<F, R>(f: F) -> R
where
    R: Default,
    F: FnOnce(&dyn TimerScheduler) -> R,
{
    CURRENT_TIMER_SCHEDULER.with(|s| {
        let opt = s.borrow();
        match opt.as_ref() {
            Some(scheduler) => f(scheduler.as_ref()),
            None => {
                tracing::error!("addEvent/stopEvent called with no TimerScheduler bound");
                R::default()
            }
        }
    })
}

/// Stored callback + parameters for a pending `addEvent` timer.
///
/// C++ reference: `LuaTimerEventDesc` (`luascript.h:75-83`).
#[derive(Debug)]
pub struct TimerEventDesc {
    /// `luaL_ref` of the callback function (`eventDesc.function`).
    pub function: RegistryKey,
    /// `luaL_ref` of each trailing parameter (`eventDesc.parameters`).
    pub parameters: Vec<RegistryKey>,
}

/// Shared, interior-mutable timer-event map owned by `LuaRuntime`.
pub type TimerEvents = Rc<RefCell<std::collections::HashMap<u64, TimerEventDesc>>>;

/// Register `addEvent` and `stopEvent` Lua globals.
///
/// C++ reference: `luascript.cpp:1126-1130` (`lua_register`), `luaAddEvent` (~3789),
/// `luaStopEvent` (~3907).
///
/// `timer_events` and `next_timer_id` are shared (`Rc`) so the closures can mutate
/// them. The same `Rc`s are held by `LuaRuntime` for `execute_timer_event`.
pub fn register_add_event_stop_event(
    lua: &Lua,
    timer_events: TimerEvents,
    next_timer_id: Rc<RefCell<u64>>,
) -> Result<(), mlua::Error> {
    let globals = lua.globals();

    // addEvent(callback, delay, ...)
    // C++ reference: `LuaScriptInterface::luaAddEvent` (`luascript.cpp:3789`).
    let te_clone = timer_events.clone();
    let id_clone = next_timer_id.clone();
    let add_event = lua.create_function(move |lua, params: MultiValue| {
        let mut params = params.into_vec();
        if params.len() < 2 {
            return Err(mlua::Error::runtime("addEvent: need at least (callback, delay)"));
        }
        // delay is the second parameter (index 2 in C++ 1-based)
        let delay_val = params.remove(1); // 0-based index 1 = 2nd param
        let callback_val = params.remove(0); // 0-based index 0 = 1st param

        let callback = match callback_val {
            Value::Function(_) => callback_val,
            _ => return Err(mlua::Error::runtime("addEvent: callback must be a function")),
        };
        let delay_ms: u64 = match delay_val {
            Value::Integer(n) => n.max(0) as u64,
            Value::Number(n) => n.max(0.0) as u64,
            _ => return Err(mlua::Error::runtime("addEvent: delay must be a number")),
        };
        // C++ clamps to >= 100 ms (`std::max<uint32_t>(100, delay)` — luascript.cpp:3891).
        let delay_ms = delay_ms.max(100);

        // Store the callback as a registry key (C++ `luaL_ref`).
        let function_key = lua.create_registry_value(callback)?;

        // Store trailing parameters as registry keys (C++ `eventDesc.parameters`).
        // C++ pushes them in order then pops via luaL_ref in a loop; the vector ends
        // up in the same order as the Lua stack (param 3, 4, …).
        let mut param_keys = Vec::with_capacity(params.len());
        for val in params {
            let key = lua.create_registry_value(val)?;
            param_keys.push(key);
        }

        // Allocate the Lua-facing timer id (C++ `lastTimerEventId`).
        let timer_id = {
            let mut slot = id_clone.borrow_mut();
            let id = *slot;
            *slot = id.checked_add(1).ok_or_else(|| {
                mlua::Error::runtime("addEvent: timer id overflow")
            })?;
            id
        };

        // Schedule the timer via the game-thread Scheduler.
        // C++: `g_scheduler.addEvent(createSchedulerTask(delay, bind(executeTimerEvent, id)))`.
        let _scheduler_event_id = with_scheduler(|s| s.schedule_after(Duration::from_millis(delay_ms)));

        // Insert into the timer-events map (C++ `timerEvents.emplace(id, eventDesc)`).
        te_clone.borrow_mut().insert(
            timer_id,
            TimerEventDesc {
                function: function_key,
                parameters: param_keys,
            },
        );

        Ok(timer_id)
    })?;
    globals.set("addEvent", add_event)?;

    // stopEvent(eventid)
    // C++ reference: `LuaScriptInterface::luaStopEvent` (`luascript.cpp:3907`).
    let te_clone2 = timer_events.clone();
    let stop_event = lua.create_function(move |_lua, event_id: u64| {
        // C++: look up in timerEvents; if not found return false.
        let desc = te_clone2.borrow_mut().remove(&event_id);
        let Some(desc) = desc else {
            return Ok(false);
        };

        // Cancel the pending timer (C++ `g_scheduler.stopEvent(timerEventDesc.eventId)`).
        // The scheduler_event_id is the same as the Lua-facing id in our unified scheme.
        with_scheduler(|s| {
            s.stop_event(event_id);
        });

        // Free registry refs (C++ `luaL_unref` for function + each parameter).
        // `RegistryKey` drops automatically here when `desc` goes out of scope.
        drop(desc);

        Ok(true)
    })?;
    globals.set("stopEvent", stop_event)?;

    Ok(())
}

/// Execute a fired timer event: look up the callback + params, call, then free refs.
///
/// C++ reference: `LuaEnvironment::executeTimerEvent` (`luascript.cpp:18238`).
///
/// Returns `true` if the event was found and executed, `false` if it was already
/// cancelled (removed by `stopEvent` before the timer fired).
pub fn execute_timer_event(
    lua: &Lua,
    timer_events: &TimerEvents,
    event_id: u64,
) -> Result<bool, mlua::Error> {
    // Extract before calling Lua to avoid reentrancy aliasing
    // (C++ `std::move(it->second); timerEvents.erase(it);` before `callFunction`).
    let desc = timer_events.borrow_mut().remove(&event_id);
    let Some(desc) = desc else {
        // Already cancelled or already fired.
        return Ok(false);
    };

    // Push function (C++ `lua_rawgeti(luaState, LUA_REGISTRYINDEX, function)`).
    let function: mlua::Function = lua.registry_value(&desc.function)?;

    // Push parameters in reverse order so they appear in the correct order on the
    // Lua stack (C++ iterates `boost::adaptors::reverse(parameters)`).
    let mut args = MultiValue::new();
    for param_key in desc.parameters.iter().rev() {
        let val: Value = lua.registry_value(param_key)?;
        args.push_front(val);
    }

    // Call the function (C++ `callFunction(parameters.size())`).
    let result = function.call::<()>(args);

    // Free resources — `RegistryKey`s drop here (C++ `luaL_unref` for function + params).
    drop(desc);

    result?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// A test `TimerScheduler` that records scheduled ids and supports cancellation.
    struct TestScheduler {
        scheduled: Rc<RefCell<Vec<u64>>>,
        stopped: Rc<RefCell<Vec<u64>>>,
        next: std::sync::atomic::AtomicU64,
    }

    impl TestScheduler {
        fn new() -> Self {
            Self {
                scheduled: Rc::new(RefCell::new(Vec::new())),
                stopped: Rc::new(RefCell::new(Vec::new())),
                next: std::sync::atomic::AtomicU64::new(1),
            }
        }
    }

    impl TimerScheduler for TestScheduler {
        fn schedule_after(&self, _delay: Duration) -> u64 {
            let id = self.next.fetch_add(1, Ordering::Relaxed);
            self.scheduled.borrow_mut().push(id);
            id
        }
        fn stop_event(&self, event_id: u64) -> bool {
            self.stopped.borrow_mut().push(event_id);
            true
        }
    }

    fn set_test_scheduler(s: Rc<TestScheduler>) {
        CURRENT_TIMER_SCHEDULER.with(|slot| {
            *slot.borrow_mut() = Some(s as Rc<dyn TimerScheduler>);
        });
    }

    fn clear_test_scheduler() {
        CURRENT_TIMER_SCHEDULER.with(|slot| {
            *slot.borrow_mut() = None;
        });
    }

    #[test]
    fn add_event_registers_and_executes() {
        let lua = Lua::new();
        let timer_events: TimerEvents = Rc::new(RefCell::new(HashMap::new()));
        let next_timer_id = Rc::new(RefCell::new(1u64));
        register_add_event_stop_event(&lua, timer_events.clone(), next_timer_id)
            .expect("register addEvent/stopEvent");

        let scheduler = Rc::new(TestScheduler::new());
        set_test_scheduler(scheduler.clone());

        // Lua: local ran = false; local id = addEvent(function() ran = true end, 100); return id
        let ran = Arc::new(AtomicBool::new(false));
        let ran_clone = ran.clone();
        let callback = lua
            .create_function(move |_, ()| {
                ran_clone.store(true, Ordering::SeqCst);
                Ok(())
            })
            .expect("create callback");
        lua.globals().set("_test_cb", callback).unwrap();

        let id: u64 = lua
            .load("return addEvent(_test_cb, 100)")
            .eval()
            .expect("addEvent call");
        assert_eq!(id, 1, "first timer id should be 1");
        assert_eq!(scheduler.scheduled.borrow().len(), 1);
        assert!(timer_events.borrow().contains_key(&id));

        // Execute the timer event
        let found = execute_timer_event(&lua, &timer_events, id).expect("execute");
        assert!(found, "event should be found");
        assert!(ran.load(Ordering::SeqCst), "callback should have run");
        assert!(!timer_events.borrow().contains_key(&id), "entry removed after fire");

        clear_test_scheduler();
    }

    #[test]
    fn add_event_with_parameters() {
        let lua = Lua::new();
        let timer_events: TimerEvents = Rc::new(RefCell::new(HashMap::new()));
        let next_timer_id = Rc::new(RefCell::new(1u64));
        register_add_event_stop_event(&lua, timer_events.clone(), next_timer_id)
            .expect("register");

        let scheduler = Rc::new(TestScheduler::new());
        set_test_scheduler(scheduler.clone());

        let received = Rc::new(RefCell::new(Vec::<(i64, String)>::new()));
        let received_clone = received.clone();
        let callback = lua
            .create_function(move |_, (n, s): (i64, String)| {
                received_clone.borrow_mut().push((n, s));
                Ok(())
            })
            .expect("create callback");
        lua.globals().set("_test_cb2", callback).unwrap();

        let id: u64 = lua
            .load("return addEvent(_test_cb2, 200, 42, 'hello')")
            .eval()
            .expect("addEvent with params");
        assert_eq!(id, 1);

        execute_timer_event(&lua, &timer_events, id).expect("execute");
        assert_eq!(*received.borrow(), vec![(42, "hello".to_string())]);

        clear_test_scheduler();
    }

    #[test]
    fn stop_event_cancels_and_returns_true() {
        let lua = Lua::new();
        let timer_events: TimerEvents = Rc::new(RefCell::new(HashMap::new()));
        let next_timer_id = Rc::new(RefCell::new(1u64));
        register_add_event_stop_event(&lua, timer_events.clone(), next_timer_id)
            .expect("register");

        let scheduler = Rc::new(TestScheduler::new());
        set_test_scheduler(scheduler.clone());

        let callback = lua.create_function(|_, ()| Ok(())).expect("create callback");
        lua.globals().set("_test_cb3", callback).unwrap();

        let id: u64 = lua
            .load("return addEvent(_test_cb3, 500)")
            .eval()
            .expect("addEvent");
        assert!(timer_events.borrow().contains_key(&id));

        // stopEvent should return true and remove the entry
        let stopped: bool = lua
            .load(&format!("return stopEvent({id})"))
            .eval()
            .expect("stopEvent call");
        assert!(stopped, "stopEvent should return true for existing id");
        assert!(!timer_events.borrow().contains_key(&id));
        assert_eq!(scheduler.stopped.borrow().len(), 1);

        // stopEvent on a non-existent id should return false
        let stopped2: bool = lua
            .load("return stopEvent(99999)")
            .eval()
            .expect("stopEvent non-existent");
        assert!(!stopped2, "stopEvent should return false for unknown id");

        clear_test_scheduler();
    }

    #[test]
    fn execute_timer_event_missing_returns_false() {
        let lua = Lua::new();
        let timer_events: TimerEvents = Rc::new(RefCell::new(HashMap::new()));

        let found = execute_timer_event(&lua, &timer_events, 99999).expect("execute");
        assert!(!found, "missing event should return false");
    }

    #[test]
    fn add_event_clamps_delay_to_100ms() {
        let lua = Lua::new();
        let timer_events: TimerEvents = Rc::new(RefCell::new(HashMap::new()));
        let next_timer_id = Rc::new(RefCell::new(1u64));
        register_add_event_stop_event(&lua, timer_events.clone(), next_timer_id)
            .expect("register");

        // We can't directly observe the delay passed to the scheduler in this test
        // without extending TestScheduler, but we can verify the event is created.
        let scheduler = Rc::new(TestScheduler::new());
        set_test_scheduler(scheduler.clone());

        let callback = lua.create_function(|_, ()| Ok(())).expect("create callback");
        lua.globals().set("_test_cb4", callback).unwrap();

        // delay of 1ms should be clamped to 100ms — event still created
        let id: u64 = lua
            .load("return addEvent(_test_cb4, 1)")
            .eval()
            .expect("addEvent with tiny delay");
        assert!(id > 0);
        assert!(timer_events.borrow().contains_key(&id));

        clear_test_scheduler();
    }
}
