# niri-lua Extensions Specification

## Scope

This specification covers **niri-lua crate extensions only**. It documents:
- Async/loop integration (`niri.loop` extensions)
- State watch system (`niri.state.watch` API)
- State interface improvements (rich event payloads, `niri.state.refresh()`)
- Event system enhancements (pattern matching, handler IDs)
- Lua table extractors (`FromLuaTable` trait)
- Complex configuration extractors (animations, window rules, outputs)

**Out of Scope** (documented elsewhere):
- niri-ui Lua APIs (`niri.ui.*`) → see `lua_bindings.md`
- Compositor integration → see `compositor-integration.md`
- Widget system → see widget component specs

## Overview

The niri-lua crate provides the foundation for Lua configuration and scripting in niri. This specification extends the core functionality with:

1. **Async/Loop Extensions**: Convenience helpers for timers, debouncing, and blocking waits
2. **State Watch System**: React to compositor state changes with watchers
3. **State Interface Improvements**: Rich event payloads and explicit state refresh
4. **Event System Enhancements**: Pattern matching and handler management
5. **Lua Table Extractors**: Type-safe extraction of Rust types from Lua tables

These extensions enable sophisticated event-driven scripting while maintaining simplicity for common cases.

**Priority**: P0 (foundational for niri-ui and advanced scripting)
**Scope**: niri-lua crate changes only; no compositor core changes required

---

## 1. Async/Loop API Extensions

### 1.1 Problem Statement

UI development requires sophisticated async primitives. Comparing niri-lua to Neovim's async infrastructure reveals gaps that make common patterns verbose:

| Capability | Neovim | niri-lua Current | Gap |
|------------|--------|------------------|-----|
| One-shot delayed callback | `vim.defer_fn(fn, ms)` | Manual timer + close | Yes |
| Wrap callback for safe scheduling | `vim.schedule_wrap(fn)` | None | Yes |
| Blocking wait with condition | `vim.wait(ms, cond, interval)` | None | Yes |
| Timer introspection | `get_due_in()`, `set_repeat()` | `is_active()` only | Partial |
| Error context for callbacks | Stack traces with timer ID | Generic errors | Yes |

### 1.2 New APIs

#### 1.2.1 `niri.loop.defer(fn, delay_ms)`

One-shot timer that auto-closes after callback execution. Equivalent to `vim.defer_fn`.

```lua
--- Execute a function after a delay (one-shot timer)
--- @param fn function Callback to execute
--- @param delay_ms number Delay in milliseconds
--- @return Timer Timer handle (can be stopped before firing)
function niri.loop.defer(fn, delay_ms)
```

**Implementation** (Lua helper in `niri-lua/src/lua/loop_helpers.lua`):

```lua
function niri.loop.defer(fn, delay_ms)
    assert(type(fn) == "function", "defer: fn must be a function")
    assert(type(delay_ms) == "number" and delay_ms >= 0, "defer: delay_ms must be non-negative number")
    
    local timer = niri.loop.new_timer()
    timer:start(delay_ms, 0, function()
        timer:close()
        fn()
    end)
    return timer
end
```

**Usage**:

```lua
-- Simple delayed execution
niri.loop.defer(function()
    print("Executed after 100ms")
end, 100)

-- Can cancel before firing
local timer = niri.loop.defer(show_notification, 5000)
-- User dismissed early:
timer:stop()
timer:close()
```

#### 1.2.2 `niri.schedule_wrap(fn)`

Returns a wrapped function that schedules the original to run on the main event loop. Equivalent to `vim.schedule_wrap`.

```lua
--- Wrap a function to auto-schedule on main event loop
--- @param fn function Function to wrap
--- @return function Wrapped function that schedules original
function niri.schedule_wrap(fn)
```

**Implementation**:

```lua
function niri.schedule_wrap(fn)
    assert(type(fn) == "function", "schedule_wrap: fn must be a function")
    return function(...)
        local args = {...}
        niri.schedule(function()
            fn(table.unpack(args))
        end)
    end
end
```

**Usage**:

```lua
-- Wrap a callback for safe API access
local safe_update = niri.schedule_wrap(function(value)
    niri.ui.update_widget("label", { text = tostring(value) })
end)

-- Can be called from any context (timer callbacks, event handlers)
timer:start(100, 100, safe_update)
```

#### 1.2.3 `niri.loop.wait(timeout_ms, condition, interval_ms)`

Blocking wait with condition polling. Equivalent to `vim.wait`. **Note**: This blocks the Lua coroutine but not the compositor event loop.

```lua
--- Wait for a condition with timeout
--- @param timeout_ms number Maximum time to wait
--- @param condition function|nil Condition function; wait exits when it returns truthy
--- @param interval_ms number|nil Polling interval (default: 10ms, min: 1ms)
--- @return boolean, any True if condition met, plus condition's return value; false if timeout
function niri.loop.wait(timeout_ms, condition, interval_ms)
```

**Implementation** (Rust-side for proper event loop integration):

```rust
// niri-lua/src/loop_api.rs

fn wait(
    lua: &Lua,
    (timeout_ms, condition, interval_ms): (u64, Option<LuaFunction>, Option<u64>),
) -> LuaResult<(bool, LuaValue)> {
    let interval = interval_ms.unwrap_or(10).max(1);
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    
    loop {
        // Check condition
        if let Some(ref cond) = condition {
            let result: LuaValue = cond.call(())?;
            if result.as_boolean().unwrap_or(false) || !matches!(result, LuaValue::Nil | LuaValue::Boolean(false)) {
                return Ok((true, result));
            }
        }
        
        // Check timeout
        if start.elapsed() >= timeout {
            return Ok((false, LuaValue::Nil));
        }
        
        // Yield to event loop
        std::thread::sleep(Duration::from_millis(interval));
    }
}
```

**Usage**:

```lua
-- Wait for a window to appear (max 5 seconds)
local found, window = niri.loop.wait(5000, function()
    return niri.get_focused_window()
end, 100)

if found then
    print("Window appeared:", window.title)
else
    print("Timeout waiting for window")
end

-- Simple sleep (no condition)
niri.loop.wait(500) -- Sleep for 500ms
```

**Warning**: Use sparingly. Blocking waits can cause UI jank if overused. Prefer event-driven patterns with `niri.state.watch` or `niri.events`.

#### 1.2.4 Timer Introspection Extensions

Extend the existing Timer userdata with additional methods:

```lua
--- Get time until next callback (0 if not active)
--- @return number Milliseconds until next fire, or 0
function timer:get_due_in()

--- Change repeat interval (0 to make one-shot)
--- @param repeat_ms number New repeat interval
function timer:set_repeat(repeat_ms)

--- Get current repeat interval
--- @return number Current repeat interval in ms
function timer:get_repeat()
```

**Implementation** (Rust-side in `loop_api.rs`):

```rust
impl TimerHandle {
    fn get_due_in(&self) -> u64 {
        if !self.is_active() {
            return 0;
        }
        let now = Instant::now();
        let next = self.next_fire.lock().unwrap();
        next.saturating_duration_since(now).as_millis() as u64
    }
    
    fn set_repeat(&mut self, repeat_ms: u64) {
        *self.repeat_ms.lock().unwrap() = repeat_ms;
    }
    
    fn get_repeat(&self) -> u64 {
        *self.repeat_ms.lock().unwrap()
    }
}
```

