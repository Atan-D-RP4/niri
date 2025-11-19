# Phase 3 Quick Reference Card

**Goal:** Enable Lua scripts to query Niri's runtime state  
**Pattern:** Event Loop Message Passing (same as IPC server)  
**Status:** Ready for implementation  

## Design Decision Summary

**Problem:** Lua runtime dropped after config parsing (no runtime state access)  
**Solution:** Event loop message passing (proven pattern from IPC server)  
**Reference:** `src/ipc/server.rs:291-326`  

**Why This Pattern:**
- ✅ Zero unsafe code, no lifetime issues
- ✅ Proven in production (IPC server)
- ✅ Generic `RuntimeApi<S>` avoids circular dependencies
- ✅ Synchronous API from Lua's perspective

**Alternatives Rejected:**
- ❌ Direct state references (lifetime issues)
- ❌ Arc<Mutex<State>> (deadlock risk, not idiomatic)
- ❌ Callback closures (complex lifetimes)  

## Key Files to Create

```
niri-lua/src/
├── ipc_bridge.rs         # ~150 lines - IPC type → Lua table conversions
├── runtime_api.rs        # ~250 lines - Generic RuntimeApi<S> with event loop
└── tests/
    └── runtime_api_tests.rs  # ~100 lines - Integration tests

docs/
└── LUA_TIER3_SPEC.md     # Complete API reference
```

## Key Files to Modify

```diff
src/niri.rs (line ~200)
+ pub lua_runtime: Option<niri_lua::LuaRuntime>,

src/main.rs (lines ~175-220)
  let runtime = lua_config.runtime();
  apply_lua_config(runtime, &mut config)?;
+ let lua_runtime = lua_config.take_runtime();
+ state.niri.lua_runtime = Some(lua_runtime);

src/main.rs (after State creation, lines ~400-500)
+ if let Some(lua_runtime) = &mut state.niri.lua_runtime {
+     lua_runtime.register_runtime_api(event_loop.handle())?;
+ }

niri-lua/src/lib.rs (line ~20)
+ pub mod ipc_bridge;
+ pub mod runtime_api;
+ pub use ipc_bridge::{window_to_lua, workspace_to_lua, output_to_lua};
+ pub use runtime_api::RuntimeApi;

niri-lua/src/runtime.rs (line ~60)
+ pub fn register_runtime_api<S>(&mut self, event_loop: LoopHandle<'static, S>) -> LuaResult<()>
+ where S: 'static
+ { /* ... */ }
```

## Core Pattern: Message Passing

```rust
// Pattern used throughout runtime API
let (tx, rx) = mpsc::channel();

event_loop.insert_idle(move |state| {
    let result = /* query or action on state */;
    tx.send(result).unwrap();
});

rx.recv().unwrap() // Blocks Lua until compositor processes
```

## API Surface (User-Facing Lua)

### Phase 3 Tier 1 (Core Queries)
```lua
-- Window queries
local windows = niri.windows.get_all()          -- Array of window tables
local focused = niri.windows.get_focused()      -- Single window table or nil

-- Workspace queries  
local workspaces = niri.workspaces.get_all()    -- Array of workspace tables
local active = niri.workspaces.get_active()     -- Single workspace table

-- Output queries
local outputs = niri.outputs.get_all()          -- Array of output tables
```

### Phase 3 Tier 2 (Actions)
```lua
-- Window actions
niri.windows.close(window_id)
niri.windows.focus(window_id)
niri.windows.move_to_workspace(window_id, workspace_name)
niri.windows.set_floating(window_id, true)
```

### Phase 3 Bonus: IPC Lua REPL
```bash
# Interactive REPL mode
niri msg action lua

# One-shot code execution
niri msg action lua "return #niri.windows.get_all()"
niri msg action lua "print(niri.windows.get_focused().title)"

# Scripted automation
niri msg action lua "
  for _, w in ipairs(niri.windows.get_all()) do
    if w.app_id == 'firefox' then
      niri.windows.close(w.id)
    end
  end
"
```

## Data Structures

### Window Table
```lua
{
    id = 123,                    -- u64
    title = "Firefox",           -- String
    app_id = "firefox",          -- String or nil
    is_focused = true,           -- Boolean
    is_floating = false,         -- Boolean
    workspace_id = 1,            -- u64
    geometry = {                 -- Table
        x = 0,
        y = 0,
        width = 1920,
        height = 1080
    }
}
```

