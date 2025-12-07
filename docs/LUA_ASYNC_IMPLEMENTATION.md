# Lua Runtime Async Implementation Guide

This document outlines the implementation plan for improving the niri-lua runtime to address blocking concerns, remove unnecessary dependencies, and provide async capabilities inspired by Neovim's `vim.schedule()` and `vim.loop` APIs.

## Table of Contents

1. [Problem Statement](#problem-statement)
2. [Goals](#goals)
3. [Design Philosophy: Luau with Timeout Protection](#design-philosophy-luau-with-timeout-protection)
4. [Design Decisions Summary](#design-decisions-summary)
5. [Phase 1: Remove parking_lot Dependency](#phase-1-remove-parking_lot-dependency)
6. [Phase 2: Execution Timeouts with Luau](#phase-2-execution-timeouts-with-luau)
7. [Phase 3: niri.schedule() Implementation](#phase-3-nirischedule-implementation)
8. [Phase 4: niri.loop API](#phase-4-niriloop-api)
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
| Shared state | `Arc<parking_lot::Mutex<T>>` | Unnecessary - should use `std::sync::Mutex` |
| Timeout protection | None | Scripts can hang compositor indefinitely |
| Async primitives | None | No way to defer or schedule work |
| Event loop integration | Calloop available but unused by Lua | Missed opportunity |

### Comparison with Other Projects

| Project | Language | Timeout Protection | Async Primitives | Can Block? |
|---------|----------|-------------------|------------------|------------|
| Neovim | Lua (LuaJIT) | No | Yes (`vim.schedule`, `vim.uv`) | Yes, but avoidable |
| AwesomeWM | Lua | No | Yes (`awful.spawn.easy_async`) | Yes (known issue) |
| Wezterm | Lua | No | Yes (`create_async_function`) | Partially mitigated |
| Qtile | Python | No | Yes (`asyncio`) | Yes, but avoidable |
| **niri** | **Luau** | **Yes** (`set_interrupt`) | **Yes** (after this work) | **No** (protected) |

---

## Goals

1. **Remove `parking_lot` dependency** - Use `std::sync::Mutex` for shared state (re-entrancy safe)
2. **Add timeout protection** - Use Luau's `set_interrupt` for wall-clock timeouts
3. **Implement `niri.schedule(fn)`** - Defer Lua callbacks to the compositor event loop
4. **Provide `niri.loop` API** - Timer capabilities integrated with calloop

---

## Design Philosophy: Luau with Timeout Protection

### Why Luau Instead of LuaJIT?

We use **Luau** (Roblox's Lua dialect) instead of LuaJIT for one critical reason: **reliable timeout protection**.

**The LuaJIT Problem**: LuaJIT's debug hooks (required for instruction counting) don't fire when the JIT compiler is active. To make timeouts work with LuaJIT, we'd have to either:
1. Disable JIT around all user code (10-20% performance hit), or
2. Use unsafe signal handlers with `pthread_kill` (~150 LOC of unsafe code)

**The Luau Solution**: Luau provides `set_interrupt`, a callback that fires periodically during execution, even in optimized code. This allows clean, safe wall-clock timeout protection.

### Luau Compatibility

Luau is based on Lua 5.1 (same as LuaJIT/Neovim), so most Lua code is compatible:

| Direction | Compatibility |
|-----------|---------------|
| Neovim Lua → niri | ✅ High - standard Lua 5.1 code works |
| niri → Neovim Lua | ⚠️ Partial - avoid Luau-specific syntax |

**Luau-specific features** (not available in LuaJIT):
- Type annotations: `local x: number = 5`
- `continue` statement
- Compound assignment: `x += 1`
- Interpolated strings: `` `Hello {name}!` ``

Users can choose to use these features (better tooling) or stick to portable Lua 5.1 syntax.

### Timeout Protection

With Luau's `set_interrupt`, we implement wall-clock timeout protection:

| Scenario | What Happens |
|----------|--------------|
| `while true do end` in config | Script times out after 1 second, error reported |
| `while true do end` in event handler | Callback times out, compositor continues |
| `while true do end` in REPL | Command times out, REPL remains usable |
| Long computation in handler | Times out, use `niri.schedule()` or workers |

**This is a major improvement over Neovim/AwesomeWM** which have no timeout protection.

---

## Design Decisions Summary

Key architectural decisions:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Lua runtime | **Luau** (not LuaJIT) | `set_interrupt` for reliable timeouts |
| Compiler optimization | **Level 2** | Function inlining, loop unrolling, constant folding |
| Shared state primitive | `std::sync::Mutex` (not `RefCell`) | Re-entrancy safety for nested event emission |
| mlua `send` feature | **Not used** | Performance overhead unacceptable |
| Timeout mechanism | `set_interrupt` with wall-clock | Clean, no unsafe code, reliable |
| Default timeout | 1 second | Sufficient for config/events, catches infinite loops |
| Callback queue flush | Hybrid with limit (16) | Bounds latency while allowing some chaining |
| Timer lifetime | Persist until explicit `close()` | Matches Neovim/libuv semantics |

---

## Phase 1: Remove parking_lot Dependency

**Status: ✅ Complete**

### Rationale

The Lua runtime is single-threaded, but we need re-entrancy safety for nested event emission. Using `std::sync::Mutex` instead of `parking_lot::Mutex` removes the dependency while maintaining safety.

### Changes Made

1. Replaced all `parking_lot::Mutex` with `std::sync::Mutex`
2. Updated all `.lock()` calls to `.lock().unwrap()`
3. Removed `parking_lot` from `Cargo.toml`
4. Added `#[allow(clippy::arc_with_non_send_sync)]` with explanation

---

## Phase 2: Execution Timeouts with Luau

**Status: ✅ Complete**

### Rationale

Timeout protection prevents runaway scripts from freezing the compositor. Luau's `set_interrupt` provides this capability without sacrificing performance or requiring unsafe code.

### Implementation

```rust
/// Configuration for Lua execution timeouts.
#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    /// Maximum wall-clock time per Lua execution.
    /// Default is 1 second. Duration::ZERO = unlimited.
    pub timeout: Duration,
}

impl LuaRuntime {
    pub fn new_with_limits(limits: ExecutionLimits) -> LuaResult<Self> {
        let lua = Lua::new();
        let deadline = Rc::new(Cell::new(None::<Instant>));

        // Set up Luau interrupt callback
        if limits.timeout > Duration::ZERO {
            let deadline_clone = deadline.clone();
            lua.set_interrupt(move |_lua| {
                if let Some(dl) = deadline_clone.get() {
                    if Instant::now() > dl {
                        return Err(LuaError::external("Script execution timeout"));
                    }
                }
                Ok(LuaVmState::Continue)
            });
        }
        // ...
    }

    fn set_deadline(&self) {
        if self.limits.timeout > Duration::ZERO {
            self.deadline.set(Some(Instant::now() + self.limits.timeout));
        }
    }

    fn clear_deadline(&self) {
        self.deadline.set(None);
    }
}
```

### API

```rust
// Default: 1 second timeout
let rt = LuaRuntime::new()?;

// Custom timeout
let rt = LuaRuntime::new_with_limits(ExecutionLimits::with_timeout(
    Duration::from_millis(500)
))?;

// Unlimited (for trusted code only)
let rt = LuaRuntime::new_with_limits(ExecutionLimits::unlimited())?;

// Execute with timeout protection
let result = rt.eval_with_timeout::<i64>("return 1 + 1")?;
let result = rt.call_with_timeout::<()>(&callback, args)?;
```

### Key Properties

- **Wall-clock time**: Uses `Instant::now()`, not instruction counting
- **Configurable**: Default 1 second, can be customized or disabled
- **Clean termination**: Returns `LuaError`, no undefined behavior
- **No performance impact**: Interrupt callback is lightweight
- **Automatic**: `load_file()`, `load_string()`, `flush_scheduled()` all use timeouts

### Compiler Optimization

In addition to timeout protection, we use Luau's `Compiler` with optimization level 2:

```rust
let compiler = Compiler::new()
    .set_optimization_level(2)  // Aggressive optimizations
    .set_debug_level(1);        // Keep line info for errors

// All code loading uses compiled bytecode
let bytecode = self.compiler.compile(code)?;
let result = self.lua.load(bytecode).eval();
```

**Optimization level 2 enables:**
- Function inlining
- Loop unrolling
- Constant folding
- Dead code elimination

**Debug level 1 preserves:**
- Line numbers in error messages
- Function names for stack traces

---

## Phase 3: niri.schedule() Implementation

**Status: ✅ Complete**

### Rationale

`niri.schedule(fn)` allows Lua code to defer execution to the next compositor event loop iteration, preventing long-running handlers from blocking frame rendering.

### API

```lua
-- Defer execution to next event loop iteration
niri.schedule(function()
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

### Implementation Details

- Callbacks stored in `VecDeque<LuaRegistryKey>`
- Maximum 16 callbacks executed per flush cycle (bounds latency)
- Maximum 1000 queued callbacks (prevents unbounded growth)
- Callbacks scheduled during flush may execute in same cycle up to limit
- Errors are logged but don't stop other callbacks

### Key Methods

```rust
impl LuaRuntime {
    /// Initialize scheduler, registers niri.schedule()
    pub fn init_scheduler(&self) -> LuaResult<()>;
    
    /// Execute queued callbacks (call from event loop)
    pub fn flush_scheduled(&self) -> (usize, Vec<LuaError>);
    
    /// Check if callbacks are pending
    pub fn has_scheduled(&self) -> bool;
    
    /// Get number of queued callbacks
    pub fn scheduled_count(&self) -> usize;
}
```

---

## Phase 4: niri.loop API

**Status: ✅ Complete**

### Rationale

A `niri.loop` API (inspired by Neovim's `vim.uv`) provides direct access to the compositor's event loop for timers and time-related operations.

### Scope

| Feature | Include? | Rationale |
|---------|----------|-----------|
| Timers | Yes | Animations, polling, debouncing |
| Time functions | Yes | Get current time, monotonic clock |
| Process spawn | **No** | Already exists via `niri.action.spawn()` |
| Filesystem | No | Security risk, out of scope |
| Network | No | Out of scope for a compositor |

### API

```lua
-- One-shot timer
local timer = niri.loop.new_timer()
timer:start(1000, 0, function()  -- 1000ms delay, no repeat
    print("Timer fired!")
    timer:close()  -- Clean up
end)

-- Repeating timer
local repeating = niri.loop.new_timer()
repeating:start(0, 500, function()  -- No delay, repeat every 500ms
    print("Tick")
end)
-- Later: repeating:close()

-- Time functions
local now = niri.loop.now()  -- Monotonic time in milliseconds
```

### Timer Lifetime

Following Neovim's model, timers **persist until explicitly closed**:

| Aspect | Behavior |
|--------|----------|
| GC of timer handle | Timer **continues running** |
| Callback invocation | **Still fires** after handle is GC'd |
| Resource cleanup | **Explicit `timer:close()` required** |

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
```

### Unified LuaContext Design

All async operations share a unified context stored in Lua app_data:

```rust
pub struct LuaContext {
    /// Event loop handle (main thread only, Clone)
    pub loop_handle: LoopHandle<'static, State>,
    
    /// Unified callback storage for timers AND workers
    pub callbacks: Rc<RefCell<CallbackStore>>,
    
    /// Channel to send worker results back to main thread
    pub worker_tx: Sender<WorkerResult>,
    
    /// Channel to wake event loop for niri.schedule()
    pub wake_tx: Sender<()>,
    
    /// Active timer registrations (for cleanup)
    pub active_timers: Rc<RefCell<HashMap<u64, RegistrationToken>>>,
}
```

### Implementation

Key components in `niri-lua/src/loop_api.rs`:
- `Timer` userdata with `start()`, `stop()`, `close()`, `is_active()` methods
- `TimerManager` tracks all timer state and firing schedules
- `fire_due_timers()` executes callbacks for timers that are due
- `niri.loop.now()` provides monotonic time in milliseconds

```rust
// Register the loop API
let timer_manager = create_timer_manager();
register_loop_api(&lua, timer_manager.clone())?;

// In event loop: fire due timers
let (count, errors) = fire_due_timers(&lua, &timer_manager);
```

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_schedule_callback() {
    let rt = LuaRuntime::new().unwrap();
    rt.load_string("niri = {}").unwrap();
    rt.init_scheduler().unwrap();
    
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
    timer:close()
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
```

---

## Migration Guide

### For Users

#### Timeout Protection

With Luau's `set_interrupt`, scripts now have automatic timeout protection (default 1 second). Infinite loops will be terminated with an error instead of freezing the compositor:

```lua
-- This will timeout after 1 second with an error
while true do end
-- Error: Script execution timeout
```

#### Writing Efficient Code

Even with timeout protection, it's good practice to write non-blocking Lua code:

```lua
-- OK but may timeout if slow:
for i = 1, 10000000 do
    process(i)
end

-- BETTER: Chunked processing with schedule (respects frame timing)
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
-- OK but blocks main thread until timeout or completion:
local result = expensive_computation(data)

-- BETTER: Use niri.schedule() to break up work
local function process_chunk()
    -- Process in chunks, deferring to avoid blocking
    niri.schedule(process_next_chunk)
end
```

### For Developers

#### API Summary

| API | Purpose |
|-----|---------|
| `LuaRuntime::new()` | Create runtime with default 1s timeout |
| `LuaRuntime::new_with_limits()` | Create runtime with custom timeout |
| `ExecutionLimits::default()` | 1 second timeout |
| `ExecutionLimits::unlimited()` | No timeout (trusted code only) |
| `ExecutionLimits::with_timeout()` | Custom timeout duration |
| `rt.eval_with_timeout()` | Execute code with timeout |
| `rt.call_with_timeout()` | Call function with timeout |
| `LuaRuntime::init_scheduler()` | Set up `niri.schedule()` |
| `LuaRuntime::flush_scheduled()` | Execute queued callbacks |
| `niri.schedule(fn)` | Defer Lua callback to next event loop iteration |
| `niri.loop.new_timer()` | Create timer integrated with calloop |

#### Integration Points

1. Call `flush_scheduled()` in the compositor's refresh cycle
2. Set up wake channel for `niri.schedule()` to wake event loop
3. Call `fire_due_timers()` to execute timer callbacks
4. Call `cleanup_all()` on callback store during runtime shutdown

---

## Implementation Order

1. **Phase 1**: Remove `parking_lot` → `std::sync::Mutex` ✅ Complete
2. **Phase 2**: Execution timeouts with Luau `set_interrupt` ✅ Complete
3. **Phase 3**: Implement `niri.schedule()` ✅ Complete
4. **Phase 4**: Implement `niri.loop` timers ✅ Complete

All phases are complete. The implementation provides:
- Timeout protection via Luau's `set_interrupt`
- Deferred execution via `niri.schedule(fn)`
- Timer functionality via `niri.loop`

### Dependencies

```
Phase 1 (parking_lot removal) ✅
    ↓
Phase 2 (Luau timeouts) ✅
    ↓
Phase 3 (niri.schedule) ✅
    ↓
Phase 4 (timers) ✅ ←── uses registry pattern
```

---

## References

- [mlua documentation](https://docs.rs/mlua/latest/mlua/)
- [Luau language](https://luau-lang.org/)
- [Neovim vim.schedule](https://neovim.io/doc/user/lua.html#vim.schedule())
- [Neovim vim.uv (libuv)](https://neovim.io/doc/user/luvref.html)
- [AwesomeWM async spawn](https://awesomewm.org/doc/api/libraries/awful.spawn.html)
- [calloop documentation](https://docs.rs/calloop/latest/calloop/)
