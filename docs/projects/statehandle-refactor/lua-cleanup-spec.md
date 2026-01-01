# Lua Integration Cleanup Specification

**Status:** Draft  
**Created:** 2026-01-01  
**Depends on:** config-api-simplification-spec.md (completed)

## Executive Summary

This specification covers code cleanup, DRY improvements, and architectural unification for niri-lua following the Config API simplification refactor. The goal is to reduce ~4000 lines of boilerplate while improving type safety and maintainability.

### Scope

| Phase | Component | LOC Reduction | Priority |
|-------|-----------|---------------|----------|
| 1 | Enable mlua features (serde, userdata-wrappers) | 0 (prep) | HIGH |
| 2 | extractors.rs → serde Deserialize | ~1500 | HIGH |
| 3 | Derive macro accessor generation | ~1200 | HIGH |
| 4 | collections.rs → serde Serialize/Deserialize | ~800 | MEDIUM |
| 5 | traits.rs simplification | ~300 | MEDIUM |
| 6 | Dead code removal & test cleanup | ~200 | LOW |

**Total estimated reduction: ~4000 LOC (12% of niri-lua)**

---

## Phase 1: Enable mlua Features

### 1.1 Cargo.toml Changes

**File:** `Cargo.toml` (workspace root, line 28)

```toml
# Current
mlua = { version = "0.11.4", features = ["luau", "luau-jit", "vendored"] }

# Target
mlua = { version = "0.11.4", features = [
    "luau",
    "luau-jit", 
    "vendored",
    "serde",              # Auto FromLua/IntoLua via serde traits
    "userdata-wrappers",  # Auto UserData for Rc<T>, Arc<T>, Rc<RefCell<T>>
] }
```

### 1.2 Dependencies

The `serde` feature transitively adds:
- `serde` (already in workspace for niri-config)
- `erased-serde`
- `serde-value`

No new direct dependencies required.

### 1.3 Verification

```bash
cargo build -p niri-lua
cargo test -p niri-lua
```

### 1.4 Acceptance Criteria

- [ ] Build succeeds with new features
- [ ] All existing tests pass
- [ ] `lua.to_value()` and `lua.from_value()` are available

---

## Phase 2: extractors.rs Migration to Serde

### 2.1 Current State

**File:** `niri-lua/src/extractors.rs` (1860 LOC)

Current pattern - manual field-by-field extraction:

```rust
pub fn extract_keyboard(table: &LuaTable) -> LuaResult<Option<Keyboard>> {
    let xkb = extract_xkb(table)?;
    let repeat_delay = extract_int_opt(table, "repeat_delay")?.map(|v| v as u16);
    let repeat_rate = extract_int_opt(table, "repeat_rate")?.map(|v| v as u16);
    let track_layout = extract_string_opt(table, "track_layout")?
        .map(|s| parse_track_layout(&s))
        .transpose()?;
    // ... 10+ more fields manually extracted ...
    
    if xkb.is_none() && repeat_delay.is_none() && repeat_rate.is_none() /* && ... */ {
        return Ok(None);
    }
    Ok(Some(Keyboard { xkb, repeat_delay, repeat_rate, track_layout, /* ... */ }))
}
```

### 2.2 Target State

Replace manual extraction with serde Deserialize:

```rust
use mlua::LuaSerdeExt;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Keyboard {
    pub xkb: Option<Xkb>,
    pub repeat_delay: Option<u16>,
    pub repeat_rate: Option<u16>,
    #[serde(deserialize_with = "deserialize_track_layout")]
    pub track_layout: Option<TrackLayout>,
    // ...
}

// Usage becomes one line:
pub fn extract_keyboard(lua: &Lua, value: LuaValue) -> LuaResult<Option<Keyboard>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}
```

### 2.3 Structs to Migrate

| Struct | Fields | Custom Deserializers Needed |
|--------|--------|----------------------------|
| `Xkb` | 6 | None |
| `Keyboard` | 12 | `track_layout` (enum) |
| `Touchpad` | 15 | `accel_profile`, `scroll_method`, `tap_button_map`, `click_method` (enums) |
| `Mouse` | 8 | `accel_profile`, `scroll_method` |
| `Trackpoint` | 6 | `accel_profile` |
| `Trackball` | 6 | `accel_profile`, `scroll_method` |
| `Tablet` | 3 | None |
| `Touch` | 1 | None |
| `Output` | 10 | `transform`, `vrr` (enums) |
| `Mode` | 4 | None (parse from string) |
| `Position` | 2 | None |
| `Bind` | 8 | `action` (complex) |
| `Gesture` | 6 | `direction`, `action` |

