# Tier 3 Specification: Runtime State Access & Introspection

**Duration:** Weeks 5-6  
**Estimated LOC:** 350-400 Rust + 150 documentation  
**Complexity:** High (State synchronization required)

## Overview

Tier 3 enables Lua scripts to **query and inspect Niri's runtime state**. Users can:
- Query all open windows with their properties
- Query workspaces and monitor layout
- Make decisions based on current state
- Implement custom window management logic
- Create status bars and monitoring tools

This tier is critical because it enables truly interactive Lua scripts that respond to Niri's current state.

---

## Architecture

```
State Query API Structure:
  niri.windows
    ├── .get_all() → [Window]           # All windows
    ├── .get_focused() → Window?        # Currently focused window
    ├── .get_on_workspace(id) → [Window]
    └── .get_by_app_id(id) → [Window]
  
  niri.workspaces
    ├── .get_all() → [Workspace]
    ├── .get_active() → Workspace?
    ├── .get_by_name(name) → Workspace?
    └── .get_by_index(idx) → Workspace?
  
  niri.monitors
    ├── .get_all() → [Monitor]
    ├── .get_active() → Monitor?
    └── .get_by_name(name) → Monitor?
  
  niri.layout
    ├── .get_current() → "tiling" | "floating"
    ├── .is_tiling_focused() → bool
    ├── .column_count() → int
    └── .get_column(idx) → ColumnInfo?

UserData Types (with methods):
  - Window { id, title, app_id, geometry, ... }
  - Workspace { id, name, monitor, windows, ... }
  - Monitor { name, scale, refresh_rate, geometry, ... }
  - ColumnInfo { width, height, index, ... }
```

---

## Detailed Specifications

### 1. Window Query API (`src/lua_extensions/window_api.rs`)

#### Purpose
Query information about open windows.

#### Window UserData Type

```rust
#[derive(Debug, Clone)]
pub struct LuaWindow {
    pub id: u64,
    pub title: String,
    pub app_id: String,
    pub workspace_id: u64,
    pub monitor_index: u32,
    pub is_floating: bool,
    pub is_fullscreen: bool,
    pub is_focused: bool,
    pub geometry: (i32, i32, i32, i32),  // x, y, width, height
    pub tile_width_percentage: f32,
}

impl mlua::UserData for LuaWindow {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        // Getters
        methods.add_method("id", |_, this, ()| Ok(this.id));
        methods.add_method("title", |_, this, ()| Ok(this.title.clone()));
        methods.add_method("app_id", |_, this, ()| Ok(this.app_id.clone()));
        
        methods.add_method("is_floating", |_, this, ()| Ok(this.is_floating));
        methods.add_method("is_fullscreen", |_, this, ()| Ok(this.is_fullscreen));
        methods.add_method("is_focused", |_, this, ()| Ok(this.is_focused));
        
        methods.add_method("workspace_id", |_, this, ()| Ok(this.workspace_id));
        methods.add_method("monitor", |_, this, ()| Ok(this.monitor_index));
        
        methods.add_method("geometry", |_, this, ()| {
            Ok((this.geometry.0, this.geometry.1, this.geometry.2, this.geometry.3))
        });
        
        methods.add_method("width", |_, this, ()| Ok(this.geometry.2));
        methods.add_method("height", |_, this, ()| Ok(this.geometry.3));
        methods.add_method("x", |_, this, ()| Ok(this.geometry.0));
        methods.add_method("y", |_, this, ()| Ok(this.geometry.1));
        
        methods.add_method("tile_width_percentage", |_, this, ()| {
            Ok(this.tile_width_percentage)
        });
        
        // String representation for debugging
        methods.add_method("__tostring", |_, this, ()| {
            Ok(format!(
                "Window(id={}, title='{}', app_id='{}', floating={}, fullscreen={})",
                this.id, this.title, this.app_id, this.is_floating, this.is_fullscreen
            ))
        });
    }
}
```

#### Window Query Functions

