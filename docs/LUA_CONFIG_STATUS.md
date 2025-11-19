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

4. **Keybindings can be defined in Lua** ✨ **NEW**:
   - ✅ Define keybindings in `binds` table
   - ✅ Support for 40+ actions (window management, workspace navigation, screenshots, etc.)
   - ✅ Support for `spawn` and `spawn-sh` actions with arguments
   - ✅ Optional `repeat` parameter for keybinding behavior
   - ✅ Graceful handling of unsupported/invalid actions with warnings
   - See [Lua Keybinding Examples](#lua-keybinding-examples) below

## Recent Implementation (Session 2-3)

### Session 2: Configuration Infrastructure

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

### Session 3: Keybinding Support

1. **Keybinding Extraction System** (`src/lua_extensions/runtime.rs`):
   - `get_keybindings()` - Extracts keybindings from Lua `binds` table
   - Parses table of keybinding objects with `key`, `action`, and optional `args` fields
   - Defensive parsing - invalid entries are logged and skipped
   - Supports both actions without args and spawn/spawn-sh with arguments

2. **Keybinding Conversion** (`src/lua_extensions/config_converter.rs`):
   - `LuaKeybinding` struct - Represents a keybinding parsed from Lua
   - `lua_keybinding_to_bind()` - Converts Lua keybinding to Niri Bind struct
   - **40+ supported actions** including:
     - Window management: focus, move, maximize, fullscreen, tabbed display
     - Column operations: center, expand, preset widths/heights
     - Monitor focus and movement
     - Workspace navigation and movement
     - Screenshot variants
     - Utility actions: toggle overview, hotkey overlay, keyboard inhibit
     - System actions: quit, suspend

3. **Integration with Configuration Loading**:
   - Keybindings extracted from Lua are merged into existing config
   - Invalid keybindings are logged with warnings and gracefully skipped
   - Supports optional `repeat` parameter for keybinding behavior

4. **Keybinding Test Suite** (`src/tests/lua_config.rs`):
   - Tests for single keybinding extraction
   - Tests for keybindings with spawn arguments
   - Tests for action-only keybindings
   - Tests for invalid action handling
   - Tests for mixed valid/invalid keybindings
   - Comprehensive edge case testing

## What Doesn't Work Yet

⚠️ **Limited configuration scope - only simple boolean/string/number settings are supported.**

Partially supported (some actions work, others need arguments):
- ⚠️ Keybinding actions requiring arguments: `focus-workspace`, `move-column-to-workspace`, `set-column-width`, `set-window-height` (TODO: add proper argument parsing)

Unsupported (would require additional implementation):
- ❌ Input configuration (keyboard, mouse, touchpad settings)
- ❌ Output/display configuration  
- ❌ Layout configuration (gaps, focus ring, borders, etc.)
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

-- Define keybindings
binds = {
    -- Spawn terminal
    {
        key = "Super+Return",
        action = "spawn",
        args = { "alacritty" }
    },
    -- Window management
    {
        key = "Super+Q",
        action = "close-window",
        args = {}
    },
    {
        key = "Super+F",
        action = "fullscreen-window",
        args = {}
    },
    -- Focus navigation
    {
        key = "Super+J",
        action = "focus-window-down",
        args = {}
    },
    {
        key = "Super+K",
        action = "focus-window-up",
        args = {}
    },
    {
        key = "Super+H",
        action = "focus-column-left",
        args = {}
    },
    {
        key = "Super+L",
        action = "focus-column-right",
        args = {}
    },
    -- Screenshot
    {
        key = "Super+Print",
        action = "screenshot",
        args = {}
    },
}

-- Future: more settings can be added here as they're implemented
```

### Lua Keybinding Examples

The `binds` table in Lua allows you to define keybindings using Lua syntax. Each keybinding is a table with the following fields:

- `key` (required): Key combination (e.g., `"Super+Return"`, `"Ctrl+Alt+Delete"`)
- `action` (required): Action name as a string (e.g., `"spawn"`, `"close-window"`)
- `args` (optional): Array of arguments for the action (required for `spawn` and `spawn-sh`)

#### Supported Actions

**Window Management:**
- `close-window` - Close the focused window
- `fullscreen-window` - Toggle fullscreen for the focused window
- `toggle-windowed-fullscreen` - Toggle windowed fullscreen
- `toggle-window-floating` - Toggle floating mode for the focused window
- `maximize-column` - Maximize the column width
- `center-column` - Center the column
- `center-visible-columns` - Center all visible columns

**Window Focus & Movement:**
- `focus-window-down` / `focus-window-up` - Move focus between windows in a column
- `focus-window-or-workspace-down` / `focus-window-or-workspace-up` - Focus window or switch workspace
- `move-window-down` / `move-window-up` - Move window within column
- `switch-focus-between-floating-and-tiling` - Toggle focus between floating and tiling windows

**Column Focus & Movement:**
- `focus-column-left` / `focus-column-right` - Move focus between columns
- `focus-column-first` / `focus-column-last` - Move focus to first/last column
- `move-column-left` / `move-column-right` - Move column left/right
- `move-column-to-first` / `move-column-to-last` - Move column to start/end
- `consume-or-expel-window-left` / `consume-or-expel-window-right` - Move window to adjacent column
- `consume-window-into-column` - Consume window into same column
- `expel-window-from-column` - Expel window from column
- `toggle-column-tabbed-display` - Toggle tabbed display mode

**Monitor Focus & Movement:**
- `focus-monitor-left` / `focus-monitor-right` / `focus-monitor-down` / `focus-monitor-up` - Focus adjacent monitor
- `move-column-to-monitor-left` / etc. - Move column to adjacent monitor

**Workspace Focus & Movement:**
- `focus-workspace-down` / `focus-workspace-up` - Move workspace down/up
- `move-workspace-down` / `move-workspace-up` - Move workspace down/up
- `move-column-to-workspace-down` / `move-column-to-workspace-up` - Move column to workspace

**Column Operations:**
- `reset-window-height` - Reset window height
- `expand-column-to-available-width` - Expand column to use available width
- `switch-preset-column-width` - Cycle through preset column widths
- `switch-preset-window-height` - Cycle through preset window heights

**Special Actions:**
- `spawn "command"` - Spawn a command (requires args)
- `spawn-sh "shell-command"` - Spawn a shell command (requires args)
- `screenshot` - Take a screenshot
- `screenshot-screen` - Take a screenshot of current screen
- `screenshot-window` - Take a screenshot of the focused window
- `toggle-overview` - Toggle the workspace overview
- `show-hotkey-overlay` - Show the keybinding overlay
- `toggle-keyboard-shortcuts-inhibit` - Toggle keyboard shortcuts inhibit
- `power-off-monitors` - Turn off monitors
- `quit` - Quit Niri
- `suspend` - Suspend the system

#### Examples

**Terminal Launch:**
```lua
{
    key = "Super+Return",
    action = "spawn",
    args = { "alacritty" }
}
```

**Shell Command:**
```lua
{
    key = "Super+E",
    action = "spawn-sh",
    args = { "nautilus" }
}
```

**Window Focus:**
```lua
{
    key = "Super+J",
    action = "focus-window-down",
    args = {}
}
```

**Complex Key Combination:**
```lua
{
    key = "Ctrl+Alt+Delete",
    action = "screenshot",
    args = {}
}
```

#### Optional Parameters

You can optionally add a `repeat` field to control whether a binding repeats when held:

```lua
{
    key = "Super+J",
    action = "focus-window-down",
    args = {},
    repeat = true  -- Default: true
}
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
- **Keybinding Extraction**: `src/lua_extensions/runtime.rs` method `get_keybindings()`
- **Configuration & Keybinding Conversion**: `src/lua_extensions/config_converter.rs`
- **Keybinding Action Mapping**: `src/lua_extensions/config_converter.rs` lines 43-157
- **Integration in Main**: `src/main.rs` lines 159-180
- **Tests**: `src/tests/lua_config.rs`
- **Config Loading**: `niri-config/src/lib.rs` lines 498-525

## Future Work

### High Priority
1. **Expand keybinding support**: Add support for parameterized actions
   - `focus-workspace N` - Focus workspace by number
   - `move-column-to-workspace N` - Move column to workspace
   - `set-column-width N` / `set-window-height N` - Set specific sizes

2. **Expand configuration scope**: Add support for more Config fields
   - Cursor settings (xcursor_size, hide_when_typing)
   - Screenshot path
   - Notification settings
   - Gesture settings

3. **Improve error handling**: Better error messages for configuration issues
   - Distinguish between "setting not supported yet" and "invalid value type"
   - Provide helpful suggestions

### Medium Priority
1. **Table-based config format**: Allow returning config tables for more structured configs
2. **Per-monitor configuration**: Apply different settings to different monitors
3. **Conditional configuration**: Set different values based on hostname, environment variables, etc.

### Lower Priority  
1. **Runtime configuration**: Call Lua functions from Niri at runtime (not just at startup)
2. **Lua-based keybinding callbacks**: Define keybinding actions as Lua functions
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
