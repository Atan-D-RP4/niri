# niri-lua

Lua scripting API for the Niri compositor.

This crate provides Lua scripting capabilities to Niri, allowing users to configure and extend Niri using Lua scripts.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Configuration API | ✅ Complete | Full KDL parity |
| Action System | ✅ Complete | ~90 actions implemented |
| State Queries | ✅ Partial | 4 queries (windows, focused_window, workspaces, outputs) |
| Event System | ⚠️ Partial | Infrastructure complete, most events not wired |
| Plugin System | 🚧 Stub | Discovery works, sandbox/lifecycle not implemented |
| Hot Reload | ✅ Complete | Uses polling (not inotify) |
| Async/Safety | 🚧 Planned | No execution timeouts yet (see LUA_ASYNC_IMPLEMENTATION.md) |
| LSP Support | 🚧 Planned | EmmyLua type definitions not yet generated |

> **TODO: Simplify config_proxy.rs** - The config proxy uses `serde_json::Value` as an intermediary
> format. Consider whether direct Lua-to-Config conversion would be more efficient.

> **TODO: Unify event_emitter.rs** - Contains two parallel implementations (Rust struct and
> Lua-based global tables). Evaluate which is better and prune the unused code.

## Features

- **Reactive Configuration API**: Configure Niri using `niri.config.*` proxy tables
- **Event System**: Subscribe to compositor events via `niri.events:on()`
- **Action System**: Execute compositor actions via `niri.action:*()`
- **State Queries**: Query windows, workspaces, outputs via `niri.state.*`
- **Runtime Changes**: Modify configuration at runtime via IPC

## Architecture

### Core Components

- `config_proxy`: Reactive configuration system with `niri.config.*` proxies
- `config_converter`: Applies pending config changes to Niri Config
- `action_proxy`: ~90 compositor actions via `niri.action:*()`
- `events_proxy`: Event subscription via `niri.events:on/once/off()`
- `runtime_api`: State queries via `niri.state.*`
- `runtime`: Lua runtime management

## Usage

```rust
use niri_lua::{LuaConfig, apply_pending_lua_config};
use niri_config::Config;

// Load Lua configuration
let lua_config = LuaConfig::from_file("config.lua")?;

// Get pending changes from the reactive API
let pending = lua_config.runtime().get_pending_config_changes()?;

// Apply to Niri config
let mut config = Config::default();
apply_pending_lua_config(&pending, &mut config)?;
```

## Example Lua Configuration

```lua
-- Layout configuration
niri.config.layout.gaps = 16
niri.config.layout.center_focused_column = "never"

-- Input configuration
niri.config.input.keyboard.xkb.layout = "us"
niri.config.input.touchpad.natural_scroll = true

-- Add keybindings (uses :add() for collections)
niri.config.binds:add({
    { key = "Super+Return", action = "spawn", args = { "alacritty" } },
    { key = "Super+Q", action = "close-window" },
    { key = "Super+H", action = "focus-column-left" },
    { key = "Super+L", action = "focus-column-right" },
})

-- Add window rules
niri.config.window_rules:add({
    {
        match = { app_id = "firefox" },
        open_maximized = true,
    },
})

-- Spawn programs at startup
niri.action:spawn({ "waybar" })
niri.action:spawn({ "mako" })

-- Subscribe to events
niri.events:on("window:open", function(data)
    niri.utils.log("Window opened: " .. data.app_id)
end)
```

## API Namespaces

| Namespace | Purpose | Example |
|-----------|---------|---------|
| `niri.config` | Configuration proxy | `niri.config.layout.gaps = 16` |
| `niri.action` | Compositor actions | `niri.action:spawn({"kitty"})` |
| `niri.events` | Event system | `niri.events:on("window:open", fn)` |
| `niri.state` | Query compositor state | `niri.state.windows()` |
| `niri.utils` | Logging and utilities | `niri.utils.log("msg")` |

## Dependencies

- `mlua`: Lua bindings for Rust
- `niri-config`: Niri configuration structures
- `niri-ipc`: Niri IPC types
- `anyhow`: Error handling
- `serde_json`: JSON serialization for config changes

## Testing

Run tests with:

```bash
cargo test -p niri-lua
```
