# Specification: Config API Simplification

## Status: Approved
## Author: AI Assistant
## Date: 2024-12
## RFC: config-api-simplification-rfc.md

---


## 1. Overview

This specification details the implementation of the config API simplification as approved in the RFC. The goal is to replace 40+ generated proxy structs with a unified dynamic property system based on a `PropertyRegistry` and a single `ConfigProxy` UserData.

### Key Decisions

| Decision | Choice |
|----------|--------|
| Enum handling | Strings with validation (case-sensitive, clear error messages) |
| Nested struct access | ConfigProxy + `__pairs` + `:snapshot()` |
| Array validation | Validate each element on assignment |
| Macro approach | Derive macro (single source of truth) |
| Global registry | Hybrid: `OnceLock` for registry + `app_data` for state |

### Key Changes

- **Single ConfigProxy**: Replace all generated proxy structs with one `ConfigProxy` struct that handles all config access via `__index`/`__newixdex`.
- **PropertyRegistry**: Central registry mapping config paths to property descriptors with getter/setter functions.
- **New derive macro**: `#[derive(ConfigProperties)]` generates property registration code (simpler than current `LuaConfigProxy`).
- **Auto DirtyFlag inference**: Infer `DirtyFlag` from path prefix (`cursor.*` → `DirtyFlag::Cursor`).
- **Signal emission**: Emit `config::<path>` events on property changes.
- **Arc→Rc migration**: Convert `Arc<Mutex>` to `Rc<RefCell>` for all non-cross-thread components.

### Scope

This specification covers:

- Data structures and APIs
- Derive macro specification
- DirtyFlag inference rules
- Signal emission API
- Arc→Rc migration steps
- Test plan and acceptance criteria

---


## 2. Data Structures

### 2.1 PropertyRegistry

Central registry for all config properties. Thread-local (main thread only).

```rust
use std::collections::BTreeMap;
use std::sync::OnceLock;

use mlua::Value;
use crate::config_state::DirtyFlag;

pub struct PropertyRegistry {
    properties: BTreeMap<String, PropertyDescriptor>,
}

// Global registry using OnceLock (safe, immutable after init)
static REGISTRY: OnceLock<PropertyRegistry> = OnceLock::new();

impl PropertyRegistry {
    pub fn new() -> Self {
        Self {
            properties: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, path: &str, descriptor: PropertyDescriptor) {
        self.properties.insert(path.to_string(), descriptor);
    }
    
    pub fn get(&self, path: &str) -> Option<&PropertyDescriptor> {
        self.properties.get(path)
    }
    
    pub fn contains(&self, path: &str) -> bool {
        self.properties.contains_key(path)
    }
    
    /// Returns iterator over child property keys for a given prefix
    pub fn children(&self, prefix: &str) -> impl Iterator<Item = &str> {
        let prefix_dot = format!("{}.", prefix);
        self.properties.keys()
            .filter(move |k| k.starts_with(&prefix_dot) && !k[prefix_dot.len()..].contains('.'))
            .map(|k| &k[prefix_dot.len()..])
    }
    
    /// Initialize global registry (call once at startup)
    pub fn init() -> &'static PropertyRegistry {
        REGISTRY.get_or_init(|| {
            let mut registry = PropertyRegistry::new();
            // Register all config properties
            Config::register(&mut registry);
            registry
        })
    }
    
    /// Get global registry (panics if not initialized)
    pub fn global() -> &'static PropertyRegistry {
        REGISTRY.get().expect("PropertyRegistry not initialized")
    }
}
```

### 2.2 PropertyDescriptor

Describes a single config property with getter/setter functions. Getters and setters receive `Lua` context for creating values.

