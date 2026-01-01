# RFC: Config API Simplification

## Status: Approved (pending spec)
## Author: AI Assistant (research phase)
## Date: 2024-12
## Decision: Option C (Hybrid) with Arc→Rc conversion

---

## Executive Summary

This RFC proposes simplifying niri-lua's configuration bridge by replacing 40+ generated proxy structs with a unified dynamic property system. The research compares three production implementations (Neovim, AwesomeWM, WezTerm) and recommends a hybrid approach inspired by WezTerm's intermediate Value type.

---

## Problem Statement

### Current Architecture

niri-lua uses a derive macro (`LuaConfigProxy`) that generates a separate proxy struct for each config section:

```rust
#[derive(LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "cursor", dirty = "Cursor")]
pub struct CursorConfig {
    pub xcursor_size: u8,
    pub xcursor_theme: String,
    pub hide_when_typing: bool,
}
// Generates: CursorConfigProxy with UserData impl, getters/setters, __toString
```

**Current counts:**
- 40+ proxy structs generated
- 21 DirtyFlag enum variants (manual mapping)
- ~2000 lines of generated code (estimated)
- Each new config field requires: struct field + dirty attribute + proxy regeneration

### Pain Points

1. **Boilerplate explosion**: Every config struct needs a corresponding proxy
2. **Manual dirty flag mapping**: `#[lua_proxy(dirty = "Cursor")]` must be specified per-struct
3. **Two systems**: `config_proxies.rs` (runtime mutable) vs `config_api.rs` (read-only tables) - potential divergence
4. **Compile time**: Derive macros increase build times
5. **Debugging difficulty**: Generated code is hard to trace

---

## Comparative Analysis

### 1. Neovim (vim.o / vim.opt)

**Architecture**: Centralized option registry with metamethod dispatch

```lua
-- User-facing API
vim.o.number = true           -- Simple __newindex
vim.opt.wildignore:append("*.o")  -- Returns Option object with methods

-- Internal: runtime/lua/vim/_options.lua
local function make_option_accessor(scope)
  return setmetatable({}, {
    __index = function(_, name)
      return nvim_get_option_value(name, {scope = scope})
    end,
    __newindex = function(_, name, value)
      nvim_set_option_value(name, value, {scope = scope})
    end
  })
end
```