```rust
pub struct WindowApi;

impl WindowApi {
    /// Register to Lua
    pub fn register_to_lua(lua: &Lua, state: &State) -> LuaResult<()> {
        let windows_table = lua.create_table()?;
        
        let state_clone = Arc::new(state.clone());  // Share state reference
        
        // get_all() → [Window]
        let get_all = {
            let state = state_clone.clone();
            lua.create_function(move |lua, ()| {
                let windows = state.get_all_windows();
                let table = lua.create_table()?;
                for (i, window) in windows.iter().enumerate() {
                    table.set(i + 1, window.to_lua(lua)?)?;
                }
                Ok(table)
            })?
        };
        windows_table.set("get_all", get_all)?;
        
        // get_focused() → Window?
        let get_focused = {
            let state = state_clone.clone();
            lua.create_function(move |lua, ()| {
                if let Some(window) = state.get_focused_window() {
                    Ok(Some(window.to_lua(lua)?))
                } else {
                    Ok(None::<LuaWindow>)
                }
            })?
        };
        windows_table.set("get_focused", get_focused)?;
        
        // get_on_workspace(workspace_id) → [Window]
        let get_on_ws = {
            let state = state_clone.clone();
            lua.create_function(move |lua, ws_id: u64| {
                let windows = state.get_windows_on_workspace(ws_id);
                let table = lua.create_table()?;
                for (i, window) in windows.iter().enumerate() {
                    table.set(i + 1, window.to_lua(lua)?)?;
                }
                Ok(table)
            })?
        };
        windows_table.set("get_on_workspace", get_on_ws)?;
        
        // get_by_app_id(app_id) → [Window]
        let get_by_app = {
            let state = state_clone.clone();
            lua.create_function(move |lua, app_id: String| {
                let windows = state.get_windows_by_app_id(&app_id);
                let table = lua.create_table()?;
                for (i, window) in windows.iter().enumerate() {
                    table.set(i + 1, window.to_lua(lua)?)?;
                }
                Ok(table)
            })?
        };
        windows_table.set("get_by_app_id", get_by_app)?;
        
        let niri_table = lua.globals().get::<_, LuaTable>("niri")?;
        niri_table.set("windows", windows_table)?;
        
        Ok(())
    }
}
```

#### Example Lua Usage

```lua
-- Get all windows
local all_windows = niri.windows.get_all()
for _, window in ipairs(all_windows) do
    print(window:title() .. " (" .. window:app_id() .. ")")
end

-- Get focused window
if niri.windows.get_focused() then
    local focused = niri.windows.get_focused()
    niri.log("Focused: " .. focused:title())
end

-- Get Firefox windows
local firefox_windows = niri.windows.get_by_app_id("firefox")
for _, window in ipairs(firefox_windows) do
    if window:is_fullscreen() then
        niri.log("Firefox is fullscreen")
    end
end
```

---

### 2. Workspace Query API (`src/lua_extensions/workspace_api.rs`)

#### Purpose
Query information about workspaces.

#### Workspace UserData Type

```rust
#[derive(Debug, Clone)]
pub struct LuaWorkspace {
    pub id: u64,
    pub name: String,
    pub monitor_index: u32,
    pub is_active: bool,
    pub window_count: u32,
    pub layout_type: String,  // "tiling" or "floating"
}

impl mlua::UserData for LuaWorkspace {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| Ok(this.id));
        methods.add_method("name", |_, this, ()| Ok(this.name.clone()));
        methods.add_method("monitor_index", |_, this, ()| Ok(this.monitor_index));
        methods.add_method("is_active", |_, this, ()| Ok(this.is_active));
        methods.add_method("window_count", |_, this, ()| Ok(this.window_count));
        methods.add_method("layout_type", |_, this, ()| Ok(this.layout_type.clone()));
        
        methods.add_method("__tostring", |_, this, ()| {
            Ok(format!("Workspace(id={}, name='{}', active={})", 
                this.id, this.name, this.is_active))
        });
    }
}
```

#### Workspace Query Functions

```rust
pub struct WorkspaceApi;

impl WorkspaceApi {
    pub fn register_to_lua(lua: &Lua, state: &State) -> LuaResult<()> {
        let workspaces_table = lua.create_table()?;
        
        let state_clone = Arc::new(state.clone());
        
        // get_all() → [Workspace]
        let get_all = {
            let state = state_clone.clone();
            lua.create_function(move |lua, ()| {
                let workspaces = state.get_all_workspaces();
                let table = lua.create_table()?;
                for (i, ws) in workspaces.iter().enumerate() {
                    table.set(i + 1, ws.to_lua(lua)?)?;
                }
                Ok(table)
            })?
        };
        workspaces_table.set("get_all", get_all)?;
        
        // get_active() → Workspace?
        let get_active = {
            let state = state_clone.clone();
            lua.create_function(move |lua, ()| {
                if let Some(ws) = state.get_active_workspace() {
                    Ok(Some(ws.to_lua(lua)?))
                } else {
                    Ok(None::<LuaWorkspace>)
                }
            })?
        };
        workspaces_table.set("get_active", get_active)?;
        
        // get_by_name(name) → Workspace?
        let get_by_name = {
            let state = state_clone.clone();
            lua.create_function(move |lua, name: String| {
                if let Some(ws) = state.get_workspace_by_name(&name) {
                    Ok(Some(ws.to_lua(lua)?))
                } else {
                    Ok(None::<LuaWorkspace>)
                }
            })?
        };
        workspaces_table.set("get_by_name", get_by_name)?;
        
        // get_by_index(idx) → Workspace?
        let get_by_idx = {
            let state = state_clone.clone();
            lua.create_function(move |lua, idx: u32| {
                if let Some(ws) = state.get_workspace_by_index(idx) {
                    Ok(Some(ws.to_lua(lua)?))
                } else {
                    Ok(None::<LuaWorkspace>)
                }
            })?
        };
        workspaces_table.set("get_by_index", get_by_idx)?;
        
        let niri_table = lua.globals().get::<_, LuaTable>("niri")?;
        niri_table.set("workspaces", workspaces_table)?;
        
        Ok(())
    }
}
```