**Usage**:

```lua
local timer = niri.loop.new_timer()
timer:start(1000, 500, update_clock)

-- Later: check when it will fire
local due = timer:get_due_in()
print("Next tick in", due, "ms")

-- Slow down updates
timer:set_repeat(1000)
```

#### 1.2.5 Enhanced Error Context

Timer callbacks should include timer identification in error messages.

**Current behavior**:
```
Error in timer callback: attempt to index nil value
```

**Improved behavior**:
```
Error in timer callback (timer_id=42, repeat=100ms): attempt to index nil value
    stack traceback:
        [C]: in function 'index'
        user_script.lua:15: in function <user_script.lua:10>
```

**Implementation**: Wrap callback invocation in `loop_api.rs`:

```rust
fn invoke_callback(&self, lua: &Lua) -> LuaResult<()> {
    let callback = self.callback.lock().unwrap();
    if let Some(ref cb) = *callback {
        let func: LuaFunction = lua.registry_value(cb)?;
        func.call(()).map_err(|e| {
            LuaError::CallbackError {
                traceback: format!(
                    "timer_id={}, repeat={}ms",
                    self.id,
                    self.get_repeat()
                ),
                cause: Arc::new(e),
            }
        })
    } else {
        Ok(())
    }
}
```

### 1.3 Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-L1 | `niri.loop.defer(fn, 100)` executes fn after ~100ms and auto-closes timer |
| AC-L2 | `niri.schedule_wrap(fn)` returns function that defers original to event loop |
| AC-L3 | `niri.loop.wait(500)` blocks for ~500ms without condition |
| AC-L4 | `niri.loop.wait(5000, cond, 100)` returns early when cond() is truthy |
| AC-L5 | `niri.loop.wait` returns `(false, nil)` on timeout |
| AC-L6 | `timer:get_due_in()` returns ms until next fire, 0 if inactive |
| AC-L7 | `timer:set_repeat(ms)` changes interval for running timer |
| AC-L8 | Timer errors include timer ID and repeat interval in message |

### 1.4 Tests

```lua
-- test_loop_extensions.lua
describe("niri.loop.defer", function()
    it("executes callback after delay", function()
        local executed = false
        niri.loop.defer(function() executed = true end, 50)
        niri.loop.wait(100)
        assert(executed, "callback should have executed")
    end)
    
    it("can be cancelled before firing", function()
        local executed = false
        local timer = niri.loop.defer(function() executed = true end, 100)
        timer:stop()
        timer:close()
        niri.loop.wait(150)
        assert(not executed, "cancelled timer should not execute")
    end)
    
    it("auto-closes timer after execution", function()
        local timer = niri.loop.defer(function() end, 10)
        niri.loop.wait(50)
        assert(not timer:is_active(), "timer should be closed")
    end)
end)

describe("niri.schedule_wrap", function()
    it("wraps function to schedule execution", function()
        local value = nil
        local wrapped = niri.schedule_wrap(function(v) value = v end)
        wrapped(42)
        -- Value not set yet (scheduled for next iteration)
        assert(value == nil)
        niri.loop.wait(10)
        assert(value == 42, "wrapped function should have executed")
    end)
    
    it("preserves all arguments", function()
        local args = nil
        local wrapped = niri.schedule_wrap(function(a, b, c) args = {a, b, c} end)
        wrapped(1, "two", true)
        niri.loop.wait(10)
        assert.same({1, "two", true}, args)
    end)
end)

describe("niri.loop.wait", function()
    it("blocks for specified time without condition", function()
        local start = niri.loop.now()
        niri.loop.wait(100)
        local elapsed = niri.loop.now() - start
        assert(elapsed >= 95 and elapsed < 150, "should wait ~100ms")
    end)
    
    it("returns early when condition is met", function()
        local counter = 0
        local start = niri.loop.now()
        local ok, result = niri.loop.wait(1000, function()
            counter = counter + 1
            return counter >= 3
        end, 10)
        local elapsed = niri.loop.now() - start
        assert(ok, "condition should be met")
        assert(elapsed < 100, "should return early")
    end)
    
    it("returns false on timeout", function()
        local ok, _ = niri.loop.wait(50, function() return false end, 10)
        assert(not ok, "should timeout")
    end)
end)

describe("timer introspection", function()
    it("get_due_in returns time until next fire", function()
        local timer = niri.loop.new_timer()
        timer:start(100, 0, function() end)
        local due = timer:get_due_in()
        assert(due > 0 and due <= 100, "should be between 0 and 100ms")
        timer:close()
    end)
    
    it("set_repeat changes interval", function()
        local timer = niri.loop.new_timer()
        timer:start(100, 50, function() end)
        assert.equals(50, timer:get_repeat())
        timer:set_repeat(200)
        assert.equals(200, timer:get_repeat())
        timer:close()
    end)
end)
```

---

## 2. Reactive State Watch Helper

### 2.1 Problem Statement

UI components need to react to compositor state changes (window focus, workspace switches, output changes). Currently, scripts must either:
- Poll state repeatedly (inefficient, laggy)
- Manually subscribe to multiple events and filter (verbose, error-prone)

A `niri.state.watch` helper provides a clean abstraction.

### 2.2 Design Decision: Lua-level vs Rust-level

**Chosen approach**: Lua-level helper (P0)

**Rationale**:
- A Rust-level watch registry requires intrusive hooks at every state mutation point, delta detection, quota enforcement, and careful synchronization — this is P2 complexity.
- A Lua helper composing `niri.events:on()` with targeted queries and debounce covers 95%+ of UI use-cases with minimal effort.
- The helper can be upgraded to Rust later if profiling shows need.

### 2.3 API Specification

```lua
--- Watch for state changes matching a pattern
--- @param opts table Options for the watcher
--- @param callback function Callback invoked on matching events
--- @return Subscription Object with cancel() and is_active() methods
function niri.state.watch(opts, callback)
```

**Options table**:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `events` | `string[]` | Yes | Event names or patterns to subscribe to |
| `filter` | `function(payload) -> bool` | No | Filter function; callback only invoked if returns true |
| `immediate` | `boolean` | No | If true, invoke callback once immediately with current state |
| `debounce_ms` | `number` | No | Coalesce rapid events; only invoke callback after this many ms of quiet |

**Returns**: Subscription object

```lua
--- Subscription object returned by niri.state.watch
--- @class Subscription
--- @field cancel function Stops the subscription and removes handlers
--- @field is_active function Returns true if subscription is still active
```

### 2.4 Usage Examples

```lua
-- Watch for window focus changes
local sub = niri.state.watch({
    events = {"window:focus"},
    immediate = true,
}, function(payload)
    update_title_bar(payload.title or "")
end)

-- Watch for specific window resize with debounce
local sub = niri.state.watch({
    events = {"window:resize", "window:move"},
    filter = function(p) return p.id == my_window_id end,
    debounce_ms = 100,
}, function(payload)
    update_window_preview(payload)
end)

-- Clean up
sub:cancel()
```

### 2.5 Implementation

Location: `niri-lua/src/lua/state_watch.lua` (new file, loaded at runtime init)