```rust
use mlua::prelude::*;

pub enum PropertyType {
    Bool,
    Integer,  // i8-i64, u8-u64
    Number,   // f32, f64
    String,
    Enum {
        name: &'static str,
        variants: &'static [&'static str],
    },
    Array(Box<PropertyType>),
    Nested,  // struct that can be indexed into
}

pub struct PropertyDescriptor {
    pub path: &'static str,
    pub ty: PropertyType,
    pub dirty_flag: DirtyFlag,
    pub getter: fn(&Lua, &Config) -> LuaResult<Value>,
    pub setter: fn(&Lua, &mut Config, Value) -> LuaResult<()>,
    pub signal: bool,
}

impl PropertyDescriptor {
    pub const fn new(
        path: &'static str,
        ty: PropertyType,
        dirty_flag: DirtyFlag,
        getter: fn(&Lua, &Config) -> LuaResult<Value>,
        setter: fn(&Lua, &mut Config, Value) -> LuaResult<()>,
    ) -> Self {
        Self {
            path,
            ty,
            dirty_flag,
            getter,
            setter,
            signal: true,
        }
    }
    
    pub const fn no_signal(mut self) -> Self {
        self.signal = false;
        self
    }
}

// Helper for enum validation with clear error messages
pub fn parse_enum_variant<T: std::str::FromStr>(
    value: &str,
    variants: &[&str],
    enum_name: &str,
) -> LuaResult<T> {
    value.parse().map_err(|_| {
        LuaError::external(format!(
            "invalid value \"{}\", expected one of: {}",
            value,
            variants.join(", ")
        ))
    })
}

// Helper for array element validation
pub fn validate_array_elements<T, F>(
    lua: &Lua,
    table: &LuaTable,
    convert: F,
) -> LuaResult<Vec<T>>
where
    F: Fn(&Lua, Value) -> LuaResult<T>,
{
    let mut vec = Vec::new();
    for pair in table.clone().pairs::<i64, Value>() {
        let (idx, value) = pair?;
        let elem = convert(lua, value).map_err(|e| {
            LuaError::external(format!("invalid element at index {}: {}", idx, e))
        })?;
        vec.push(elem);
    }
    Ok(vec)
}
```

### 2.3 ConfigProxy

Single UserData that handles all config access via `__index`/`__newindex`. Supports iteration via `__pairs` and explicit `:snapshot()` for copying.

