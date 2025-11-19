# Phase 3 Implementation Guide: Runtime State Access

**Status:** Ready for implementation  
**Estimated Time:** 1-2 days focused work  
**Complexity:** Medium-High  

## Session History & Context

This implementation plan was developed during a design session that examined the current Lua integration and identified critical gaps for runtime state access.

**Session Timeline:**
1. **Session resumed** from planning phase with comprehensive Phase 3 roadmap
2. **Problem discovered**: Lua runtime dropped after config parsing (line 177 in `src/main.rs`)
3. **Architecture analysis**: Evaluated three approaches for state access
4. **Pattern discovery**: Found IPC server already uses message passing (`src/ipc/server.rs:291-326`)
5. **Decision made**: Adopt same pattern for consistency and proven safety
6. **Custom events decision**: Selected Neovim-style custom events for Phase 4
7. **Documentation created**: Complete implementation guide with step-by-step instructions

**Key Files Examined During Session:**
- `src/main.rs` (lines 170-220) - Discovered runtime being dropped
- `src/niri.rs` (lines 189-300) - Analyzed Niri struct for runtime field placement
- `src/ipc/server.rs` (lines 291-326) - Discovered message passing pattern
- `niri-lua/src/runtime.rs` - Examined current LuaRuntime implementation
- `niri-lua/src/lib.rs` - Checked current module exports
- `niri-ipc/src/state.rs` - Reviewed IPC type definitions for bridge layer

**Status at Session Start:**
- ✅ Phase 1 (Module System) - Complete
- ✅ Phase 2 (Configuration API) - Complete  
- ⚙️ Phase 3 (Runtime State Access) - Planning complete, ready to implement
- 📋 Phase 4 (Event System) - Architecture decision made

## Overview

This guide provides step-by-step instructions for implementing Phase 3 of the Lua integration roadmap. The goal is to enable Lua scripts to query Niri's runtime state (windows, workspaces, outputs) using a safe event loop message passing pattern.

## Architecture Decision

We're using **Event Loop Message Passing** - the same pattern the IPC server uses (see `src/ipc/server.rs:291-326`).

### Design Rationale (From Implementation Session)

**Problem Identified:**
During the previous implementation session, we discovered that the Lua runtime was being **dropped immediately after config parsing** (in `src/main.rs:177`). This meant:
- No connection to the event loop or compositor state
- Lua couldn't access windows, workspaces, or execute actions at runtime
- The system was just a config preprocessing tool, not a true extension API

**Alternatives Considered:**

1. **Direct State References (Rejected)**
   - Store `&State` or `Arc<Mutex<State>>` in Lua runtime
   - **Problem**: Lifetime issues - Lua runtime outlives borrow checker guarantees
   - **Problem**: Circular dependency between `niri` and `niri-lua` crates
   - **Problem**: Potential deadlocks with Arc<Mutex<>>

2. **Callback Functions (Rejected)**
   - Pass closures that capture State to Lua
   - **Problem**: Complex lifetime annotations
   - **Problem**: Not idiomatic for Smithay's event loop architecture
   - **Problem**: Difficult to test in isolation

3. **Event Loop Message Passing (Selected)** ✅
   - Same pattern IPC server uses successfully
   - **Discovered**: `src/ipc/server.rs:291-326` already implements this exact pattern
   - **Advantage**: Zero unsafe code, no lifetime issues
   - **Advantage**: Synchronous from Lua's perspective (blocks until ready)
   - **Advantage**: Fits naturally with calloop architecture
   - **Advantage**: Generic `RuntimeApi<S>` avoids circular dependencies

**Key Insight from Session:**
The IPC server implementation proved this pattern works in production. By following the exact same approach, we inherit all the safety guarantees and avoid reinventing the wheel.

**Architecture Decision Made:**
Use generic `RuntimeApi<S>` struct that doesn't directly reference the `State` type, eliminating circular dependency between crates while maintaining type safety.

### Why This Pattern?

