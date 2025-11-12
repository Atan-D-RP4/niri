# Niri Lua Extension Guide

This guide explains how to use and extend Lua scripting in Niri WM.

## Quick Start

### 1. Loading a Lua Configuration

In your Niri initialization code:

```rust
use niri::lua_extensions::LuaConfig;

// Load a Lua configuration file
let config = LuaConfig::from_file("~/.config/niri/niri.lua")?;

// Get values from Lua
let animation_duration: i32 = config.get("animation_duration")?;

// Call Lua functions
config.call_function::<(), ()>("on_startup", ())?;
```

### 2. Using the Niri API in Lua

Once loaded, Lua scripts have access to:

```lua
-- Logging
niri.log("Information message")
niri.debug("Debug message")
niri.warn("Warning message")
niri.error("Error message")

-- Configuration helpers
local version = niri.config.version()
print("Running Niri " .. version)

-- Utility functions
pprint({key = "value", number = 42})
```

## Creating Custom Components

### Example: Window Management Component

Create `src/lua_extensions/window_api.rs`:

```rust
use mlua::prelude::*;
use crate::lua_extensions::LuaComponent;

pub struct WindowApi;

#[derive(Clone)]
pub struct LuaWindow {
    id: u32,
    title: String,
}

impl mlua::UserData for LuaWindow {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_id", |_, this, ()| {
            Ok(this.id)
        });

        methods.add_method("get_title", |_, this, ()| {
            Ok(this.title.clone())
        });

        methods.add_method("set_title", |_, this, title: String| {
            // Implementation would interact with actual window
            Ok(())
        });
    }
}

impl LuaComponent for WindowApi {
    fn register_to_lua(lua: &Lua) -> LuaResult<()> {
        let globals = lua.globals();
        let windows = lua.create_table()?;

        let get_active = lua.create_function(|_, ()| {
            // Return active window as LuaWindow
            Ok(LuaWindow {
                id: 1,
                title: "Example Window".to_string(),
            })
        })?;
        windows.set("get_active", get_active)?;

        let get_all = lua.create_function(|lua, ()| {
            // Return all windows as a table
            let result = lua.create_table()?;
            // ... populate with windows
            Ok(result)
        })?;
        windows.set("get_all", get_all)?;

        globals.set("windows", windows)?;
        Ok(())
    }
}
```

Then register in `src/lua_extensions/mod.rs`:

```rust
pub mod window_api;
pub use window_api::WindowApi;
```

### Example: Layout Component

Create `src/lua_extensions/layout_api.rs`:

```rust
use mlua::prelude::*;
use crate::lua_extensions::LuaComponent;

pub struct LayoutApi;

impl LuaComponent for LayoutApi {
    fn register_to_lua(lua: &Lua) -> LuaResult<()> {
        let globals = lua.globals();
        let layout = lua.create_table()?;

        // Current layout getter
        let get_current = lua.create_function(|_, ()| {
            Ok("dwindle") // Would query actual layout
        })?;
        layout.set("get_current", get_current)?;

        // Set layout
        let set_layout = lua.create_function(|_, name: String| {
            println!("Setting layout to: {}", name);
            Ok(())
        })?;
        layout.set("set", set_layout)?;

        // Available layouts
        let list = lua.create_function(|lua, ()| {
            let layouts = lua.create_table()?;
            layouts.raw_set(1, "dwindle")?;
            layouts.raw_set(2, "paper")?;
            layouts.raw_set(3, "grid")?;
            Ok(layouts)
        })?;
        layout.set("list", list)?;

        globals.set("layout", layout)?;
        Ok(())
    }
}
```

### Example: Input/Keybind Component

Create `src/lua_extensions/keybind_api.rs`:

```rust
use mlua::prelude::*;
use crate::lua_extensions::LuaComponent;

pub struct KeybindApi;

impl LuaComponent for KeybindApi {
    fn register_to_lua(lua: &Lua) -> LuaResult<()> {
        let globals = lua.globals();
        let keybinds = lua.create_table()?;

        // Register a custom keybind
        let register = lua.create_function(|_, (key, modifiers, callback_name): (String, String, String)| {
            println!("Registering: {} + {} -> {}", modifiers, key, callback_name);
            Ok(())
        })?;
        keybinds.set("register", register)?;

        globals.set("keybinds", keybinds)?;
        Ok(())
    }
}
```