### 2.4 Enum Deserialization Pattern

For enums that serialize as strings:

```rust
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Copy)]
pub enum AccelProfile {
    Adaptive,
    Flat,
}

impl<'de> Deserialize<'de> for AccelProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "adaptive" => Ok(AccelProfile::Adaptive),
            "flat" => Ok(AccelProfile::Flat),
            _ => Err(serde::de::Error::custom(format!(
                "invalid accel_profile '{}', expected 'adaptive' or 'flat'",
                s
            ))),
        }
    }
}
```

**Alternative:** Use `#[serde(rename_all = "snake_case")]` if enum variants match Lua strings.

### 2.5 Complex Field Handling

For fields requiring custom parsing (e.g., Mode from "1920x1080@60"):

```rust
#[derive(Debug, Clone)]
pub struct Mode {
    pub width: u16,
    pub height: u16,
    pub refresh: Option<f64>,
}

impl<'de> Deserialize<'de> for Mode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Accept either string "1920x1080@60" or table {width=1920, height=1080, refresh=60}
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ModeRepr {
            String(String),
            Table { width: u16, height: u16, refresh: Option<f64> },
        }
        
        match ModeRepr::deserialize(deserializer)? {
            ModeRepr::String(s) => parse_mode_string(&s).map_err(serde::de::Error::custom),
            ModeRepr::Table { width, height, refresh } => Ok(Mode { width, height, refresh }),
        }
    }
}
```

### 2.6 Helper Functions to Keep

Some helpers are still needed for non-serde contexts:

```rust
// Keep: Used in contexts where we don't have LuaValue
pub fn parse_mode_string(s: &str) -> Result<Mode, String> { ... }
pub fn parse_color_string(s: &str) -> Result<Color, String> { ... }

// Remove: Replaced by serde
pub fn extract_bool_opt(...) -> ... { ... }  // DELETE
pub fn extract_int_opt(...) -> ... { ... }   // DELETE
pub fn extract_float_opt(...) -> ... { ... } // DELETE
pub fn extract_string_opt(...) -> ... { ... } // DELETE
pub fn extract_table_opt(...) -> ... { ... } // DELETE
```

### 2.7 Migration Steps

1. Add `#[derive(Deserialize)]` to leaf structs first (Xkb, Mode, Position)
2. Add custom deserializers for enums
3. Add `#[derive(Deserialize)]` to container structs (Keyboard, Touchpad, etc.)
4. Replace `extract_*` functions with `lua.from_value()`
5. Remove unused `extract_*_opt` helpers
6. Update call sites

### 2.8 Acceptance Criteria

- [ ] All structs in extractors.rs derive `Deserialize`
- [ ] `extract_*_opt` helper functions removed
- [ ] All `extract_*` functions reduced to 1-3 lines
- [ ] extractors.rs reduced to ~300 LOC
- [ ] All tests pass
- [ ] Error messages remain user-friendly

---

## Phase 3: Derive Macro Accessor Generation

### 3.1 Current State

**File:** `niri-lua/src/config_accessors.rs` (1259 LOC)

The derive macro generates metadata but NOT accessor closures:

```rust
// Current: Manual registration (repeated 80+ times)
registry.update_accessor(
    "cursor.xcursor_size",
    |config, lua| Ok(LuaValue::Integer(config.cursor.xcursor_size as i64)),
    |config, lua, value| {
        config.cursor.xcursor_size = match value {
            LuaValue::Integer(i) => i as u8,
            _ => return Err(LuaError::external("expected integer")),
        };
        Ok(())
    },
);
```

### 3.2 Target State

Extend `#[derive(ConfigProperties)]` to generate accessors:

```rust
// In niri-config/src/cursor.rs
#[derive(ConfigProperties, Deserialize)]
#[config(prefix = "cursor", dirty = "Cursor")]
pub struct CursorConfig {
    pub xcursor_size: u8,
    pub xcursor_theme: String,
    #[config(no_signal)]
    pub hide_when_typing: bool,
    pub hide_after_inactive_ms: Option<u32>,
}

// Generated by macro:
impl ConfigProperties for CursorConfig {
    fn register(registry: &mut PropertyRegistry) {
        registry.add(PropertyDescriptor {
            path: "cursor.xcursor_size".into(),
            ty: PropertyType::Integer,
            dirty_flag: DirtyFlag::Cursor,
            getter: |config, lua| Ok(LuaValue::Integer(config.cursor.xcursor_size as i64)),
            setter: |config, lua, value| {
                config.cursor.xcursor_size = lua.from_value(value)?;
                Ok(())
            },
            signal: true,
        });
        // ... repeat for each field ...
    }
}
```

### 3.3 Type-to-Accessor Mapping

| Rust Type | PropertyType | Getter Pattern | Setter Pattern |
|-----------|--------------|----------------|----------------|
| `bool` | `Bool` | `LuaValue::Boolean(v)` | `lua.from_value(v)?` |
| `i32/i64` | `Integer` | `LuaValue::Integer(v as i64)` | `lua.from_value(v)?` |
| `u8-u64` | `Integer` | `LuaValue::Integer(v as i64)` | `lua.from_value(v)?` |
| `f32/f64` | `Number` | `LuaValue::Number(v as f64)` | `lua.from_value(v)?` |
| `String` | `String` | `LuaValue::String(lua.create_string(&v)?)` | `lua.from_value(v)?` |
| `Option<T>` | (wrapped) | `match v { Some(x) => ..., None => Nil }` | `lua.from_value(v)?` |
| `Vec<T>` | `Array(Box<T>)` | `lua.to_value(&v)?` | `lua.from_value(v)?` |
| `enum` | `Enum{..}` | `LuaValue::String(v.to_string())` | custom parse |
| nested struct | `Nested` | return child ConfigProxy | delegate |

### 3.4 Serde Integration in Accessors

With serde enabled, setters simplify dramatically:

```rust
// Before (manual parsing)
setter: |config, lua, value| {
    config.cursor.xcursor_size = match value {
        LuaValue::Integer(i) if i >= 0 && i <= 255 => i as u8,
        LuaValue::Integer(_) => return Err(LuaError::external("value out of range")),
        _ => return Err(LuaError::external("expected integer")),
    };
    Ok(())
}

// After (serde-powered)
setter: |config, lua, value| {
    config.cursor.xcursor_size = lua.from_value(value)?;
    Ok(())
}
```

### 3.5 Macro Attribute Reference

```rust
#[derive(ConfigProperties)]
#[config(
    prefix = "path.prefix",  // Required: Property path prefix
    dirty = "FlagName",      // Optional: DirtyFlag variant (inferred from prefix if omitted)
)]
pub struct MyConfig {
    #[config(skip)]          // Skip this field entirely
    internal_field: u32,
    
    #[config(no_signal)]     // Don't emit signal on change
    silent_field: bool,
    
    #[config(rename = "lua_name")]  // Use different name in Lua
    rust_name: String,
    
    #[config(getter = "custom_getter_fn")]  // Custom getter function
    #[config(setter = "custom_setter_fn")]  // Custom setter function
    complex_field: ComplexType,
}
```

### 3.6 config_accessors.rs Target State

```rust
// Target: ~50 LOC
use niri_config::*;

pub fn register_all_accessors(registry: &mut PropertyRegistry) {
    // Each struct's register() is called
    CursorConfig::register(registry);
    InputConfig::register(registry);
    LayoutConfig::register(registry);
    // ... one line per config struct ...
}
```

### 3.7 Acceptance Criteria

- [ ] Derive macro generates getter/setter closures
- [ ] All `#[config(...)]` attributes supported
- [ ] config_accessors.rs reduced to ~50 LOC
- [ ] Serde used for type conversion in setters
- [ ] All tests pass
- [ ] Compile time increase < 5 seconds

---

## Phase 4: collections.rs Serde Migration

### 4.1 Current State

**File:** `niri-lua/src/collections.rs` (1853 LOC)

Manual bidirectional conversion:

```rust
fn extract_output(table: &LuaTable) -> LuaResult<Output> {
    let name = table.get::<String>("name")?;
    let enable = extract_bool_opt(table, "enable")?;
    // ... 8 more fields ...
    Ok(Output { name, enable, /* ... */ })
}

fn output_to_table(lua: &Lua, output: &Output) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set("name", output.name.clone())?;
    if let Some(enable) = output.enable {
        t.set("enable", enable)?;
    }
    // ... 8 more fields ...
    Ok(t)
}
```