✅ **Safe state access** - No lifetime issues, no unsafe code  
✅ **Synchronous from Lua's perspective** - Lua code blocks until result ready  
✅ **Proven in production** - IPC server uses this successfully (see `src/ipc/server.rs:291-326`)  
✅ **Thread-safe** - Compositor state accessed only from event loop thread  
✅ **No circular dependencies** - Generic over state type  
✅ **Testable** - Can mock event loop in tests  

### Pattern Example (Copied from IPC Server)

```rust
// Lua calls a runtime API function
niri.windows.get_all()

// Internally, we:
let (tx, rx) = mpsc::channel();
event_loop.insert_idle(move |state| {
    let windows = state.niri.layout.windows(); // Access state safely
    let result = convert_to_lua_tables(windows);
    tx.send(result).unwrap();
});
rx.recv().unwrap() // Block until compositor processes
```

## Implementation Roadmap

### Phase 1: Infrastructure (Tasks 1-3)
**Goal:** Keep Lua runtime alive throughout compositor lifetime

#### Task 1: Add lua_runtime field to Niri struct
**File:** `src/niri.rs` (around line 200)

**Before:**
```rust
pub struct Niri {
    pub config: Rc<RefCell<Config>>,
    pub config_file_output_config: niri_config::Outputs,
    pub config_file_watcher: Option<Watcher>,
    // ... other fields
}
```

**After:**
```rust
pub struct Niri {
    pub config: Rc<RefCell<Config>>,
    pub config_file_output_config: niri_config::Outputs,
    pub config_file_watcher: Option<Watcher>,
    
    /// Lua runtime for scripting and configuration
    pub lua_runtime: Option<niri_lua::LuaRuntime>,
    
    // ... other fields
}
```

**Implementation Notes:**
- Add field after `config_file_watcher` for logical grouping
- Use `Option<>` because runtime may not always be present (e.g., KDL-only configs)
- Import `niri_lua` crate at top of file if not already imported

#### Task 2: Keep runtime alive in main.rs
**File:** `src/main.rs` (around lines 175-220)

**Current Behavior:**
```rust
// Around line 177
let runtime = lua_config.runtime();
match apply_lua_config(runtime, &mut config) {
    Ok(_) => { /* ... */ }
    Err(e) => { /* ... */ }
}
// runtime is DROPPED here - this is the problem!
```

**New Behavior:**
```rust
// Around line 177
let runtime = lua_config.runtime();
match apply_lua_config(runtime, &mut config) {
    Ok(_) => { /* ... */ }
    Err(e) => { /* ... */ }
}

// KEEP runtime alive by extracting it from lua_config
let lua_runtime = lua_config.take_runtime(); // We'll need to add this method

// Later, when creating State (search for "let mut state = State")
// Store runtime in state.niri.lua_runtime
state.niri.lua_runtime = Some(lua_runtime);
```

**Prerequisites:**
- Add `take_runtime()` method to `LuaConfig` in `niri-lua/src/config.rs`

#### Task 3: Verify compositor starts
**Action:** Build and run Niri

```bash
cargo build
cargo run
```

**Expected Result:**
- Compositor starts normally
- No crashes or errors
- Lua config still loads correctly
- Runtime persists but doesn't do anything yet

---

### Phase 2: IPC Bridge (Tasks 4-6)
**Goal:** Convert IPC types to Lua-friendly tables

#### Task 4: Create ipc_bridge.rs
**File:** `niri-lua/src/ipc_bridge.rs` (new file, ~150 lines)

**Purpose:** Convert `niri_ipc::Window`, `Workspace`, `Output` to Lua tables

