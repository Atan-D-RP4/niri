# niri-lua

Lua scripting API for the Niri compositor.

This crate provides Lua scripting capabilities to Niri, allowing users to configure and extend Niri using Lua scripts.

## Features

- **Configuration API**: Define Niri configuration using Lua scripts
- **Event System**: Event emitter for handling Niri events
- **Hot Reload**: Automatic reloading of Lua configuration files on change
- **Plugin System**: Support for Lua plugins
- **Module Loader**: Custom Lua module loading system
- **Validators**: Configuration validation helpers
- **Type-safe Bindings**: Lua types that map to Niri configuration structures

## Architecture

The niri-lua crate is organized into several tiers:

### Tier 1: Foundation Layer
- `module_loader`: Custom Lua module loading
- `plugin_system`: Plugin discovery and management
- `event_emitter`: Event handling system
- `hot_reload`: File watching and hot reloading

### Tier 2: Configuration API
- `lua_types`: Lua representations of Niri types
- `validators`: Configuration validation
- `config_api`: Configuration access from Lua

### Core Components
- `config`: Lua configuration loading
- `config_converter`: Converting Lua config to Niri Config
- `niri_api`: Niri API exposed to Lua scripts
- `runtime`: Lua runtime management

## Usage

```rust
use niri_lua::{LuaConfig, apply_lua_config};
use niri_config::Config;

// Load Lua configuration
let lua_config = LuaConfig::from_file("config.lua")?;

// Apply to Niri config
let mut config = Config::default();
apply_lua_config(&lua_config, &mut config)?;
```

## Example Lua Configuration

```lua
niri.config.binds = {
    { key = "Super+A", action = "spawn", args = { "alacritty" } },
    { key = "Super+Q", action = "close-window", args = {} },
}

niri.config.startup = {
    { command = { "waybar" } },
}
```

## Dependencies

- `mlua`: Lua bindings for Rust
- `niri-config`: Niri configuration structures
- `niri-ipc`: Niri IPC types
- `anyhow`: Error handling
- `log`: Logging
- `regex`: Regular expressions
- `serde`: Serialization

## Testing

Run tests with:

```bash
cargo test -p niri-lua
```