```lua
-- niri.state.watch implementation
local active_subscriptions = setmetatable({}, { __mode = "v" }) -- weak values for GC

function niri.state.watch(opts, callback)
    assert(type(opts) == "table", "watch: opts must be a table")
    assert(type(callback) == "function", "watch: callback must be a function")
    assert(opts.events and #opts.events > 0, "watch: events list required")

    local events = opts.events
    local filter = opts.filter
    local immediate = opts.immediate
    local debounce_ms = opts.debounce_ms

    local active = true
    local timer = nil
    local last_payload = nil
    local handler_ids = {}

    -- Debounced callback wrapper
    local function invoke(payload)
        if not active then return end
        if filter and not filter(payload) then return end

        if debounce_ms and debounce_ms > 0 then
            last_payload = payload
            if timer then timer:stop() end
            timer = niri.loop.new_timer(function()
                if active and last_payload then
                    callback(last_payload)
                    last_payload = nil
                end
            end, debounce_ms)
        else
            callback(payload)
        end
    end

    -- Subscribe to events
    for _, event_name in ipairs(events) do
        local id = niri.events:on(event_name, invoke)
        table.insert(handler_ids, { event = event_name, id = id })
    end

    -- Immediate delivery
    if immediate then
        niri.utils.defer(function()
            if active then
                callback({ immediate = true })
            end
        end)
    end

    -- Subscription object
    local sub = {
        cancel = function()
            if not active then return end
            active = false
            for _, h in ipairs(handler_ids) do
                niri.events:off(h.event, h.id)
            end
            if timer then timer:stop() end
            handler_ids = {}
        end,
        is_active = function()
            return active
        end,
    }

    -- Track for GC cleanup
    active_subscriptions[sub] = sub

    -- GC cleanup via weak reference + __gc proxy
    local weak_sub = setmetatable({}, {
        __gc = function()
            if active then sub.cancel() end
        end
    })
    rawset(sub, "__weak_ref", weak_sub)

    return sub
end
```

### 2.6 Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-1 | `immediate=true` invokes callback once synchronously (via defer) with `{ immediate = true }` payload |
| AC-2 | Multiple rapid events coalesce into single callback when `debounce_ms > 0` |
| AC-3 | `filter` function prevents callback invocation when it returns false |
| AC-4 | `sub:cancel()` removes all event handlers and prevents future callbacks |
| AC-5 | Subscription is cleaned up on garbage collection if not explicitly cancelled |
| AC-6 | Multiple subscriptions to same events work independently |

### 2.7 Tests

```lua
-- test_state_watch.lua
describe("niri.state.watch", function()
    it("invokes callback on matching event", function()
        local called = false
        local sub = niri.state.watch({
            events = {"test:event"},
        }, function() called = true end)

        niri.events:emit("test:event", {})
        assert(called)
        sub:cancel()
    end)

    it("respects filter function", function()
        local count = 0
        local sub = niri.state.watch({
            events = {"test:event"},
            filter = function(p) return p.value > 5 end,
        }, function() count = count + 1 end)

        niri.events:emit("test:event", { value = 3 })
        niri.events:emit("test:event", { value = 10 })
        assert.equals(1, count)
        sub:cancel()
    end)

    it("debounces rapid events", function()
        local count = 0
        local sub = niri.state.watch({
            events = {"test:event"},
            debounce_ms = 50,
        }, function() count = count + 1 end)

        for i = 1, 10 do
            niri.events:emit("test:event", { i = i })
        end
        -- Wait for debounce
        niri.loop.sleep(100)
        assert.equals(1, count)
        sub:cancel()
    end)

    it("supports immediate delivery", function()
        local called = false
        local sub = niri.state.watch({
            events = {"test:event"},
            immediate = true,
        }, function(p)
            if p.immediate then called = true end
        end)

        niri.loop.run_pending()
        assert(called)
        sub:cancel()
    end)

    it("cancellation stops callbacks", function()
        local count = 0
        local sub = niri.state.watch({
            events = {"test:event"},
        }, function() count = count + 1 end)

        niri.events:emit("test:event", {})
        sub:cancel()
        niri.events:emit("test:event", {})
        assert.equals(1, count)
    end)
end)
```

---

## 3. State Interface Improvements

### 3.1 Background: Why State Snapshots Exist

The `niri.state.*` APIs currently use **state snapshots** — point-in-time copies of compositor state captured before Lua event handlers run. This design exists to prevent deadlocks:

```
Event occurs → Main thread holds State borrow → Calls Lua handler →
Handler calls niri.state.windows() → Tries to query live state → DEADLOCK
```

**The constraint chain:**
1. Lua event handlers execute **synchronously on the main thread**
2. The main thread is **blocked waiting for Lua to complete**
3. Any attempt to query live state via `insert_idle()` + `recv_blocking()` **deadlocks**
4. Therefore, state must be **captured before** the Lua handler runs

**Trade-offs of snapshots:**
| Benefit | Limitation |
|---------|------------|
| No deadlocks | State may be stale after actions execute |
| Consistent view within handler | Memory overhead from cloning |
| Simple mental model | Can't see effects of own actions |

### 3.2 The Staleness Problem

When a handler executes an action and then queries state, the snapshot doesn't reflect the action:

```lua
niri.events:on("window:open", function(ev)
    niri.action:move_window_to_workspace({ id = 2 })
    
    -- BUG: This still shows window on original workspace!
    local windows = niri.state.windows()
    for _, w in ipairs(windows) do
        print(w.workspace_id)  -- Shows old workspace
    end
end)
```

**Current mitigations:**
1. Use event payload data directly (not re-querying)
2. Defer queries via `niri.loop.defer()`
3. Chain through separate event handlers

### 3.3 Recommended Improvements

Rather than eliminating snapshots (which is not feasible due to deadlock constraints), we improve the state interface by:

1. **Rich event payloads** — Include complete data in events so handlers rarely need to query
2. **`niri.state.refresh()`** — Explicit opt-in to capture fresh snapshot mid-handler
3. **Event-driven patterns** — Promote `niri.state.watch()` over polling

### 3.4 Rich Event Payloads (P0)

**Problem:** Current events include minimal data, forcing handlers to query state:

```lua
-- Current: minimal payload
niri.events:on("window:open", function(ev)
    -- ev only has { id, title, app_id }
    -- Must query for full window info
    local windows = niri.state.windows()
    local window = find_by_id(windows, ev.id)  -- Wasteful
end)
```

**Solution:** Include complete IPC types in event payloads:

```lua
-- Improved: complete payload
niri.events:on("window:open", function(ev)
    -- ev.window is full niri_ipc::Window
    print(ev.window.workspace_id)
    print(ev.window.output)
    print(ev.window.is_floating)
    -- No state query needed!
end)
```

**Required payload enhancements:**

| Event | Current Payload | Enhanced Payload |
|-------|-----------------|------------------|
| `window:open` | `{ id, title, app_id }` | `{ window: Window }` (full IPC type) |
| `window:close` | `{ id }` | `{ window: Window }` (last known state) |
| `window:focus` | `{ id }` | `{ window: Window, previous: Window? }` |
| `window:move` | `{ id }` | `{ window: Window, old_x, old_y }` |
| `window:resize` | `{ id }` | `{ window: Window, old_width, old_height }` |
| `workspace:activate` | `{ id, idx }` | `{ workspace: Workspace, previous: Workspace? }` |
| `output:connect` | `{ name }` | `{ output: Output }` (full IPC type) |
| `output:disconnect` | `{ name }` | `{ output: Output }` (last known state) |