**Structure:**
```rust
use mlua::prelude::*;
use niri_ipc::{Window, Workspace, Output};

/// Convert a niri_ipc::Window to a Lua table
pub fn window_to_lua(lua: &Lua, window: &Window) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    table.set("id", window.id)?;
    table.set("title", window.title.clone())?;
    table.set("app_id", window.app_id.clone())?;
    table.set("is_focused", window.is_focused)?;
    table.set("is_floating", false)?; // TODO: Get from actual state
    // Add geometry, workspace_id, etc.
    Ok(table)
}

/// Convert a niri_ipc::Workspace to a Lua table
pub fn workspace_to_lua(lua: &Lua, workspace: &Workspace) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    table.set("id", workspace.id)?;
    table.set("name", workspace.name.as_ref().map(|s| s.as_str()))?;
    table.set("is_active", workspace.is_active)?;
    table.set("output", workspace.output.as_ref().map(|s| s.as_str()))?;
    Ok(table)
}

/// Convert a niri_ipc::Output to a Lua table  
pub fn output_to_lua(lua: &Lua, output: &Output) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    table.set("name", output.name.clone())?;
    table.set("make", output.make.clone())?;
    table.set("model", output.model.clone())?;
    // Add modes, current mode, position, etc.
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_conversion() {
        let lua = Lua::new();
        let window = Window {
            id: 123,
            title: "Test Window".to_string(),
            app_id: Some("firefox".to_string()),
            is_focused: true,
            // ... other fields
        };
        
        let table = window_to_lua(&lua, &window).unwrap();
        assert_eq!(table.get::<_, u64>("id").unwrap(), 123);
        assert_eq!(table.get::<_, String>("title").unwrap(), "Test Window");
    }
}
```

#### Task 5: Implement Window conversion
**Details in Task 4** - Focus on getting all Window fields mapped correctly

**Key Fields to Include:**
- `id` (u64)
- `title` (String)
- `app_id` (Option<String>)
- `is_focused` (bool)
- `workspace_id` (u64) - Will need to query from actual state
- `is_floating` (bool) - Will need to query from actual state
- Geometry info (x, y, width, height)

#### Task 6: Implement Workspace/Output conversions
**Similar to Task 5** but for Workspace and Output types

**Workspace Fields:**
- `id` (u64)
- `name` (Option<String>)
- `is_active` (bool)
- `output` (Option<String>) - Which monitor it's on
- `windows` (array of window IDs)

**Output Fields:**
- `name` (String)
- `make` (String)
- `model` (String)
- `serial` (String)
- `physical_size` (width_mm, height_mm)
- `current_mode` (width, height, refresh_rate)

---

### Phase 3: Runtime API Core (Tasks 7-10)
**Goal:** Create the actual Lua API that users will call

#### Task 7: Create runtime_api.rs
**File:** `niri-lua/src/runtime_api.rs` (new file, ~250 lines)

**Structure:**
```rust
use mlua::prelude::*;
use calloop::LoopHandle;
use std::sync::mpsc;

/// Generic runtime API that can work with any State type
/// This avoids circular dependencies between niri and niri-lua
pub struct RuntimeApi<S> {
    event_loop: LoopHandle<'static, S>,
}

impl<S> RuntimeApi<S> {
    pub fn new(event_loop: LoopHandle<'static, S>) -> Self {
        Self { event_loop }
    }
    
    /// Register this API to the Lua runtime
    pub fn register_to_lua<F>(&self, lua: &Lua, query_fn: F) -> LuaResult<()>
    where
        F: Fn(&str) -> Vec<u8> + 'static + Send + Sync,
    {
        let windows_table = lua.create_table()?;
        
        // Register niri.windows.get_all()
        let query_fn_clone = query_fn.clone();
        windows_table.set("get_all", lua.create_function(move |lua, ()| {
            let result_bytes = query_fn_clone("windows::get_all");
            // Deserialize and convert to Lua tables
            // ... implementation
            Ok(result)
        })?)?;
        
        // Add to niri global
        let niri: LuaTable = lua.globals().get("niri")?;
        niri.set("windows", windows_table)?;
        
        Ok(())
    }
}
```

**Key Design Points:**
- Generic over `S` (State type) to avoid circular dependency
- Uses closure `query_fn` to abstract state access
- Returns serialized data (Vec<u8>) from compositor
- Deserializes and converts to Lua tables

#### Task 8: Implement get_windows() with message passing
**In runtime_api.rs:**

```rust
pub fn get_windows<S, F>(
    event_loop: &LoopHandle<'static, S>,
    state_query: F
) -> Vec<u8>
where
    S: 'static,
    F: FnOnce(&S) -> Vec<niri_ipc::Window> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    
    event_loop.insert_idle(move |state| {
        let windows = state_query(state);
        // Serialize windows
        let bytes = bincode::serialize(&windows).unwrap();
        tx.send(bytes).unwrap();
    });
    
    // Block until result ready
    rx.recv().expect("Failed to receive windows from compositor")
}
```