### 4.2 Target State

```rust
use mlua::LuaSerdeExt;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    // ...
}

// Usage:
fn extract_output(lua: &Lua, value: LuaValue) -> LuaResult<Output> {
    lua.from_value(value)
}

fn output_to_table(lua: &Lua, output: &Output) -> LuaResult<LuaValue> {
    lua.to_value(output)
}
```

### 4.3 Structs to Migrate

| Struct | Serialize | Deserialize | Notes |
|--------|-----------|-------------|-------|
| `Output` | Yes | Yes | Full bidirectional |
| `Workspace` | Yes | Yes | |
| `Bind` | Yes | Yes | Complex action field |
| `WindowRule` | Yes | Yes | Multiple match fields |
| `LayerRule` | Yes | Yes | |
| `Animation` | Yes | Yes | Curve field needs custom |

### 4.4 CollectionProxy Simplification

```rust
// Current define_collection! macro generates verbose code
// Can simplify with serde:

impl<T: Serialize + for<'de> Deserialize<'de>> CollectionProxyBase for Vec<T> {
    fn list(&self, lua: &Lua) -> LuaResult<LuaValue> {
        lua.to_value(self)
    }
    
    fn add(&mut self, lua: &Lua, value: LuaValue) -> LuaResult<()> {
        let item: T = lua.from_value(value)?;
        self.push(item);
        Ok(())
    }
}
```

### 4.5 Acceptance Criteria

- [ ] All collection structs derive `Serialize` + `Deserialize`
- [ ] `extract_*` and `*_to_table` functions use serde
- [ ] collections.rs reduced to ~1000 LOC
- [ ] All tests pass

---

## Phase 5: traits.rs Simplification

### 5.1 Current State

**File:** `niri-lua/src/traits.rs` (~500 LOC)

```rust
pub trait LuaFieldConvert: Sized {
    type LuaType: IntoLua + FromLua;
    fn to_lua(&self) -> Self::LuaType;
    fn from_lua(value: Self::LuaType) -> LuaResult<Self>;
}

macro_rules! impl_lua_field_convert_copy {
    ($($t:ty),*) => { ... }
}
impl_lua_field_convert_copy!(bool, i32, i64, u8, u16, u32, u64, f64);

impl LuaFieldConvert for String { ... }
impl LuaFieldConvert for Duration { ... }
impl LuaFieldConvert for Color { ... }
```

### 5.2 Target State

With serde, most of `LuaFieldConvert` becomes unnecessary:

```rust
// Keep only for types needing custom Lua representation
// that differs from their serde representation

/// Duration serializes as milliseconds (u64)
pub mod duration_as_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;
    
    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_u64(duration.as_millis() as u64)
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where D: Deserializer<'de> {
        let ms = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(ms))
    }
}

/// Color serializes as "#RRGGBBAA" string
pub mod color_as_string {
    // Similar pattern
}

// Usage in structs:
#[derive(Serialize, Deserialize)]
pub struct AnimationConfig {
    #[serde(with = "duration_as_millis")]
    pub duration: Duration,
    
    #[serde(with = "color_as_string")]
    pub color: Color,
}
```

### 5.3 What to Remove

- `LuaFieldConvert` trait (replaced by serde)
- `impl_lua_field_convert_copy!` macro
- All primitive `LuaFieldConvert` impls
- `LuaEnumConvert` trait (replaced by serde enum handling)

### 5.4 What to Keep

- `duration_as_millis` serde module
- `color_as_string` serde module
- `GradientTable` and `SpawnAtStartupTable` (complex UserData types)
- `parse_color_string` helper (used outside serde)

### 5.5 Acceptance Criteria

- [ ] `LuaFieldConvert` trait removed
- [ ] Serde modules for Duration, Color created
- [ ] traits.rs reduced to ~200 LOC
- [ ] All tests pass

---

## Phase 6: Dead Code Removal & Cleanup

### 6.1 Dead Code Detection

Run after phases 2-5:

```bash
# Unused dependencies
cargo +nightly udeps -p niri-lua

# Dead code warnings
cargo clippy -p niri-lua -- -W dead_code -W unused_imports

# Unused functions
cargo clippy -p niri-lua -- -W unused_must_use
```

### 6.2 Expected Removals