**Implementation location:** `src/lua_event_hooks.rs`

```rust
// Before (current):
pub fn emit_window_open(state: &State, window_id: u32) {
    let data = lua.create_table()?;
    data.set("id", window_id)?;
    // Minimal data...
}

// After (enhanced):
pub fn emit_window_open(state: &State, window_id: u32) {
    let window = state.niri.ipc_window(window_id);  // Get full IPC Window
    let data = lua.create_table()?;
    data.set("window", window_to_lua(lua, &window)?)?;
    // Complete data in payload
}
```

**Benefits:**
- Handlers use event data directly — no state queries needed
- Fresh data at event time (not stale snapshot)
- Reduced memory churn (no full state clone if handler only needs event window)

### 3.5 `niri.state.refresh()` (P1)

**Purpose:** Explicit opt-in to capture a fresh snapshot mid-handler for cases where handlers need to see effects of their own actions.

```lua
--- Refresh the state snapshot with current compositor state.
--- 
--- CAUTION: This is an expensive operation that replaces the current
--- snapshot. Use sparingly — prefer event payloads or deferred queries.
---
--- @return nil
function niri.state.refresh()
```

**Usage:**

```lua
niri.events:on("window:open", function(ev)
    -- Move window to workspace 2
    niri.action:move_window_to_workspace({ id = 2 })
    
    -- Explicitly refresh to see the move
    niri.state.refresh()
    
    -- Now this reflects the updated state
    local windows = niri.state.windows()
    for _, w in ipairs(windows) do
        if w.id == ev.window.id then
            assert(w.workspace_id == 2)  -- Now correct!
        end
    end
end)
```

**Implementation:**

```rust
// niri-lua/src/runtime_api.rs

fn refresh_state(lua: &Lua, _: ()) -> LuaResult<()> {
    // This only works in event context where we have state access
    // For REPL/timer context, this is a no-op (state is always fresh via idle callback)
    
    if is_in_event_context() {
        // Get fresh snapshot from EventStreamState
        // The EventStreamState is updated after actions execute
        let fresh_snapshot = EVENT_LOOP_HANDLE.with(|h| {
            // Use the compositor's event stream state cache
            // This is already updated after actions
            h.borrow().as_ref().map(|handle| {
                // Query fresh state synchronously
                // Safe because we're on the main thread during event handling
                StateSnapshot::from_event_stream_state(handle)
            })
        });
        
        if let Some(snapshot) = fresh_snapshot {
            set_event_context_state(snapshot);
        }
    }
    
    Ok(())
}
```

**Important considerations:**

1. **Timing:** `refresh()` captures state from `EventStreamState`, which is updated after actions execute. The action must complete before `refresh()` will see its effects.

2. **Performance:** Each `refresh()` clones all state fields. Document that this is expensive and should be used sparingly.

3. **Consistency:** After `refresh()`, subsequent `niri.state.*` calls use the new snapshot until handler exits or another `refresh()`.

**When to use vs. alternatives:**

| Scenario | Recommended Approach |
|----------|---------------------|
| Read data about event subject | Use event payload (P0) |
| React to state changes | Use `niri.state.watch()` (Section 2) |
| Need state after action in same handler | Use `niri.state.refresh()` (P1) |
| Complex multi-action workflow | Defer to next event loop via `niri.loop.defer()` |

### 3.6 Event-Driven Architecture Guidance

**Best practice:** Design scripts to be event-driven rather than state-polling.

**Anti-pattern (polling):**
```lua
-- BAD: Polling state in a timer
local timer = niri.loop.new_timer()
timer:start(0, 100, function()
    local focused = niri.state.focused_window()
    if focused and focused.app_id == "firefox" then
        update_ui_for_firefox()
    end
end)
```

**Recommended (event-driven):**
```lua
-- GOOD: React to events
niri.state.watch({
    events = {"window:focus"},
    filter = function(ev) return ev.window.app_id == "firefox" end,
}, function(ev)
    update_ui_for_firefox()
end)
```

**Benefits of event-driven design:**
- No wasted CPU cycles polling unchanged state
- Immediate response to changes (no polling interval delay)
- Cleaner code structure
- Better integration with snapshot architecture

### 3.7 Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-S1 | `window:open` event includes full `Window` object in payload |
| AC-S2 | `window:focus` event includes current and previous `Window` objects |
| AC-S3 | `workspace:activate` event includes full `Workspace` object |
| AC-S4 | `niri.state.refresh()` updates snapshot to reflect actions executed in handler |
| AC-S5 | `niri.state.refresh()` is a no-op in REPL/timer context (already uses fresh queries) |
| AC-S6 | Documentation warns about `refresh()` performance cost |
| AC-S7 | Event payload types are documented in EmmyLua annotations |

### 3.8 Implementation Checklist

**Phase 1: Rich Event Payloads (P0)**
- [ ] Update `emit_window_open()` to include full `Window` object
- [ ] Update `emit_window_close()` to include full `Window` object
- [ ] Update `emit_window_focus()` to include current and previous `Window`
- [ ] Update `emit_workspace_activate()` to include full `Workspace` object
- [ ] Update `emit_output_connect()` to include full `Output` object
- [ ] Add helper function `window_to_lua()` for IPC Window → Lua table
- [ ] Add helper function `workspace_to_lua()` for IPC Workspace → Lua table
- [ ] Add helper function `output_to_lua()` for IPC Output → Lua table
- [ ] Update EmmyLua annotations with enhanced payload types

**Phase 2: State Refresh (P1)**
- [ ] Implement `niri.state.refresh()` in `runtime_api.rs`
- [ ] Add event context detection for refresh behavior
- [ ] Document performance implications in code comments
- [ ] Add tests for refresh behavior
- [ ] Update wiki documentation with refresh usage guidance

### 3.9 Tests

```lua
-- test_state_improvements.lua

describe("rich event payloads", function()
    it("window:open includes full Window object", function()
        local received_window = nil
        niri.events:on("window:open", function(ev)
            received_window = ev.window
        end)
        
        -- Trigger window open (test harness)
        test.open_window({ app_id = "test-app", title = "Test" })
        
        assert(received_window, "should receive window object")
        assert(received_window.app_id == "test-app")
        assert(received_window.title == "Test")
        assert(received_window.workspace_id, "should include workspace_id")
        assert(received_window.output, "should include output")
    end)
    
    it("window:focus includes previous window", function()
        local current, previous = nil, nil
        niri.events:on("window:focus", function(ev)
            current = ev.window
            previous = ev.previous
        end)
        
        test.open_window({ app_id = "first" })
        test.open_window({ app_id = "second" })
        
        assert(current.app_id == "second")
        assert(previous.app_id == "first")
    end)
end)

describe("niri.state.refresh", function()
    it("updates snapshot to reflect actions", function()
        local workspace_after_move = nil
        
        niri.events:on("window:open", function(ev)
            -- Window opens on workspace 1
            assert(ev.window.workspace_id == 1)
            
            -- Move to workspace 2
            niri.action:move_window_to_workspace({ id = 2 })
            
            -- Without refresh, snapshot is stale
            local windows_stale = niri.state.windows()
            local w_stale = find_by_id(windows_stale, ev.window.id)
            assert(w_stale.workspace_id == 1, "stale snapshot shows old workspace")
            
            -- Refresh to get fresh state
            niri.state.refresh()
            
            -- Now snapshot reflects the move
            local windows_fresh = niri.state.windows()
            local w_fresh = find_by_id(windows_fresh, ev.window.id)
            workspace_after_move = w_fresh.workspace_id
        end)
        
        test.open_window({ app_id = "test" })
        
        assert(workspace_after_move == 2, "refresh should show updated workspace")
    end)
    
    it("is safe to call multiple times", function()
        niri.events:on("window:open", function(ev)
            niri.state.refresh()
            niri.state.refresh()
            niri.state.refresh()
            -- Should not error or corrupt state
            local windows = niri.state.windows()
            assert(#windows > 0)
        end)
        
        test.open_window({ app_id = "test" })
    end)
end)
```