**Integration in main.rs:**
```rust
// After creating State, register runtime API
if let Some(lua_runtime) = &mut state.niri.lua_runtime {
    let event_loop_handle = event_loop.handle();
    
    lua_runtime.register_runtime_api(event_loop_handle, |query_type| {
        // This closure will be called from Lua
        match query_type {
            "windows::get_all" => {
                get_windows(&event_loop_handle, |state| {
                    // Extract windows from state.niri.layout
                    state.niri.layout.windows()
                        .map(|w| window_to_ipc(w))
                        .collect()
                })
            }
            _ => vec![]
        }
    })?;
}
```

#### Task 9: Implement get_focused_window()
**Similar to Task 8** but queries only the focused window

```rust
niri.windows.get_focused() -- Returns single window or nil
```

#### Task 10: Test message passing mechanism
**Create integration test:**

**File:** `niri-lua/tests/runtime_api_tests.rs` (new file)

```rust
#[test]
fn test_message_passing_basic() {
    // Create mock event loop
    let event_loop = EventLoop::try_new().unwrap();
    let handle = event_loop.handle();
    
    // Create runtime API
    let api = RuntimeApi::new(handle);
    
    // Test get_windows
    let result = api.get_windows(|_state| {
        vec![/* mock windows */]
    });
    
    assert!(!result.is_empty());
}
```

---

### Phase 4: Integration (Tasks 11-14)
**Goal:** Wire everything together

#### Task 11: Add register_runtime_api() to LuaRuntime
**File:** `niri-lua/src/runtime.rs` (add method around line 60)

```rust
impl LuaRuntime {
    // ... existing methods ...
    
    /// Register the runtime API for state queries
    pub fn register_runtime_api<S>(
        &mut self,
        event_loop: LoopHandle<'static, S>
    ) -> LuaResult<()>
    where
        S: 'static,
    {
        let api = RuntimeApi::new(event_loop);
        api.register_to_lua(&self.lua, |query_type| {
            // Query implementation will be provided by main.rs
            unimplemented!("Query handler should be provided by caller")
        })
    }
}
```

#### Task 12: Export modules in lib.rs
**File:** `niri-lua/src/lib.rs` (add around line 20)

```rust
// Tier 3: Runtime State Access
pub mod ipc_bridge;
pub mod runtime_api;

// In exports section
pub use ipc_bridge::{window_to_lua, workspace_to_lua, output_to_lua};
pub use runtime_api::RuntimeApi;
```

#### Task 13: Wire up in main.rs
**File:** `src/main.rs` (around lines 400-500, after State creation)

```rust
// After creating state, register runtime API if Lua config present
if let Some(lua_runtime) = &mut state.niri.lua_runtime {
    info!("Registering Lua runtime API");
    
    let event_loop_handle = event_loop.handle();
    lua_runtime.register_runtime_api(event_loop_handle)?;
    
    info!("Lua runtime API registered successfully");
}
```

#### Task 14: End-to-end test
**Create test Lua script:** `test_runtime_api.lua`

```lua
-- Test runtime API
print("Testing Niri runtime API...")

-- Query all windows
local windows = niri.windows.get_all()
print(string.format("Found %d windows", #windows))

for i, win in ipairs(windows) do
    print(string.format("  Window %d: %s (%s)", win.id, win.title, win.app_id or "unknown"))
end

-- Query focused window
local focused = niri.windows.get_focused()
if focused then
    print(string.format("Focused window: %s", focused.title))
else
    print("No focused window")
end
```

**Run test:**
```bash
# Start niri with test config
cargo run -- --config test_runtime_api.lua
```

---

### Phase 5: API Expansion (Tasks 15-17)
**Goal:** Add workspace queries and actions

#### Task 15: Add workspace queries
**In runtime_api.rs**, add:

