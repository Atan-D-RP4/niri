# niri-lua

Lua scripting API for the Niri compositor.

This crate provides Lua scripting capabilities to Niri, allowing users to configure and extend Niri using Lua scripts.

## Vision

The niri-lua crate is the foundation for a **Neovim-like extensibility model** for the Niri compositor. Combined with the [niri-ui specification](../docs/NIRI_UI_SPECIFICATION.md), this enables:

- **Full Desktop Environment Creation**: Build complete desktop shells (panels, launchers, notification centers) entirely in Lua
- **Plugin Ecosystem**: Load user modules via standard Lua `require()` from config-relative paths (Neovim model)
- **IDE-like Extensibility**: Similar to how Neovim can be extended into a full IDE, Niri can be extended into a full DE

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Configuration API | ✅ Complete | Full KDL parity |
| Action System | ✅ Complete | ~90 actions implemented |
| Process API | ✅ Complete | spawn() with opts, ProcessHandle, streaming callbacks |
| State Queries | ✅ Partial | 4 queries (windows, focused_window, workspaces, outputs) |
| Event System | ⚠️ Partial | Infrastructure complete, most events not wired |
| Module System | ✅ Complete | Standard Lua `require()` with config-relative paths |
| Hot Reload | 🚧 Planned | Not yet implemented |
| Async/Safety | 🚧 Planned | No execution timeouts yet (see LUA_ASYNC_IMPLEMENTATION.md) |
| LSP Support | ✅ Available | EmmyLua type definitions generated in `types/api.lua` |

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
| `niri.action` | Process spawning | `niri.action:spawn({"cmd"}, {capture_stdout=true})` |
| `niri.events` | Event system | `niri.events:on("window:open", fn)` |
| `niri.state` | Query compositor state | `niri.state.windows()` |
| `niri.utils` | Logging and utilities | `niri.utils.log("msg")` |

## Dependencies

- `mlua`: Lua bindings for Rust (with Luau runtime)
- `niri-config`: Niri configuration structures
- `niri-ipc`: Niri IPC types
- `anyhow`: Error handling

## Future API Improvements

The following features are planned for future development:

### Targeted State Queries
Currently `niri.state.*` returns full collections. Planned improvements:
- `niri.state.get_window(id)` - Query a specific window by ID
- `niri.state.get_workspace(reference)` - Query workspace by ID, index, or name
- `niri.state.get_output(name)` - Query a specific output by name

### Reactive State Subscriptions
- `niri.state.subscribe(event, callback)` - Subscribe to state changes reactively
- This would enable patterns like watching for specific window property changes

### Event Handler State Freshness
Currently, event handlers see a pre-captured state snapshot (for deadlock avoidance).
Planned: Option to request fresh state within handlers for multi-action scenarios.

## Testing

Run tests with:

```bash
cargo test -p niri-lua
```
