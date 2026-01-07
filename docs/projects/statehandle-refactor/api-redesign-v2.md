# Niri Lua API Refactor Specification v2.0

**Status:** Draft  
**Created:** 2026-01-07  
**Previous Version:** v1.0 (Serde Migration)  
**Dependency:** Lua Cleanup Phase 2 Complete

## Executive Summary

Building on Phase 2's serde migration success, this specification redesigns niri-lua's configuration API to eliminate the complex Bind serde problem while providing a more intuitive, Neovim-inspired interface. The result will be a dramatic complexity reduction while maintaining full functionality and improving user experience.

### Strategic Analysis: Why Current Architecture Creates Problems

The current property-based access pattern (`niri.config.animations.off = true`) requires users to understand Niri's internal config structure. This leads to:

1. **Discoverability Gap**: Users must read source code or have external documentation to learn valid paths
2. **Complexity Explosion**: `Bind` contains 50+ Action variants, requiring massive serde implementations  
3. **Poor User Experience**: Simple intent requires complex nested objects instead of clear commands
4. **Maintenance Burden**: Any config struct change requires updating 3-4 layers (serde, extractor, registry)

### Neovim API Design Patterns: Proven Solutions

| Pattern | Current Problem | Neovim Solution | Result |
|---------|----------------|------------------|---------|
| **Configuration Access** | `config.field.subfield = value` | `vim.keymap.set('n', '<leader>w')` | Simple string commands |
| **Complex Actions** | `action = {CloseWindow = {params}}` | `vim.keymap.set('n', '<leader>q')` | High-level intent functions |
| **Rule Systems** | Complex matcher structures | `autocmd BufWritePre` | Event-driven, clear intent |
| **Collection Management** | Manual serde parsing | `vim.lsp.buf_format()` | Direct API functions |

### Target Architecture: Namespace-Based Intent Commands

```
niri.window.focus("right")        -- Clear intent
niri.workspace.switch("2")          -- High-level operation  
niri.layout.set_preset("tall")       -- User-friendly naming
niri.rule.add_class("terminal", {floating = true})  -- Pattern-based
niri.anim.set_preset("fade", 300)  -- Named presets
```

## Phase 3A: Core Command API (Week 1-2)

### 3.1 Window Operations Namespace

**Scope**: Basic window management commands

```rust
pub fn register_window_api(lua: &Lua) -> mlua::Result<()> {
    let commands = lua.create_table();
    
    // Focus commands
    commands.set("focus", lua.create_function(|direction: String| {
        match direction.as_str() {
            "left" => niri_api::focus_left(),
            "right" => niri_api::focus_right(),
            "up" => niri_api::focus_up(),
            "down" => niri_api::focus_down(),
            "next" => niri_api::focus_next(),
            "prev" => niri_api::focus_prev(),
            _ => Err(LuaError::external("Invalid direction")),
        }
    })?)?;
    
    // State commands
    commands.set("fullscreen", lua.create_function(|()| {
        niri_api::toggle_fullscreen()
    })?)?;
    
    commands.set("minimize", lua.create_function(|()| {
        niri_api::minimize_window()
    })?)?;
    
    commands.set("maximize", lua.create_function(|()| {
        niri_api::maximize_window()
    })?)?;
    
    commands.set("close", lua.create_function(|()| {
        niri_api::close_window()
    })?)?;
    
    // Movement commands
    commands.set("move_to_workspace", lua.create_function(|workspace_id: String| {
        let workspace_id = workspace_id.parse::<i32>()
            .map_err(|_| LuaError::external("Invalid workspace id"))?;
        niri_api::move_window_to_workspace_id(workspace_id)
    })?)?;
    
    lua.globals().set("niri.window", commands)?;
}
```

### 3.2 Workspace Operations Namespace

**Scope**: Workspace management commands

```rust
pub fn register_workspace_api(lua: &Lua) -> mlua::Result<()> {
    let commands = lua.create_table();
    
    commands.set("switch", lua.create_function(|workspace_id: String| {
        let workspace_id = workspace_id.parse::<i32>()
            .map_err(|_| LuaError::external("Invalid workspace id"))?;
        niri_api::switch_to_workspace_id(workspace_id)
    })?)?;
    
    commands.set("add", lua.create_function(|()| {
        niri_api::add_workspace()
    })?)?;
    
    commands.set("remove", lua.create_function(|workspace_id: String| {
        let workspace_id = workspace_id.parse::<i32>()
            .map_err(|_| LuaError::external("Invalid workspace id"))?;
        niri_api::remove_workspace_id(workspace_id)
    })?)?;
    
    commands.set("list", lua.create_function(|()| {
        let workspaces = niri_api::list_workspaces();
        lua.to_value(&workspaces)
    })?)?;
    
    lua.globals().set("niri.workspace", commands)?;
}
```

