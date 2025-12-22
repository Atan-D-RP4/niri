//! Event loop API for Lua timers.
//!
//! This module provides the `niri.loop` API that gives Lua scripts access to
//! timer functionality integrated with the compositor's event loop.
//!
//! # API Overview
//!
//! - `niri.loop.new_timer()` - Create a new timer
//! - `niri.loop.now()` - Get monotonic time in milliseconds
//!
//! Timer methods:
//! - `timer:start(delay_ms, repeat_ms, callback)` - Start the timer
//! - `timer:stop()` - Stop the timer (can be restarted)
//! - `timer:close()` - Stop and clean up the timer
//! - `timer:is_active()` - Check if timer is running
//!
//! # Example
//!
//! ```lua
//! -- One-shot timer
//! local timer = niri.loop.new_timer()
//! timer:start(1000, 0, function()
//!     print("Timer fired after 1 second!")
//!     timer:close()
//! end)
//!
//! -- Repeating timer
//! local tick = niri.loop.new_timer()
//! tick:start(0, 500, function()
//!     print("Tick every 500ms")
//! end)
//! -- Later: tick:close()
//!
//! -- Get current time
//! local now = niri.loop.now()
//! ```
//!
//! # Timer Lifetime
//!
//! Following Neovim's model, timers persist until explicitly closed:
//! - GC of timer handle does NOT stop the timer
//! - Callbacks continue to fire after handle is GC'd
//! - Explicit `timer:close()` is required for cleanup

use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use mlua::prelude::*;

/// Counter for generating unique timer IDs.
static NEXT_TIMER_ID: AtomicU64 = AtomicU64::new(1);

/// Monotonic clock start time for `niri.loop.now()`.
static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn get_start_time() -> Instant {
    *START_TIME.get_or_init(Instant::now)
}

/// Timer state stored in the timer manager.
#[derive(Debug)]
pub struct TimerState {
    /// Timer ID.
    pub id: u64,
    /// Delay before first fire (milliseconds).
    pub delay_ms: u64,
    /// Repeat interval (0 = one-shot).
    pub repeat_ms: u64,
    /// Callback registry key.
    pub callback_key: LuaRegistryKey,
    /// Whether the timer is currently active.
    pub active: bool,
    /// When the timer was started.
    pub started_at: Option<Instant>,
    /// When the timer should next fire.
    pub next_fire: Option<Instant>,
}

/// Manages all active timers.
///
/// This struct holds the state for all timers and provides methods to
/// fire timers when they're due. It's designed to be polled from the
/// compositor's event loop.
pub struct TimerManager {
    /// All registered timers.
    timers: HashMap<u64, TimerState>,
    /// Min-heap of (fire_time, timer_id) for efficient next-timer lookup.
    fire_queue: BinaryHeap<Reverse<(Instant, u64)>>,
}

impl TimerManager {
    /// Create a new timer manager.
    pub fn new() -> Self {
        Self {
            timers: HashMap::new(),
            fire_queue: BinaryHeap::new(),
        }
    }

    /// Register a new timer (called by timer:start).
    pub fn register(
        &mut self,
        id: u64,
        delay_ms: u64,
        repeat_ms: u64,
        callback_key: LuaRegistryKey,
    ) {
        let now = Instant::now();
        let next_fire = now + Duration::from_millis(delay_ms);

        self.timers.insert(
            id,
            TimerState {
                id,
                delay_ms,
                repeat_ms,
                callback_key,
                active: true,
                started_at: Some(now),
                next_fire: Some(next_fire),
            },
        );

        self.fire_queue.push(Reverse((next_fire, id)));
    }

    /// Stop a timer (can be restarted).
    pub fn stop(&mut self, id: u64) {
        if let Some(timer) = self.timers.get_mut(&id) {
            timer.active = false;
            timer.next_fire = None;
        }
    }

    /// Close and remove a timer.
    pub fn close(&mut self, id: u64) -> Option<LuaRegistryKey> {
        self.timers.remove(&id).map(|t| t.callback_key)
    }

    /// Check if a timer is active.
    pub fn is_active(&self, id: u64) -> bool {
        self.timers.get(&id).is_some_and(|t| t.active)
    }