```rust
use std::rc::Rc;
use std::cell::RefCell;
use mlua::prelude::*;

pub struct ConfigProxy {
    pub current_path: String,  // current path ("" for root, "cursor", "layout.struts")
}

impl UserData for ConfigProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__index", |lua, this, key: String| {
            let path = if this.current_path.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", this.current_path, key)
            };
            
            let registry = PropertyRegistry::global();
            
            // Check if this is a nested struct (has children)
            if registry.children(&path).next().is_some() {
                // Return new ConfigProxy for chained access
                return Ok(Value::UserData(lua.create_userdata(ConfigProxy {
                    current_path: path,
                })?));
            }
            
            // Check for direct property access
            match registry.get(&path) {
                Some(desc) => {
                    let state = lua.app_data_ref::<Rc<RefCell<ConfigState>>>()
                        .ok_or_else(|| LuaError::external("config state not initialized"))?;
                    let config = state.borrow();
                    (desc.getter)(lua, &config.config)
                }
                None => Err(LuaError::external(format!("unknown config property: {}", path)))
            }
        });
        
        methods.add_meta_method("__newindex", |lua, this, (key, value): (String, Value)| {
            let path = if this.current_path.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", this.current_path, key)
            };
            
            let registry = PropertyRegistry::global();
            
            let desc = registry.get(&path)
                .ok_or_else(|| LuaError::external(format!("unknown config property: {}", path)))?;
            
            // Get mutable config state
            let state = lua.app_data_ref::<Rc<RefCell<ConfigState>>>()
                .ok_or_else(|| LuaError::external("config state not initialized"))?;
            
            // Call setter (validates internally)
            {
                let mut state = state.borrow_mut();
                (desc.setter)(lua, &mut state.config, value.clone())?;
                state.mark_dirty(desc.dirty_flag);
            }
            
            // Emit signal if enabled
            if desc.signal {
                if let Some(events) = lua.app_data_ref::<Rc<RefCell<EventSystem>>>() {
                    events.borrow().emit(lua, &format!("config::{}", path), |lua| {
                        let payload = lua.create_table()?;
                        payload.set("path", path.as_str())?;
                        payload.set("value", value.clone())?;
                        Ok(Value::Table(payload))
                    })?;
                }
            }
            
            Ok(())
        });
        
        methods.add_meta_method("__pairs", |lua, this, ()| {
            let registry = PropertyRegistry::global();
            let keys: Vec<String> = registry.children(&this.current_path)
                .map(|s| s.to_string())
                .collect();
            
            let current_path = this.current_path.clone();
            let idx = Rc::new(RefCell::new(0usize));
            
            let next = lua.create_function(move |lua, _: ()| {
                let mut i = idx.borrow_mut();
                if *i >= keys.len() {
                    return Ok((Value::Nil, Value::Nil));
                }
                let key = &keys[*i];
                *i += 1;
                
                // Get value via registry
                let path = if current_path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", current_path, key)
                };
                
                let registry = PropertyRegistry::global();
                let value = if let Some(desc) = registry.get(&path) {
                    let state = lua.app_data_ref::<Rc<RefCell<ConfigState>>>().unwrap();
                    (desc.getter)(lua, &state.borrow().config)?
                } else {
                    // Nested struct - return proxy
                    Value::UserData(lua.create_userdata(ConfigProxy {
                        current_path: path,
                    })?)
                };
                
                Ok((Value::String(lua.create_string(key)?), value))
            })?;
            
            Ok((next, Value::Nil, Value::Nil))
        });
        
        methods.add_method("snapshot", |lua, this, ()| {
            let table = lua.create_table()?;
            let registry = PropertyRegistry::global();
            let state = lua.app_data_ref::<Rc<RefCell<ConfigState>>>()
                .ok_or_else(|| LuaError::external("config state not initialized"))?;
            let config = &state.borrow().config;
            
            for key in registry.children(&this.current_path) {
                let path = if this.current_path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", this.current_path, key)
                };
                
                if let Some(desc) = registry.get(&path) {
                    let value = (desc.getter)(lua, config)?;
                    table.set(key, value)?;
                }
                // Skip nested structs in snapshot (only direct values)
            }
            
            Ok(table)
        });
        
        methods.add_meta_method("__tostring", |_, this, ()| {
            if this.current_path.is_empty() {
                Ok("ConfigProxy(root)".to_string())
            } else {
                Ok(format!("ConfigProxy({})", this.current_path))
            }
        });
    }
}
```

---


## 3. Derive Macro Specification

### 3.1 Macro: `#[derive(ConfigProperties)]`

Generates `ConfigProperties` trait implementation that registers all fields with the `PropertyRegistry`.

**Supported Field Types:**
- Primitives: `bool`, `u8`-`u64`, `i8`-`i64`, `f32`, `f64`, `String`
- Enums: Any enum with `#[derive(ConfigProperties)]` or known variants
- Options: `Option<T>` where T is a supported type
- Arrays: `Vec<T>` where T is a supported type
- Nested structs: Any struct with `#[derive(ConfigProperties)]`

**Attributes:**
- `#[config(prefix = "path")]` - Required. The config path prefix for this struct.
- `#[config(dirty = "Flag")]` - Optional. Override inferred DirtyFlag.
- `#[config(no_signal)]` - Optional. Disable signal emission for struct or field.
- `#[config(skip)]` - Optional. Skip field from Lua exposure.

```rust
// Example usage
#[derive(ConfigProperties)]
#[config(prefix = "cursor")]
pub struct CursorConfig {
    pub xcursor_size: u8,
    pub xcursor_theme: String,
    #[config(no_signal)]
    pub hide_after_inactive_ms: Option<u32>,
    #[config(skip)]
    internal_cache: Option<CursorCache>,
}

#[derive(ConfigProperties)]
#[config(prefix = "layout")]
pub struct LayoutConfig {
    pub gaps: f64,
    pub center_focused_column: CenterFocusedColumn,  // enum
    pub struts: Struts,  // nested struct
    pub preset_column_widths: Vec<ColumnWidth>,  // array
}
```

