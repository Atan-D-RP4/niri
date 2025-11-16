# Tier 2 Specification: Full Configuration API

**Duration:** Weeks 3-4  
**Estimated LOC:** 400-500 Rust + 150 documentation  
**Complexity:** High (complex type mappings)

## Overview

Tier 2 enables complete configuration of Niri from Lua, achieving **configuration parity with KDL**. Users can:
- Configure all Niri settings from Lua
- Use complex types (animations, gestures, filters)
- Get early validation and clear error messages
- Migrate existing KDL configs to Lua
- Mix KDL and Lua configuration

This tier is critical because it demonstrates that Lua is a first-class configuration language for Niri, not a second-class citizen.

---

## Architecture

```
Configuration API Structure:
  niri.config
    ├── .animations = { ... }      # Animation configuration
    ├── .input = { ... }           # Keyboard, mouse, touchpad settings
    ├── .layout = { ... }          # Window rules, layout settings
    ├── .gestures = { ... }        # Gesture recognition
    ├── .appearance = { ... }      # Visual appearance
    ├── .output = { ... }          # Monitor/display settings
    └── functions
        ├── apply() → applies all pending changes
        ├── get_all() → returns all current settings
        └── validate(key, value) → validates before applying

Complex Types (via UserData):
  - Animation { frames, duration, curve, ... }
  - Gesture { fingers, edges, action, ... }
  - WindowRule { filter, action, ... }
  - Filter { match_app_id, match_title, ... }
  - InputSettings { repeat_delay, repeat_rate, ... }
  - OutputSettings { scale, refresh_rate, ... }

Type Validation:
  - Schema validation at setting time
  - Early error reporting
  - Default values for optional settings
  - Type coercion where sensible
```

---

## Detailed Specifications

### 1. Configuration API Module (`src/lua_extensions/config_api.rs`)

#### Purpose
Provide `niri.config.*` modules for all Niri settings.

#### Key Structures

```rust
pub struct ConfigurationApi {
    animations: AnimationConfig,
    input: InputConfig,
    layout: LayoutConfig,
    gestures: GestureConfig,
    appearance: AppearanceConfig,
    output: OutputConfig,
}

impl ConfigurationApi {
    pub fn register_to_lua(lua: &Lua, config: &Config) -> LuaResult<()> {
        let globals = lua.globals();
        
        // Create niri.config table
        let config_table = lua.create_table()?;
        
        // Add submodules
        Self::register_animations(lua, &config_table, &config.animations)?;
        Self::register_input(lua, &config_table, &config.input)?;
        Self::register_layout(lua, &config_table, &config.layout)?;
        Self::register_gestures(lua, &config_table, &config.gestures)?;
        Self::register_appearance(lua, &config_table, &config.appearance)?;
        Self::register_output(lua, &config_table, &config.outputs)?;
        
        // Add utility functions
        Self::register_functions(lua, &config_table)?;
        
        // Set niri.config
        let niri_table = globals.get::<_, LuaTable>("niri")?;
        niri_table.set("config", config_table)?;
        
        Ok(())
    }
}
```

#### Submodule: Animation Configuration

```rust
fn register_animations(lua: &Lua, config_table: &LuaTable, anim_config: &AnimationsConfig) -> LuaResult<()> {
    let animations = lua.create_table()?;
    
    // Get/Set animation duration
    animations.set("duration", anim_config.duration.as_millis())?;
    
    // Animation curves (read-only reference)
    let curves = lua.create_table()?;
    curves.set("linear", "linear")?;
    curves.set("ease_in_out_cubic", "ease_in_out_cubic")?;
    curves.set("ease_out_cubic", "ease_out_cubic")?;
    animations.set("curves", curves)?;
    
    config_table.set("animations", animations)?;
    Ok(())
}

// Example Lua usage:
// niri.config.animations.duration = 200
// niri.config.animations.output_open_curves = "ease_out_cubic"
```

#### Submodule: Input Configuration