### Workspace Table
```lua
{
    id = 1,                      -- u64
    name = "main",               -- String or nil
    is_active = true,            -- Boolean
    output = "DP-1",             -- String or nil
    windows = {123, 456, 789}    -- Array of window IDs
}
```

### Output Table
```lua
{
    name = "DP-1",               -- String
    make = "Samsung",            -- String
    model = "U28E590",           -- String
    serial = "...",              -- String
    physical_size = {            -- Table
        width_mm = 620,
        height_mm = 340
    },
    current_mode = {             -- Table
        width = 3840,
        height = 2160,
        refresh_rate = 60.0
    }
}
```

## Implementation Checklist

### Phase 1: Infrastructure ☐
- [ ] Add `lua_runtime` field to `Niri` struct
- [ ] Add `take_runtime()` method to `LuaConfig`  
- [ ] Keep runtime alive in `main.rs`
- [ ] Verify compositor builds and starts

### Phase 2: IPC Bridge ☐
- [ ] Create `niri-lua/src/ipc_bridge.rs`
- [ ] Implement `window_to_lua()`
- [ ] Implement `workspace_to_lua()`
- [ ] Implement `output_to_lua()`
- [ ] Write conversion unit tests

### Phase 3: Runtime API Core ☐
- [ ] Create `niri-lua/src/runtime_api.rs`
- [ ] Implement generic `RuntimeApi<S>` struct
- [ ] Implement `get_windows()` with message passing
- [ ] Implement `get_focused_window()`
- [ ] Write integration tests

### Phase 4: Integration ☐
- [ ] Add `register_runtime_api()` to `LuaRuntime`
- [ ] Export modules in `niri-lua/src/lib.rs`
- [ ] Wire up in `main.rs` after State creation
- [ ] End-to-end test with live Lua script

### Phase 5: API Expansion ☐
- [ ] Add workspace query methods
- [ ] Add output query methods
- [ ] Implement window action methods
- [ ] Create `docs/LUA_TIER3_SPEC.md`

### Phase 6: IPC Lua REPL (Bonus) ☐
- [ ] Add `Request::Lua` variant to `niri-ipc/src/lib.rs`
- [ ] Add `Response::LuaResult` variant to `niri-ipc/src/lib.rs`
- [ ] Implement Lua handler in `src/ipc/server.rs`
- [ ] Add `execute_string()` method to `LuaRuntime`
- [ ] Add `Msg::Lua` variant to CLI in `src/cli.rs`
- [ ] Implement interactive REPL loop in IPC client
- [ ] Document usage examples and security considerations

## Dependencies

### Cargo.toml Check
Ensure these dependencies exist in `niri-lua/Cargo.toml`:

```toml
[dependencies]
mlua = { version = "0.11.4", features = ["luajit", "vendored", "serialize"] }
calloop = "0.14"  # Should already exist via niri dependency
niri-ipc = { path = "../niri-ipc" }
bincode = "1.3"   # For serialization (if needed)
```

## Testing Commands

```bash
# Build only
cargo build

# Build and run
cargo run

# Run with test config
cargo run -- --config test_runtime_api.lua

# Run tests
cargo test --package niri-lua runtime_api

# Run with verbose logging
RUST_LOG=niri=debug cargo run

# Test IPC Lua REPL (after Phase 6 implementation)
niri msg action lua  # Interactive mode
niri msg action lua "return #niri.windows.get_all()"  # One-shot mode
```

## Example Test Script

**File:** `test_runtime_api.lua`

```lua
print("=== Niri Runtime API Test ===")

-- Test window queries
local windows = niri.windows.get_all()
print(string.format("Found %d windows:", #windows))
for _, win in ipairs(windows) do
    local focused = win.is_focused and " [FOCUSED]" or ""
    print(string.format("  - %s (%s)%s", win.title, win.app_id or "unknown", focused))
end

-- Test focused window
local focused = niri.windows.get_focused()
if focused then
    print(string.format("\nFocused: %s (ID: %d)", focused.title, focused.id))
end

-- Test workspace queries
local workspaces = niri.workspaces.get_all()
print(string.format("\nFound %d workspaces:", #workspaces))
for _, ws in ipairs(workspaces) do
    local active = ws.is_active and " [ACTIVE]" or ""
    local name = ws.name or string.format("#%d", ws.id)
    print(string.format("  - %s%s", name, active))
end

print("\n=== Test Complete ===")
```

