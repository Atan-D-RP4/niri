# Lua Configuration Implementation Status

## Current Status

Niri has **experimental Lua configuration support** that is now **partially functional**. Lua scripts can now define configuration values that are extracted and applied to Niri's configuration. This document describes the current state, implemented features, and remaining limitations.

## What Works Now

1. **Lua files can be loaded** - `~/.config/niri/niri.lua` or `~/.config/niri/init.lua` can be created and will be executed at startup
2. **Basic Lua API functions**:
   - `niri.log(message)` - Log messages to stdout
   - `niri.debug(message)` - Debug messages
   - `niri.warn(message)` - Warning messages
   - `niri.error(message)` - Error messages
   - `niri.version()` - Get Niri version
   - `niri.spawn(command)` - Spawn a command (currently just logs, not fully functional)
3. **Configuration values can be set from Lua**:
   - ✅ `prefer_no_csd` - Set whether to prefer no client-side decorations
   - More settings can be easily added by extending `apply_lua_config()` in `src/lua_extensions/config_converter.rs`

## Recent Implementation (Session 2)

### What Was Implemented

1. **Configuration Extraction System** (`src/lua_extensions/runtime.rs`):
   - `get_global_string_opt()` - Extract optional string values from Lua globals
   - `get_global_bool_opt()` - Extract optional boolean values from Lua globals
   - `get_global_int_opt()` - Extract optional integer values from Lua globals
   - `has_global()` - Check if a global variable exists

2. **Configuration Application System** (`src/lua_extensions/config_converter.rs`):
   - `apply_lua_config()` - Extracts values from Lua globals and applies them to Niri's Config struct
   - Graceful error handling - unknown or invalid settings are logged and skipped, not treated as errors
   - Type mismatch handling - if a setting has the wrong type, it's simply not applied

3. **Integration with Main Config Loading** (`src/main.rs` lines 159-180):
   - After loading KDL config, the system now loads Lua config
   - If Lua config file exists, it's executed and `apply_lua_config()` is called
   - Lua settings override/extend the KDL defaults
   - Errors in Lua are logged but don't crash Niri

4. **Test Suite** (`src/tests/lua_config.rs`):
   - Tests that Lua config files load successfully
   - Tests that `prefer_no_csd` can be set from Lua
   - Tests that undefined variables don't cause errors
   - Tests that type mismatches are handled gracefully
   - Tests empty configs, comments, and whitespace handling

## What Doesn't Work Yet

⚠️ **Limited configuration scope - only simple boolean/string/number settings are supported.**

Unsupported (would require additional implementation):
- ❌ Input configuration (keyboard, mouse, touchpad settings)
- ❌ Output/display configuration  
- ❌ Layout configuration (gaps, focus ring, borders, etc.)
- ❌ Keybinding registration
- ❌ Window rules
- ❌ Animation settings
- ❌ Startup commands

## How It Works Now

### Configuration Flow

```
Lua Script (.lua file)
    ↓
LuaRuntime::load_file() - Executes Lua code
    ↓
Lua code sets globals: prefer_no_csd = true
    ↓
apply_lua_config(runtime, &mut config) - Extracts and applies globals
    ↓
Config struct updated with Lua values
    ↓
Niri compositor receives modified config
```

### Example Lua Configuration

```lua
-- Set whether to prefer no client-side decorations
prefer_no_csd = true

-- Future: more settings can be added here as they're implemented
```

## How to Extend Configuration Support

To add a new Lua-configurable setting:

1. **Update `apply_lua_config()` in `src/lua_extensions/config_converter.rs`**:
   ```rust
   // Example: adding support for a new setting
   if let Ok(Some(new_setting)) = runtime.get_global_bool_opt("new_setting_name") {
       debug!("Setting new_setting_name = {}", new_setting);
       config.field_name = new_setting;
   }
   ```

2. **Add tests in `src/tests/lua_config.rs`**:
   ```rust
   #[test]
   fn test_apply_new_setting() {
       let lua_code = "new_setting_name = true";
       let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
       let mut config = Config::default();
       apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");
       assert_eq!(config.field_name, true);
   }
   ```

3. **Document the new setting in examples/niri.lua**

## Design Decisions

### Why Global Variables, Not Returned Tables?

The current implementation extracts configuration from Lua global variables rather than requiring scripts to return a configuration table. This approach:

- **Pros**: 
  - Simple and intuitive for users
  - Doesn't require serializing Config struct to Lua format
  - Flexible - users can compute values or conditionally set settings
  - Works with partial configurations (only set what you want)

- **Cons**:
  - Less structured than returning a config table
  - Relies on convention (users must know which globals are supported)

Example usage (current, simple):
```lua
prefer_no_csd = true
```

Alternative (not implemented, more structured):
```lua
local config = {
    prefer_no_csd = true
}
return config
```

## Testing

### Run Lua Configuration Tests

```bash
cargo test --lib lua_config
```

Tests verify:
- Lua files load without errors
- Configuration values are extracted from globals
- Invalid types are handled gracefully
- Empty configs don't cause errors
- Comments and whitespace are handled correctly

### Manual Testing

1. Create `~/.config/niri/niri.lua`:
   ```lua
   prefer_no_csd = false
   ```

2. Start Niri and verify the setting is applied (check logs or observe behavior)

3. Verify KDL config still takes precedence if both exist

## Code Locations

- **Configuration Extraction**: `src/lua_extensions/runtime.rs` lines 76-161
- **Configuration Application**: `src/lua_extensions/config_converter.rs`
- **Integration in Main**: `src/main.rs` lines 159-180
- **Tests**: `src/tests/lua_config.rs`
- **Config Loading**: `niri-config/src/lib.rs` lines 498-525

## Future Work

### High Priority
1. **Expand configuration scope**: Add support for more Config fields
   - Cursor settings (xcursor_size, hide_when_typing)
   - Screenshot path
   - Notification settings
   - Gesture settings

2. **Improve error handling**: Better error messages for configuration issues
   - Distinguish between "setting not supported yet" and "invalid value type"
   - Provide helpful suggestions

3. **Documentation**: Create Lua configuration guide
   - Document all supported settings
   - Provide example configurations
   - Show how to compute settings based on system info

### Medium Priority
1. **Table-based config format**: Allow returning config tables for more structured configs
2. **Per-monitor configuration**: Apply different settings to different monitors
3. **Conditional configuration**: Set different values based on hostname, environment variables, etc.

### Lower Priority  
1. **Runtime configuration**: Call Lua functions from Niri at runtime (not just at startup)
2. **Lua-based keybinding actions**: Define keybinding callbacks in Lua
3. **Hot reload**: Reload Lua config without restarting Niri

## Migration Path

When new Lua config features are implemented:

1. Current KDL users: No change needed, KDL still works and takes precedence
2. New Lua users: Create `~/.config/niri/niri.lua` for simple settings
3. Advanced users: Use KDL for complex configuration, Lua for overrides

## See Also

- Astra project (inspiration for Lua integration)
- KDL syntax: https://kdl.dev
- Niri configuration wiki: https://yalter.github.io/niri/Configuration:-Introduction
- Lua official documentation: https://www.lua.org/manual/