## Advanced Usage

### Async Operations

```rust
// Create async function in component
let async_fn = lua.create_async_function(|_, param: String| async move {
    // Perform async work
    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(format!("Processed: {}", param))
})?;
```

### Returning Complex Types

```rust
#[derive(Clone)]
pub struct Config {
    pub timeout: i32,
    pub enabled: bool,
}

impl mlua::UserData for Config {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_timeout", |_, this, ()| {
            Ok(this.timeout)
        });

        methods.add_field_getter("enabled", |_, this| {
            Ok(this.enabled)
        });
    }
}
```

### Error Handling

```rust
let lua_function = lua.create_function(|_, arg: i32| {
    if arg < 0 {
        return Err(LuaError::RuntimeError("Argument must be positive".to_string()));
    }
    Ok(arg * 2)
})?;
```

## Integration with Niri State

To integrate Lua with Niri's State structure:

```rust
// In src/niri.rs
pub struct State {
    // ... existing fields
    pub lua_runtime: Option<LuaRuntime>,
}

impl State {
    pub fn new(...) -> Result<Self> {
        // ... existing initialization
        
        let lua_runtime = LuaRuntime::new().ok();
        if let Some(ref runtime) = lua_runtime {
            NiriApi::register_to_lua(runtime.inner())?;
            // Register other components
            WindowApi::register_to_lua(runtime.inner())?;
            LayoutApi::register_to_lua(runtime.inner())?;
            KeybindApi::register_to_lua(runtime.inner())?;
        }

        Ok(Self {
            lua_runtime,
            // ... other fields
        })
    }
}
```

## Testing Your Components

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_component() {
        let lua = Lua::new();
        lua.load_from_std_lib(LuaStdLib::ALL).unwrap();
        MyComponent::register_to_lua(&lua).unwrap();

        let result = lua.load(r#"
            return my_function("test")
        "#).eval().unwrap();

        assert_eq!(result.to_string().unwrap(), "test");
    }
}
```

## Best Practices

1. **Error Messages**: Provide clear, actionable error messages
2. **Documentation**: Document all Lua functions with examples
3. **Type Safety**: Use strong typing in function signatures
4. **Performance**: Avoid blocking operations in Lua callbacks
5. **Logging**: Use niri.log/debug for troubleshooting
6. **Testing**: Include unit tests for all components
7. **Naming**: Use clear, consistent naming conventions

## Debugging Lua Scripts

### Enable Lua Logging

```bash
RUST_LOG=niri=debug niri
```

Then in Lua:
```lua
niri.debug("Variable value: " .. tostring(my_var))
```

### Common Issues

**Issue**: "attempt to index a nil value"
```lua
-- Make sure to check for nil
if config and config.value then
    print(config.value)
end
```

**Issue**: Type mismatches
```lua
-- Convert to correct type
local num = tonumber(string_value)
local str = tostring(number_value)
```

**Issue**: Function not found
```lua
-- Make sure component is registered
if niri and niri.log then
    niri.log("Niri API is available")
end
```

## Performance Tips

1. Cache frequently accessed values:
```lua
local log = niri.log  -- Cache function reference
log("Message 1")
log("Message 2")
```

2. Use local variables for loops:
```lua
local log = niri.log
for i = 1, 1000 do
    log("Iteration " .. i)
end
```

3. Avoid repeated table lookups:
```lua
local config = niri.config
local version = config.version()
```

## See Also

- [LUA_EMBEDDING.md](./LUA_EMBEDDING.md) - Architecture overview
- [examples/niri.lua](./examples/niri.lua) - Example configuration
- [mlua Documentation](https://docs.rs/mlua/)
- [Lua Reference Manual](https://www.lua.org/manual/5.1/)