```lua
niri.workspaces.get_all()      -- All workspaces
niri.workspaces.get_active()   -- Currently active workspace
niri.workspaces.get_by_name(name) -- Find by name
```

#### Task 16: Add action execution
**In runtime_api.rs**, add:

```lua
niri.windows.close(window_id)
niri.windows.focus(window_id)
niri.windows.move_to_workspace(window_id, workspace_name)
niri.windows.set_floating(window_id, floating)
```

**Implementation Note:**
Actions use the same message passing pattern, but send commands instead of queries:

```rust
let (tx, rx) = mpsc::channel();
event_loop.insert_idle(move |state| {
    state.niri.close_window(window_id);
    tx.send(()).unwrap();
});
rx.recv().unwrap(); // Wait for action to complete
```

#### Task 17: Document in LUA_TIER3_SPEC.md
Create comprehensive API documentation with:
- Function signatures
- Parameter descriptions  
- Return value specifications
- Usage examples
- Error handling patterns

---

### Phase 6: IPC Lua REPL (Bonus Feature)
**Goal:** Add interactive Lua REPL and one-shot code execution via IPC

This is a bonus feature that enables debugging and scripting by executing Lua code in the live compositor runtime through the IPC interface.

#### Task 18: Add Request::Lua variant to IPC
**File:** `niri-ipc/src/lib.rs` (around line 118, before closing of `Request` enum)

**Add new variant:**
```rust
pub enum Request {
    // ... existing variants ...
    OverviewState,
    
    /// Execute Lua code in the embedded runtime.
    ///
    /// If `code` is `None`, this starts an interactive REPL session.
    /// If `code` is `Some(string)`, executes the string and returns the result.
    ///
    /// This is useful for debugging, scripting, and live configuration changes.
    Lua {
        /// Lua code to execute, or None for REPL mode
        code: Option<String>,
    },
}
```

**Add corresponding Response variant:**
```rust
pub enum Response {
    // ... existing variants ...
    
    /// Result from Lua code execution.
    LuaResult(String),
}
```

#### Task 19: Implement Lua handler in IPC server
**File:** `src/ipc/server.rs` (around line 450, in the match statement for requests)

**Add handler using message passing pattern:**
```rust
Request::Lua { code } => {
    let (tx, rx) = async_channel::bounded(1);
    
    ctx.event_loop.insert_idle(move |state| {
        let result = if let Some(lua_runtime) = &state.niri.lua_runtime {
            match code {
                Some(code_str) => {
                    // Execute code and capture result
                    match lua_runtime.execute_string(&code_str) {
                        Ok(result) => format!("Success: {}", result),
                        Err(e) => format!("Error: {}", e),
                    }
                }
                None => {
                    // REPL mode not supported in message handler
                    "Error: REPL mode must be handled by CLI".to_string()
                }
            }
        } else {
            "Error: Lua runtime not available (using KDL config?)".to_string()
        };
        
        let _ = tx.send_blocking(result);
    });
    
    let result = rx.recv().await
        .map_err(|_| String::from("error executing Lua code"))?;
    Response::LuaResult(result)
}
```

**Prerequisites:**
- Add `execute_string()` method to `LuaRuntime` in `niri-lua/src/runtime.rs`

```rust
impl LuaRuntime {
    /// Execute a Lua string and return the result
    pub fn execute_string(&self, code: &str) -> LuaResult<String> {
        // Execute code in Lua environment
        let result: mlua::Value = self.lua.load(code).eval()?;
        
        // Convert result to string representation
        match result {
            mlua::Value::Nil => Ok("nil".to_string()),
            mlua::Value::Boolean(b) => Ok(b.to_string()),
            mlua::Value::Integer(i) => Ok(i.to_string()),
            mlua::Value::Number(n) => Ok(n.to_string()),
            mlua::Value::String(s) => Ok(s.to_str()?.to_string()),
            mlua::Value::Table(t) => {
                // Pretty-print table
                let serialized = self.lua.load(r#"
                    local tbl = ...
                    local function serialize(t, indent)
                        indent = indent or ""
                        local s = "{\n"
                        for k, v in pairs(t) do
                            s = s .. indent .. "  [" .. tostring(k) .. "] = "
                            if type(v) == "table" then
                                s = s .. serialize(v, indent .. "  ")
                            else
                                s = s .. tostring(v)
                            end
                            s = s .. ",\n"
                        end
                        return s .. indent .. "}"
                    end
                    return serialize(tbl)
                "#).call::<_, String>(t)?;
                Ok(serialized)
            }
            _ => Ok(format!("{:?}", result)),
        }
    }
}
```