    /// Get time until next timer fires (for event loop sleep).
    pub fn time_until_next(&self) -> Option<Duration> {
        let now = Instant::now();
        // Peek the heap but validate against current state
        if let Some(&Reverse((fire_time, id))) = self.fire_queue.peek() {
            if let Some(timer) = self.timers.get(&id) {
                if timer.active && timer.next_fire == Some(fire_time) {
                    return Some(fire_time.saturating_duration_since(now));
                }
            }
            // Stale entry - fallback to O(n) scan below
        }
        // Fallback to O(n) scan for correctness
        self.timers
            .values()
            .filter(|t| t.active && t.next_fire.is_some())
            .filter_map(|t| t.next_fire.map(|nf| nf.saturating_duration_since(now)))
            .min()
    }

    /// Get IDs of timers that are due to fire.
    pub fn get_due_timers(&mut self) -> Vec<u64> {
        let now = Instant::now();
        let mut due = Vec::new();

        // Pop entries from heap that are due
        while let Some(&Reverse((fire_time, id))) = self.fire_queue.peek() {
            if fire_time > now {
                break; // No more due timers
            }

            self.fire_queue.pop();

            // Validate against current state
            if let Some(timer) = self.timers.get(&id) {
                if timer.active && timer.next_fire == Some(fire_time) {
                    due.push(id);
                }
            }
            // Stale entries are simply discarded
        }

        due
    }

    /// Fire a timer and update its state.
    ///
    /// Returns the callback key if the timer should fire.
    /// Updates next_fire for repeating timers, or deactivates one-shot timers.
    pub fn fire_timer(&mut self, id: u64) -> Option<&LuaRegistryKey> {
        let timer = self.timers.get_mut(&id)?;

        if !timer.active {
            return None;
        }

        let callback_key = &timer.callback_key;

        // Update for next fire
        if timer.repeat_ms > 0 {
            // Repeating timer
            let now = Instant::now();
            let next_fire = now + Duration::from_millis(timer.repeat_ms);
            timer.next_fire = Some(next_fire);
            self.fire_queue.push(Reverse((next_fire, id)));
        } else {
            // One-shot timer
            timer.active = false;
            timer.next_fire = None;
        }

        Some(callback_key)
    }

    /// Get number of registered timers.
    pub fn count(&self) -> usize {
        self.timers.len()
    }

    /// Get number of active timers.
    pub fn active_count(&self) -> usize {
        self.timers.values().filter(|t| t.active).count()
    }