---

## 4. Event System Extensions

### 3.1 Required Extensions

The `niri.events` interface needs small extensions to support `niri.state.watch` and UI event handling:

#### 3.1.1 Pattern-based subscription

```lua
-- Subscribe to events matching a pattern
niri.events:on("window:*", function(event_name, payload)
    -- Matches window:focus, window:move, window:resize, etc.
end)
```

**Implementation**: Add glob matching in `events_proxy.rs`:

```rust
impl EventsProxy {
    fn matches_pattern(pattern: &str, event_name: &str) -> bool {
        if pattern.ends_with(":*") {
            let prefix = &pattern[..pattern.len() - 2];
            event_name.starts_with(prefix) && event_name.contains(':')
        } else if pattern == "*" {
            true
        } else {
            pattern == event_name
        }
    }
}
```

#### 3.1.2 Handler ID return for targeted removal

```lua
local id = niri.events:on("window:focus", handler)
niri.events:off("window:focus", id)  -- Remove specific handler
```

**Current state**: `events:off(event)` removes ALL handlers for that event.
**Required**: Return handler ID from `:on()` and support targeted removal.

```rust
// In events_proxy.rs
fn on(&mut self, event: String, handler: LuaFunction) -> LuaResult<u64> {
    let id = self.next_handler_id;
    self.next_handler_id += 1;
    self.handlers
        .entry(event)
        .or_default()
        .push((id, lua.create_registry_value(handler)?));
    Ok(id)
}

fn off(&mut self, event: String, id: Option<u64>) -> LuaResult<()> {
    if let Some(id) = id {
        // Remove specific handler
        if let Some(handlers) = self.handlers.get_mut(&event) {
            handlers.retain(|(h_id, _)| *h_id != id);
        }
    } else {
        // Remove all handlers for event
        self.handlers.remove(&event);
    }
    Ok(())
}
```

#### 3.1.3 Rich event payloads

Ensure all compositor-emitted events include sufficient data for UI handlers:

| Event | Required Payload Fields |
|-------|------------------------|
| `window:focus` | `{ id, title, app_id, output, workspace_id }` |
| `window:move` | `{ id, x, y, output }` |
| `window:resize` | `{ id, width, height }` |
| `window:close` | `{ id, app_id }` |
| `workspace:activate` | `{ id, idx, output, name }` |
| `output:connect` | `{ name, make, model, width, height, scale }` |
| `output:disconnect` | `{ name }` |
| `popup:show` | `{ name, window, anchor_rect, position, output, scale }` |
| `popup:dismiss` | `{ name, reason }` |

### 3.2 Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-7 | Pattern subscription `"window:*"` matches all window events |
| AC-8 | `events:on()` returns a handler ID |
| AC-9 | `events:off(event, id)` removes only that handler |
| AC-10 | All listed events emit payloads with required fields |

---

## 5. Extractor Trait Abstraction

### 4.1 Problem Statement

`niri-lua/src/extractors.rs` contains ~2000 lines of extraction functions for converting Lua tables to Rust types. These follow a repetitive pattern:

```rust
pub fn extract_foo(table: &LuaTable) -> LuaResult<Option<Foo>> {
    let field1 = extract_string_opt(table, "field1")?;
    let field2 = extract_int_opt(table, "field2")?;
    // ... 10-20 more fields
    if field1.is_some() || field2.is_some() /* || ... */ {
        Ok(Some(Foo { field1, field2, ... }))
    } else {
        Ok(None)
    }
}
```

`niri-ui` needs its own extractors (PopupConfig, SliderConfig, Style, etc.) but copy-pasting this pattern leads to maintenance burden and inconsistency.

### 4.2 Proposed Solution: `FromLuaTable` Trait

Create a trait that types can implement to enable automatic extraction:

```rust
// niri-lua/src/extractors/traits.rs (new file)

use mlua::prelude::*;

/// Trait for types that can be extracted from a Lua table.
///
/// Implementors define how to extract their fields from a Lua table,
/// with automatic handling of optional fields and validation.
pub trait FromLuaTable: Sized {
    /// Extract this type from a Lua table.
    ///
    /// Returns `Ok(Some(Self))` if any relevant fields were present,
    /// `Ok(None)` if the table had no relevant fields (all defaults),
    /// or `Err` if extraction failed due to type mismatch or validation error.
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>>;

    /// Extract this type, returning default if no fields present.
    fn from_lua_table_or_default(table: &LuaTable) -> LuaResult<Self>
    where
        Self: Default,
    {
        Ok(Self::from_lua_table(table)?.unwrap_or_default())
    }

    /// Extract a required instance (error if no fields present).
    fn from_lua_table_required(table: &LuaTable) -> LuaResult<Self> {
        Self::from_lua_table(table)?.ok_or_else(|| {
            LuaError::FromLuaConversionError {
                from: "table",
                to: std::any::type_name::<Self>(),
                message: Some("table has no valid fields".to_string()),
            }
        })
    }
}

/// Trait for extracting a nested table field as a type.
pub trait ExtractField<T> {
    fn extract_field(&self, field: &str) -> LuaResult<Option<T>>;
}

impl<T: FromLuaTable> ExtractField<T> for LuaTable<'_> {
    fn extract_field(&self, field: &str) -> LuaResult<Option<T>> {
        match self.get::<LuaValue>(field)? {
            LuaValue::Nil => Ok(None),
            LuaValue::Table(t) => T::from_lua_table(&t),
            _ => Ok(None), // Wrong type, treat as missing
        }
    }
}
```

### 4.3 Derive Macro (Optional, P1)

For common patterns, a derive macro reduces boilerplate:

```rust
// niri-lua-derive/src/lib.rs (extension)

use proc_macro::TokenStream;

/// Derive FromLuaTable for a struct.
///
/// # Example
/// ```rust
/// #[derive(FromLuaTable)]
/// struct PopupConfig {
///     #[lua(required)]
///     name: String,
///     #[lua(default = "bottom")]
///     anchor_edge: String,
///     #[lua(nested)]
///     offset: Option<Point>,
///     #[lua(skip)]
///     internal_state: u32,
/// }
/// ```
#[proc_macro_derive(FromLuaTable, attributes(lua))]
pub fn derive_from_lua_table(input: TokenStream) -> TokenStream {
    // ... macro implementation
}
```

### 4.4 Manual Implementation Pattern

Until the derive macro is implemented, use this pattern:

```rust
impl FromLuaTable for PopupConfig {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let name = extract_string_opt(table, "name")?;
        let anchor_to = table.extract_field::<PopupAnchor>("anchor_to")?;
        let offset_x = extract_float_opt(table, "offset_x")?.unwrap_or(0.0);
        let offset_y = extract_float_opt(table, "offset_y")?.unwrap_or(0.0);
        let dismiss_on_outside_click = extract_bool_opt(table, "dismiss_on_outside_click")?.unwrap_or(true);
        let dismiss_on_escape = extract_bool_opt(table, "dismiss_on_escape")?.unwrap_or(true);
        let auto_flip = extract_bool_opt(table, "auto_flip")?.unwrap_or(true);