```rust
fn register_input(lua: &Lua, config_table: &LuaTable, input_config: &InputConfig) -> LuaResult<()> {
    let input = lua.create_table()?;
    
    // Keyboard settings
    let keyboard = lua.create_table()?;
    keyboard.set("xkb_layout", &input_config.keyboard.xkb_layout)?;
    keyboard.set("xkb_variant", &input_config.keyboard.xkb_variant)?;
    keyboard.set("xkb_options", &input_config.keyboard.xkb_options)?;
    keyboard.set("repeat_delay", input_config.keyboard.repeat_delay.as_millis())?;
    keyboard.set("repeat_rate", input_config.keyboard.repeat_rate)?;
    input.set("keyboard", keyboard)?;
    
    // Mouse settings
    let mouse = lua.create_table()?;
    mouse.set("accel_speed", input_config.mouse.accel_speed)?;
    mouse.set("accel_profile", &input_config.mouse.accel_profile)?;
    input.set("mouse", mouse)?;
    
    // Touchpad settings
    let touchpad = lua.create_table()?;
    touchpad.set("accel_speed", input_config.touchpad.accel_speed)?;
    touchpad.set("accel_profile", &input_config.touchpad.accel_profile)?;
    touchpad.set("tap", input_config.touchpad.tap)?;
    input.set("touchpad", touchpad)?;
    
    config_table.set("input", input)?;
    Ok(())
}

// Example Lua usage:
// niri.config.input.keyboard.repeat_delay = 300
// niri.config.input.keyboard.xkb_layout = "us,ru"
// niri.config.input.mouse.accel_speed = 1.0
```

#### Submodule: Layout Configuration

```rust
fn register_layout(lua: &Lua, config_table: &LuaTable, layout_config: &LayoutConfig) -> LuaResult<()> {
    let layout = lua.create_table()?;
    
    // Window rules (as Lua table)
    let rules = lua.create_table()?;
    for (i, rule) in layout_config.window_rules.iter().enumerate() {
        let rule_table = create_window_rule_table(lua, rule)?;
        rules.set(i + 1, rule_table)?;  // Lua tables are 1-indexed
    }
    layout.set("window_rules", rules)?;
    
    // Other layout settings
    layout.set("default_width_percent", layout_config.default_width_percent)?;
    layout.set("default_height_percent", layout_config.default_height_percent)?;
    layout.set("preset_column_widths", layout_config.preset_column_widths.clone())?;
    
    config_table.set("layout", layout)?;
    Ok(())
}
```

#### Submodule: Gesture Configuration

```rust
fn register_gestures(lua: &Lua, config_table: &LuaTable, gesture_config: &GestureConfig) -> LuaResult<()> {
    let gestures = lua.create_table()?;
    
    // Touchpad gesture settings
    let touchpad = lua.create_table()?;
    touchpad.set("enabled", gesture_config.touchpad_gesture.enabled)?;
    touchpad.set("tilt_threshold", gesture_config.touchpad_gesture.tilt_threshold)?;
    gestures.set("touchpad", touchpad)?;
    
    // Shortcut gestures (as Lua table)
    let shortcuts = lua.create_table()?;
    for (i, gesture) in gesture_config.gesture_actions.iter().enumerate() {
        let gesture_table = create_gesture_table(lua, gesture)?;
        shortcuts.set(i + 1, gesture_table)?;
    }
    gestures.set("shortcuts", shortcuts)?;
    
    config_table.set("gestures", gestures)?;
    Ok(())
}
```

#### Submodule: Appearance Configuration

```rust
fn register_appearance(lua: &Lua, config_table: &LuaTable, app_config: &AppearanceConfig) -> LuaResult<()> {
    let appearance = lua.create_table()?;
    
    // Gaps and borders
    appearance.set("gaps", app_config.gaps)?;
    appearance.set("border_width", app_config.border_width)?;
    
    // Colors
    let colors = lua.create_table()?;
    colors.set("border_active", format_color(&app_config.border_color_active))?;
    colors.set("border_inactive", format_color(&app_config.border_color_inactive))?;
    colors.set("focus_ring", format_color(&app_config.focus_ring_color))?;
    appearance.set("colors", colors)?;
    
    // Window decoration
    appearance.set("prefer_no_csd", app_config.prefer_no_csd)?;
    
    // Font settings
    let font = lua.create_table()?;
    font.set("family", &app_config.font_family)?;
    font.set("size", app_config.font_size)?;
    appearance.set("font", font)?;
    
    config_table.set("appearance", appearance)?;
    Ok(())
}

// Example Lua usage:
// niri.config.appearance.gaps = 8
// niri.config.appearance.border_width = 2
// niri.config.appearance.colors.border_active = "#ff00ff"
```

#### Submodule: Output Configuration