### 3.2 Generated Code Example

For `CursorConfig`, the macro generates:

```rust
impl ConfigProperties for CursorConfig {
    fn register(registry: &mut PropertyRegistry) {
        // Register nested path marker
        registry.add("cursor", PropertyDescriptor::new(
            "cursor",
            PropertyType::Nested,
            DirtyFlag::Cursor,
            |_, _| Ok(Value::Nil),
            |_, _, _| Err(LuaError::external("cannot assign to nested config")),
        ));
        
        // xcursor_size: u8
        registry.add("cursor.xcursor_size", PropertyDescriptor::new(
            "cursor.xcursor_size",
            PropertyType::Integer,
            DirtyFlag::Cursor,
            |lua, config| {
                Ok(Value::Integer(config.cursor.xcursor_size as i64))
            },
            |lua, config, value| {
                let v = value.as_integer()
                    .ok_or_else(|| LuaError::external("expected integer"))?;
                if v < 0 || v > u8::MAX as i64 {
                    return Err(LuaError::external(format!(
                        "value {} out of range for u8 (0-255)", v
                    )));
                }
                config.cursor.xcursor_size = v as u8;
                Ok(())
            },
        ));
        
        // xcursor_theme: String
        registry.add("cursor.xcursor_theme", PropertyDescriptor::new(
            "cursor.xcursor_theme",
            PropertyType::String,
            DirtyFlag::Cursor,
            |lua, config| {
                Ok(Value::String(lua.create_string(&config.cursor.xcursor_theme)?))
            },
            |lua, config, value| {
                let s = value.as_str()
                    .ok_or_else(|| LuaError::external("expected string"))?;
                config.cursor.xcursor_theme = s.to_string();
                Ok(())
            },
        ));
        
        // hide_after_inactive_ms: Option<u32> (no_signal)
        registry.add("cursor.hide_after_inactive_ms", PropertyDescriptor::new(
            "cursor.hide_after_inactive_ms",
            PropertyType::Integer,
            DirtyFlag::Cursor,
            |lua, config| {
                match config.cursor.hide_after_inactive_ms {
                    Some(v) => Ok(Value::Integer(v as i64)),
                    None => Ok(Value::Nil),
                }
            },
            |lua, config, value| {
                if value.is_nil() {
                    config.cursor.hide_after_inactive_ms = None;
                } else {
                    let v = value.as_integer()
                        .ok_or_else(|| LuaError::external("expected integer or nil"))?;
                    if v < 0 || v > u32::MAX as i64 {
                        return Err(LuaError::external(format!(
                            "value {} out of range for u32", v
                        )));
                    }
                    config.cursor.hide_after_inactive_ms = Some(v as u32);
                }
                Ok(())
            },
        ).no_signal());
    }
}
```

### 3.3 Enum Handling

Enums are exposed as strings. The macro generates variant lists and parsing functions.

```rust
#[derive(ConfigProperties)]
#[config(prefix = "layout.center_focused_column")]
pub enum CenterFocusedColumn {
    Never,
    Always,
    OnOverflow,
}

// Generated:
impl ConfigProperties for CenterFocusedColumn {
    const VARIANTS: &'static [&'static str] = &["Never", "Always", "OnOverflow"];
    
    fn to_lua_string(&self) -> &'static str {
        match self {
            Self::Never => "Never",
            Self::Always => "Always",
            Self::OnOverflow => "OnOverflow",
        }
    }
    
    fn from_lua_string(s: &str) -> LuaResult<Self> {
        match s {
            "Never" => Ok(Self::Never),
            "Always" => Ok(Self::Always),
            "OnOverflow" => Ok(Self::OnOverflow),
            _ => Err(LuaError::external(format!(
                "invalid value \"{}\", expected one of: Never, Always, OnOverflow",
                s
            ))),
        }
    }
    
    fn register(registry: &mut PropertyRegistry) {
        // Enum fields are registered by the parent struct
    }
}

// Used in parent struct's generated code:
registry.add("layout.center_focused_column", PropertyDescriptor::new(
    "layout.center_focused_column",
    PropertyType::Enum {
        name: "CenterFocusedColumn",
        variants: CenterFocusedColumn::VARIANTS,
    },
    DirtyFlag::Layout,
    |lua, config| {
        Ok(Value::String(lua.create_string(
            config.layout.center_focused_column.to_lua_string()
        )?))
    },
    |lua, config, value| {
        let s = value.as_str()
            .ok_or_else(|| LuaError::external("expected string"))?;
        config.layout.center_focused_column = CenterFocusedColumn::from_lua_string(s)?;
        Ok(())
    },
));
```

