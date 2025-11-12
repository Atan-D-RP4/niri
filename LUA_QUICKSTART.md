# Lua Integration Quick Start for Niri WM

## Overview

Niri now has complete Lua scripting support using mlua with LuaJIT. This document provides a quick start guide.

## Installation & Setup

### 1. Build with Lua Support

```bash
cd /home/atan/Develop/repos/niri
cargo build --release
```

The mlua dependency with LuaJIT is vendored, so no external Lua installation is needed.

### 2. Create Your Lua Config

Copy the example configuration:

```bash
mkdir -p ~/.config/niri
cp examples/niri.lua ~/.config/niri/niri.lua
```

Edit `~/.config/niri/niri.lua` with your settings.

## Basic Usage

### Logging

In Lua:
```lua
niri.log("Information message")
niri.debug("Debug message")
niri.warn("Warning message")
niri.error("Error message")
```

Enable debug logging when running:
```bash
RUST_LOG=niri=debug niri
```

### Getting Version Info

```lua
local version = niri.config.version()
print("Running " .. version)
```

### Pretty Print

```lua
pprint({key = "value", number = 42})
-- Output: {"key": "value", "number": 42}
```

## Loading Lua Config in Niri

(Example of how to integrate in Rust code)

```rust
use niri::lua_extensions::LuaConfig;

// In your initialization code:
let config = LuaConfig::from_file("~/.config/niri/niri.lua")?;

// Get values
let animation_duration: i32 = config.get("animation_duration").ok();

// Call functions
config.call_function::<(), ()>("on_startup", ()).ok();
```

## Creating a Custom Component

### 1. Define Your Component

Create `src/lua_extensions/my_feature.rs`:

```rust
use mlua::prelude::*;
use crate::lua_extensions::LuaComponent;

pub struct MyFeature;

impl LuaComponent for MyFeature {
    fn register_to_lua(lua: &Lua) -> LuaResult<()> {
        let globals = lua.globals();

        let my_fn = lua.create_function(|_, arg: String| {
            println!("From Lua: {}", arg);
            Ok(format!("Processed: {}", arg))
        })?;

        globals.set("my_function", my_fn)?;
        Ok(())
    }
}
```

### 2. Register in Module

Update `src/lua_extensions/mod.rs`:

```rust
pub mod my_feature;
pub use my_feature::MyFeature;
```

### 3. Use in Lua

```lua
local result = my_function("test")
print(result)  -- prints "Processed: test"
```

## Testing

### Run Unit Tests

```bash
cargo test lua_extensions
```

### Manual Testing

Create a test script `test.lua`:

```lua
niri.log("Test message")
test_var = 42
return test_var
```

Load it in Rust:

```rust
use niri::lua_extensions::LuaConfig;

let config = LuaConfig::from_string(r#"
    niri.log("Test message")
    test_var = 42
"#)?;

let value: i32 = config.get("test_var")?;
assert_eq!(value, 42);
```

## Documentation

For detailed information, see:

- **LUA_EMBEDDING.md** - Architecture and technical details
- **LUA_GUIDE.md** - Comprehensive developer guide
- **LUA_INTEGRATION_SUMMARY.md** - Implementation overview
- **examples/niri.lua** - Example configuration file

## File Structure

```
src/lua_extensions/
├── mod.rs              # LuaComponent trait
├── runtime.rs          # LuaRuntime struct
├── niri_api.rs         # Niri logging & config
└── config.rs           # Config loading

examples/
└── niri.lua            # Example configuration
```

## Common Tasks

### Run a Script File

```rust
use niri::lua_extensions::LuaRuntime;

let runtime = LuaRuntime::new()?;
let result = runtime.load_file("script.lua")?;
```

### Execute Code String

```rust
let runtime = LuaRuntime::new()?;
runtime.load_string("print('Hello from Lua')")?;
```

### Call a Lua Function

```rust
let runtime = LuaRuntime::new()?;
runtime.register_component::<NiriApi>()?;
runtime.load_string("function add(a, b) return a + b end")?;

let result: i32 = runtime.call_function("add", (10, 20))?;
println!("Result: {}", result); // Output: 30
```

### Return Complex Data

```rust
#[derive(Clone)]
pub struct MyData {
    pub value: i32,
}

impl mlua::UserData for MyData {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get", |_, this, ()| Ok(this.value));
    }
}
```

Use in Lua:
```lua
local data = get_my_data()
local val = data:get()
```

## Troubleshooting

### "attempt to index a nil value"

Make sure the object exists before accessing:

```lua
if config and config.value then
    print(config.value)
else
    print("Config not available")
end
```

### Function not found

Check that the component is registered and the name is correct:

```lua
if niri and type(niri.log) == "function" then
    niri.log("Niri API available")
end
```

### Type mismatch

Convert types explicitly:

```lua
local num = tonumber("42")           -- Convert string to number
local str = tostring(42)              -- Convert number to string
local bool = (value ~= nil and true)  -- Convert to boolean
```

## Performance Tips

1. Cache function references:
```lua
local log = niri.log
log("Message 1")
log("Message 2")
```

2. Use local variables in loops:
```lua
local log = niri.log
for i = 1, 1000 do
    log("i=" .. i)
end
```

3. Minimize table lookups:
```lua
local config = niri.config
local version = config.version()
```

## Next Steps

1. Copy `examples/niri.lua` to `~/.config/niri/niri.lua`
2. Customize it for your needs
3. Create custom components as needed
4. Check the full documentation for advanced features

## References

- [Astra Project](https://github.com/ArkForgeLabs/Astra) - Component architecture inspiration
- [mlua Documentation](https://docs.rs/mlua/) - Rust-Lua bindings
- [LuaJIT](https://luajit.org/) - JIT-compiled Lua
- [Lua Manual](https://www.lua.org/manual/5.1/) - Lua language reference