**Key patterns:**
- Single source of truth: `src/nvim/options.lua` defines all option metadata
- Code generation at build time for C structs
- `vim.opt` returns lazy Option wrapper objects (don't fetch until `:get()`)
- Scoped accessors: `vim.bo[bufnr].filetype` creates new accessor dynamically

**Pros**: Single registry, lazy evaluation, excellent ergonomics
**Cons**: Requires build-time code generation, complex Option object implementation

### 2. AwesomeWM (luaA_class property system)

**Architecture**: C-side property registration with binary search lookup

```c
// Property registration (objects/client.c)
luaA_class_add_property(&client_class, "name",
    (lua_class_propfunc_t) luaA_client_set_name,  // constructor
    (lua_class_propfunc_t) luaA_client_get_name,  // getter  
    (lua_class_propfunc_t) luaA_client_set_name); // setter

// __index metamethod dispatch
static int luaA_class_index(lua_State *L) {
    // 1. Check metatable for methods
    // 2. Binary search property array
    // 3. Call property->index callback
    // 4. Fallback to index_miss_handler
}
```

**Key patterns:**
- Properties stored in sorted array (O(log n) binary search)
- Three callbacks per property: constructor, getter, setter (NULL = read-only)
- Signal emission on change: `luaA_object_emit_signal(L, idx, "property::name", 0)`
- Macros reduce boilerplate: `DO_CLIENT_SET_PROPERTY(name, type)`

**Pros**: Fast lookup, signal integration, flexible callbacks
**Cons**: C-specific, verbose registration, no type safety

### 3. WezTerm (wezterm-dynamic)

**Architecture**: Intermediate Value enum with derive macros for conversion

```rust
// Intermediate representation
pub enum Value {
    Null, Bool(bool), String(String),
    Array(Vec<Value>), Object(BTreeMap<String, Value>),
    U64(u64), I64(i64), F64(f64),
}

// Derive macros handle conversion
#[derive(FromDynamic, ToDynamic)]
struct Config {
    #[dynamic(default = "default_font_size")]
    font_size: f64,
    #[dynamic(try_from = "String")]
    color: Color,
}

// Lua workflow:
// 1. config_builder() returns plain Lua table
// 2. User assigns: config.font_size = 14
// 3. On return: table → Value::Object → Config::from_dynamic()
```

**Key patterns:**
- Plain Lua tables (no UserData complexity)
- Validation happens at conversion time, not assignment time
- Derive macros generate FromDynamic/ToDynamic impls
- No runtime reflection - all compile-time

**Pros**: Simple Lua side, type-safe conversion, good error messages
**Cons**: No immediate validation, no change signals, batch conversion only

---

## Proposed Solutions

### Option A: PropertyRegistry (Neovim-inspired)

**Concept**: Single registry mapping paths to property descriptors with dynamic `__index`/`__newindex`.

```rust
pub struct PropertyDescriptor {
    path: &'static str,           // "cursor.xcursor_size"
    ty: PropertyType,             // Bool, U8, String, Enum, Nested
    dirty_flag: DirtyFlag,        // Inferred from path prefix
    getter: fn(&Config) -> Value,
    setter: fn(&mut Config, Value) -> Result<()>,
}

pub struct PropertyRegistry {
    properties: BTreeMap<&'static str, PropertyDescriptor>,
}

// Single UserData for all config access
impl UserData for ConfigProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__index", |lua, this, key: String| {
            let path = format!("{}.{}", this.current_path, key);
            match this.registry.get(&path) {
                Some(desc) if desc.ty.is_nested() => {
                    // Return new ConfigProxy with extended path
                    Ok(ConfigProxy { current_path: path, ..this.clone() })
                }
                Some(desc) => {
                    // Return actual value
                    let config = this.state.borrow();
                    Ok((desc.getter)(&config))
                }
                None => Err(LuaError::external(format!("unknown config: {}", path)))
            }
        });
        
        methods.add_meta_method_mut("__newindex", |lua, this, (key, value): (String, Value)| {
            let path = format!("{}.{}", this.current_path, key);
            let desc = this.registry.get(&path)
                .ok_or_else(|| LuaError::external(format!("unknown config: {}", path)))?;
            
            let mut config = this.state.borrow_mut();
            (desc.setter)(&mut config, value)?;
            this.dirty_flags.mark(desc.dirty_flag);
            Ok(())
        });
    }
}
```

**DirtyFlag inference**:
```rust
fn infer_dirty_flag(path: &str) -> DirtyFlag {
    let prefix = path.split('.').next().unwrap();
    match prefix {
        "cursor" => DirtyFlag::Cursor,
        "layout" => DirtyFlag::Layout,
        "animations" => DirtyFlag::Animations,
        "input" => DirtyFlag::Input,
        // ... etc
        _ => DirtyFlag::Misc,
    }
}
```

**Pros**:
- Single proxy struct replaces 40+
- DirtyFlag auto-inferred from path
- Lazy nested access (no upfront construction)
- Easy to add new properties

**Cons**:
- Requires getter/setter fn pointers per property
- Registration still somewhat verbose
- No compile-time path validation

### Option B: Value Intermediate Type (WezTerm-inspired)

**Concept**: Convert config to/from intermediate `Value` type, use plain Lua tables.

```rust
// Reuse or adapt wezterm-dynamic pattern
#[derive(FromValue, ToValue)]
pub struct CursorConfig {
    #[value(default = 24)]
    pub xcursor_size: u8,
    pub xcursor_theme: String,
    #[value(default = false)]
    pub hide_when_typing: bool,
}

// Expose config as plain table
fn expose_config(lua: &Lua, config: &Config) -> LuaResult<LuaTable> {
    config.to_value()?.into_lua(lua)
}

// On config change: convert back
fn apply_config(lua: &Lua, table: LuaTable) -> LuaResult<Config> {
    let value = Value::from_lua(table, lua)?;
    Config::from_value(value).map_err(|e| LuaError::external(e))
}
```

**Dirty flag tracking**:
```rust
// Track which top-level keys were modified
struct ConfigTransaction {
    original: Value,
    current: LuaTable,
}

impl ConfigTransaction {
    fn commit(&self) -> (Config, Vec<DirtyFlag>) {
        let new_value = Value::from_lua(self.current.clone())?;
        let dirty = diff_values(&self.original, &new_value)
            .map(|path| infer_dirty_flag(path))
            .collect();
        (Config::from_value(new_value)?, dirty)
    }
}
```

**Pros**:
- Simplest Lua-side API (just tables)
- Derive macros already exist (wezterm-dynamic or similar)
- Batch validation with good error messages
- No UserData complexity

**Cons**:
- No immediate validation on assignment
- No change signals during modification
- Full config copy on each transaction
- Diffing for dirty flags is O(n) on config size

### Option C: Hybrid (Recommended)

**Concept**: Keep derive macros for type conversion, but use PropertyRegistry for runtime access.

```rust
// Derive macro generates property registration, not full proxy
#[derive(ConfigProperties)]
#[config(prefix = "cursor")]
pub struct CursorConfig {
    pub xcursor_size: u8,
    pub xcursor_theme: String,
    pub hide_when_typing: bool,
}

// Expands to:
impl ConfigProperties for CursorConfig {
    fn register(registry: &mut PropertyRegistry) {
        registry.add("cursor.xcursor_size", PropertyDescriptor {
            ty: PropertyType::U8,
            dirty_flag: DirtyFlag::Cursor,  // auto from prefix
            getter: |c| Value::U64(c.cursor.xcursor_size as u64),
            setter: |c, v| { c.cursor.xcursor_size = v.as_u8()?; Ok(()) },
        });
        // ... etc
    }
}

// Single ConfigProxy UserData with __index/__newindex
// accessing the shared PropertyRegistry
```

**Pros**:
- Best of both: derive macros for safety, dynamic dispatch for simplicity
- Single UserData struct
- Auto DirtyFlag inference
- Compile-time property validation
- Runtime path-based access

**Cons**:
- Still requires derive macro (but simpler than current)
- fn pointer indirection (negligible perf impact)

---

## Recommendation

**Option C (Hybrid)** provides the best balance:

1. **Reduces proxy count**: 40+ → 1 (ConfigProxy with PropertyRegistry)
2. **Auto DirtyFlag**: Inferred from path prefix, no manual annotation
3. **Type safety**: Derive macro validates at compile time
4. **Flexibility**: Easy to add computed properties or transformations
5. **Debuggability**: Single __index/__newindex to trace

### Migration Path

1. **Phase 1**: Create PropertyRegistry infrastructure alongside existing proxies
2. **Phase 2**: Add `#[derive(ConfigProperties)]` to existing config structs
3. **Phase 3**: Switch niri.config to use new ConfigProxy
4. **Phase 4**: Remove old LuaConfigProxy derive macro and generated code
5. **Phase 5**: Remove config_api.rs read-only table builder (if redundant)

### Risk Mitigation

- Keep existing API surface (`niri.config.cursor.xcursor_size = 24`)
- Add integration tests comparing old vs new behavior before switch
- Feature flag for gradual rollout

---

## Decisions

### 1. Approach: Option C (Hybrid)
Combine derive macros for type-safe registration with a single PropertyRegistry for dynamic dispatch.

### 2. Validation: On Assignment
Keep immediate validation (current behavior) rather than WezTerm's batch validation. Provides better user feedback.

### 3. Signal Emission: Yes
Emit `config::<path>` events on property changes (AwesomeWM pattern). Enables reactive scripts.

### 4. Arc<Mutex> → Rc<RefCell>: **APPROVED (with exception)**

**Analysis Summary:**

| Component | Current | Cross-Thread? | Decision |
|-----------|---------|---------------|----------|
| **ProcessManager** | `Arc<Mutex>` | **YES** - worker threads | **KEEP Arc<Mutex>** |
| **ConfigState** | `Arc<Mutex>` | No | **→ Rc<RefCell>** |
| **SharedEventHandlers** | `Arc<Mutex>` | No | **→ Rc<RefCell>** |
| **StateHandle fields** | `Arc<Mutex>` | No | **→ Rc<RefCell>** |
| **IpcLuaExecutor** | `Arc<Mutex>` | No | **→ Rc<RefCell>** |
| **ConfigWrapper** | `Arc<Mutex>` | No | **→ Rc<RefCell>** |

**Rationale:**
- Niri is single-threaded (main event loop, no tokio/async)
- Only ProcessManager has legitimate cross-thread needs (stdout/stderr worker threads use mpsc channels)
- mlua's `Lua` is `!Send` - UserData cannot cross threads anyway
- `RefCell::borrow()` is ~5x faster than `Mutex::lock()` on uncontended path
- `RefCell` panics on borrow violation (easier to debug than mutex deadlock)

**Safety:**
- All Lua callbacks execute synchronously on main thread
- No async runtime that could interleave borrows
- ProcessManager worker threads communicate via channels, not shared state

### 5. Nested Collections
Handle `layout.preset_column_widths` etc. with `ArrayProperty` descriptor containing element type info. Details in spec.

### 6. Gradual Rollout: Not Required
Project not in production - do full migration in single pass.

---

## Appendix A: Arc→Rc Migration Scope

**Files requiring changes:**

```
niri-lua/src/config_state.rs      - ConfigState wrapper
niri-lua/src/config_wrapper.rs    - ConfigWrapper UserData  
niri-lua/src/config_proxies.rs    - All proxy structs (will be replaced)
niri-lua/src/state_handle.rs      - StateHandle fields
niri-lua/src/events_proxy.rs      - SharedEventHandlers
niri-lua/src/ipc_repl.rs          - IpcLuaExecutor runtime
```

**Files to keep Arc<Mutex>:**

```
niri-lua/src/process.rs           - ProcessManager (cross-thread)
```

---

## Appendix B: Current vs Proposed Code Size

| Component | Current | Option C |
|-----------|---------|----------|
| Proxy structs | 40+ | 1 |
| Derive macro complexity | High | Medium |
| Generated code | ~2000 lines | ~500 lines |
| DirtyFlag annotations | Manual per-struct | Auto from prefix |
| config_api.rs | Separate | Unified |

---

## Next Steps

1. ~~Review RFC and choose approach~~ → **Option C approved**
2. ~~Arc→Rc analysis~~ → **Approved with ProcessManager exception**
3. **Draft detailed specification document** ← NEXT
4. Implement PropertyRegistry infrastructure
5. Add `#[derive(ConfigProperties)]` macro
6. Migrate config sections
7. Remove old LuaConfigProxy system