        // Name is required
        let Some(name) = name else {
            return Ok(None);
        };

        Ok(Some(PopupConfig {
            name,
            anchor_to,
            offset: Point { x: offset_x, y: offset_y },
            dismiss_on_outside_click,
            dismiss_on_escape,
            auto_flip,
        }))
    }
}
```

### 4.5 Migration Path

1. **Phase 1 (P0)**: Add `FromLuaTable` trait and helper implementations
2. **Phase 2 (P1)**: Add derive macro for common patterns
3. **Phase 3 (P1)**: Migrate existing extractors to use trait (backwards-compatible)
4. **Phase 4 (P2)**: Remove legacy `extract_*` functions (deprecation period)

### 4.6 niri-ui Extractors

Using the trait, implement extractors for UI types:

```rust
// niri-ui/src/lua/extractors.rs

use niri_lua::extractors::{FromLuaTable, ExtractField};

impl FromLuaTable for PopupConfig { /* ... */ }
impl FromLuaTable for PopupAnchor { /* ... */ }
impl FromLuaTable for SliderConfig { /* ... */ }
impl FromLuaTable for Style { /* ... */ }
impl FromLuaTable for WindowConfig { /* ... */ }
impl FromLuaTable for WidgetProps { /* ... */ }
```

### 4.7 Outstanding Extractor TODOs

The existing `niri-lua/src/extractors.rs` has two outstanding TODOs that should be addressed using the `FromLuaTable` trait:

#### 4.7.1 Individual Animation Settings (Line 156)

**Current state**: `extract_animations()` only extracts `off`, `on`, and `slowdown`. The `Animations` struct has 11 individual animation properties that are not extracted.

**Animation types to support**:

| Animation | Type | Default Kind |
|-----------|------|--------------|
| `workspace_switch` | WorkspaceSwitchAnim | Spring |
| `window_open` | WindowOpenAnim | Easing |
| `window_close` | WindowCloseAnim | Easing |
| `horizontal_view_movement` | HorizontalViewMovementAnim | Spring |
| `window_movement` | WindowMovementAnim | Spring |
| `window_resize` | WindowResizeAnim | Spring |
| `config_notification_open_close` | ConfigNotificationOpenCloseAnim | Spring |
| `exit_confirmation_open_close` | ExitConfirmationOpenCloseAnim | Spring |
| `screenshot_ui_open` | ScreenshotUiOpenAnim | Easing |
| `overview_open_close` | OverviewOpenCloseAnim | Easing |
| `recent_windows_close` | RecentWindowsCloseAnim | Spring |

**Implementation using `FromLuaTable`**:

```rust
impl FromLuaTable for Animation {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let off = extract_bool_opt(table, "off")?.unwrap_or(false);
        let duration_ms = extract_int_opt(table, "duration_ms")?.map(|v| v as u32);
        let curve = extract_string_opt(table, "curve")?;
        
        // Spring parameters
        let damping_ratio = extract_float_opt(table, "damping_ratio")?;
        let stiffness = extract_int_opt(table, "stiffness")?.map(|v| v as u32);
        let epsilon = extract_float_opt(table, "epsilon")?;
        
        // Cubic bezier parameters
        let bezier = extract_table_opt(table, "bezier")?;
        let (x1, y1, x2, y2) = if let Some(ref b) = bezier {
            (
                extract_float_opt(b, "x1")?,
                extract_float_opt(b, "y1")?,
                extract_float_opt(b, "x2")?,
                extract_float_opt(b, "y2")?,
            )
        } else {
            (None, None, None, None)
        };
        
        // Determine animation kind
        let kind = if damping_ratio.is_some() || stiffness.is_some() || epsilon.is_some() {
            Kind::Spring(SpringParams {
                damping_ratio: damping_ratio.unwrap_or(1.0),
                stiffness: stiffness.unwrap_or(1000),
                epsilon: epsilon.unwrap_or(0.0001),
            })
        } else if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (x1, y1, x2, y2) {
            Kind::Easing(EasingParams {
                duration_ms: duration_ms.unwrap_or(200),
                curve: Curve::CubicBezier(x1, y1, x2, y2),
            })
        } else if let Some(curve_str) = curve {
            let curve_enum = match curve_str.to_lowercase().replace('-', "_").as_str() {
                "linear" => Curve::Linear,
                "ease_out_quad" => Curve::EaseOutQuad,
                "ease_out_cubic" => Curve::EaseOutCubic,
                "ease_out_expo" => Curve::EaseOutExpo,
                _ => return Ok(None), // Invalid curve name
            };
            Kind::Easing(EasingParams {
                duration_ms: duration_ms.unwrap_or(200),
                curve: curve_enum,
            })
        } else if duration_ms.is_some() {
            Kind::Easing(EasingParams {
                duration_ms: duration_ms.unwrap(),
                curve: Curve::EaseOutCubic,
            })
        } else if off {
            // Only "off" specified, use default kind
            Kind::Easing(EasingParams {
                duration_ms: 200,
                curve: Curve::EaseOutCubic,
            })
        } else {
            return Ok(None);
        };
        
        Ok(Some(Animation { off, kind }))
    }
}
```

**Lua usage**:

```lua
niri.config.animations = {
    slowdown = 1.0,
    
    -- Spring animation
    workspace_switch = {
        damping_ratio = 1.0,
        stiffness = 1000,
        epsilon = 0.0001,
    },
    
    -- Easing animation with named curve
    window_open = {
        duration_ms = 150,
        curve = "ease-out-expo",
    },
    
    -- Easing animation with custom bezier
    window_close = {
        duration_ms = 150,
        bezier = { x1 = 0.2, y1 = 0.0, x2 = 0.0, y2 = 1.0 },
    },
    
    -- Disable specific animation
    screenshot_ui_open = { off = true },
}
```

**Priority**: P1 (needed for full animation customization parity with KDL config)

#### 4.7.2 Complex Configuration Extractors (Line 1007-1013)

**Listed TODOs**:
- Output configuration
- Window rules
- Layer rules
- Workspaces
- Switch Events
- Named Workspaces

**Priority assessment**:

| Extractor | Priority | Rationale |
|-----------|----------|-----------|
| Window rules | **P0** | Critical for window management; most requested feature |
| Output configuration | P1 | Essential for multi-monitor setups |
| Workspaces | P1 | Important for workspace customization |
| Layer rules | P2 | Niche; layer-shell clients self-configure |
| Switch Events | P2 | Niche; laptop lid/tablet mode |
| Named Workspaces | P2 | Less common than numbered workspaces |

**Window Rules Extractor** (P0):

```rust
impl FromLuaTable for WindowRule {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        // Match conditions
        let matches = if let Some(m) = extract_table_opt(table, "matches")? {
            extract_window_matches(&m)?
        } else {
            Vec::new()
        };
        