## IPC Lua REPL Examples (Phase 6 Bonus)

### Interactive REPL Session
```bash
$ niri msg action lua
=== Niri Lua REPL ===
Type Lua code to execute. Use 'exit' or Ctrl+D to quit.
Available: niri.windows, niri.workspaces, niri.outputs

lua> local wins = niri.windows.get_all()
lua> print(#wins)
3
lua> for _, w in ipairs(wins) do print(w.title) end
Firefox
VSCode
Terminal
lua> local focused = niri.windows.get_focused()
lua> print(focused.app_id)
firefox
lua> exit
Exiting REPL...
```

### One-Shot Execution
```bash
# Count windows
$ niri msg action lua "return #niri.windows.get_all()"
Success: 3

# Get focused window title
$ niri msg action lua "local w = niri.windows.get_focused(); return w and w.title or 'none'"
Success: Firefox

# Close all Firefox windows
$ niri msg action lua "
  local count = 0
  for _, w in ipairs(niri.windows.get_all()) do
    if w.app_id == 'firefox' then
      niri.windows.close(w.id)
      count = count + 1
    end
  end
  return 'Closed ' .. count .. ' Firefox windows'
"
Success: Closed 2 Firefox windows

# Move terminals to workspace 'dev'
$ niri msg action lua "
  for _, w in ipairs(niri.windows.get_all()) do
    if w.app_id and w.app_id:match('term') then
      niri.windows.move_to_workspace(w.id, 'dev')
    end
  end
  return 'Done'
"
Success: Done
```

### Debugging State
```bash
# Inspect all workspace names
$ niri msg action lua "
  local names = {}
  for _, ws in ipairs(niri.workspaces.get_all()) do
    table.insert(names, ws.name or '#' .. ws.id)
  end
  return table.concat(names, ', ')
"
Success: main, browser, dev, music

# Check output configuration
$ niri msg action lua "
  local outputs = niri.outputs.get_all()
  for _, o in ipairs(outputs) do
    print(o.name .. ': ' .. o.current_mode.width .. 'x' .. o.current_mode.height)
  end
"
Success: nil
# (Output appears in compositor log)
```

## Common Issues & Solutions

### Issue: Circular dependency between niri and niri-lua
**Solution:** Use generic `RuntimeApi<S>` that doesn't reference `State` directly

### Issue: Lifetime errors with event loop handle  
**Solution:** Use `'static` lifetime, store handle in struct

### Issue: Tables not updating when state changes
**Solution:** Expected - tables are snapshots, call API again for fresh data

### Issue: Lua blocking compositor
**Solution:** Only blocks in `rx.recv()`, event loop processes idle callbacks

## Success Criteria

After implementation, you should be able to:

```lua
-- Query all windows
local windows = niri.windows.get_all()
assert(type(windows) == "table")

-- Get focused window
local focused = niri.windows.get_focused()
if focused then
    print(focused.title)  -- Works!
end

-- Close a window
niri.windows.close(focused.id)

-- Move window to workspace
niri.windows.move_to_workspace(focused.id, "browser")
```

**Phase 6 Bonus - IPC REPL:**
```bash
# Interactive debugging
$ niri msg action lua
lua> return niri.windows.get_focused().title
Success: Firefox

# One-shot scripting
$ niri msg action lua "print('Hello from Lua!')"
Success: nil
```

## Performance Targets

- Query overhead: < 1ms per call
- Memory overhead: < 5MB for runtime state
- Compositor impact: < 5% CPU increase
- No frame drops during queries

## Next Phase Preview

**Phase 4: Event Handling System**
- Build on runtime API for event listeners
- Implement custom user events (Neovim-style)
- Hook events into Niri core
- Enable reactive Lua plugins

```lua
-- Phase 4 preview
niri.on('window_opened', function(window)
    print("New window:", window.title)
end)

niri.emit('CustomPlugin::Ready', { version = "1.0" })
```

## References

- Full guide: `docs/PHASE3_IMPLEMENTATION_GUIDE.md`
- IPC reference: `src/ipc/server.rs:291-326`
- Roadmap: `LUA_IMPLEMENTATION_ROADMAP.md`
- IPC types: `niri-ipc/src/state.rs`