```rust
fn register_output(lua: &Lua, config_table: &LuaTable, outputs: &HashMap<String, OutputConfig>) -> LuaResult<()> {
    let output_table = lua.create_table()?;
    
    for (name, config) in outputs {
        let output = lua.create_table()?;
        output.set("enabled", config.enabled)?;
        output.set("scale", config.scale)?;
        output.set("refresh_rate", config.refresh_rate)?;
        output.set("position", (config.position.x, config.position.y))?;
        output_table.set(name.clone(), output)?;
    }
    
    config_table.set("output", output_table)?;
    Ok(())
}

// Example Lua usage:
// niri.config.output["HDMI-1"].scale = 2.0
// niri.config.output["DP-1"].refresh_rate = 144
```

#### Utility Functions

```rust
fn register_functions(lua: &Lua, config_table: &LuaTable) -> LuaResult<()> {
    // config.apply() - Apply pending configuration
    let apply = lua.create_function(|_, ()| {
        // Apply all pending changes
        niri::apply_configuration_changes()?;
        Ok("Configuration applied")
    })?;
    config_table.set("apply", apply)?;
    
    // config.get_all() - Get all current settings
    let get_all = lua.create_function(|lua, ()| {
        // Return all settings as Lua table
        let all_config = build_config_table(lua)?;
        Ok(all_config)
    })?;
    config_table.set("get_all", get_all)?;
    
    // config.validate(key, value) - Validate before applying
    let validate = lua.create_function(|_, (key, value): (String, LuaValue)| {
        validate_config_value(&key, &value)?;
        Ok(true)
    })?;
    config_table.set("validate", validate)?;
    
    Ok(())
}
```

---

### 2. Lua Types Module (`src/lua_extensions/lua_types.rs`)

#### Purpose
Define complex types as Lua UserData with getter/setter methods.

#### Key Types

```rust
// Animation type with LuaJIT optimization
#[derive(Debug, Clone)]
pub struct LuaAnimation {
    pub name: String,
    pub duration_ms: i32,
    pub curve: String,  // "linear", "ease_in_out_cubic", etc.
}

impl mlua::UserData for LuaAnimation {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_name", |_, this, ()| Ok(this.name.clone()));
        methods.add_method("get_duration", |_, this, ()| Ok(this.duration_ms));
        methods.add_method("get_curve", |_, this, ()| Ok(this.curve.clone()));
        
        methods.add_mut_method("set_duration", |_, this, ms: i32| {
            if ms <= 0 || ms > 5000 {
                return Err(anyhow!("Duration must be between 1 and 5000 ms").into());
            }
            this.duration_ms = ms;
            Ok(())
        });
        
        methods.add_mut_method("set_curve", |_, this, curve: String| {
            if !["linear", "ease_in_out_cubic", "ease_out_cubic"].contains(&curve.as_str()) {
                return Err(anyhow!("Unknown curve: {}", curve).into());
            }
            this.curve = curve;
            Ok(())
        });
    }
}

// Window rule type
#[derive(Debug, Clone)]
pub struct LuaWindowRule {
    pub filter: LuaFilter,
    pub floating: Option<bool>,
    pub fullscreen: Option<bool>,
    pub tile: Option<bool>,
}

impl mlua::UserData for LuaWindowRule {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("matches", |_, this, (app_id, title): (String, String)| {
            Ok(this.filter.matches(&app_id, &title))
        });
        
        methods.add_method("get_floating", |_, this, ()| Ok(this.floating));
    }
}

// Filter type for matching windows
#[derive(Debug, Clone)]
pub struct LuaFilter {
    pub match_app_id: Option<String>,
    pub match_title: Option<String>,
    pub regex_app_id: Option<Regex>,
    pub regex_title: Option<Regex>,
}

impl LuaFilter {
    pub fn matches(&self, app_id: &str, title: &str) -> bool {
        if let Some(ref regex) = self.regex_app_id {
            if !regex.is_match(app_id) {
                return false;
            }
        } else if let Some(ref match_app) = self.match_app_id {
            if app_id != match_app {
                return false;
            }
        }
        
        if let Some(ref regex) = self.regex_title {
            if !regex.is_match(title) {
                return false;
            }
        } else if let Some(ref match_title) = self.match_title {
            if !title.contains(match_title) {
                return false;
            }
        }
        
        true
    }
}

impl mlua::UserData for LuaFilter {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("matches", |_, this, (app_id, title): (String, String)| {
            Ok(this.matches(&app_id, &title))
        });
    }
}

// Gesture type
#[derive(Debug, Clone)]
pub struct LuaGesture {
    pub gesture_type: String,  // "swipe", "pinch", "hold"
    pub fingers: u32,
    pub direction: Option<String>,  // "up", "down", "left", "right"
    pub action: String,
}

impl mlua::UserData for LuaGesture {
    // Methods for gesture manipulation
}
```

