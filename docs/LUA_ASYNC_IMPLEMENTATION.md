# Lua Runtime Async Implementation Guide

This document outlines the implementation plan for improving the niri-lua runtime to address blocking concerns, remove unnecessary dependencies, and provide async capabilities inspired by Neovim's `vim.schedule()` and `vim.loop` APIs.

## Table of Contents

1. [Problem Statement](#problem-statement)
2. [Goals](#goals)
3. [Design Decisions Summary](#design-decisions-summary)
4. [Phase 1: Remove parking_lot Dependency](#phase-1-remove-parking_lot-dependency)
5. [Phase 2: Execution Timeouts](#phase-2-execution-timeouts)
6. [Phase 3: niri.schedule() Implementation](#phase-3-nirischedule-implementation)
7. [Phase 4: Worker Threads](#phase-4-worker-threads)
8. [Phase 5: niri.loop API](#phase-5-niriloop-api)
9. [Testing Strategy](#testing-strategy)
10. [Migration Guide](#migration-guide)

---

## Problem Statement

### Current Issues

1. **Clippy Warning**: `Arc<parking_lot::Mutex<EventHandlers>>` triggers `arc_with_non_send_sync` because `LuaFunction` is not `Send + Sync` without mlua's `send` feature.

2. **Blocking Compositor**: Lua code executes synchronously on the main compositor thread. An infinite loop or heavy computation in a Lua event handler will freeze the entire compositor:
   ```lua
   -- This freezes niri completely
   niri.events:on("window:open", function(ev)
       while true do end
   end)
   ```

3. **No Async Primitives**: Unlike AwesomeWM (`awful.spawn.easy_async`) or Neovim (`vim.schedule`), niri-lua provides no way to defer execution or run code asynchronously.

4. **Unnecessary Dependency**: `parking_lot` is overkill for single-threaded Lua execution.

### Analysis Summary

| Aspect | Current State | Issue |
|--------|---------------|-------|
| Thread model | Single-threaded | Correct for mlua without `send` |
| Shared state | `Arc<parking_lot::Mutex<T>>` | Unnecessary - should use `Rc<RefCell<T>>` |
| Timeout protection | None | Scripts can hang compositor indefinitely |
| Async primitives | None | No way to defer or schedule work |
| Event loop integration | Calloop available but unused by Lua | Missed opportunity |

### Comparison with Other Projects

| Project | Language | Timeout Protection | Async Primitives | Can Block? |
|---------|----------|-------------------|------------------|------------|
| Neovim | Lua | No | Yes (`vim.schedule`, `vim.uv`) | Yes, but avoidable |
| AwesomeWM | Lua | No | Yes (`awful.spawn.easy_async`) | Yes (known issue) |
| Wezterm | Lua | No | Yes (`create_async_function`) | Partially mitigated |
| Qtile | Python | No | Yes (`asyncio`) | Yes, but avoidable |
| **niri** | Lua | **No** | **No** | **Yes** |

---

## Goals

1. **Remove `parking_lot` dependency** - Use `std::sync::Mutex` for shared state (re-entrancy safe)
2. **Add execution timeouts** - Kill runaway scripts via `lua.set_hook()`
3. **Implement `niri.schedule(fn)`** - Defer Lua callbacks to the compositor event loop
4. **Support worker threads** - Allow heavy computation off the main thread (Neovim model)
5. **Provide `niri.loop` API** - Timer capabilities integrated with calloop

---

## Design Decisions Summary

Key architectural decisions made during review:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Shared state primitive | `std::sync::Mutex` (not `RefCell`) | Re-entrancy safety for nested event emission |
| mlua `send` feature | **Not used** | 10-20% performance overhead unacceptable for compositor |
| Worker data transfer | JSON serialization | Simple, no `send` feature required |
| Callback queue flush | Hybrid with limit (16) | Bounds latency while allowing some chaining |
| Event loop wake | Queue + calloop channel | Immediate wake without `send` requirement |
| Worker callback timeout | User-configurable with default | Prevents registry key leaks |
| Timer lifetime | Persist until explicit `close()` | Matches Neovim/libuv semantics |
| LoopHandle access | Unified `LuaContext` in app_data | Direct access, shared callback registry |

---

## Phase 1: Remove parking_lot Dependency

### Rationale

The Lua runtime is single-threaded, but we need re-entrancy safety for nested event emission (e.g., an event handler that triggers another event). Using `std::sync::Mutex` instead of `parking_lot::Mutex` removes the dependency while maintaining safety. We avoid `RefCell` because it would panic on nested borrows.

### Why `std::sync::Mutex` over `RefCell`

Consider this scenario:
```lua
niri.events:on("window:open", function(ev)
    -- This triggers another event emission
    niri.events:emit("custom:nested", {})
end)
```

With `RefCell`:
1. `emit()` calls `borrow_mut()` on handlers
2. Handler executes, calls `emit()` again  
3. Second `emit()` tries `borrow_mut()` → **panic**

With `std::sync::Mutex`:
1. `emit()` locks handlers, clones callback list, unlocks
2. Handler executes callbacks outside the lock
3. Nested `emit()` can acquire lock safely

### Changes Required

#### 1.1 Update `event_system.rs`

```rust
// Before
use std::sync::Arc;
use parking_lot::Mutex;
pub type SharedEventHandlers = Arc<parking_lot::Mutex<EventHandlers>>;

// After
use std::sync::{Arc, Mutex};
pub type SharedEventHandlers = Arc<Mutex<EventHandlers>>;
```

#### 1.2 Update `runtime.rs`

```rust
// Before (line 204)
let handlers = Arc::new(parking_lot::Mutex::new(EventHandlers::new()));

// After
let handlers = Arc::new(std::sync::Mutex::new(EventHandlers::new()));
```

#### 1.3 Update `events_proxy.rs`

Replace all `.lock()` calls with `.lock().unwrap()`:

```rust
// Before
let mut h = handlers.lock();
h.register_handler(event_type, callback, once);

// After
let mut h = handlers.lock().unwrap();
h.register_handler(event_type, callback, once);
```

#### 1.4 Update `config_proxy.rs`

All `parking_lot::Mutex` usages need migration:

```rust
// Line 91: SharedPendingChanges
pub type SharedPendingChanges = Arc<std::sync::Mutex<PendingConfigChanges>>;

// Line 127: ConfigSectionProxy::current_values
pub current_values: Arc<std::sync::Mutex<HashMap<String, serde_json::Value>>>,

// Line 234: ConfigCollectionProxy::current_items  
pub current_items: Arc<std::sync::Mutex<Vec<serde_json::Value>>>,

// Lines 380-384: ConfigProxy internal fields
pub section_proxies: Arc<std::sync::Mutex<HashMap<String, ConfigSectionProxy>>>,
pub collection_proxies: Arc<std::sync::Mutex<HashMap<String, ConfigCollectionProxy>>>,
pub top_level_scalars: Arc<std::sync::Mutex<HashMap<String, serde_json::Value>>>,
```

#### 1.5 Update `Cargo.toml`

```toml
# Remove this line
parking_lot = "0.12.3"
```

### Verification

```bash
cargo clippy -p niri-lua 2>&1 | grep arc_with_non_send_sync
# Should return no results

cargo build -p niri-lua
# Should compile without parking_lot
```

---

## Phase 2: Execution Timeouts

### Rationale

Without timeouts, a malicious or buggy Lua script can freeze the compositor indefinitely. mlua's `set_hook()` allows interrupting execution after a configurable number of VM instructions.

### Timeout Behavior

When a script times out:
1. The Lua call returns `Err(LuaError::external("Script execution timeout"))`
2. The compositor logs a warning via `tracing` (viewable in console or piped to file)
3. The compositor continues running (Lua error is isolated)
4. **The event handler remains registered** - it may timeout again on next trigger
5. **This is the user's responsibility** - logs provide visibility, user must fix their script

### Implementation

#### 2.1 Add Timeout Configuration

```rust
// In runtime.rs

/// Configuration for Lua execution timeouts
pub struct ExecutionLimits {
    /// Maximum instructions per Lua call (0 = unlimited)
    pub max_instructions: u32,
    /// Interval for checking instruction count
    pub check_interval: u32,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_instructions: 1_000_000, // ~100ms on typical hardware
            check_interval: 10_000,      // Check every 10k instructions
        }
    }
}
```

#### 2.2 Install Instruction Hook

```rust
// In LuaRuntime

use std::cell::Cell;
use std::rc::Rc;

pub struct LuaRuntime {
    lua: Lua,
    pub event_system: Option<EventSystem>,
    pub pending_config: Option<SharedPendingChanges>,
    /// Instruction counter for timeout enforcement
    instruction_count: Option<Rc<Cell<u32>>>,
    /// Configured limits
    limits: ExecutionLimits,
}

impl LuaRuntime {
    pub fn new_with_limits(limits: ExecutionLimits) -> LuaResult<Self> {
        let lua = Lua::new();
        lua.load_std_libs(LuaStdLib::ALL_SAFE)?;

        let instruction_count = if limits.max_instructions > 0 {
            let counter = Rc::new(Cell::new(0u32));
            let max = limits.max_instructions;
            let interval = limits.check_interval;

            let counter_clone = counter.clone();
            lua.set_hook(
                mlua::HookTriggers::every_nth_instruction(interval),
                move |_lua, _debug| {
                    let count = counter_clone.get() + interval;
                    counter_clone.set(count);

                    if count > max {
                        tracing::warn!(
                            "Lua script exceeded instruction limit ({} > {})",
                            count, max
                        );
                        Err(mlua::Error::external("Script execution timeout"))
                    } else {
                        Ok(mlua::VmState::Continue)
                    }
                },
            )?;
            Some(counter)
        } else {
            None
        };

        Ok(Self {
            lua,
            event_system: None,
            pending_config: None,
            instruction_count,
            limits,
        })
    }

    /// Reset the instruction counter (call before each Lua invocation)
    pub fn reset_instruction_counter(&self) {
        if let Some(ref counter) = self.instruction_count {
            counter.set(0);
        }
    }

    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> LuaResult<LuaValue> {
        self.reset_instruction_counter();
        let code = std::fs::read_to_string(path)
            .map_err(|e| LuaError::external(format!("Failed to read Lua file: {}", e)))?;
        self.lua.load(&code).eval()
    }
}
```

#### 2.3 Expose Limits to Lua (Optional)

```lua
-- Allow scripts to check their limits (read-only)
niri.runtime.execution_limit  -- Read current limit
niri.runtime.instructions_used  -- Read current count (for debugging)
```

### Performance Considerations

The hook is called every `check_interval` instructions (default 10,000). This adds overhead proportional to script length. Benchmarking is recommended to tune `check_interval`:

- Lower interval = more responsive timeout, higher overhead
- Higher interval = less overhead, less precise timeout

**TODO**: Add benchmarks to measure actual overhead with different intervals.

---

## Phase 3: niri.schedule() Implementation

### Rationale

`niri.schedule(fn)` allows Lua code to defer execution to the next compositor event loop iteration, preventing long-running handlers from blocking frame rendering.

### Design

```lua
-- API
niri.schedule(function()
    -- This runs on the next event loop iteration
    print("Deferred execution")
end)

-- Use case: break up long operations
niri.events:on("window:open", function(ev)
    -- Do quick work synchronously
    local window_id = ev.id

    -- Defer heavy work
    niri.schedule(function()
        -- This won't block the window opening
        do_expensive_analysis(window_id)
    end)
end)
```

### Callback Ordering and Priority

- **FIFO order**: Callbacks execute in the order they were scheduled
- **Lower priority than compositor events**: Compositor input, rendering, and Wayland protocol events are processed first
- **Callbacks scheduled during flush**: Execute up to a limit in the same cycle, remainder deferred to next cycle

### Implementation

#### 3.1 Add Callback Queue to LuaRuntime

```rust
// In runtime.rs

use std::collections::VecDeque;

/// Maximum callbacks to execute per flush cycle
const MAX_CALLBACKS_PER_FLUSH: usize = 16;

/// Maximum queue size to prevent unbounded growth
const MAX_QUEUE_SIZE: usize = 1000;

pub struct LuaRuntime {
    lua: Lua,
    pub event_system: Option<EventSystem>,
    pub pending_config: Option<SharedPendingChanges>,
    instruction_count: Option<Rc<Cell<u32>>>,
    limits: ExecutionLimits,
    /// Queue of callbacks to execute on next flush
    scheduled_callbacks: Rc<RefCell<VecDeque<LuaFunction>>>,
    /// Flag to track if a wake timer is pending
    flush_pending: Rc<Cell<bool>>,
}
```

#### 3.2 Register niri.schedule() Function

```rust
// In niri_api.rs or a new scheduler.rs

pub fn register_scheduler(
    lua: &Lua,
    queue: Rc<RefCell<VecDeque<LuaFunction>>>,
    flush_pending: Rc<Cell<bool>>,
    wake_sender: calloop::channel::Sender<()>,
) -> LuaResult<()> {
    let niri_table: LuaTable = lua.globals().get("niri")?;

    let schedule_fn = lua.create_function(move |_, callback: LuaFunction| {
        let mut q = queue.borrow_mut();
        
        // Enforce queue size limit
        if q.len() >= MAX_QUEUE_SIZE {
            return Err(LuaError::external(format!(
                "Schedule queue full (max {} callbacks)", MAX_QUEUE_SIZE
            )));
        }
        
        q.push_back(callback);
        
        // Wake event loop if not already pending
        if !flush_pending.get() {
            flush_pending.set(true);
            let _ = wake_sender.send(());
        }
        
        Ok(())
    })?;

    niri_table.set("schedule", schedule_fn)?;
    Ok(())
}
```

#### 3.3 Flush Scheduled Callbacks (Hybrid Approach)

```rust
impl LuaRuntime {
    /// Execute scheduled callbacks with limit per cycle.
    ///
    /// Returns the number of callbacks executed and any errors encountered.
    /// Callbacks scheduled during this flush may execute in the same cycle
    /// up to MAX_CALLBACKS_PER_FLUSH total.
    pub fn flush_scheduled(&self) -> (usize, Vec<LuaError>) {
        let mut executed = 0;
        let mut errors = Vec::new();

        // Execute up to limit, allowing newly scheduled callbacks within limit
        while executed < MAX_CALLBACKS_PER_FLUSH {
            let callback = self.scheduled_callbacks.borrow_mut().pop_front();
            match callback {
                Some(cb) => {
                    self.reset_instruction_counter();
                    match cb.call::<()>(()) {
                        Ok(()) => executed += 1,
                        Err(e) => {
                            tracing::error!("Scheduled Lua callback failed: {}", e);
                            errors.push(e);
                            executed += 1;
                        }
                    }
                }
                None => break, // Queue empty
            }
        }

        // Reset flush_pending flag
        if self.scheduled_callbacks.borrow().is_empty() {
            self.flush_pending.set(false);
        }

        (executed, errors)
    }

    /// Check if there are pending scheduled callbacks
    pub fn has_scheduled(&self) -> bool {
        !self.scheduled_callbacks.borrow().is_empty()
    }
}
```

#### 3.4 Integrate with Calloop (Queue + Timer Wake)

In `src/niri.rs`, set up the wake channel and flush during refresh:

```rust
// During Lua runtime initialization
let (wake_tx, wake_rx) = calloop::channel::channel::<()>();

// Insert wake channel as calloop source - wakes event loop when callback scheduled
event_loop.insert_source(wake_rx, |event, _, state| {
    if let calloop::channel::Event::Msg(()) = event {
        // Event loop woken, flush will happen in refresh_and_flush_clients
    }
})?;

// Pass wake_tx to Lua runtime for niri.schedule()
lua_runtime.setup_scheduler(wake_tx);
```

```rust
// In refresh_and_flush_clients()
pub fn refresh_and_flush_clients(&mut self) {
    // ... existing refresh logic ...

    // Flush Lua scheduled callbacks
    if let Some(ref lua_runtime) = self.lua_runtime {
        if lua_runtime.has_scheduled() {
            let (count, errors) = lua_runtime.flush_scheduled();
            if count > 0 {
                tracing::debug!("Executed {} scheduled Lua callbacks", count);
            }
            // Errors already logged in flush_scheduled
        }
    }

    // ... rest of refresh ...
}
```

### Why Queue + Timer (Approach C)

We evaluated three approaches:

| Approach | Event Loop Wake | Requires `send`? | Complexity |
|----------|-----------------|------------------|------------|
| A: Simple queue | No (waits for next event) | No | Low |
| B: Calloop channel | Yes | Yes (`LuaFunction` must be `Send`) | Medium |
| **C: Queue + wake channel** | **Yes** | **No** (channel only sends `()`) | **Medium** |

Approach C gives immediate wake-up (low latency) without requiring the `send` feature, since the channel only transmits a unit signal, not the `LuaFunction` itself.

---

## Phase 4: Worker Threads

### Rationale

For truly heavy computation (large data processing, complex algorithms), even `niri.schedule()` isn't enough - the work still runs on the main thread. Worker threads allow offloading to a separate OS thread.

### Design Decision: Isolated Lua States (Neovim Model)

We use **isolated Lua states** per worker thread, avoiding the mlua `send` feature:

| Approach | Requires `send` feature? | Performance Impact | Complexity |
|----------|-------------------------|-------------------|------------|
| Shared Lua state | Yes | 10-20% overhead on ALL Lua ops | High |
| **Isolated Lua states** | **No** | **No overhead** | **Medium** |

The 10-20% performance penalty from `send` is unacceptable for a compositor. Workers should only return simple data (numbers, strings, plain tables).

### Data Serialization

Workers use JSON serialization for data transfer:
- **Supported**: `nil`, booleans, numbers, strings, arrays, objects
- **Not supported**: functions, metatables, userdata, circular references

This is intentional - workers are for pure computation, not for passing complex Lua objects.

### Implementation

#### 4.1 Worker Thread API

```lua
-- Create a worker with a Lua script
local worker = niri.worker.new([[
    -- This runs in a separate thread with isolated Lua state
    -- Limited API: no niri.events, no niri.action, only pure computation
    local result = 0
    for i = 1, 1000000 do
        result = result + i
    end
    return result
]])

-- Execute with callback (non-blocking)
-- Optional timeout in milliseconds (default: 300000 = 5 minutes)
worker:run(function(result, err)
    if err then
        print("Worker error:", err)
    else
        print("Worker result:", result)
    end
end)

-- With arguments and custom timeout
local worker2 = niri.worker.new([[
    local args = ...
    return args.x + args.y
]])
worker2:run({ x = 10, y = 20 }, {
    timeout = 60000,  -- 1 minute timeout
    callback = function(result, err)
        print("Sum:", result)  -- 30
    end
})

-- Cancel a pending worker (cleans up callback)
worker:cancel()
```

#### 4.2 Callback Storage and Registry Management

Since `LuaFunction` is not `Send`, callbacks are stored in the Lua registry on the main thread using the **unified `CallbackStore`** (defined in Phase 5's `LuaContext`).

Workers use the same callback infrastructure as timers:

```rust
// Workers store callbacks with CallbackKind::Worker
let callback_id = ctx.callbacks.borrow_mut().store(
    registry_key,
    timeout,
    CallbackKind::Worker,
);
```

See [Phase 5: LuaContext Design](#loophandle-access-pattern-resolved) for the full `CallbackStore` implementation.

#### 4.3 Worker Result Delivery via Calloop Channel

```rust
use calloop::channel::Sender;
use serde_json::Value as JsonValue;

/// Result from a worker thread
pub struct WorkerResult {
    pub id: u64,
    pub result: Result<JsonValue, String>,
}

/// Set up worker result delivery channel (called during init)
pub fn setup_worker_channel(
    event_loop: &calloop::LoopHandle<State>,
) -> Sender<WorkerResult> {
    let (tx, rx) = calloop::channel::channel::<WorkerResult>();

    event_loop.insert_source(rx, |event, _, state| {
        if let calloop::channel::Event::Msg(worker_result) = event {
            deliver_worker_result(state, worker_result);
        }
    }).expect("Failed to insert worker result channel");

    tx
}

fn deliver_worker_result(state: &mut State, worker_result: WorkerResult) {
    let Some(ref runtime) = state.niri.lua_runtime else { return };
    let lua = runtime.inner();

    // Access unified callback store via LuaContext
    let ctx: &LuaContext = match lua.app_data_ref() {
        Some(c) => c,
        None => return,
    };

    // Retrieve callback from unified store
    let Some(pending) = ctx.callbacks.borrow_mut().take(worker_result.id) else {
        tracing::debug!("Worker {} result ignored (cancelled or timed out)", worker_result.id);
        return;
    };

    let callback: LuaFunction = match lua.registry_value(&pending.registry_key) {
        Ok(cb) => cb,
        Err(e) => {
            tracing::error!("Failed to retrieve worker callback: {}", e);
            return;
        }
    };

    // Clean up registry
    lua.remove_registry_value(pending.registry_key).ok();

    // Call the callback with (result, err) or (nil, err_string)
    runtime.reset_instruction_counter();
    let call_result = match worker_result.result {
        Ok(json_value) => {
            match json_to_lua(lua, &json_value) {
                Ok(lua_value) => callback.call::<()>((lua_value, LuaValue::Nil)),
                Err(e) => callback.call::<()>((LuaValue::Nil, e.to_string())),
            }
        }
        Err(err_msg) => {
            callback.call::<()>((LuaValue::Nil, err_msg))
        }
    };

    if let Err(e) = call_result {
        tracing::error!("Worker callback execution failed: {}", e);
    }
}
```

#### 4.4 Worker Execution in Isolated State

```rust
/// A worker thread with an isolated Lua state
pub struct LuaWorker {
    script: String,
    pending_id: Option<u64>,
}

impl LuaWorker {
    pub fn new(script: String) -> Self {
        Self { script, pending_id: None }
    }

    /// Run the worker script with arguments and callback
    pub fn run(
        &mut self,
        lua: &Lua,
        args: Option<LuaValue>,
        timeout_ms: u64,
        callback: LuaFunction,
    ) -> LuaResult<()> {
        // Access unified LuaContext
        let ctx: &LuaContext = lua.app_data_ref()
            .ok_or_else(|| LuaError::external("LuaContext not initialized"))?;
        
        // Store callback in unified registry
        let registry_key = lua.create_registry_value(callback)?;
        let timeout = Duration::from_millis(timeout_ms);
        let worker_id = ctx.callbacks.borrow_mut().store(
            registry_key,
            timeout,
            CallbackKind::Worker,
        );
        self.pending_id = Some(worker_id);

        // Serialize arguments
        let args_json = match args {
            Some(v) => Some(lua_to_json(&v)?),
            None => None,
        };

        // Clone sender (Sender is Clone + Send)
        let tx = ctx.worker_tx.clone();
        let script = self.script.clone();

        // Spawn worker thread with only the data it needs
        std::thread::spawn(move || {
            let result = execute_in_isolated_lua(&script, args_json);
            let _ = tx.send(WorkerResult { id: worker_id, result });
        });

        Ok(())
    }

    /// Cancel the pending worker (callback will be cleaned up)
    pub fn cancel(&mut self, lua: &Lua) -> LuaResult<()> {
        if let Some(id) = self.pending_id.take() {
            let ctx: &LuaContext = lua.app_data_ref()
                .ok_or_else(|| LuaError::external("LuaContext not initialized"))?;
            
            if let Some(cb) = ctx.callbacks.borrow_mut().take(id) {
                lua.remove_registry_value(cb.registry_key).ok();
            }
        }
        Ok(())
    }
}

fn execute_in_isolated_lua(script: &str, args: Option<JsonValue>) -> Result<JsonValue, String> {
    // Create fresh isolated Lua state
    let lua = Lua::new();
    lua.load_std_libs(LuaStdLib::ALL_SAFE).map_err(|e| e.to_string())?;

    // Install timeout hook (workers get same limits as main runtime)
    // ... (similar to Phase 2 implementation)

    // Set up arguments
    if let Some(args) = args {
        let lua_args = json_to_lua(&lua, &args).map_err(|e| e.to_string())?;
        lua.globals().set("__worker_args", lua_args).map_err(|e| e.to_string())?;
    }

    // Wrap script to accept arguments via ...
    let wrapped = format!(
        "local __args = __worker_args or nil; return (function(...) {} end)(__args)",
        script
    );

    let result: LuaValue = lua.load(&wrapped).eval().map_err(|e| e.to_string())?;

    lua_to_json(&result).map_err(|e| e.to_string())
}
```

#### 4.5 Periodic Cleanup Integration

```rust
// In refresh_and_flush_clients() or a periodic timer
pub fn refresh_and_flush_clients(&mut self) {
    // ... existing logic ...

    // Periodically clean up stale callbacks (every ~60 seconds)
    // This cleans up both worker AND timer callbacks
    if self.last_callback_cleanup.elapsed() > Duration::from_secs(60) {
        if let Some(ref lua_runtime) = self.lua_runtime {
            let lua = lua_runtime.inner();
            if let Some(ctx) = lua.app_data_ref::<LuaContext>() {
                let cleaned = ctx.callbacks.borrow_mut().cleanup_stale(lua);
                if cleaned > 0 {
                    tracing::info!("Cleaned up {} stale callbacks", cleaned);
                }
            }
        }
        self.last_callback_cleanup = Instant::now();
    }
}
```

### Worker Cancellation

Workers can be cancelled, which cleans up the callback registry:

```lua
local worker = niri.worker.new([[ ... ]])
worker:run(args, { callback = function(r) ... end })

-- Later, if result no longer needed:
worker:cancel()  -- Callback cleaned up, result ignored if/when worker completes
```

**Note**: Cancellation does not stop the worker thread itself (that would require unsafe mechanisms). The worker continues to completion, but its result is discarded.

---

## Phase 5: niri.loop API

### Rationale

A `niri.loop` API (inspired by Neovim's `vim.uv`) provides direct access to the compositor's event loop for timers and time-related operations.

### Scope

Focus on compositor-relevant operations:

| Feature | Include? | Rationale |
|---------|----------|-----------|
| Timers | Yes | Animations, polling, debouncing |
| Time functions | Yes | Get current time, monotonic clock |
| Process spawn | **No** | Already exists via `niri.action.spawn()` |
| Filesystem | No | Security risk, out of scope |
| Network | No | Out of scope for a compositor |

### API Design

```lua
-- Timers
local timer = niri.loop.new_timer()
timer:start(1000, 0, function()  -- 1000ms delay, no repeat
    print("Timer fired!")
end)
timer:stop()
timer:close()

-- Repeating timer
local repeating = niri.loop.new_timer()
repeating:start(0, 500, function()  -- No delay, repeat every 500ms
    print("Tick")
end)

-- Time functions
local now = niri.loop.now()  -- Monotonic time in milliseconds
```

### Timer Lifetime Management (Resolved)

**Decision**: Follow Neovim's model - timers **persist until explicitly closed**.

| Aspect | Behavior |
|--------|----------|
| GC of timer handle | Timer **continues running** |
| Callback invocation | **Still fires** after handle is GC'd |
| Resource cleanup | **Explicit `timer:close()` required** |
| Event loop | Stays alive while timer is active |

This matches libuv/Neovim semantics and is simpler to implement than auto-stop-on-GC.

#### Recommended Patterns

```lua
-- One-shot timer with auto-cleanup
local function set_timeout(timeout_ms, callback)
    local timer = niri.loop.new_timer()
    timer:start(timeout_ms, 0, function()
        timer:stop()
        timer:close()  -- Explicit cleanup
        callback()
    end)
    return timer
end

-- Repeating timer (user must close when done)
local ticker = niri.loop.new_timer()
ticker:start(0, 1000, function()
    print("tick")
end)
-- Later: ticker:close()
```

#### Documentation Requirements

Users must be clearly informed that:
1. Timers continue running even if the Lua variable is garbage collected
2. `timer:close()` must be called to release resources
3. Helper functions like `set_timeout` handle cleanup automatically

### LoopHandle Access Pattern (Resolved)

**Decision**: Use a unified `LuaContext` struct stored in Lua app_data.

Since `LoopHandle` is `Clone`, we store it along with other shared infrastructure in a single struct accessible from all Lua functions on the main thread.

#### Unified LuaContext Design

```rust
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use calloop::{LoopHandle, RegistrationToken};
use calloop::channel::Sender;
use mlua::prelude::*;

/// Unified context for all Lua async operations
/// Stored in lua.set_app_data() and accessed via lua.app_data_ref()
pub struct LuaContext {
    /// Event loop handle (main thread only, Clone)
    pub loop_handle: LoopHandle<'static, State>,
    
    /// Unified callback storage for timers AND workers
    pub callbacks: Rc<RefCell<CallbackStore>>,
    
    /// Channel to send worker results back to main thread
    /// (Sender is Clone + Send, can be given to worker threads)
    pub worker_tx: Sender<WorkerResult>,
    
    /// Channel to wake event loop for niri.schedule()
    pub wake_tx: Sender<()>,
    
    /// Active timer registrations (for cleanup)
    pub active_timers: Rc<RefCell<HashMap<u64, RegistrationToken>>>,
}

/// Unified callback storage for async operations
pub struct CallbackStore {
    pending: HashMap<u64, PendingCallback>,
    next_id: u64,
}

pub struct PendingCallback {
    pub registry_key: LuaRegistryKey,
    pub created_at: Instant,
    pub timeout: Duration,
    pub kind: CallbackKind,
}

pub enum CallbackKind {
    Timer,
    Worker,
}

impl CallbackStore {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            next_id: 0,
        }
    }

    /// Store a callback, returns unique ID
    pub fn store(&mut self, key: LuaRegistryKey, timeout: Duration, kind: CallbackKind) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.pending.insert(id, PendingCallback {
            registry_key: key,
            created_at: Instant::now(),
            timeout,
            kind,
        });
        id
    }

    /// Retrieve and remove a callback by ID
    pub fn take(&mut self, id: u64) -> Option<PendingCallback> {
        self.pending.remove(&id)
    }

    /// Clean up stale callbacks (call periodically)
    pub fn cleanup_stale(&mut self, lua: &Lua) -> usize {
        let now = Instant::now();
        let mut removed = 0;
        self.pending.retain(|id, cb| {
            if now.duration_since(cb.created_at) > cb.timeout {
                tracing::warn!("Callback {} ({:?}) timed out, cleaning up", id, cb.kind);
                lua.remove_registry_value(cb.registry_key.clone()).ok();
                removed += 1;
                false
            } else {
                true
            }
        });
        removed
    }

    /// Clean up all callbacks (call on shutdown)
    pub fn cleanup_all(&mut self, lua: &Lua) {
        for (_, cb) in self.pending.drain() {
            lua.remove_registry_value(cb.registry_key).ok();
        }
    }
}
```

#### Benefits of Unified Design

| Benefit | Description |
|---------|-------------|
| Single app_data struct | Clean organization, no slot conflicts |
| Shared callback registry | Same cleanup logic for timers and workers |
| Direct LoopHandle access | Timers created directly, no command channel indirection |
| Consistent patterns | Both timers and workers use ID-based callback lookup |

#### How Each Component Uses LuaContext

| Component | Accesses | Thread |
|-----------|----------|--------|
| `niri.schedule()` | `wake_tx`, callback queue | Main |
| `niri.loop.new_timer()` | `loop_handle`, `callbacks`, `active_timers` | Main |
| Timer callbacks | `callbacks` (to look up and execute) | Main |
| `niri.worker.new()` | `worker_tx.clone()`, `callbacks` | Main |
| Worker thread | Only the cloned `Sender<WorkerResult>` | Worker |
| Worker result delivery | `callbacks` (to look up and execute) | Main |

#### Initialization

```rust
pub fn setup_lua_context(
    lua: &Lua,
    loop_handle: LoopHandle<'static, State>,
    worker_tx: Sender<WorkerResult>,
    wake_tx: Sender<()>,
) {
    let ctx = LuaContext {
        loop_handle,
        callbacks: Rc::new(RefCell::new(CallbackStore::new())),
        worker_tx,
        wake_tx,
        active_timers: Rc::new(RefCell::new(HashMap::new())),
    };
    lua.set_app_data(ctx);
}

// Accessing from Lua functions:
fn some_lua_function(lua: &Lua, ...) -> LuaResult<...> {
    let ctx: &LuaContext = lua.app_data_ref()
        .ok_or_else(|| LuaError::external("LuaContext not initialized"))?;
    
    // Now have access to loop_handle, callbacks, etc.
    ctx.loop_handle.insert_source(...);
}
```

### Implementation

With the unified LuaContext design, timer implementation is straightforward:

```rust
impl LuaUserData for LuaTimer {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("start", |lua, this, (delay_ms, repeat_ms, callback): (u64, u64, LuaFunction)| {
            let ctx: &LuaContext = lua.app_data_ref()
                .ok_or_else(|| LuaError::external("LuaContext not initialized"))?;
            
            // Store callback in unified registry
            let registry_key = lua.create_registry_value(callback)?;
            let callback_id = ctx.callbacks.borrow_mut().store(
                registry_key,
                Duration::from_secs(300), // 5 min timeout
                CallbackKind::Timer,
            );
            this.callback_id = Some(callback_id);
            
            // Create actual calloop timer
            let timer = calloop::timer::Timer::from_duration(Duration::from_millis(delay_ms));
            let callbacks = ctx.callbacks.clone();
            
            let token = ctx.loop_handle.insert_source(timer, move |_, _, state| {
                // Look up and execute callback
                if let Some(cb) = callbacks.borrow_mut().take(callback_id) {
                    if let Some(ref rt) = state.niri.lua_runtime {
                        let lua = rt.inner();
                        if let Ok(func) = lua.registry_value::<LuaFunction>(&cb.registry_key) {
                            rt.reset_instruction_counter();
                            if let Err(e) = func.call::<()>(()) {
                                tracing::error!("Timer callback failed: {}", e);
                            }
                        }
                        lua.remove_registry_value(cb.registry_key).ok();
                    }
                }
                
                if repeat_ms > 0 {
                    calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(repeat_ms))
                } else {
                    calloop::timer::TimeoutAction::Drop
                }
            })?;
            
            ctx.active_timers.borrow_mut().insert(callback_id, token);
            Ok(())
        });

        methods.add_method_mut("stop", |lua, this, ()| {
            if let Some(id) = this.callback_id {
                let ctx: &LuaContext = lua.app_data_ref()
                    .ok_or_else(|| LuaError::external("LuaContext not initialized"))?;
                
                if let Some(token) = ctx.active_timers.borrow_mut().remove(&id) {
                    ctx.loop_handle.remove(token);
                }
            }
            Ok(())
        });

        methods.add_method_mut("close", |lua, this, ()| {
            if let Some(id) = this.callback_id.take() {
                let ctx: &LuaContext = lua.app_data_ref()
                    .ok_or_else(|| LuaError::external("LuaContext not initialized"))?;
                
                // Remove timer from event loop
                if let Some(token) = ctx.active_timers.borrow_mut().remove(&id) {
                    ctx.loop_handle.remove(token);
                }
                
                // Clean up callback from registry
                if let Some(cb) = ctx.callbacks.borrow_mut().take(id) {
                    lua.remove_registry_value(cb.registry_key).ok();
                }
            }
            Ok(())
        });
    }
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_limit_timeout() {
        let rt = LuaRuntime::new_with_limits(ExecutionLimits {
            max_instructions: 1000,
            log_timeouts: false,
        }).unwrap();

        let result = rt.load_string("while true do end");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timeout"));
    }

    #[test]
    fn test_schedule_callback() {
        let rt = LuaRuntime::new().unwrap();
        rt.load_string(r#"
            __scheduled = false
            niri.schedule(function()
                __scheduled = true
            end)
        "#).unwrap();

        // Not executed yet
        let scheduled: bool = rt.inner().globals().get("__scheduled").unwrap();
        assert!(!scheduled);

        // Flush
        rt.flush_scheduled();

        // Now executed
        let scheduled: bool = rt.inner().globals().get("__scheduled").unwrap();
        assert!(scheduled);
    }

    #[test]
    fn test_worker_isolated_state() {
        let worker = LuaWorker::new("return 1 + 1".to_string());
        let rx = worker.run(None);
        let result = rx.recv().unwrap();
        assert_eq!(result.unwrap(), serde_json::json!(2));
    }
}
```

### Integration Tests

```rust
// In tests/lua_async.rs

#[test]
fn test_schedule_integrates_with_event_loop() {
    // Set up minimal compositor state with calloop
    // Register Lua runtime
    // Execute script that schedules callback
    // Run event loop iteration
    // Verify callback executed
}
```

### Manual Testing

```lua
-- Test script: test_async.lua

-- Test 1: Schedule
print("Before schedule")
niri.schedule(function()
    print("Scheduled callback executed")
end)
print("After schedule (callback not yet run)")

-- Test 2: Timer
local timer = niri.loop.new_timer()
timer:start(1000, 0, function()
    print("Timer fired after 1 second")
end)

-- Test 3: Worker
local worker = niri.worker.new([[
    local sum = 0
    for i = 1, 1000000 do sum = sum + i end
    return sum
]])
worker:run(function(result)
    print("Worker result:", result)
end)

-- Test 4: Timeout protection
-- This should NOT freeze the compositor
-- niri.events:on("test", function()
--     while true do end  -- Should timeout
-- end)
```

---

## Migration Guide

### For Users

Most changes are additive. Existing Lua code continues to work, but may now timeout if it runs too long.

#### Execution Limits

Scripts that run more than ~1 million instructions will timeout. Break up long-running code using `niri.schedule()`:

```lua
-- Before (might timeout)
for i = 1, 10000000 do
    process(i)
end

-- After (chunked processing)
local i = 0
local function process_chunk()
    local chunk_end = math.min(i + 10000, 10000000)
    while i < chunk_end do
        process(i)
        i = i + 1
    end
    if i < 10000000 then
        niri.schedule(process_chunk)
    end
end
process_chunk()
```

#### Heavy Computation

Move CPU-intensive code to workers:

```lua
-- Before (blocks compositor)
local result = expensive_computation(data)

-- After (runs in background)
niri.worker.new([[
    return expensive_computation(...)
]]):run(data, {
    callback = function(result)
        use_result(result)
    end
})
```

### For Developers

#### API Changes

| Old | New | Notes |
|-----|-----|-------|
| `Arc<parking_lot::Mutex<T>>` | `Arc<std::sync::Mutex<T>>` | Internal, re-entrancy safe |
| N/A | `LuaRuntime::new_with_limits()` | New constructor with timeout config |
| N/A | `LuaRuntime::flush_scheduled()` | Call from event loop |
| N/A | `LuaRuntime::reset_instruction_counter()` | Call before Lua invocations |
| N/A | `niri.schedule(fn)` | New Lua API |
| N/A | `niri.worker.new(script)` | New Lua API |
| N/A | `niri.loop.*` | New Lua API (Phase 5, deferred) |

#### Integration Points

1. Call `flush_scheduled()` in the compositor's refresh cycle
2. Set up wake channel for `niri.schedule()` to wake event loop
3. Set up worker result channel for delivering worker results
4. Periodically call `cleanup_stale()` on worker callback store
5. Call `cleanup_all()` on runtime shutdown

---

## Implementation Order

1. **Phase 1**: Remove `parking_lot` → `std::sync::Mutex` (low risk, immediate clippy fix)
2. **Phase 2**: Add execution timeouts (critical safety improvement)
3. **Phase 3**: Implement `niri.schedule()` (enables async patterns)
4. **Phase 4**: Add worker threads (for heavy computation)
5. **Phase 5**: Implement `niri.loop` timers (deferred until Phases 1-4 stable)

Phases 1-3 are the minimum viable implementation. Phase 4 builds on Phase 3's infrastructure. Phase 5 has open design questions and is lower priority.

### Dependencies

```
Phase 1 (parking_lot removal)
    ↓
Phase 2 (execution timeouts) ──────────────────┐
    ↓                                          │
Phase 3 (niri.schedule) ←── uses timeout reset │
    ↓                                          │
Phase 4 (workers) ←── uses calloop channel,    │
    │                 registry pattern,        │
    │                 JSON serialization       │
    ↓                                          │
Phase 5 (timers) ←── similar registry pattern ─┘
```

---

## References

- [mlua documentation](https://docs.rs/mlua/latest/mlua/)
- [Neovim vim.schedule](https://neovim.io/doc/user/lua.html#vim.schedule())
- [Neovim vim.uv (libuv)](https://neovim.io/doc/user/luvref.html)
- [AwesomeWM async spawn](https://awesomewm.org/doc/api/libraries/awful.spawn.html)
- [calloop executor](https://docs.rs/calloop/latest/calloop/futures/)