        let excludes = if let Some(e) = extract_table_opt(table, "excludes")? {
            extract_window_matches(&e)?
        } else {
            Vec::new()
        };
        
        // Window properties
        let default_column_width = table.extract_field::<PresetSize>("default_column_width")?;
        let open_on_output = extract_string_opt(table, "open_on_output")?;
        let open_on_workspace = extract_string_opt(table, "open_on_workspace")?;
        let open_maximized = extract_bool_opt(table, "open_maximized")?;
        let open_fullscreen = extract_bool_opt(table, "open_fullscreen")?;
        let open_floating = extract_bool_opt(table, "open_floating")?;
        let block_out_from = extract_string_opt(table, "block_out_from")?
            .and_then(|s| parse_block_out_from(&s));
        let opacity = extract_float_opt(table, "opacity")?;
        let draw_border_with_background = extract_bool_opt(table, "draw_border_with_background")?;
        // ... additional properties
        
        if matches.is_empty() && excludes.is_empty() {
            return Ok(None);
        }
        
        Ok(Some(WindowRule {
            matches,
            excludes,
            default_column_width,
            open_on_output,
            open_on_workspace,
            open_maximized,
            open_fullscreen,
            open_floating,
            block_out_from,
            opacity,
            draw_border_with_background,
            // ... set remaining fields
            ..Default::default()
        }))
    }
}

fn extract_window_matches(table: &LuaTable) -> LuaResult<Vec<WindowMatch>> {
    let mut matches = Vec::new();
    for i in 1..=table.len()? {
        if let Ok(match_table) = table.get::<LuaTable>(i) {
            if let Some(m) = WindowMatch::from_lua_table(&match_table)? {
                matches.push(m);
            }
        }
    }
    Ok(matches)
}

impl FromLuaTable for WindowMatch {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let app_id = extract_string_opt(table, "app_id")?;
        let title = extract_string_opt(table, "title")?;
        let is_active = extract_bool_opt(table, "is_active")?;
        let is_focused = extract_bool_opt(table, "is_focused")?;
        let is_active_in_column = extract_bool_opt(table, "is_active_in_column")?;
        let at_startup = extract_bool_opt(table, "at_startup")?;
        
        if app_id.is_none() && title.is_none() && is_active.is_none() 
            && is_focused.is_none() && is_active_in_column.is_none() && at_startup.is_none() {
            return Ok(None);
        }
        
        Ok(Some(WindowMatch {
            app_id: app_id.map(|s| Regex::new(&s).ok()).flatten(),
            title: title.map(|s| Regex::new(&s).ok()).flatten(),
            is_active,
            is_focused,
            is_active_in_column,
            at_startup,
        }))
    }
}
```

**Lua usage for window rules**:

```lua
niri.config.window_rules = {
    -- Float all dialogs
    {
        matches = {{ title = "^Save As$" }, { title = "^Open File$" }},
        open_floating = true,
    },
    
    -- Open Firefox on workspace 2
    {
        matches = {{ app_id = "^firefox$" }},
        open_on_workspace = "2",
    },
    
    -- Exclude PiP windows from rules
    {
        matches = {{ app_id = ".*" }},
        excludes = {{ title = "Picture.in.Picture" }},
        opacity = 0.95,
    },
}
```

**Output Configuration Extractor** (P1):

```rust
impl FromLuaTable for OutputConfig {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let name = extract_string_opt(table, "name")?;
        let scale = extract_float_opt(table, "scale")?;
        let transform = extract_string_opt(table, "transform")?
            .and_then(|s| parse_transform(&s));
        let position = table.extract_field::<Position>("position")?;
        let mode = table.extract_field::<Mode>("mode")?;
        let variable_refresh_rate = extract_bool_opt(table, "variable_refresh_rate")?;
        let background_color = extract_color_opt(table, "background_color")?;
        
        let Some(name) = name else {
            return Ok(None);
        };
        
        Ok(Some(OutputConfig {
            name,
            scale,
            transform,
            position,
            mode,
            variable_refresh_rate,
            background_color,
            ..Default::default()
        }))
    }
}
```

**Lua usage for outputs**:

```lua
niri.config.outputs = {
    {
        name = "DP-1",
        scale = 1.5,
        position = { x = 0, y = 0 },
        mode = { width = 2560, height = 1440, refresh = 144.0 },
        variable_refresh_rate = true,
    },
    {
        name = "eDP-1",
        scale = 2.0,
        position = { x = 2560, y = 0 },
    },
}
```

### 4.8 Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-11 | `FromLuaTable` trait is defined in niri-lua |
| AC-12 | `ExtractField` helper trait works for nested tables |
| AC-13 | Existing extractors continue to work (backwards-compatible) |
| AC-14 | niri-ui can implement extractors for its types |
| AC-15 | `Animation` extractor supports spring parameters (damping_ratio, stiffness, epsilon) |
| AC-16 | `Animation` extractor supports easing parameters (duration_ms, curve, bezier) |
| AC-17 | All 11 individual animation types can be configured via Lua |
| AC-18 | `WindowRule` extractor supports matches/excludes with regex patterns |
| AC-19 | `WindowRule` extractor supports all window properties (opacity, open_floating, etc.) |
| AC-20 | `OutputConfig` extractor supports scale, transform, position, mode, VRR |

---

## 6. API Schema Updates

### 6.1 EmmyLua Annotations

Generate/update `niri-lua/types/api.lua` with types for new APIs:

```lua
-- Async/Loop API Extensions

---@class Timer
---@field start fun(delay_ms: number, repeat_ms: number, callback: fun()) Start the timer
---@field stop fun() Stop the timer
---@field close fun() Close and release the timer
---@field is_active fun(): boolean Check if timer is running
---@field get_due_in fun(): number Milliseconds until next fire (0 if inactive)
---@field set_repeat fun(repeat_ms: number) Change repeat interval
---@field get_repeat fun(): number Get current repeat interval
---@field id number Readonly timer ID

--- Execute a function after a delay (one-shot, auto-closes)
---@param fn function Callback to execute
---@param delay_ms number Delay in milliseconds
---@return Timer Timer handle (can be stopped before firing)
function niri.loop.defer(fn, delay_ms) end

--- Wait for a condition with timeout (blocks Lua, not compositor)
---@param timeout_ms number Maximum time to wait
---@param condition? fun(): any Condition function; exits when truthy
---@param interval_ms? number Polling interval (default: 10ms)
---@return boolean, any True + condition result if met; false + nil if timeout
function niri.loop.wait(timeout_ms, condition, interval_ms) end

--- Wrap a function to auto-schedule on main event loop
---@param fn function Function to wrap
---@return function Wrapped function that schedules original
function niri.schedule_wrap(fn) end

-- State Watch Types

---@class Subscription
---@field cancel fun() Stop the subscription
---@field is_active fun(): boolean Check if subscription is active

--- Watch for state changes
---@param opts { events: string[], filter?: fun(payload: table): boolean, immediate?: boolean, debounce_ms?: number }
---@param callback fun(payload: table)
---@return Subscription
function niri.state.watch(opts, callback) end

--- Refresh the state snapshot with current compositor state.
--- CAUTION: Expensive operation. Prefer event payloads or deferred queries.
function niri.state.refresh() end

-- Rich Event Payload Types