#### Type Registration

```rust
pub fn register_types_to_lua(lua: &Lua) -> LuaResult<()> {
    // Register all UserData types globally
    lua.globals().set("LuaAnimation", LuaAnimation::metatable())?;
    lua.globals().set("LuaWindowRule", LuaWindowRule::metatable())?;
    lua.globals().set("LuaFilter", LuaFilter::metatable())?;
    lua.globals().set("LuaGesture", LuaGesture::metatable())?;
    
    Ok(())
}
```

#### Example Lua Usage

```lua
-- Create animation
local anim = {
    name = "output_open",
    duration = 200,
    curve = "ease_out_cubic"
}
niri.config.animations.output_open = anim

-- Create window rule
local rule = {
    match_app_id = "firefox",
    floating = false,
    fullscreen = false,
}
table.insert(niri.config.layout.window_rules, rule)

-- Create filter
local filter = {
    match_app_id = "alacritty",
    -- match_title = "Editor"  -- optional
}

-- Create gesture
local gesture = {
    gesture_type = "swipe",
    fingers = 3,
    direction = "left",
    action = "focus_workspace_left"
}
table.insert(niri.config.gestures.shortcuts, gesture)
```

---

### 3. Validators Module (`src/lua_extensions/validators.rs`)

#### Purpose
Validate configuration values before applying them.

#### Key Functions

```rust
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate entire configuration
    pub fn validate_config(config: &LuaValue) -> anyhow::Result<()> {
        match config {
            LuaValue::Table(table) => Self::validate_table(table),
            _ => Err(anyhow!("Configuration must be a table")),
        }
    }
    
    /// Validate specific setting
    pub fn validate_setting(key: &str, value: &LuaValue) -> anyhow::Result<()> {
        match key {
            "gaps" => Self::validate_gaps(value),
            "border_width" => Self::validate_border_width(value),
            "animations.duration" => Self::validate_duration(value),
            "input.keyboard.repeat_delay" => Self::validate_repeat_delay(value),
            // ... more validators
            _ => Err(anyhow!("Unknown setting: {}", key)),
        }
    }
    
    // Specific validators
    fn validate_gaps(value: &LuaValue) -> anyhow::Result<()> {
        match value {
            LuaValue::Integer(n) => {
                if *n < 0 || *n > 100 {
                    return Err(anyhow!("gaps must be between 0 and 100, got {}", n));
                }
                Ok(())
            }
            _ => Err(anyhow!("gaps must be an integer")),
        }
    }
    
    fn validate_border_width(value: &LuaValue) -> anyhow::Result<()> {
        match value {
            LuaValue::Integer(n) => {
                if *n < 0 || *n > 20 {
                    return Err(anyhow!("border_width must be between 0 and 20"));
                }
                Ok(())
            }
            _ => Err(anyhow!("border_width must be an integer")),
        }
    }
    
    fn validate_duration(value: &LuaValue) -> anyhow::Result<()> {
        match value {
            LuaValue::Integer(n) => {
                if *n <= 0 || *n > 5000 {
                    return Err(anyhow!("Duration must be between 1 and 5000 ms"));
                }
                Ok(())
            }
            _ => Err(anyhow!("Duration must be an integer")),
        }
    }
    
    fn validate_repeat_delay(value: &LuaValue) -> anyhow::Result<()> {
        match value {
            LuaValue::Integer(n) => {
                if *n < 25 || *n > 2000 {
                    return Err(anyhow!("repeat_delay must be between 25 and 2000 ms"));
                }
                Ok(())
            }
            _ => Err(anyhow!("repeat_delay must be an integer")),
        }
    }
}
```

#### Validation Error Messages