#### Task 20: Add CLI command
**File:** `src/cli.rs` (around line 100, in `Msg` enum)

**Add new variant:**
```rust
#[derive(Subcommand)]
pub enum Msg {
    // ... existing variants ...
    PickColor,
    
    /// Execute Lua code in the embedded runtime.
    ///
    /// Without arguments, starts an interactive REPL session.
    /// With a code argument, executes the code and prints the result.
    ///
    /// Examples:
    ///   niri msg action lua
    ///   niri msg action lua "return niri.windows.get_all()"
    ///   niri msg action lua "print('Hello from Lua!')"
    Lua {
        /// Lua code to execute (omit for interactive REPL)
        code: Option<String>,
    },
    
    Action {
        #[command(subcommand)]
        action: Action,
    },
    // ... rest of variants
}
```

**Implementation Note:**
The `Msg::Lua` variant should map to `Request::Lua` in the IPC client code (likely in `src/ipc/client.rs` or similar).

#### Task 21: Implement interactive REPL loop
**File:** `src/ipc/client.rs` (or wherever CLI commands are processed)

**Add REPL implementation:**
```rust
// When handling Msg::Lua with code = None
fn handle_lua_repl(socket: &mut UnixStream) -> Result<()> {
    use std::io::{self, Write, BufRead};
    
    println!("=== Niri Lua REPL ===");
    println!("Type Lua code to execute. Use 'exit' or Ctrl+D to quit.");
    println!("Available: niri.windows, niri.workspaces, niri.outputs");
    println!();
    
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line_buffer = String::new();
    
    loop {
        // Print prompt
        print!("lua> ");
        io::stdout().flush()?;
        
        // Read line
        line_buffer.clear();
        let bytes_read = reader.read_line(&mut line_buffer)?;
        
        // Check for EOF (Ctrl+D)
        if bytes_read == 0 {
            println!("\nExiting REPL...");
            break;
        }
        
        let code = line_buffer.trim();
        
        // Check for exit command
        if code == "exit" || code == "quit" {
            println!("Exiting REPL...");
            break;
        }
        
        // Skip empty lines
        if code.is_empty() {
            continue;
        }
        
        // Send Lua execution request
        let request = Request::Lua {
            code: Some(code.to_string()),
        };
        
        // Write request to socket
        let json = serde_json::to_string(&request)?;
        writeln!(socket, "{}", json)?;
        
        // Read response
        let mut response_buffer = String::new();
        let mut socket_reader = BufReader::new(socket);
        socket_reader.read_line(&mut response_buffer)?;
        
        // Parse and display result
        match serde_json::from_str::<Reply>(&response_buffer)? {
            Ok(Response::LuaResult(result)) => {
                println!("{}", result);
            }
            Ok(_) => {
                println!("Unexpected response type");
            }
            Err(error) => {
                println!("Error: {}", error);
            }
        }
    }
    
    Ok(())
}
```

#### Task 22: Document usage and examples
**Create examples in documentation:**

**Example 1: Interactive REPL**
```bash
$ niri msg action lua
=== Niri Lua REPL ===
Type Lua code to execute. Use 'exit' or Ctrl+D to quit.
Available: niri.windows, niri.workspaces, niri.outputs

lua> local wins = niri.windows.get_all()
lua> print(#wins)
3
lua> print(wins[1].title)
Firefox
lua> exit
Exiting REPL...
```

**Example 2: One-shot execution**
```bash
$ niri msg action lua "return #niri.windows.get_all()"
Success: 3

$ niri msg action lua "local w = niri.windows.get_focused(); return w.title"
Success: Firefox

$ niri msg action lua "for _, w in ipairs(niri.windows.get_all()) do print(w.title) end"
Success: nil
# (prints window titles to compositor log)
```