---@class WindowEventPayload
---@field window Window Full window object from niri_ipc
---@field previous? Window Previous window (for focus events)

---@class WorkspaceEventPayload
---@field workspace Workspace Full workspace object
---@field previous? Workspace Previous workspace (for activate events)

---@class OutputEventPayload
---@field output Output Full output object from niri_ipc

-- Event payload examples:
-- window:open    -> { window: Window }
-- window:close   -> { window: Window }
-- window:focus   -> { window: Window, previous?: Window }
-- workspace:activate -> { workspace: Workspace, previous?: Workspace }
-- output:connect -> { output: Output }
```

**Note**: UI-specific types (`HitTestResult`, `WidgetBounds`, popup APIs) are documented in `lua_bindings.md`.

---

## 7. Implementation Checklist

### Phase 1: Foundation (P0)

- [ ] Implement `niri.loop.defer(fn, delay_ms)` Lua helper
- [ ] Implement `niri.schedule_wrap(fn)` Lua helper
- [ ] Implement `niri.loop.wait(timeout, cond, interval)` in Rust
- [ ] Add timer introspection: `get_due_in()`, `set_repeat()`, `get_repeat()`
- [ ] Add enhanced error context for timer callbacks
- [ ] Add `FromLuaTable` trait to niri-lua
- [ ] Add `ExtractField` helper trait
- [ ] Implement `WindowRule` extractor (P0 - critical for window management)
- [ ] Implement `WindowMatch` extractor with regex support
- [ ] Extend `niri.events` with handler ID returns
- [ ] Extend `niri.events` with pattern matching
- [ ] Implement `niri.state.watch` Lua helper
- [ ] **Rich event payloads**: Update `emit_window_open()` to include full `Window` object
- [ ] **Rich event payloads**: Update `emit_window_close()` to include full `Window` object  
- [ ] **Rich event payloads**: Update `emit_window_focus()` with current and previous `Window`
- [ ] **Rich event payloads**: Update `emit_workspace_activate()` with full `Workspace` object
- [ ] **Rich event payloads**: Add `window_to_lua()`, `workspace_to_lua()` helper functions
- [ ] Add unit tests for all new helpers

### Phase 2: Extended Features (P1)

- [ ] Implement `niri.state.refresh()` in `runtime_api.rs`
- [ ] Add event context detection for `refresh()` behavior
- [ ] Document `refresh()` performance implications
- [ ] Implement `Animation` extractor with spring/easing support
- [ ] Implement extractors for all 11 individual animation types
- [ ] Implement `OutputConfig` extractor
- [ ] Implement `Workspace` extractor
- [ ] Add `FromLuaTable` derive macro
- [ ] Migrate existing extractors to trait (backwards-compatible)

### Phase 3: Polish (P1)

- [ ] Generate EmmyLua type definitions for new extractors
- [ ] Add integration tests for extractors
- [ ] Document extractor API in wiki

### Phase 4: Complete Coverage (P2)

- [ ] Implement `LayerRule` extractor
- [ ] Implement `SwitchEvent` extractor
- [ ] Implement `NamedWorkspace` extractor
- [ ] Remove legacy `extract_*` functions (after deprecation period)

**Note**: UI integration tasks (hit_test, get_widget_bounds, popup control) are tracked in `lua_bindings.md`.

---

## 8. File Changes Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `niri-lua/src/lua/loop_helpers.lua` | New | `defer()` and `schedule_wrap()` Lua helpers |
| `niri-lua/src/loop_api.rs` | Modify | Add `wait()`, timer introspection, enhanced error context |
| `niri-lua/src/runtime_api.rs` | Modify | Add `niri.state.refresh()` function |
| `niri-lua/src/extractors/mod.rs` | New | Module for extractor trait |
| `niri-lua/src/extractors/traits.rs` | New | `FromLuaTable` trait definition |
| `niri-lua/src/extractors/animations.rs` | New | `Animation`, spring/easing extractors |
| `niri-lua/src/extractors/window_rules.rs` | New | `WindowRule`, `WindowMatch` extractors |
| `niri-lua/src/extractors/outputs.rs` | New | `OutputConfig`, `Mode`, `Position` extractors |
| `niri-lua/src/events_proxy.rs` | Modify | Add handler IDs, pattern matching |
| `niri-lua/src/lua/state_watch.lua` | New | Lua helper implementation |
| `niri-lua/src/runtime.rs` | Modify | Load loop_helpers.lua and state_watch.lua at init |
| `niri-lua/types/api.lua` | Modify | Add EmmyLua annotations for new APIs |
| `niri-lua-derive/src/lib.rs` | Modify | Add FromLuaTable derive (P1) |
| `src/lua_event_hooks.rs` | Modify | Rich event payloads with full IPC objects |

**Note**: niri-ui file changes (extractors, API registration) are documented in `lua_bindings.md`.

---

## 9. Dependencies

```
niri-lua-extensions (this spec)
    ├── Async/Loop API Extensions
    │   ├── niri.loop.defer (Lua helper)
    │   │   └── depends on: niri.loop.new_timer (existing)
    │   ├── niri.schedule_wrap (Lua helper)
    │   │   └── depends on: niri.schedule (existing)
    │   ├── niri.loop.wait (Rust impl)
    │   │   └── standalone, event loop integration
    │   └── Timer introspection (Rust impl)
    │       └── extends existing TimerHandle
    │
    ├── State Interface Improvements
    │   ├── Rich event payloads (P0)
    │   │   └── depends on: niri_ipc types (existing)
    │   │   └── modifies: src/lua_event_hooks.rs
    │   └── niri.state.refresh (P1)
    │       └── depends on: EventStreamState cache (existing)
    │       └── depends on: thread-local event context (existing)
    │
    ├── niri.state.watch helper
    │   └── depends on: niri.events extensions (handler IDs, patterns)
    │   └── depends on: niri.loop.new_timer (existing)
    │   └── depends on: niri.schedule (existing)
    │
    ├── FromLuaTable trait
    │   └── standalone, no dependencies
    │
    └── Complex Extractors (use FromLuaTable)
        ├── WindowRule extractor (P0)
        │   └── depends on: FromLuaTable trait
        │   └── depends on: regex crate (existing)
        ├── Animation extractor (P1)
        │   └── depends on: FromLuaTable trait
        │   └── depends on: niri-config animation types
        └── OutputConfig extractor (P1)
            └── depends on: FromLuaTable trait
```

---

## 10. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| `niri.loop.wait` blocks Lua execution | Document clearly; recommend event-driven patterns for most use cases |
| GC cleanup of subscriptions may not fire immediately | Document that `sub:cancel()` is preferred; GC is a safety net |
| Pattern matching may have performance impact | Use simple prefix matching; avoid regex |
| Breaking changes to events API | Handler ID is additive; old code continues to work |
| Extractor trait adds learning curve | Provide clear examples; existing functions still work |
| Timer error context adds overhead | Only format error string on actual error, not on success path |
| Rich event payloads increase memory per event | Only clone data that's actually needed; IPC types are lightweight |
| `niri.state.refresh()` called too frequently | Document as expensive; recommend event payloads for most cases |
| `refresh()` doesn't see action effects immediately | Document that EventStreamState must be updated first; action must complete |
| Event payload changes break existing handlers | Additive change: new fields added, existing fields preserved |