### 3.3 Layout Operations Namespace

**Scope**: Layout management commands and presets

```rust
pub fn register_layout_api(lua: &Lua) -> mlua::Result<()> {
    let commands = lua.create_table();
    
    // Preset management
    commands.set("set_preset", lua.create_function(|name: String| {
        niri_api::set_layout_preset(name)
    })?)?;
    
    commands.set("get_preset", lua.create_function(|()| {
        let preset = niri_api::get_layout_preset();
        lua.to_value(&preset)
    })?)?;
    
    // Focus ring configuration
    commands.set("focus_ring_width", lua.create_function(|width: i32| {
        niri_api::set_focus_ring_width(width as u32)
    })?)?;
    
    commands.set("focus_ring_active_color", lua.create_function(|color: String| {
        let color = parse_color_string(&color)?;
        niri_api::set_focus_ring_active_color(color)
    })?)?;
    
    commands.set("border_width", lua.create_function(|width: i32| {
        niri_api::set_border_width(width as u32)
    })?)?;
    
    lua.globals().set("niri.layout", commands)?;
}
```

### 3.4 Animation Operations Namespace

**Scope**: Animation presets and management

```rust
pub fn register_animation_api(lua: &Lua) -> mlua::Result<()> {
    let commands = lua.create_table();
    
    // Preset management
    commands.set("set_preset", lua.create_function(|name: String| {
        niri_api::set_animation_preset(name)
    })?)?;
    
    commands.set("get_preset", lua.create_function(|()| {
        let preset = niri_api::get_animation_preset();
        lua.to_value(&preset)
    })?)?;
    
    // Create custom animations
    commands.set("create", lua.create_function(|name: String, duration_ms: i64, easing: String| {
        let duration = Duration::from_millis(duration_ms);
        let easing = parse_easing(&easing)?;
        niri_api::create_animation(name, duration, easing)
    })?)?;
    
    commands.set("stop", lua.create_function(|()| {
        niri_api::stop_animations()
    })?)?;
    
    lua.globals().set("niri.animation", commands)?;
}
```

### 3.5 Bind Management Namespace

**Scope**: Advanced keybinding operations (maintained for power users)

```rust
pub fn register_bind_api(lua: &Lua) -> mlua::Result<()> {
    let commands = lua.create_table();
    
    // Simple bind management
    commands.set("add", lua.create_function(|mods: Vec<String>, key: String, action: String| {
        let action = parse_action_string(&action)?;
        niri_api::add_bind(mods, key, action)
    })?)?;
    
    commands.set("remove", lua.create_function(|mods: Vec<String>, key: String| {
        niri_api::remove_bind(mods, key)
    })?)?;
    
    commands.set("list", lua.create_function(|()| {
        let binds = niri_api::list_binds();
        lua.to_value(&binds)
    })?)?;
    
    // Advanced bind search (for power users)
    commands.set("find", lua.create_function(|criteria: LuaTable| {
        let criteria = extract_bind_criteria(&criteria)?;
        let results = niri_api::find_binds(criteria);
        lua.to_value(&results)
    })?)?;
    
    lua.globals().set("niri.bind", commands)?;
}
```

## Phase 3B: Rule-Based Configuration (Week 2-3)

### 3.1 Window Rule Templates

**Scope**: Simplified window rule creation using patterns vs complex structs

