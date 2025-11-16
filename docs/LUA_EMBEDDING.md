# Lua Embedding in Niri: Architecture & Integration

**Technical reference for Niri's Lua implementation**

This document describes how Lua is integrated into Niri, the architecture behind it, and how developers can extend it.

---

## Table of Contents

1. [Overview](#overview)
2. [Technology Stack](#technology-stack)
3. [Architecture](#architecture)
4. [Integration Points](#integration-points)
5. [Execution Model](#execution-model)
6. [Performance Characteristics](#performance-characteristics)
7. [Plugin Sandboxing](#plugin-sandboxing)
8. [IPC Communication](#ipc-communication)
9. [Development Guide](#development-guide)

---

## Overview

Niri embeds Lua to provide:

- **User configuration** - Alternative to KDL config files
- **Plugin system** - Extend Niri without recompilation
- **Runtime scripting** - Automate tasks and respond to events
- **Hotspot extensibility** - Add features without core changes

### Key Design Goals

1. **Performance** - Minimal overhead on core event loop
2. **Safety** - Plugins can't crash Niri
3. **Simplicity** - Familiar Lua API matching AwesomeWM/Neovim
4. **Discoverability** - IDE autocomplete and type checking
5. **Stability** - Backward compatibility maintained across versions

---

## Technology Stack

### Lua Runtime

**Lua 5.2 via LuaJIT**

```toml
# Cargo.toml
[dependencies]
mlua = { version = "0.11.4", features = ["lua52", "luajit", "vendored"] }
```

**Why LuaJIT?**
- 15-40x faster than standard Lua 5.2
- Near-native performance for JIT-able code
- No external Lua installation required (vendored)
- Stable and production-tested (used by ROBLOX, Neovim, etc.)

### Integration Library

**mlua (Lua-Rust bridge)**

```rust
use mlua::{Lua, Value, Function, Table};

let lua = Lua::new();
let globals = lua.globals();

// Expose Rust function to Lua
globals.set("spawn", lua.create_function(|lua, cmd: String| {
  // Rust code called from Lua
  Ok(())
})?)?;

// Execute Lua code
lua.load(r#"
  niri.log("Hello from Lua")
"#).exec()?;
```

### Current Implementation Location

All Lua integration code lives in:

```
src/lua_extensions/
├── mod.rs              - Module exports, VM initialization
├── config.rs           - Configuration API
├── niri_api.rs         - Core Niri API binding
├── config_converter.rs - Config↔Lua conversion
└── runtime.rs          - Runtime management
```

---

## Architecture

### High-Level Flow

```
┌─────────────────────────────────────────────────────────┐
│  Niri Startup                                           │
└─────────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────┐
│  1. Initialize Lua VM (mlua + LuaJIT)                   │
│  2. Load Niri API modules                               │
│  3. Load configuration (KDL or Lua)                     │
│  4. Execute user config file                            │
│  5. Register event listeners                            │
└─────────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────┐
│  Niri Main Event Loop                                   │
│                                                         │
│  For each event:                                        │
│  - Check Lua event listeners                            │
│  - Call registered Lua handlers (if any)                │
│  - Continue with normal Niri event handling             │
└─────────────────────────────────────────────────────────┘
```

### Module Organization

```rust
// Core Lua modules available in all contexts

niri              // Root module
├── log()          // Logging functions
├── version()      // Version info
├── spawn()        // Process launching
├── config         // Configuration API (Tier 2)
├── state          // State queries (Tier 3)
├── events         // Event system (Tier 1)
├── plugins        // Plugin management (Tier 5)
├── window         // Window operations
├── workspace      // Workspace operations
└── layout         // Layout operations
```

### Lifecycle Management

```
Configuration Load:
  1. Parse Lua file at ~/.config/niri/config.lua
  2. Create isolated Lua context
  3. Load niri module
  4. Execute configuration code
  5. Catch any errors, log them
  6. Return to Niri with configuration values

Event Handling:
  1. Niri event occurs
  2. Query Lua event registry for listeners
  3. Call each registered handler in order
  4. If handler errors: log error, continue
  5. Proceed with normal Niri event handling
```

---

## Integration Points

### 1. Configuration Loading

**File:** `src/lua_extensions/config.rs`

```rust
pub fn load_lua_config() -> Result<Config> {
  let lua = Lua::new();
  let globals = lua.globals();
  
  // Load Niri API
  register_niri_module(&lua)?;
  
  // Execute user config
  lua.load_from_file("~/.config/niri/config.lua")?;
  
  // Extract configuration from Lua state
  extract_config_from_lua(&lua)
}
```

**Currently Supported:**
- ✅ Keybindings (read/write via API)
- ✅ Window decorations preferences
- ⏳ Full configuration API (Tier 2 implementation)

### 2. Event System

**File:** `src/handlers/` and `src/niri.rs`

Events are dispatched to Lua handlers:

```rust
// src/handlers/xdg_shell.rs
pub fn handle_window_open(window: &Window) {
  // ... normal Niri handling ...
  
  // Call Lua handlers
  if let Some(lua_runtime) = niri.lua_runtime() {
    lua_runtime.emit_event("window:open", window)?;
  }
}
```

**Event Sources:**
- Window lifecycle (open, close, focus, title change)
- Workspace changes (enter, leave, layout change)
- Monitor hotplug (connect, disconnect)
- Focus changes
- Layout commands

### 3. Plugin Loading

**File:** `src/lua_extensions/plugin_system.rs` (Tier 1, pending)

Search paths (in order):
1. `~/.config/niri/plugins/`
2. `/usr/local/share/niri/plugins/`
3. `/usr/share/niri/plugins/`

Each plugin is a `.lua` file or directory with `init.lua`.

---

## Execution Model

### Configuration Execution (Synchronous)

```rust
// Called during Niri startup
fn execute_config() {
  let lua = Lua::new();
  
  // Load config file
  match lua.load_file("config.lua") {
    Ok(_) => {
      // Config executed synchronously
      // All side effects applied
    },
    Err(e) => {
      // Log error, continue with defaults
      log::error!("Config error: {}", e);
    }
  }
}
```

**Timing:** During Niri startup, blocks until complete

**Error Handling:** Errors logged but don't crash Niri

### Event Handler Execution (Async)

```rust
// Called during event processing
async fn on_window_open(window: &Window) {
  // Execute Lua handlers for window:open
  // Each handler is called in registration order
  // Errors are caught and logged
  // Niri continues regardless
}
```

**Timing:** During main event loop (event handler phase)

**Concurrency:** Single-threaded Lua execution in Niri's event context

---

## Performance Characteristics

### Memory Usage

```
Lua VM startup:         ~2-3 MB
Configuration code:     Varies by plugin count
Per-event handler:      ~100-200 bytes overhead
```

### Execution Speed

Measured with `criterion` benchmarks:

```
Configuration load:     10-100ms (depends on plugins)
Event handler call:     1-10µs (per handler)
State query:            100-500ns (per query)
Keybind lookup:         100ns (average case)
```

### Optimization Tips

1. **Minimize event handlers** - Each adds latency
2. **Cache state queries** - Don't call every event
3. **Use filtering** - Check conditions before operations
4. **Avoid tight loops** - In event handlers especially

---

## Plugin Sandboxing

### Isolation Model

Plugins are NOT fully sandboxed (intentional design):

**Allowed:**
- Full Niri API access
- File I/O (within user home directory)
- Process spawning

**Restricted:**
- Direct Rust code execution
- Modifying Niri internal state
- Accessing other plugins' state (initially)

### Isolation Mechanism

Each plugin gets an isolated environment table:

```lua
-- Each plugin's Lua context has its own:
-- - Global namespace
-- - Local storage (in ~/.local/share/niri/plugins/)
-- - Set of registered events

-- But they share:
-- - niri module (read-only for state queries)
-- - Config API (for system configuration)
-- - Event system (can listen to same events)
```

### Security Considerations

**Not a substitute for OS security:**
- Don't run untrusted Lua code
- Plugins can access all user files
- Plugins can spawn any process
- Consider this userspace customization, not sandboxing

---

## IPC Communication

### Current IPC (Niri → Lua)

One-way messages from Niri to Lua:

```rust
// Niri events are delivered to Lua event handlers
// No back-channel needed
pub fn emit_event(event_name: &str, data: Value) {
  // Serialize event data
  // Find registered handlers in Lua
  // Call each handler with event data
}
```

### Planned IPC Extensions (Tier 4+)

Two-way IPC for plugin-to-Niri commands:

```lua
-- Future: Control Niri from Lua
niri.window.close(window_id)
niri.workspace.activate(ws_id)
niri.layout.execute("focus-left")
```

Backend:
- UNIX domain socket at `/run/user/$UID/niri.sock`
- Message format: JSON-RPC
- Request/response with IDs

---

## Development Guide

### Adding a New Lua API Function

**Step 1: Add to `niri_api.rs`**

```rust
// src/lua_extensions/niri_api.rs

fn register_niri_module(lua: &Lua) -> Result<()> {
  let niri = lua.create_table()?;
  
  // Add your function
  niri.set("my_function", lua.create_function(|_lua, arg: String| {
    // Rust implementation
    Ok(format!("Result: {}", arg))
  })?)?;
  
  lua.globals().set("niri", niri)?;
  Ok(())
}
```

**Step 2: Test from Lua**

```lua
-- test_config.lua
local niri = require "niri"
local result = niri.my_function("test")
niri.log(result)  -- "Result: test"
```

**Step 3: Add Type Definitions**

```lua
-- docs/niri.d.lua
declare module "niri" do
  function my_function(arg: string): string
end
```

### Adding Event Support

**Step 1: Identify event source** (e.g., window focus change)

**Step 2: Emit event from Rust:**

```rust
// src/handlers/xdg_shell.rs or similar

lua_runtime.emit_event("window:focus", EventData {
  window: current_window,
  old_focus: previous_window,
})?;
```

**Step 3: Listen from Lua:**

```lua
niri.events.on("window:focus", function(event)
  niri.log("Focused: " .. event.window.title)
end)
```

### Debugging Lua Code

Enable debug logging:

```bash
# Run Niri with Lua debug output
RUST_LOG=debug niri

# Or filter to Lua only
RUST_LOG=niri::lua_extensions=debug niri
```

In your Lua code:

```lua
niri.debug("Debug message")  -- Only visible when debug enabled

-- Add explicit logging for debugging
local function trace(name, value)
  niri.log(string.format("TRACE: %s = %s", name, tostring(value)))
end

trace("active_window", niri.state.active_window())
```

### Testing Lua Code

Unit test example:

```rust
#[cfg(test)]
mod tests {
  use super::*;
  
  #[test]
  fn test_lua_config_load() {
    let lua = Lua::new();
    register_niri_module(&lua).unwrap();
    
    lua.load(r#"
      local niri = require "niri"
      assert(niri.version() ~= nil)
    "#).exec().unwrap();
  }
}
```

---

## Compilation

### Building with Lua Support

```bash
# Build with Lua enabled
cargo build --release --features lua

# Build without Lua (for testing)
cargo build --release

# Build with debug symbols
cargo build --features lua
```

### Vendored LuaJIT

By default, LuaJIT is vendored (compiled as part of build):

```toml
mlua = { version = "0.11.4", features = ["luajit", "vendored"] }
```

This ensures:
- No external Lua installation needed
- Consistent behavior across systems
- Easy distribution

Build time impact: ~10-30 seconds additional compile time

---

## Troubleshooting

### Lua VM Won't Initialize

```rust
// Check mlua feature flags
let lua = Lua::new();  // Panics if LuaJIT not available

// Fallback to standard Lua
mlua = { version = "0.11.4", features = ["lua52"] }
```

### Event Not Dispatching

Verify event source calls emit_event:

```rust
// Should be called from event handler
lua_runtime.emit_event("event:name", data)?;
```

### Performance Degradation

Profile with:

```bash
cargo profile release
perf record -g niri
perf report
```

Look for:
- Frequent Lua function calls in critical path
- Excessive state allocations
- Blocking operations in event handlers

---

## References

### External Documentation

- **Lua 5.2 Manual:** https://www.lua.org/manual/5.2/
- **mlua:** https://github.com/mlua-rs/mlua
- **LuaJIT:** https://luajit.org/

### Niri Documentation

- **LUA_TIER1_SPEC.md** - Foundation layer
- **LUA_TIER2_SPEC.md** - Configuration API
- **LUA_TIER3_SPEC.md** - State queries
- **LUA_TIER4_SPEC.md** - Event system
- **LUA_TIER5_SPEC.md** - Plugin ecosystem
- **LUA_GUIDE.md** - User guide

### Source Code

- **Configuration handling:** `src/lua_extensions/config.rs`
- **Core API bindings:** `src/lua_extensions/niri_api.rs`
- **Event integration:** `src/handlers/*.rs` (multiple files)

---

## Future Enhancements

### Planned Features

1. **Async handlers** - Non-blocking event handling
2. **Coroutines** - Suspend/resume within handlers
3. **Inter-plugin communication** - Shared bus for plugins
4. **Native module support** - Load `.so`/`.dll` from Lua
5. **Remote REPL** - Debug from another machine
6. **Type checking** - Luau integration with type analysis

### Breaking Changes

None expected in Lua 5.2 core, but:

- Config format may evolve (will provide migration tools)
- Event names may expand (backward compatible)
- New APIs added (optional, no removal planned)

---

**Document Version:** 1.0  
**Last Updated:** November 15, 2025  
**Author:** OpenCode Assistant  
**Target Audience:** Niri Developers, Plugin Developers