| Category | Estimated Items |
|----------|-----------------|
| Unused `extract_*` helpers | ~20 functions |
| Orphaned type conversions | ~10 impls |
| Duplicate test utilities | ~5 functions |
| Commented-out code | ~50 lines |

### 6.3 Test Cleanup

Look for:
- Duplicate test setup code → extract to fixtures
- Tests for removed functionality → delete
- Overly verbose test helpers → simplify with serde

### 6.4 Acceptance Criteria

- [ ] `cargo +nightly udeps` reports no unused dependencies
- [ ] `cargo clippy` passes with `-W dead_code`
- [ ] No commented-out code blocks
- [ ] Test helpers consolidated

---

## Implementation Gap Check

### 7.1 Review Checklist

After each phase, verify:

| Check | Command/Action |
|-------|----------------|
| Build passes | `cargo build -p niri-lua` |
| Tests pass | `cargo test -p niri-lua` |
| Clippy clean | `cargo clippy -p niri-lua` |
| Format clean | `cargo +nightly fmt -p niri-lua -- --check` |
| No regressions | Manual smoke test of Lua config |
| Error messages | Verify user-friendly error messages preserved |

### 7.2 Integration Points to Verify

| Integration | Test Method |
|-------------|-------------|
| Config loading from Lua | Load test config, verify all fields |
| Config modification at runtime | Set properties, verify dirty flags |
| Signal emission | Subscribe to config change, verify callback |
| Collection manipulation | Add/remove/update outputs, binds |
| IPC config access | `niri msg` config queries |

### 7.3 Performance Verification

```bash
# Before/after comparison
hyperfine --warmup 3 'cargo test -p niri-lua config_'

# Compile time check
cargo clean -p niri-lua && time cargo build -p niri-lua
```

### 7.4 API Compatibility Check

Verify Lua API unchanged:

```lua
-- Test script to run before and after
assert(niri.config.cursor.xcursor_size)
assert(type(niri.config.layout.gaps) == "number")
assert(niri.config.input.keyboard.xkb.layout)

-- Collections
local outputs = niri.config.outputs:list()
niri.config.outputs:add({ name = "test", enable = true })

-- Signals
niri.events:on("config::cursor.xcursor_size", function(old, new)
    print("Changed from", old, "to", new)
end)
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Serde error messages less helpful | Medium | Medium | Custom deserializers with good errors |
| Compile time increase | Low | Low | Monitor, optimize if needed |
| Breaking Lua API | Low | High | Comprehensive test suite |
| Performance regression | Low | Medium | Benchmark critical paths |

---

## Timeline Estimate

| Phase | Estimated Effort | Dependencies |
|-------|------------------|--------------|
| Phase 1: Enable features | 1 hour | None |
| Phase 2: extractors.rs | 4-6 hours | Phase 1 |
| Phase 3: Derive macro | 6-8 hours | Phase 1, 2 |
| Phase 4: collections.rs | 3-4 hours | Phase 1 |
| Phase 5: traits.rs | 2-3 hours | Phase 2, 3 |
| Phase 6: Cleanup | 2-3 hours | All above |

**Total: 18-25 hours**

---

## Appendix A: Serde Deserialize Patterns

### A.1 Optional fields with default

```rust
#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    pub enabled: bool,  // defaults to false
}
```

### A.2 Enum from string

```rust
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollMethod {
    NoScroll,
    TwoFinger,
    Edge,
    OnButtonDown,
}
```

### A.3 Untagged enum (multiple representations)

```rust
#[derive(Deserialize)]
#[serde(untagged)]
pub enum SizeSpec {
    Fixed(i32),
    Proportion { proportion: f64 },
    String(String),  // "50%" parsed separately
}
```

### A.4 Flattened nested struct

```rust
#[derive(Deserialize)]
pub struct Outer {
    pub name: String,
    #[serde(flatten)]
    pub inner: Inner,
}
```

---

## Appendix B: File Change Summary

| File | Before LOC | After LOC | Change |
|------|------------|-----------|--------|
| extractors.rs | 1860 | ~300 | -1560 |
| config_accessors.rs | 1259 | ~50 | -1209 |
| collections.rs | 1853 | ~1000 | -853 |
| traits.rs | ~500 | ~200 | -300 |
| Other cleanup | - | - | -200 |
| **Total** | - | - | **-4122** |