### 3.4 Array Handling

Arrays are validated element-by-element. Each element type must implement conversion.

```rust
// For Vec<ColumnWidth> where ColumnWidth is an enum with table variants:
registry.add("layout.preset_column_widths", PropertyDescriptor::new(
    "layout.preset_column_widths",
    PropertyType::Array(Box::new(PropertyType::Nested)),
    DirtyFlag::Layout,
    |lua, config| {
        let table = lua.create_table()?;
        for (i, width) in config.layout.preset_column_widths.iter().enumerate() {
            table.set(i + 1, width.to_lua_table(lua)?)?;
        }
        Ok(Value::Table(table))
    },
    |lua, config, value| {
        let table = value.as_table()
            .ok_or_else(|| LuaError::external("expected table"))?;
        config.layout.preset_column_widths = validate_array_elements(
            lua,
            table,
            |lua, v| ColumnWidth::from_lua_table(lua, v),
        )?;
        Ok(())
    },
));
```
```

### 3.2 Trait: `ConfigProperties`

Trait that all config structs must implement to register their properties.

```rust
pub trait ConfigProperties {
    fn register(registry: &mut PropertyRegistry);
}

// Example usage:
// #[derive(ConfigProperties)]
// #[config(prefix = "cursor")]
// pub struct CursorConfig {
//     pub xcursor_size: u8,
//     pub xcursor_theme: String,
// }
```

---


## 4. DirtyFlag Inference Rules

### 4.1 Inference Function

```rust
pub fn infer_dirty_flag(path: &str) -> DirtyFlag {
    let prefix = path.split('.').next().unwrap();
    match prefix {
        "cursor" => DirtyFlag::Cursor,
        "layout" => DirtyFlag::Layout,
        "animations" => DirtyFlag::Animations,
        "input" => DirtyFlag::Input,
        "gestures" => DirtyFlag::Gestures,
        "overview" => DirtyFlag::Overview,
        "recent_windows" => DirtyFlag::RecentWindows,
        "clipboard" => DirtyFlag::Clipboard,
        "hotkey_overlay" => DirtyFlag::HotkeyOverlay,
        "config_notification" => DirtyFlag::ConfigNotification,
        "debug" => DirtyFlag::Debug,
        "xwayland_satellite" => DirtyFlag::XwaylandSatellite,
        "window_rules" => DirtyFlag::WindowRules,
        "layer_rules" => DirtyFlag::LayerRules,
        "binds" => DirtyFlag::Binds,
        "switch_events" => DirtyFlag::SwitchEvents,
        "workspaces" => DirtyFlag::Workspaces,
        "environment" => DirtyFlag::Environment,
        "spawn_at_startup" => DirtyFlag::SpawnAtStartup,
        "misc" => DirtyFlag::Misc,
        _ => DirtyFlag::Misc,
    }
}
```

### 4.2 Override Mechanism

The `#[config(dirty = "Flag")]` attribute can override the inferred flag for a struct.

```rust
#[derive(ConfigProperties)]
#[config(prefix = "layout", dirty = "Layout")]
pub struct LayoutConfig {
    // ... fields
}
```

---


## 5. Signal Emission API

### 5.1 Event Format

On any config property change, emit a `config::<path>` event with the path and new value.

