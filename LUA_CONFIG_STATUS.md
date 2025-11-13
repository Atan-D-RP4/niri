# Lua Configuration Implementation Status

## Current Status

Niri has **experimental Lua configuration support** that is currently **incomplete and limited in functionality**. This document describes the current state, limitations, and future improvements.

## What Works

1. **Lua files can be loaded** - `~/.config/niri/niri.lua` or `~/.config/niri/init.lua` can be created and will be executed at startup
2. **Basic Lua API functions**:
   - `niri.log(message)` - Log messages to stdout
   - `niri.debug(message)` - Debug messages
   - `niri.warn(message)` - Warning messages
   - `niri.error(message)` - Error messages
   - `niri.version()` - Get Niri version
   - `niri.spawn(command)` - Spawn a command (currently just logs, not fully functional)

## What Doesn't Work

⚠️ **None of the configuration APIs actually modify Niri settings.** This is the critical limitation:

- ❌ Input configuration (keyboard, mouse, touchpad settings)
- ❌ Output/display configuration
- ❌ Layout configuration (gaps, focus ring, borders, etc.)
- ❌ Keybinding registration
- ❌ Window rules
- ❌ Animation settings
- ❌ Startup commands

## Why Doesn't It Work?

The Lua configuration system was implemented as a **placeholder architecture** with three fundamental design gaps:

### 1. Configuration Doesn't Propagate
```rust
// In src/lua_extensions/config.rs
pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
    let runtime = LuaRuntime::new()?;
    runtime.register_component(|action, args| {
        info!("Lua action: {} with args {:?}", action, args);
        Ok(())  // <-- Just logs, doesn't do anything!
    })?;
    runtime.load_file(&path)?;
    Ok(Self { runtime })
}
```

The Lua config is **executed but its return value is ignored**. There's no mechanism to apply Lua configuration to the running Niri instance.

### 2. No Configuration Return Channel
The Lua script can define configuration in tables (as shown in `examples/niri.lua`), but there's no way for that configuration to be extracted and applied to Niri's internal `Config` struct.

### 3. Config Immutability
Once Niri's `Config` struct is loaded and passed to the compositor, it's **immutable**. Runtime modifications from Lua wouldn't be possible without architectural changes.

## How It Fails in Real Sessions

When a user creates `~/.config/niri/niri.lua`:

1. ✅ The file is detected
2. ✅ Default KDL config is loaded as fallback
3. ✅ Lua file is executed
4. ❌ Lua configuration is not applied to the loaded Config
5. ❌ User gets whatever was in the default KDL config, not their Lua config

## Test Coverage Issues

The current test suite in `niri-config/src/lib.rs` only verifies:
- ✅ Lua file can be created in config directory
- ✅ No KDL file is created if Lua file exists
- ❌ Does NOT verify configuration is actually applied
- ❌ Does NOT test in actual Niri session

This is why "all tests pass" but the feature doesn't work in practice.

## Future Implementation Plan

To properly implement Lua configuration:

### Option 1: Lua-returns-Config (Recommended)
- Lua script returns a configuration structure
- Merge with KDL config in main.rs before passing to compositor
- Pros: Clean, maintainable, composable
- Cons: Requires serializing Config back to Lua-consumable format

### Option 2: Lua-replaces-KDL
- If `niri.lua` exists, skip KDL loading entirely
- Lua must return complete configuration
- Pros: Simpler flow, clearer intent
- Cons: Users must define everything in Lua

### Option 3: Runtime Lua API
- Keep KDL config as base
- Lua can call Niri actions at runtime (keybindings, mode changes, etc.)
- Pros: Maximum flexibility, non-breaking change
- Cons: More complex, different from config-as-code paradigm

## Temporary Workaround

For now, use **KDL configuration**:
- Create `~/.config/niri/config.kdl`
- Configuration is applied immediately and works correctly
- Lua files will be ignored if KDL config exists
- Expected: KDL is checked before Lua in config loading

## Examples

### Current Example Config

The `examples/niri.lua` file demonstrates the intended Lua API structure but **the configuration defined there is not actually applied**:

```lua
local config = {
    input = { /* ... */ },
    layout = { /* ... */ },
    binds = { /* ... */ },
}
return config
```

This Lua file:
- ✅ Loads without error
- ✅ Returns a valid configuration table
- ❌ Does not affect Niri's behavior

## Related Code Locations

- **Config Loading**: `niri-config/src/lib.rs` lines 498-525
- **Lua Execution**: `src/main.rs` lines 159-180
- **Lua API**: `src/lua_extensions/niri_api.rs`
- **Tests**: `niri-config/src/lib.rs` lines 2230-2244

## Migration Path

When Lua config is properly implemented:

1. Current KDL users: No change needed, KDL still works
2. New Lua users: Create `~/.config/niri/niri.lua` for configuration
3. Advanced users: Mix KDL base + Lua extensions/overrides

## See Also

- Astra project (inspiration for Lua integration)
- KDL syntax: https://kdl.dev
- Niri configuration wiki: https://yalter.github.io/niri/Configuration:-Introduction
