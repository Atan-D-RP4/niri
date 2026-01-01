# Config API Migration Notes

This document explains the ConfigProxy system and how to add new property accessors.

## Architecture Overview

The config API uses a **PropertyRegistry** pattern to expose niri configuration to Lua:

```
niri-config (Config struct)
    ↓ derive(ConfigProperties)
niri-lua-derive (generates PropertyMetadata)
    ↓
niri-lua/property_registry.rs (PropertyRegistry)
    ↓ update_accessor()
niri-lua/config_accessors.rs (getter/setter functions)
    ↓
niri-lua/config_proxy.rs (ConfigProxy UserData)
    ↓
Lua: niri.config.property_name
```

## Key Files

| File | Purpose |
|------|---------|
| `niri-lua/src/property_registry.rs` | Central registry mapping property paths to accessors |
| `niri-lua/src/config_accessors.rs` | Getter/setter implementations for each property |
| `niri-lua/src/config_proxy.rs` | Lua UserData that exposes properties via `__index`/`__newindex` |
| `niri-lua/src/config_state.rs` | Shared state holding the Config reference |
| `niri-lua-derive/src/config_properties.rs` | Derive macro generating PropertyMetadata |

## Adding a New Accessor

### 1. Ensure the derive macro covers the field

In `niri-config/src/*.rs`, the struct should derive `ConfigProperties`:

```rust
#[derive(ConfigProperties)]
pub struct MySection {
    pub my_field: bool,
}
```

### 2. Register the accessor in `config_accessors.rs`

Add registration in `register_config_accessors()`:

```rust
// Scalar bool example
registry.update_accessor(
    "my_section.my_field",
    Some(Arc::new(|state, _lua| {
        let config = state.config();
        Ok(mlua::Value::Boolean(config.my_section.my_field))
    })),
    Some(Arc::new(|state, _lua, value| {
        let val = bool::from_lua(value, _lua)?;
        state.config_mut().my_section.my_field = val;
        Ok(())
    })),
);
```

### 3. Common Patterns

**Option<T> fields:**
```rust
// Getter
Ok(match &config.section.optional_field {
    Some(v) => v.clone().into_lua(lua)?,
    None => mlua::Value::Nil,
})

// Setter
let val = if value.is_nil() {
    None
} else {
    Some(T::from_lua(value, lua)?)
};
state.config_mut().section.optional_field = val;
```

**Enum fields (string round-trip):**
```rust
// Getter - use Debug format, convert to snake_case
let s = format!("{:?}", config.section.enum_field);
Ok(mlua::Value::String(lua.create_string(&to_snake_case(&s))?))

// Setter - parse from snake_case string
let s = String::from_lua(value, lua)?;
let parsed = match s.as_str() {
    "variant_one" => MyEnum::VariantOne,
    "variant_two" => MyEnum::VariantTwo,
    _ => return Err(mlua::Error::runtime(format!("invalid value: {}", s))),
};
state.config_mut().section.enum_field = parsed;
```

**Nested sections:**
```rust
// Return a new ConfigProxy scoped to the nested path
registry.update_accessor(
    "parent.child",
    Some(Arc::new(|_state, lua| {
        let proxy = ConfigProxy::with_prefix("parent.child");
        Ok(proxy.into_lua(lua)?)
    })),
    None, // Nested sections typically aren't directly settable
);
```

## Testing

Run the niri-lua tests:

```bash
cargo test -p niri-lua
```

Key test files:
- `niri-lua/src/config_proxy.rs` - unit tests for proxy behavior
- `niri-lua/tests/integration_tests.rs` - end-to-end Lua tests

## Constraints

1. **No niri-lua dependency in niri-config** - The derive macro only generates metadata, not runtime code
2. **Single-threaded** - Use `Rc<RefCell<T>>`, not `Arc<Mutex<T>>`
3. **Consistent enum handling** - Always use snake_case strings in Lua, PascalCase in Rust

## Migration from Old API

The old `LuaConfigProxy` was replaced with:
- `PropertyRegistry` - central accessor registry
- `ConfigProxy` - UserData with dynamic property access
- `ConfigState` - shared config reference

Scripts using `niri.config.property` continue to work unchanged.