---

### 3. Monitor Query API (`src/lua_extensions/monitor_api.rs`)

#### Purpose
Query information about connected monitors.

#### Monitor UserData Type

```rust
#[derive(Debug, Clone)]
pub struct LuaMonitor {
    pub name: String,
    pub index: u32,
    pub scale: f32,
    pub refresh_rate: f32,
    pub geometry: (i32, i32, i32, i32),  // x, y, width, height
    pub is_active: bool,
}

impl mlua::UserData for LuaMonitor {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("name", |_, this, ()| Ok(this.name.clone()));
        methods.add_method("index", |_, this, ()| Ok(this.index));
        methods.add_method("scale", |_, this, ()| Ok(this.scale));
        methods.add_method("refresh_rate", |_, this, ()| Ok(this.refresh_rate));
        methods.add_method("geometry", |_, this, ()| {
            Ok((this.geometry.0, this.geometry.1, this.geometry.2, this.geometry.3))
        });
        methods.add_method("width", |_, this, ()| Ok(this.geometry.2));
        methods.add_method("height", |_, this, ()| Ok(this.geometry.3));
        methods.add_method("is_active", |_, this, ()| Ok(this.is_active));
    }
}
```

#### Monitor Query Functions

```rust
pub struct MonitorApi;

impl MonitorApi {
    pub fn register_to_lua(lua: &Lua, state: &State) -> LuaResult<()> {
        let monitors_table = lua.create_table()?;
        
        let state_clone = Arc::new(state.clone());
        
        // get_all() → [Monitor]
        let get_all = {
            let state = state_clone.clone();
            lua.create_function(move |lua, ()| {
                let monitors = state.get_all_monitors();
                let table = lua.create_table()?;
                for (i, monitor) in monitors.iter().enumerate() {
                    table.set(i + 1, monitor.to_lua(lua)?)?;
                }
                Ok(table)
            })?
        };
        monitors_table.set("get_all", get_all)?;
        
        // get_active() → Monitor?
        let get_active = {
            let state = state_clone.clone();
            lua.create_function(move |lua, ()| {
                if let Some(monitor) = state.get_active_monitor() {
                    Ok(Some(monitor.to_lua(lua)?))
                } else {
                    Ok(None::<LuaMonitor>)
                }
            })?
        };
        monitors_table.set("get_active", get_active)?;
        
        // get_by_name(name) → Monitor?
        let get_by_name = {
            let state = state_clone.clone();
            lua.create_function(move |lua, name: String| {
                if let Some(monitor) = state.get_monitor_by_name(&name) {
                    Ok(Some(monitor.to_lua(lua)?))
                } else {
                    Ok(None::<LuaMonitor>)
                }
            })?
        };
        monitors_table.set("get_by_name", get_by_name)?;
        
        let niri_table = lua.globals().get::<_, LuaTable>("niri")?;
        niri_table.set("monitors", monitors_table)?;
        
        Ok(())
    }
}
```

---

### 4. Layout Query API (`src/lua_extensions/layout_query_api.rs`)

#### Purpose
Query layout state and column information.

#### ColumnInfo UserData Type

```rust
#[derive(Debug, Clone)]
pub struct LuaColumnInfo {
    pub index: u32,
    pub width_percentage: f32,
    pub height_percentage: f32,
    pub window_count: u32,
}

impl mlua::UserData for LuaColumnInfo {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("index", |_, this, ()| Ok(this.index));
        methods.add_method("width_percentage", |_, this, ()| Ok(this.width_percentage));
        methods.add_method("height_percentage", |_, this, ()| Ok(this.height_percentage));
        methods.add_method("window_count", |_, this, ()| Ok(this.window_count));
    }
}
```

#### Layout Query Functions