```
Examples of clear error messages:

"gaps must be between 0 and 100, got 150"
"border_width must be an integer, got string 'thick'"
"Unknown animation curve 'invalid_curve', valid curves are: linear, ease_in_out_cubic, ease_out_cubic"
"Keyboard xkb_layout 'xyz' is not a valid X11 keyboard layout"
"Monitor scale must be between 0.5 and 4.0, got 5.0"
"Window rule filter must have at least one condition (match_app_id, match_title, etc.)"
```

---

## Integration with Existing Code

### Changes to `src/lua_extensions/config_converter.rs`

```rust
// Enhance apply_lua_config to use new config API
pub fn apply_lua_config(runtime: &LuaRuntime, config: &mut Config) -> anyhow::Result<()> {
    // Try to apply via new API first
    if let Ok(new_config) = runtime.get_lua_config() {
        merge_configs(config, &new_config)?;
        return Ok(());
    }
    
    // Fallback to old method for backward compatibility
    apply_legacy_lua_config(runtime, config)?;
    Ok(())
}
```

### Changes to `src/main.rs`

```rust
// Initialize config API after loading Lua
let mut config = load_kdl_config(&config_path)?;
load_lua_config(&mut config, &config_path)?;

// New config is now fully initialized with both KDL and Lua settings
```

---

## Configuration Parity Checklist

**Animation Settings:**
- [ ] Output open duration and curve
- [ ] Close animation duration and curve
- [ ] Window center animation
- [ ] Column transitions
- [ ] Focus ring animation

**Input Settings:**
- [ ] Keyboard repeat delay and rate
- [ ] XKB layout, variant, options
- [ ] Mouse acceleration speed and profile
- [ ] Touchpad acceleration, tap settings

**Layout Settings:**
- [ ] Default column width percentage
- [ ] Default window height percentage
- [ ] Preset column widths
- [ ] Window rules (all existing filters)

**Gesture Settings:**
- [ ] Touchpad gesture enable/disable
- [ ] Gesture actions (swipe, pinch, etc.)
- [ ] Gesture threshold settings

**Appearance Settings:**
- [ ] Gaps and borders
- [ ] All colors (border active, inactive, focus ring)
- [ ] Font settings
- [ ] Client-side decorations preference

**Output Settings:**
- [ ] Per-monitor scale
- [ ] Per-monitor refresh rate
- [ ] Per-monitor position
- [ ] Per-monitor rotation

---

## File Structure Summary

**New Files:**
- `src/lua_extensions/config_api.rs` (300 lines)
- `src/lua_extensions/lua_types.rs` (150 lines)
- `src/lua_extensions/validators.rs` (100 lines)

**Modified Files:**
- `src/lua_extensions/mod.rs` (+10 lines)
- `src/lua_extensions/config_converter.rs` (+30 lines)
- `src/main.rs` (+10 lines)

**Documentation:**
- `docs/LUA_TIER2_SPEC.md` (this file)
- `docs/LUA_CONFIG_REFERENCE.md` (API reference)
- `docs/LUA_CONFIG_MIGRATION.md` (KDL to Lua guide)

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_animation_config_get() {
    // Test reading animation settings
}

#[test]
fn test_animation_config_set() {
    // Test setting animation values with validation
}

#[test]
fn test_input_config_validation() {
    // Test keyboard repeat rate validation
}

#[test]
fn test_window_rule_filter() {
    // Test filter matching logic
}

#[test]
fn test_validator_gaps() {
    // Test gaps validation
}

#[test]
fn test_invalid_color_format() {
    // Test color format validation
}
```

### Integration Tests

```rust
#[test]
fn test_full_config_migration() {
    // Migrate example KDL config to Lua
    // Verify all settings preserved
}

#[test]
fn test_mixed_kdl_lua_config() {
    // Load KDL config, then override with Lua
    // Verify Lua overrides KDL
}
```

---

## Success Criteria

✅ All Niri settings configurable from Lua  
✅ Type validation catches errors early  
✅ Clear error messages for invalid settings  
✅ KDL and Lua configs can be mixed  
✅ Migration guide helps users transition  
✅ All tests passing  
✅ Performance: Config loading < 100ms  
✅ No breaking changes to existing configs  

---

## References

- [Niri Config Structure](../../niri-config/src/lib.rs)
- [Lua Table Conventions](https://www.lua.org/manual/5.2/)
- [mlua UserData](https://docs.rs/mlua/latest/mlua/trait.UserData.html)
- [Regex Crate](https://docs.rs/regex/latest/regex/)