**Example 3: Scripted window management**
```bash
# Close all Firefox windows
$ niri msg action lua "
  for _, w in ipairs(niri.windows.get_all()) do
    if w.app_id == 'firefox' then
      niri.windows.close(w.id)
    end
  end
"

# Move all terminals to workspace 'dev'
$ niri msg action lua "
  for _, w in ipairs(niri.windows.get_all()) do
    if w.app_id:match('term') then
      niri.windows.move_to_workspace(w.id, 'dev')
    end
  end
"
```

**Example 4: Debugging state**
```bash
# Inspect focused window properties
$ niri msg action lua "return niri.windows.get_focused()"
Success: {
  [id] = 12345,
  [title] = "Firefox",
  [app_id] = "firefox",
  [is_focused] = true,
  [workspace_id] = 1,
}
```

**Security Considerations:**
- Lua REPL has full access to Niri's runtime state and APIs
- Should only be used by trusted users (same privilege level as compositor)
- Consider adding a confirmation prompt for destructive operations in REPL mode
- Not intended for production automation (use config files or plugins instead)

**Use Cases:**
- **Debugging:** Inspect runtime state while compositor is running
- **Testing:** Try out Lua API functions before adding them to config
- **Quick scripting:** One-off window management tasks
- **Development:** Test new Lua features during development
- **Learning:** Explore the Lua API interactively

---

## Testing Strategy

### Unit Tests
- Test IPC conversions in isolation
- Test Lua table structure matches expectations
- Mock state data for consistent testing

### Integration Tests  
- Test message passing with real event loop
- Verify Lua can call APIs and get results
- Test error handling (nil results, invalid IDs)

### Manual Testing
- Start compositor with Lua config
- Call APIs from Lua REPL or scripts
- Open/close windows and verify queries update
- Test with multiple monitors/workspaces

---

## Common Pitfalls & Solutions

### Problem: Circular dependency between niri and niri-lua
**Solution:** Use generic `RuntimeApi<S>` that doesn't know about `State` type directly

### Problem: Lifetime issues with event loop handle
**Solution:** Use `'static` lifetime and clone handles when needed

### Problem: Lua tables not updating when state changes
**Solution:** Tables are snapshots - need to call API again to get fresh data

### Problem: Blocking Lua thread blocks compositor
**Solution:** Only block in `rx.recv()`, which waits for event loop idle callback

---

## Success Criteria Checklist

- [ ] Niri struct has lua_runtime field
- [ ] Runtime survives config loading and persists
- [ ] IPC types convert to Lua tables correctly
- [ ] Message passing works (Lua → Event Loop → Lua)
- [ ] `niri.windows.get_all()` returns array of window tables
- [ ] `niri.windows.get_focused()` returns focused window or nil
- [ ] `niri.workspaces.get_all()` returns array of workspace tables
- [ ] Actions execute successfully (close, focus, move)
- [ ] Zero unsafe code used
- [ ] No lifetime compilation errors
- [ ] Compositor performance unchanged (<5% overhead)
- [ ] Documentation complete
- [ ] **Bonus:** `niri msg action lua` starts interactive REPL
- [ ] **Bonus:** One-shot Lua execution via IPC works
- [ ] **Bonus:** REPL can query and modify runtime state

---

## Next Steps After Phase 3

Once Phase 3 is complete, we'll have:
✅ Lua scripts can query runtime state  
✅ Lua scripts can execute actions  
✅ Foundation for event system (Phase 4)  

**Phase 4 Preview:** Event Handling System
- Build on runtime API to add event listeners
- Implement custom user events (Neovim-style)
- Hook events into Niri core at key points
- Enable reactive Lua plugins

---

## References

- IPC Server implementation: `src/ipc/server.rs:291-326`
- IPC types: `niri-ipc/src/state.rs`
- Calloop documentation: https://docs.rs/calloop/
- Mlua documentation: https://docs.rs/mlua/
- Phase 4 Spec: `LUA_TIER4_SPEC.md` (to be created)