```rust
pub fn register_rule_api(lua: &Lua) -> mlua::Result<()> {
    let commands = lua.create_table();
    
    // Predefined templates
    commands.set("floating_terminal", lua.create_function(|()| {
        let rule = WindowRule::floating_terminal();
        niri_api::add_window_rule(rule)
    })?)?;
    
    commands.set("steam_games", lua.create_function(|()| {
        let rule = WindowRule::steam_games();
        niri_api::add_window_rule(rule)
    })?)?;
    
    commands.set("mpv", lua.create_function(|()| {
        let rule = WindowRule::mpv();
        niri_api::add_window_rule(rule)
    })?)?;
    
    // Pattern-based creation
    commands.set("add_class", lua.create_function(|class: String, properties: LuaTable| {
        let rule = WindowRule::class_match(class, properties)?;
        niri_api::add_window_rule(rule)
    })?)?;
    
    commands.set("add_app", lua.create_function(|app_id: String, properties: LuaTable| {
        let rule = WindowRule::app_match(app_id, properties)?;
        niri_api::add_window_rule(rule)
    })?)?;
    
    lua.globals().set("niri.rule", commands)?;
}
```

## Phase 3C: Legacy Support and Migration

### 3.1 Backward Compatibility Layer

**Scope**: Maintain property access during transition period

```rust
// Transitional registry that supports both old and new APIs
pub fn register_transitional_apis(lua: &Lua) -> mlua::Result<()> {
    // Register new command APIs
    register_window_api(lua)?;
    register_workspace_api(lua)?;
    register_layout_api(lua)?;
    register_animation_api(lua)?;
    register_bind_api(lua)?;
    
    // Keep property accessors with deprecation warnings
    register_config_accessors(lua)?;
    
    // Add migration helpers
    let helpers = lua.create_table();
    helpers.set("migrate_properties", lua.create_function(|()| {
        let deprecated = get_deprecated_property_usage();
        if !deprecated.is_empty() {
            let suggestions = suggest_new_api_calls(&deprecated);
            lua.create_table().set_from_pairs(suggestions)?;
        }
        Ok(())
    })?)?;
    
    lua.globals().set("niri.helpers", helpers)?;
}
```

## Phase 3D: Advanced Patterns

### 3.1 Context-Aware Configuration

**Scope**: Operations that adapt based on current state

```rust
pub fn register_context_apis(lua: &Lua) -> mlua::Result<()> {
    let commands = lua.create_table();
    
    commands.set("config_save", lua.create_function(|name: String| {
        let state = niri_api::get_current_state();
        let config = niri_api::export_config(&state);
        std::fs::write(format!("niri-configs/{}.lua", name), config)?;
        Ok(())
    })?)?;
    
    commands.set("config_load", lua.create_function(|name: String| {
        let config = std::fs::read_to_string(format!("niri-configs/{}.lua", name))?;
        niri_api::import_config(&config)?;
        Ok(())
    })?)?;
    
    lua.globals().set("niri.config", commands)?;
}
```

## API Integration and Backwards Compatibility

### Migration Path

1. **Phase 3A Implementation** (Weeks 1-2): Deploy new command APIs
2. **Phase 3B Implementation** (Weeks 2-3): Deploy rule-based system  
3. **Phase 3C Implementation** (Week 3): Context-aware operations
4. **Phase 4** (Weeks 4-5): Full migration to command APIs
5. **Phase 5** (Week 6): Deprecate property APIs with migration helpers
6. **Phase 6** (Week 7): Remove legacy systems

### Success Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| **API Reduction** | 70% | Count unique exposed functions |
| **Complexity Reduction** | 60% | Cyclomatic complexity analysis |
| **Discoverability** | Qualitative | User testing for findability |
| **User Satisfaction** | Qualitative | Community feedback |
| **Maintenance Burden** | 50% | LOC comparison over time |

### Risk Mitigation

1. **Incremental Deployment**: Each phase deployed independently with rollback capability
2. **Extensive Testing**: Command equivalence tests between old and new APIs
3. **Documentation**: Migration guides for advanced users
4. **Community Feedback**: Early access to niri community for input
5. **Legacy Support**: Extended support period (2+ releases) for property API

### User Experience Improvements

```lua
-- Old approach (complex, requires internal knowledge)
niri.config.binds:add({mods = {"Mod4"}, action = {CloseWindow = {}}})

-- New approach (intuitive, discoverable)
niri.bind.add({"mods": "Mod4", "key": "Return", "action": "close_window"})

-- Even simpler (discoverable)
niri.bind.add("Mod4+Return", "close_window")

-- Discover available commands
:help niri.bind
```

This specification provides a complete roadmap for transforming niri-lua's configuration architecture from a complex property-based system to an intuitive, Neovim-inspired command API while maintaining full functionality and dramatically reducing implementation complexity.