```lua
-- Example
niri.events:on("config::cursor.xcursor_size", function(path, value)
    print("Cursor size changed to", value)
end)
```

### 5.2 Emission Logic

Emission occurs in the `__newindex` method of `ConfigProxy`:

1. Property change validated and applied
2. Dirty flag marked
3. If `descriptor.signal` is true, emit `config::<path>` event via `event_system.emit`
4. Event payload: `(path: string, new_value: any)`

### 5.3 Disabling Signals

Signals can be disabled per-property using `#[config(no_signal)]`:

```rust
#[derive(ConfigProperties)]
#[config(prefix = "cursor")]
pub struct CursorConfig {
    #[config(no_signal)]
    pub hide_after_inactive_ms: Option<u32>,
    // ... other fields
}
```

---


## 6. Arc→Rc Migration Steps

### 6.1 Components to Convert

| Component | Current | New | File |
|-----------|---------|-----|------|
| ConfigState | `Arc<Mutex<Config>>` + `Arc<Mutex<ConfigDirtyFlags>>` | `Rc<RefCell<ConfigState>>` | `config_state.rs` |
| SharedEventHandlers | `Arc<Mutex<EventHandlers>>` | `Rc<RefCell<EventHandlers>>` | `events_proxy.rs` |
| StateHandle fields | `Arc<Mutex<T>>` for outputs, cursor_position, etc. | `Rc<RefCell<T>>` | `state_handle.rs` |
| IpcLuaExecutor runtime | `Arc<Mutex<Lua>>` | `Rc<RefCell<Lua>>` | `ipc_repl.rs` |
| ConfigWrapper | `Arc<Mutex<ConfigWrapper>>` | `Rc<RefCell<ConfigWrapper>>` | `config_wrapper.rs` |

### 6.2 Migration Strategy

1. **Create new wrapper types** with `Rc<RefCell<T>>`
2. **Update all access patterns** from `lock().unwrap()` to `borrow()`/`borrow_mut()`
3. **Update all creation patterns** from `Arc::new(Mutex::new(...))` to `Rc::new(RefCell::new(...))`
4. **Remove Send/Sync bounds** where no longer needed
5. **Update tests** to use new patterns

### 6.3 Exception: ProcessManager

`ProcessManager` must remain `Arc<Mutex>` because it's accessed from worker threads spawned by `std::thread::spawn` for stdout/stderr monitoring.

```rust
// process.rs - must keep Arc<Mutex>
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::sync::mpsc::{self, Receiver, Sender};

pub struct ProcessManager {
    event_queue: Arc<Mutex<VecDeque<ProcessEvent>>>,
    // ... other fields
}
```

---


## 7. Test Plan

### 7.1 Unit Tests

- **PropertyRegistry**: Test add/get/contains with various paths
- **PropertyDescriptor**: Test type validation with `is_compatible_type`
- **ConfigProxy**: Test `__index`/`__newindex` with valid/invalid paths and types
- **DirtyFlag inference**: Test `infer_dirty_flag` with all prefixes
- **Arc→Rc**: Test all converted components with borrow/borrow_mut patterns

### 7.2 Integration Tests

- **Config access**: Test `niri.config.cursor.xcursor_size = 24` works and emits event
- **Nested access**: Test `niri.config.layout.gaps = 10` works
- **Signal emission**: Test `niri.events:on("config::cursor.xcursor_size", ...)` receives events
- **Error cases**: Test invalid type assignment, unknown property access
- **Performance**: Benchmark config access before/after migration

### 7.3 Acceptance Criteria

1. **Functionality**: All existing config access patterns work identically
2. **API**: No breaking changes to `niri.config.*` Lua API
3. **Performance**: Config access performance within 10% of current
4. **Signals**: `config::<path>` events emitted on all property changes
5. **Dirty flags**: Correct `DirtyFlag` marked for all property changes
6. **Memory**: No memory leaks in `Rc<RefCell>` usage
7. **Tests**: All unit and integration tests pass

---


## 8. Migration Plan