    /// Check if any timers are registered.
    pub fn is_empty(&self) -> bool {
        self.timers.is_empty()
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared timer manager for use across Lua and Rust.
pub type SharedTimerManager = Rc<RefCell<TimerManager>>;

/// Create a new shared timer manager.
pub fn create_timer_manager() -> SharedTimerManager {
    Rc::new(RefCell::new(TimerManager::new()))
}

/// Fire all due timers and execute their callbacks.
///
/// This should be called from the compositor's event loop.
/// Returns the number of timers fired and any errors encountered.
///
/// Timer callbacks are executed with timeout protection to prevent
/// runaway scripts from freezing the compositor.
pub fn fire_due_timers(lua: &Lua, manager: &SharedTimerManager) -> (usize, Vec<LuaError>) {
    use crate::runtime::call_with_lua_timeout;

    let mut fired = 0;
    let mut errors = Vec::new();

    // Get due timer IDs
    let due_ids = manager.borrow_mut().get_due_timers();

    for id in due_ids {
        // Get callback key (need to borrow mutably to update state)
        let callback_result = {
            let mut mgr = manager.borrow_mut();
            mgr.fire_timer(id).map(|key| {
                // We need to get the callback from registry
                // but we can't clone LuaRegistryKey, so we'll retrieve it here
                lua.registry_value::<LuaFunction>(key)
            })
        };

        if let Some(callback_result) = callback_result {
            match callback_result {
                Ok(callback) => match call_with_lua_timeout::<()>(lua, &callback, ()) {
                    Ok(()) => fired += 1,
                    Err(e) => {
                        let repeat_ms = {
                            let mgr = manager.borrow();
                            mgr.timers.get(&id).map(|t| t.repeat_ms).unwrap_or(0)
                        };

                        let enriched_error = LuaError::external(format!(
                            "timer callback error (timer_id={}, repeat={}ms): {}",
                            id, repeat_ms, e
                        ));

                        log::error!("Timer callback error: {}", enriched_error);
                        errors.push(enriched_error);
                        fired += 1;
                    }
                },
                Err(e) => {
                    log::error!("Failed to get timer callback: {}", e);
                    errors.push(e);
                }
            }
        }
    }

    (fired, errors)
}

/// Fire all due timers with fresh state snapshots per callback.
///
/// This version creates a fresh `StateSnapshot` before each timer callback,
/// ensuring that each callback sees the current compositor state rather than
/// a stale snapshot captured at the start of the frame.
///
/// # Arguments
///
/// * `lua` - The Lua context
/// * `manager` - Shared timer manager
/// * `state` - Reference to compositor state for creating fresh snapshots
///
/// # Returns
///
/// Returns the number of timers fired and any errors encountered.
pub fn fire_due_timers_with_state<S: crate::CompositorState>(
    lua: &Lua,
    manager: &SharedTimerManager,
    state: &S,
) -> (usize, Vec<LuaError>) {
    use crate::runtime::call_with_lua_timeout;
    use crate::runtime_api::{clear_event_context_state, set_event_context_state, StateSnapshot};

    let mut fired = 0;
    let mut errors = Vec::new();

    // Get due timer IDs
    let due_ids = manager.borrow_mut().get_due_timers();

    for id in due_ids {
        // Get callback key (need to borrow mutably to update state)
        let callback_result = {
            let mut mgr = manager.borrow_mut();
            mgr.fire_timer(id).map(|key| {
                // We need to get the callback from registry
                // but we can't clone LuaRegistryKey, so we'll retrieve it here
                lua.registry_value::<LuaFunction>(key)
            })
        };

        if let Some(callback_result) = callback_result {
            match callback_result {
                Ok(callback) => {
                    // Create fresh snapshot for THIS callback
                    let snapshot = StateSnapshot::from_compositor_state(state);
                    set_event_context_state(snapshot);

                    let result = call_with_lua_timeout::<()>(lua, &callback, ());

                    // Clear context after callback completes
                    clear_event_context_state();

                    match result {
                        Ok(()) => fired += 1,
                        Err(e) => {
                            let repeat_ms = {
                                let mgr = manager.borrow();
                                mgr.timers.get(&id).map(|t| t.repeat_ms).unwrap_or(0)
                            };

                            let enriched_error = LuaError::external(format!(
                                "timer callback error (timer_id={}, repeat={}ms): {}",
                                id, repeat_ms, e
                            ));

                            log::error!("Timer callback error: {}", enriched_error);
                            errors.push(enriched_error);
                            fired += 1;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to get timer callback: {}", e);
                    errors.push(e);
                }
            }
        }
    }

    (fired, errors)
}

/// Check if a Lua value is truthy (not nil or false)
fn is_truthy(value: &LuaValue) -> bool {
    !matches!(value, LuaValue::Nil | LuaValue::Boolean(false))
}

fn wait(
    _lua: &Lua,
    (timeout_ms, condition, interval_ms): (u64, Option<LuaFunction>, Option<i64>),
) -> LuaResult<(bool, LuaValue)> {
    let timeout = Duration::from_millis(timeout_ms);

    if condition.is_none() {
        std::thread::sleep(timeout);
        return Ok((true, LuaValue::Nil));
    }

    // Clamp interval to minimum of 1ms (handles negative values)
    let interval = interval_ms.unwrap_or(10).max(1) as u64;
    let start = Instant::now();
    let condition = condition.unwrap();

    loop {
        let result: LuaValue = condition.call(())?;

        if is_truthy(&result) {
            return Ok((true, result));
        }

        if start.elapsed() >= timeout {
            return Ok((false, LuaValue::Nil));
        }

        let elapsed = start.elapsed();
        let remaining = timeout.saturating_sub(elapsed);
        let sleep_duration = Duration::from_millis(interval).min(remaining);
        std::thread::sleep(sleep_duration);

        if start.elapsed() >= timeout {
            return Ok((false, LuaValue::Nil));
        }
    }
}

/// Register the loop API in a Lua context.
///
/// This creates the `niri.loop` table with:
/// - `new_timer()` - Create a new timer
/// - `now()` - Get monotonic time in milliseconds
///
/// # Arguments
///
/// * `lua` - The Lua context
/// * `manager` - Shared timer manager
pub fn register_loop_api(lua: &Lua, manager: SharedTimerManager) -> LuaResult<()> {
    // Get or create the niri table
    let niri: LuaTable = match lua.globals().get("niri")? {
        LuaValue::Table(t) => t,
        _ => {
            let t = lua.create_table()?;
            lua.globals().set("niri", t.clone())?;
            t
        }
    };

    // Create the loop table
    let loop_table = lua.create_table()?;

    // niri.loop.new_timer() -> Timer
    let manager_clone = manager.clone();
    let new_timer_fn = lua.create_function(move |lua, ()| {
        let timer_id = NEXT_TIMER_ID.fetch_add(1, Ordering::SeqCst);

        let timer = Timer {
            id: timer_id,
            manager: manager_clone.clone(),
        };

        lua.create_userdata(timer)
    })?;
    loop_table.set("new_timer", new_timer_fn)?;

    // niri.loop.now() -> number (milliseconds since start)
    let now_fn = lua.create_function(|_, ()| {
        let elapsed = get_start_time().elapsed();
        Ok(elapsed.as_millis() as u64)
    })?;
    loop_table.set("now", now_fn)?;

    // niri.loop.wait(timeout_ms, condition, interval_ms) -> (bool, any)
    let wait_fn = lua.create_function(wait)?;
    loop_table.set("wait", wait_fn)?;

    niri.set("loop", loop_table)?;

    Ok(())
}

/// Timer userdata for Lua.
struct Timer {
    id: u64,
    manager: SharedTimerManager,
}

impl LuaUserData for Timer {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // timer:start(delay_ms, repeat_ms, callback)
        methods.add_method("start", |lua, this, args: (u64, u64, LuaFunction)| {
            let (delay_ms, repeat_ms, callback) = args;

            // Stop any existing timer with this ID
            this.manager.borrow_mut().stop(this.id);

            // Store callback in registry
            let callback_key = lua.create_registry_value(callback)?;

            // Register the timer
            this.manager
                .borrow_mut()
                .register(this.id, delay_ms, repeat_ms, callback_key);

            Ok(())
        });

        // timer:stop()
        methods.add_method("stop", |_lua, this, ()| {
            this.manager.borrow_mut().stop(this.id);
            Ok(())
        });

        // timer:close()
        methods.add_method("close", |lua, this, ()| {
            // Remove and get the callback key
            if let Some(key) = this.manager.borrow_mut().close(this.id) {
                // Clean up registry
                if let Err(e) = lua.remove_registry_value(key) {
                    log::warn!("Failed to remove timer callback from registry: {}", e);
                }
            }
            Ok(())
        });

        // timer:is_active()
        methods.add_method("is_active", |_lua, this, ()| {
            Ok(this.manager.borrow().is_active(this.id))
        });

        // timer:get_due_in()
        methods.add_method("get_due_in", |_lua, this, ()| {
            let manager = this.manager.borrow();

            let Some(timer) = manager.timers.get(&this.id) else {
                return Ok(0_u64);
            };

            if !timer.active {
                return Ok(0_u64);
            }

            let Some(next_fire) = timer.next_fire else {
                return Ok(0_u64);
            };

            let now = Instant::now();
            Ok(next_fire.saturating_duration_since(now).as_millis() as u64)
        });

        // timer:set_repeat(repeat_ms)
        methods.add_method("set_repeat", |_lua, this, repeat_ms: u64| {
            if let Some(timer) = this.manager.borrow_mut().timers.get_mut(&this.id) {
                timer.repeat_ms = repeat_ms;
            }

            Ok(())
        });

        // timer:get_repeat()
        methods.add_method("get_repeat", |_lua, this, ()| {
            let repeat_ms = this
                .manager
                .borrow()
                .timers
                .get(&this.id)
                .map(|timer| timer.repeat_ms)
                .unwrap_or(0);

            Ok(repeat_ms)
        });

        // timer.id (read-only property via __index)
        methods.add_meta_method(LuaMetaMethod::Index, |_lua, this, key: String| {
            match key.as_str() {
                "id" => Ok(LuaValue::Integer(this.id as i64)),
                _ => Ok(LuaValue::Nil),
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    // ========================================================================
    // niri.loop.wait() Tests
    // ========================================================================

    #[test]
    fn wait_sleeps_without_condition() {
        let lua = Lua::new();
        lua.load("niri = {}".to_string()).exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let start = Instant::now();
        let (ok, value): (bool, LuaValue) = lua.load("return niri.loop.wait(50)").eval().unwrap();
        let elapsed = start.elapsed();

        assert!(ok);
        assert!(matches!(value, LuaValue::Nil));
        assert!(elapsed >= Duration::from_millis(45));
    }

    #[test]
    fn wait_returns_early_when_condition_met() {
        let lua = Lua::new();
        lua.load("niri = {}; __flag = false".to_string())
            .exec()
            .unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let start = Instant::now();
        let (ok, value): (bool, LuaValue) = lua
            .load(
                r#"
                local start = niri.loop.now()
                local first = true
                return niri.loop.wait(200, function()
                    if first then
                        first = false
                        return nil
                    end
                    __flag = true
                    return true
                end, 5)
            "#,
            )
            .eval()
            .unwrap();
        let elapsed = start.elapsed();

        assert!(ok);
        assert!(value == LuaValue::Boolean(true));
        assert!(elapsed < Duration::from_millis(200));
    }

    #[test]
    fn wait_times_out_and_returns_false() {
        let lua = Lua::new();
        lua.load("niri = {}".to_string()).exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let start = Instant::now();
        let (ok, value): (bool, LuaValue) = lua
            .load(
                r#"
                return niri.loop.wait(30, function()
                    return false
                end, 5)
            "#,
            )
            .eval()
            .unwrap();
        let elapsed = start.elapsed();

        assert!(!ok);
        assert!(matches!(value, LuaValue::Nil));
        assert!(elapsed >= Duration::from_millis(25));
    }

    #[test]
    fn wait_with_zero_interval_uses_minimum() {
        let lua = Lua::new();
        lua.load("niri = {}; __calls = 0".to_string())
            .exec()
            .unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let (ok, value): (bool, LuaValue) = lua
            .load(
                r#"
                return niri.loop.wait(10, function()
                    __calls = __calls + 1
                    return false
                end, 0)
            "#,
            )
            .eval()
            .unwrap();
        let calls: i64 = lua.globals().get("__calls").unwrap();

        assert!(!ok);
        assert!(matches!(value, LuaValue::Nil));
        assert!(
            calls < 100,
            "expected clamped interval to avoid busy spin, got {}",
            calls
        );
    }

    #[test]
    fn wait_with_negative_interval_clamps_to_minimum() {
        let lua = Lua::new();
        lua.load("niri = {}; __calls = 0".to_string())
            .exec()
            .unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let (ok, value): (bool, LuaValue) = lua
            .load(
                r#"
                return niri.loop.wait(10, function()
                    __calls = __calls + 1
                    return false
                end, -5)
            "#,
            )
            .eval()
            .unwrap();
        let calls: i64 = lua.globals().get("__calls").unwrap();

        assert!(!ok);
        assert!(matches!(value, LuaValue::Nil));
        assert!(
            calls < 100,
            "expected clamped interval to avoid busy spin, got {}",
            calls
        );
    }

    #[test]
    fn wait_treats_table_as_truthy() {
        let lua = Lua::new();
        lua.load("niri = {}".to_string()).exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let (ok, value): (bool, LuaValue) = lua
            .load(
                r#"
                return niri.loop.wait(50, function()
                    return {}
                end, 5)
            "#,
            )
            .eval()
            .unwrap();

        assert!(ok);
        assert!(matches!(value, LuaValue::Table(_)));
    }

    #[test]
    fn wait_treats_nonzero_number_as_truthy() {
        let lua = Lua::new();
        lua.load("niri = {}".to_string()).exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let (ok, value): (bool, LuaValue) = lua
            .load(
                r#"
                return niri.loop.wait(50, function()
                    return 0
                end, 5)
            "#,
            )
            .eval()
            .unwrap();

        assert!(ok);
        match value {
            LuaValue::Integer(0) => {}
            LuaValue::Number(n) => assert_eq!(n, 0.0),
            other => panic!("unexpected value: {:?}", other),
        }
    }

    #[test]
    fn wait_treats_empty_string_as_truthy() {
        let lua = Lua::new();
        lua.load("niri = {}".to_string()).exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let (ok, value): (bool, LuaValue) = lua
            .load(
                r#"
                return niri.loop.wait(50, function()
                    return ""
                end, 5)
            "#,
            )
            .eval()
            .unwrap();

        assert!(ok);
        match value {
            LuaValue::String(s) => {
                assert_eq!(s.to_str().unwrap(), "");
            }
            other => panic!("unexpected value: {:?}", other),
        }
    }

    #[test]
    fn wait_propagates_condition_errors() {
        let lua = Lua::new();
        lua.load("niri = {}".to_string()).exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let err = lua
            .load(
                r#"
                return niri.loop.wait(50, function()
                    error("boom")
                end, 5)
            "#,
            )
            .eval::<(bool, LuaValue)>()
            .unwrap_err();

        match err {
            LuaError::RuntimeError(msg) => assert!(msg.contains("boom")),
            LuaError::CallbackError { cause, .. } => assert!(cause.to_string().contains("boom")),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    // ========================================================================
    // Timer Manager Tests
    // ========================================================================

    #[test]
    fn timer_manager_new_is_empty() {
        let manager = TimerManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.count(), 0);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn timer_manager_register_timer() {
        let lua = Lua::new();
        let mut manager = TimerManager::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        let key = lua.create_registry_value(callback).unwrap();

        manager.register(1, 100, 0, key);

        assert_eq!(manager.count(), 1);
        assert_eq!(manager.active_count(), 1);
        assert!(manager.is_active(1));
    }

    #[test]
    fn timer_manager_stop_timer() {
        let lua = Lua::new();
        let mut manager = TimerManager::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        let key = lua.create_registry_value(callback).unwrap();

        manager.register(1, 100, 0, key);
        assert!(manager.is_active(1));

        manager.stop(1);
        assert!(!manager.is_active(1));
        // Timer still exists, just not active
        assert_eq!(manager.count(), 1);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn timer_manager_close_timer() {
        let lua = Lua::new();
        let mut manager = TimerManager::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        let key = lua.create_registry_value(callback).unwrap();

        manager.register(1, 100, 0, key);
        assert_eq!(manager.count(), 1);

        let removed_key = manager.close(1);
        assert!(removed_key.is_some());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn timer_manager_one_shot_deactivates_after_fire() {
        let lua = Lua::new();
        let mut manager = TimerManager::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        let key = lua.create_registry_value(callback).unwrap();

        // One-shot timer (repeat = 0)
        manager.register(1, 0, 0, key);

        // Fire it
        let _ = manager.fire_timer(1);

        // Should no longer be active
        assert!(!manager.is_active(1));
    }

    #[test]
    fn timer_manager_repeating_stays_active_after_fire() {
        let lua = Lua::new();
        let mut manager = TimerManager::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        let key = lua.create_registry_value(callback).unwrap();

        // Repeating timer (repeat = 100ms)
        manager.register(1, 0, 100, key);

        // Fire it
        let _ = manager.fire_timer(1);

        // Should still be active
        assert!(manager.is_active(1));
    }

    #[test]
    fn timer_manager_get_due_timers() {
        let lua = Lua::new();
        let mut manager = TimerManager::new();

        // Timer with 0 delay should be immediately due
        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        let key = lua.create_registry_value(callback).unwrap();
        manager.register(1, 0, 0, key);

        let due = manager.get_due_timers();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0], 1);
    }

    #[test]
    fn timer_manager_future_timer_not_due() {
        let lua = Lua::new();
        let mut manager = TimerManager::new();

        // Timer with 1 hour delay should not be due
        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        let key = lua.create_registry_value(callback).unwrap();
        manager.register(1, 3600000, 0, key);

        let due = manager.get_due_timers();
        assert!(due.is_empty());
    }

    #[test]
    fn timer_manager_time_until_next() {
        let lua = Lua::new();
        let mut manager = TimerManager::new();

        // No timers = None
        assert!(manager.time_until_next().is_none());

        // Timer with 100ms delay
        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        let key = lua.create_registry_value(callback).unwrap();
        manager.register(1, 100, 0, key);

        let time = manager.time_until_next();
        assert!(time.is_some());
        assert!(time.unwrap() <= Duration::from_millis(100));
    }

    // ========================================================================
    // niri.loop.now() Tests
    // ========================================================================

    #[test]
    fn now_returns_monotonic_time() {
        let lua = Lua::new();
        lua.load("niri = {}").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let now1: u64 = lua.load("return niri.loop.now()").eval().unwrap();

        // Sleep a bit
        thread::sleep(Duration::from_millis(10));

        let now2: u64 = lua.load("return niri.loop.now()").eval().unwrap();

        assert!(now2 >= now1);
        assert!(now2 - now1 >= 10); // At least 10ms passed
    }

    // ========================================================================
    // Lua API Tests
    // ========================================================================

    #[test]
    fn new_timer_creates_userdata() {
        let lua = Lua::new();
        lua.load("niri = {}").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let result: LuaValue = lua.load("return niri.loop.new_timer()").eval().unwrap();
        assert!(matches!(result, LuaValue::UserData(_)));
    }

    #[test]
    fn timer_has_id() {
        let lua = Lua::new();
        lua.load("niri = {}").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let id: i64 = lua
            .load("local t = niri.loop.new_timer(); return t.id")
            .eval()
            .unwrap();
        assert!(id > 0);
    }

    #[test]
    fn timer_start_registers_timer() {
        let lua = Lua::new();
        lua.load("niri = {}").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager.clone()).unwrap();

        lua.load(
            r#"
            local t = niri.loop.new_timer()
            t:start(100, 0, function() end)
        "#,
        )
        .exec()
        .unwrap();

        assert_eq!(manager.borrow().count(), 1);
        assert_eq!(manager.borrow().active_count(), 1);
    }

    #[test]
    fn timer_stop_deactivates() {
        let lua = Lua::new();
        lua.load("niri = {}").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager.clone()).unwrap();

        lua.load(
            r#"
            __timer = niri.loop.new_timer()
            __timer:start(100, 0, function() end)
            __timer:stop()
        "#,
        )
        .exec()
        .unwrap();

        assert_eq!(manager.borrow().active_count(), 0);
    }

    #[test]
    fn timer_close_removes() {
        let lua = Lua::new();
        lua.load("niri = {}").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager.clone()).unwrap();

        lua.load(
            r#"
            local t = niri.loop.new_timer()
            t:start(100, 0, function() end)
            t:close()
        "#,
        )
        .exec()
        .unwrap();

        assert_eq!(manager.borrow().count(), 0);
    }

    #[test]
    fn timer_is_active() {
        let lua = Lua::new();
        lua.load("niri = {}").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let result: (bool, bool, bool) = lua
            .load(
                r#"
            local t = niri.loop.new_timer()
            local before = t:is_active()
            t:start(100, 0, function() end)
            local during = t:is_active()
            t:stop()
            local after = t:is_active()
            return before, during, after
        "#,
            )
            .eval()
            .unwrap();

        assert!(!result.0); // Before start
        assert!(result.1); // After start
        assert!(!result.2); // After stop
    }

    #[test]
    fn timer_get_due_in_returns_positive_for_active() {
        let lua = Lua::new();
        lua.load("niri = {}").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let due_in: u64 = lua
            .load(
                r#"
            local t = niri.loop.new_timer()
            t:start(50, 0, function() end)
            return t:get_due_in()
        "#,
            )
            .eval()
            .unwrap();

        assert!(due_in > 0);
        assert!(due_in <= 50);
    }

    #[test]
    fn timer_get_due_in_returns_zero_when_inactive() {
        let lua = Lua::new();
        lua.load("niri = {}").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let due_in: u64 = lua
            .load(
                r#"
            local t = niri.loop.new_timer()
            t:start(10, 0, function() end)
            t:stop()
            return t:get_due_in()
        "#,
            )
            .eval()
            .unwrap();

        assert_eq!(due_in, 0);
    }

    #[test]
    fn timer_repeat_can_be_read_and_updated() {
        let lua = Lua::new();
        lua.load("niri = {}").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let (initial, updated): (u64, u64) = lua
            .load(
                r#"
            local t = niri.loop.new_timer()
            t:start(0, 123, function() end)
            local before = t:get_repeat()
            t:set_repeat(456)
            local after = t:get_repeat()
            return before, after
        "#,
            )
            .eval()
            .unwrap();

        assert_eq!(initial, 123);
        assert_eq!(updated, 456);
    }

    #[test]
    fn timer_set_repeat_changes_running_interval() {
        let lua = Lua::new();
        lua.load("niri = {}; __count = 0").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager.clone()).unwrap();

        lua.load(
            r#"
            __timer = niri.loop.new_timer()
            __timer:start(0, 50, function()
                __count = __count + 1
            end)
            __timer:set_repeat(5)
        "#,
        )
        .exec()
        .unwrap();

        // First fire (immediate)
        fire_due_timers(&lua, &manager);
        let count_after_first: i64 = lua.globals().get("__count").unwrap();
        assert_eq!(count_after_first, 1);

        // Wait for the updated repeat interval
        thread::sleep(Duration::from_millis(6));

        // Second fire should happen with new repeat interval
        fire_due_timers(&lua, &manager);
        let count_after_second: i64 = lua.globals().get("__count").unwrap();
        assert_eq!(count_after_second, 2);
    }

    #[test]
    fn fire_due_timers_executes_callback() {
        let lua = Lua::new();
        lua.load("niri = {}; __fired = false").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager.clone()).unwrap();

        // Start timer with 0 delay (immediately due)
        lua.load(
            r#"
            local t = niri.loop.new_timer()
            t:start(0, 0, function()
                __fired = true
            end)
        "#,
        )
        .exec()
        .unwrap();

        // Fire due timers
        let (count, errors) = fire_due_timers(&lua, &manager);
        assert_eq!(count, 1);
        assert!(errors.is_empty());

        // Check callback was called
        let fired: bool = lua.globals().get("__fired").unwrap();
        assert!(fired);
    }

    #[test]
    fn fire_due_timers_repeating() {
        let lua = Lua::new();
        lua.load("niri = {}; __count = 0").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager.clone()).unwrap();

        // Start repeating timer with 0 delay
        lua.load(
            r#"
            __timer = niri.loop.new_timer()
            __timer:start(0, 10, function()
                __count = __count + 1
            end)
        "#,
        )
        .exec()
        .unwrap();

        // Fire once
        fire_due_timers(&lua, &manager);
        let count1: i64 = lua.globals().get("__count").unwrap();
        assert_eq!(count1, 1);

        // Timer should still be active
        assert_eq!(manager.borrow().active_count(), 1);

        // Wait for next fire
        thread::sleep(Duration::from_millis(15));

        // Fire again
        fire_due_timers(&lua, &manager);
        let count2: i64 = lua.globals().get("__count").unwrap();
        assert_eq!(count2, 2);
    }

    #[test]
    fn timer_ids_are_unique() {
        let lua = Lua::new();
        lua.load("niri = {}").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager).unwrap();

        let (id1, id2): (i64, i64) = lua
            .load(
                r#"
            local t1 = niri.loop.new_timer()
            local t2 = niri.loop.new_timer()
            return t1.id, t2.id
        "#,
            )
            .eval()
            .unwrap();

        assert_ne!(id1, id2);
    }

    #[test]
    fn timer_restart_updates_timer() {
        let lua = Lua::new();
        lua.load("niri = {}; __value = 0").exec().unwrap();

        let manager = create_timer_manager();
        register_loop_api(&lua, manager.clone()).unwrap();

        // Start timer
        lua.load(
            r#"
            __timer = niri.loop.new_timer()
            __timer:start(0, 0, function()
                __value = 1
            end)
        "#,
        )
        .exec()
        .unwrap();

        // Restart with different callback
        lua.load(
            r#"
            __timer:start(0, 0, function()
                __value = 2
            end)
        "#,
        )
        .exec()
        .unwrap();

        // Fire - should use the new callback
        fire_due_timers(&lua, &manager);

        let value: i64 = lua.globals().get("__value").unwrap();
        assert_eq!(value, 2);
    }
}