```rust
pub struct LayoutQueryApi;

impl LayoutQueryApi {
    pub fn register_to_lua(lua: &Lua, state: &State) -> LuaResult<()> {
        let layout_table = lua.create_table()?;
        
        let state_clone = Arc::new(state.clone());
        
        // get_current() → "tiling" | "floating"
        let get_current = {
            let state = state_clone.clone();
            lua.create_function(move |_, ()| {
                let layout = state.get_current_layout();
                Ok(layout.to_string())  // Returns "tiling" or "floating"
            })?
        };
        layout_table.set("get_current", get_current)?;
        
        // is_tiling_focused() → bool
        let is_tiling_focused = {
            let state = state_clone.clone();
            lua.create_function(move |_, ()| {
                Ok(state.is_tiling_focused())
            })?
        };
        layout_table.set("is_tiling_focused", is_tiling_focused)?;
        
        // column_count() → int
        let column_count = {
            let state = state_clone.clone();
            lua.create_function(move |_, ()| {
                Ok(state.get_column_count() as u32)
            })?
        };
        layout_table.set("column_count", column_count)?;
        
        // get_column(index) → ColumnInfo?
        let get_column = {
            let state = state_clone.clone();
            lua.create_function(move |lua, idx: u32| {
                if let Some(col) = state.get_column(idx) {
                    Ok(Some(col.to_lua(lua)?))
                } else {
                    Ok(None::<LuaColumnInfo>)
                }
            })?
        };
        layout_table.set("get_column", get_column)?;
        
        let niri_table = lua.globals().get::<_, LuaTable>("niri")?;
        niri_table.set("layout", layout_table)?;
        
        Ok(())
    }
}
```

---

## Integration with Existing Code

### Changes to `src/niri.rs` (State struct)

```rust
impl State {
    // New methods for Lua to query state
    pub fn get_all_windows(&self) -> Vec<LuaWindow> {
        // Convert internal Window structs to LuaWindow
    }
    
    pub fn get_focused_window(&self) -> Option<LuaWindow> {
        // Return currently focused window
    }
    
    pub fn get_windows_on_workspace(&self, ws_id: u64) -> Vec<LuaWindow> {
        // Return windows on specific workspace
    }
    
    pub fn get_windows_by_app_id(&self, app_id: &str) -> Vec<LuaWindow> {
        // Return windows matching app_id
    }
    
    // Similar for workspaces, monitors, layout...
}
```

### Changes to `src/lua_extensions/runtime.rs`

```rust
impl LuaRuntime {
    pub fn register_state_apis(&self, state: &State) -> LuaResult<()> {
        let lua = &self.lua;
        
        WindowApi::register_to_lua(lua, state)?;
        WorkspaceApi::register_to_lua(lua, state)?;
        MonitorApi::register_to_lua(lua, state)?;
        LayoutQueryApi::register_to_lua(lua, state)?;
        
        Ok(())
    }
}
```

---

## Example Use Cases

### Status Bar Implementation

```lua
-- Display window list and workspace status
function update_status()
    local active_ws = niri.workspaces.get_active()
    local all_windows = niri.windows.get_all()
    
    print("Workspace: " .. active_ws:name())
    print("Windows on this workspace: " .. active_ws:window_count())
    
    for _, window in ipairs(niri.windows.get_on_workspace(active_ws:id())) do
        if window:is_focused() then
            print("  * " .. window:title())  -- Bold indicator
        else
            print("  - " .. window:title())
        end
    end
end
```

### Custom Window Manager

```lua
-- Prevent Firefox from floating
niri.on("window:open", function(window)
    if window:app_id() == "firefox" then
        niri.command("tile-window")  -- (implemented in Tier 4)
    end
end)
```

### Monitor-Aware Layout

```lua
-- Auto-adjust layout based on monitor count
local monitors = niri.monitors.get_all()
if #monitors == 1 then
    niri.config.layout.default_width_percent = 100
else
    niri.config.layout.default_width_percent = 70
end
```

---

## File Structure Summary

**New Files:**
- `src/lua_extensions/window_api.rs` (100 lines)
- `src/lua_extensions/workspace_api.rs` (100 lines)
- `src/lua_extensions/monitor_api.rs` (100 lines)
- `src/lua_extensions/layout_query_api.rs` (50 lines)

**Modified Files:**
- `src/lua_extensions/mod.rs` (+15 lines)
- `src/lua_extensions/runtime.rs` (+30 lines)
- `src/niri.rs` (+80 lines of query methods)

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_get_all_windows() {
    // Mock state with test windows
}

#[test]
fn test_window_properties() {
    // Verify window properties are correctly converted to Lua
}

#[test]
fn test_workspace_queries() {
    // Test workspace query functions
}

#[test]
fn test_monitor_queries() {
    // Test monitor information retrieval
}
```

---

## Success Criteria

✅ All window properties queryable from Lua  
✅ Workspace and monitor queries working  
✅ Layout state inspection works  
✅ State conversions are performant (< 1ms)  
✅ All tests passing  
✅ No race conditions accessing State  

---

## References

- [Niri State Structure](../../src/niri.rs)
- [Window Types](../../src/window/mod.rs)
- [Workspace Types](../../src/layout/mod.rs)