### 8.1 Phase 1: Infrastructure

1. Create `PropertyRegistry` and `PropertyDescriptor` types
2. Implement `ConfigProxy` UserData
3. Add `infer_dirty_flag` function
4. Write unit tests for new components

### 8.2 Phase 2: Derive Macro

1. Implement `#[derive(ConfigProperties)]` macro
2. Add `ConfigProperties` trait
3. Test macro with simple structs

### 8.3 Phase 3: Registration

1. Add `#[derive(ConfigProperties)]` to all config structs
2. Register all properties at startup
3. Verify registry contains all expected paths

### 8.4 Phase 4: Switch

1. Replace `niri.config` Lua registration to use `ConfigProxy`
2. Remove old `LuaConfigProxy` derive macro and generated code
3. Remove `config_api.rs` read-only table builder (if redundant)

### 8.5 Phase 5: Arc→Rc

1. Migrate `ConfigState`, `SharedEventHandlers`, etc. to `Rc<RefCell>`
2. Update all access patterns
3. Verify no deadlocks or panics

### 8.6 Phase 6: Cleanup

1. Remove old proxy structs and derive macro
2. Update documentation
3. Final integration tests

---


## 9. Design Decisions (Finalized)

### 9.1 Enum Handling: Strings with Validation

Enums are exposed as case-sensitive strings matching variant names. Validation provides clear error messages listing valid variants.

```lua
-- Setting an enum
niri.config.layout.center_focused_column = "Always"  -- valid
niri.config.layout.center_focused_column = "alwyas"  -- ERROR: invalid value "alwyas", expected one of: Never, Always, OnOverflow

-- Getting returns string
local mode = niri.config.layout.center_focused_column  -- "Always"
```

**Implementation**: The derive macro generates variant lists from enum definitions. Setter parses string to enum via `FromStr` or match. Getter formats enum via `Debug` or explicit variant name.

```rust
// Generated for enum CenterFocusedColumn { Never, Always, OnOverflow }
fn enum_variants() -> &'static [&'static str] {
    &["Never", "Always", "OnOverflow"]
}

fn parse_enum(s: &str) -> Result<CenterFocusedColumn, String> {
    match s {
        "Never" => Ok(CenterFocusedColumn::Never),
        "Always" => Ok(CenterFocusedColumn::Always),
        "OnOverflow" => Ok(CenterFocusedColumn::OnOverflow),
        _ => Err(format!("invalid value \"{}\", expected one of: Never, Always, OnOverflow", s)),
    }
}
```

### 9.2 Nested Struct Access: ConfigProxy + `__pairs` + `:snapshot()`

Nested struct access returns a new `ConfigProxy` for chained assignment. Iteration via `__pairs` is supported. Explicit `:snapshot()` method returns a read-only table copy.

```lua
-- Chained assignment (primary use case)
niri.config.layout.gaps = 10  -- works

-- Store reference for multiple assignments
local layout = niri.config.layout  -- returns ConfigProxy("layout")
layout.gaps = 10
layout.always_center_single_column = true

-- Iteration via __pairs
for key, value in pairs(niri.config.layout) do
    print(key, value)
end

-- Explicit snapshot for copying
local snapshot = niri.config.layout:snapshot()  -- returns plain table {gaps=10, ...}
```

**Implementation**: Add `__pairs` metamethod and `:snapshot()` method to `ConfigProxy`:

```rust
impl UserData for ConfigProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // ... existing __index/__newindex ...
        
        methods.add_meta_method("__pairs", |lua, this, ()| {
            // Return iterator over child properties
            let registry = REGISTRY.get().unwrap();
            let prefix = format!("{}.", this.current_path);
            let keys: Vec<String> = registry.properties.keys()
                .filter(|k| k.starts_with(&prefix) && !k[prefix.len()..].contains('.'))
                .map(|k| k[prefix.len()..].to_string())
                .collect();
            
            let iter = lua.create_function(move |lua, (keys, idx): (Vec<String>, usize)| {
                if idx >= keys.len() {
                    return Ok((Value::Nil, Value::Nil));
                }
                let key = &keys[idx];
                let value = this.get(lua, key.clone())?;
                Ok((Value::String(lua.create_string(key)?), value))
            })?;
            
            Ok((iter, keys, 0))
        });
        
        methods.add_method("snapshot", |lua, this, ()| {
            let table = lua.create_table()?;
            let registry = REGISTRY.get().unwrap();
            let prefix = format!("{}.", this.current_path);
            
            for (path, desc) in registry.properties.iter() {
                if path.starts_with(&prefix) {
                    let key = &path[prefix.len()..];
                    if !key.contains('.') {  // Direct children only
                        let state = lua.app_data_ref::<Rc<RefCell<ConfigState>>>().unwrap();
                        let config = state.borrow().config.borrow();
                        let value = (desc.getter)(&config)?;
                        table.set(key, value)?;
                    }
                }
            }
            
            Ok(table)
        });
    }
}
```

### 9.3 Array Validation: Validate Each Element

Arrays are validated element-by-element on assignment. This aligns with the validate-on-assignment principle and provides immediate feedback.

```lua
-- Valid assignment
niri.config.layout.preset_column_widths = {
    {proportion = 0.5},
    {fixed = 800},
}

-- Invalid - error at assignment time
niri.config.layout.preset_column_widths = {
    {proportion = 0.5},
    "invalid",  -- ERROR: invalid element at index 2: expected table, got string
}
```

**Implementation**: Array setter iterates elements and validates each:

```rust
fn set_array<T: FromLuaValue>(config: &mut Config, path: &str, table: LuaTable) -> LuaResult<()> {
    let mut vec = Vec::new();
    for pair in table.pairs::<i64, Value>() {
        let (idx, value) = pair?;
        let elem = T::from_lua_value(value).map_err(|e| {
            LuaError::external(format!("invalid element at index {}: {}", idx, e))
        })?;
        vec.push(elem);
    }
    // Apply to config...
    Ok(())
}
```

### 9.4 Derive Macro: Keep (Single Source of Truth)

The derive macro is retained despite complexity because:
- Single source of truth: struct definition IS the Lua API
- Matches Rust ecosystem patterns (serde, clap)
- Refinement of existing pattern, not new complexity
- IDE support and type safety

### 9.5 Global Registry: Hybrid (`OnceLock` + `app_data`)

The PropertyRegistry (schema) uses `OnceLock` for safe static access. The ConfigState (values) uses `Lua::app_data()` for per-instance testability.

```rust
use std::sync::OnceLock;

// Registry is static - properties don't change at runtime
static REGISTRY: OnceLock<PropertyRegistry> = OnceLock::new();

impl PropertyRegistry {
    pub fn global() -> &'static PropertyRegistry {
        REGISTRY.get_or_init(|| {
            let mut registry = PropertyRegistry::new();
            // Register all properties
            Config::register(&mut registry);
            registry
        })
    }
}

// ConfigState is per-Lua-instance (in app_data)
// At Lua initialization:
lua.set_app_data(Rc::new(RefCell::new(ConfigState::new(config))));

// ConfigProxy accesses both
impl ConfigProxy {
    fn get(&self, lua: &Lua, key: String) -> LuaResult<Value> {
        let registry = PropertyRegistry::global();  // Static, no lock
        let state = lua.app_data_ref::<Rc<RefCell<ConfigState>>>()
            .ok_or_else(|| LuaError::external("config state not initialized"))?;
        // ...
    }
}
```

**Benefits**:
- Schema is genuinely static (correct semantics)
- No unsafe code
- Testable: each test can have different ConfigState
- Best performance: no lock for property lookup

---


## 10. Next Steps

1. **Review specification** - Address open questions
2. **Implement Phase 1** - Infrastructure components
3. **Implement Phase 2** - Derive macro
4. **Begin Phase 3** - Registration
5. **Start Arc→Rc migration** - Can be done in parallel

The specification is ready for review. Once approved, we can begin implementation.