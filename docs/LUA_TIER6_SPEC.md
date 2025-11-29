# Tier 6 Specification: Developer Experience

**Status:** ⚙️ **PARTIAL IMPLEMENTATION**

**Duration:** Weeks 11-12 | **Estimated LOC:** 1300 (code + docs + examples)

This tier focuses on making Niri's Lua API developer-friendly through type definitions, LSP support, comprehensive documentation, and tooling.

## Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Interactive REPL | ✅ Complete | `niri-lua/src/ipc_repl.rs`, 86 integration tests |
| Documentation | ✅ Complete | LUA_GUIDE.md, LUA_QUICKSTART.md, LUA_EMBEDDING.md, etc. |
| Example Scripts | ✅ Complete | 10 examples in `examples/` directory |
| Type Definitions | ⏳ Pending | Should use EmmyLua annotations (see note below) |
| LSP Support | ⏳ Pending | lua_ls configuration not yet provided |
| Testing Framework | ⏳ Pending | Plugin testing infrastructure not implemented |

### Code References (Implemented Components)

**IPC REPL** (`niri-lua/src/ipc_repl.rs`, ~120 lines):
- `IpcLuaExecutor` struct: `:24` - Handler for executing Lua code from IPC requests
- `IpcLuaExecutor::new()`: `:34` - Creates executor with `Arc<Mutex<Option<LuaRuntime>>>`
- `IpcLuaExecutor::execute()`: `:49` - Executes Lua code string, returns `(output, success)`
- Unit tests: `:60+` - `test_lua_executor_basic`, `test_lua_executor_print`, `test_lua_executor_error`

**Integration Tests** (`niri-lua/tests/repl_integration.rs`):
- 86 REPL integration tests covering runtime API, config API, and error handling

**Documentation** (`docs/`):
- `LUA_GUIDE.md` - Comprehensive user guide
- `LUA_QUICKSTART.md` - Quick start tutorial
- `LUA_EMBEDDING.md` - Embedding Lua in niri
- `LUA_REPL.md` - REPL usage documentation
- `LUA_EVENT_HOOKS.md` - Event system documentation
- `LUA_RUNTIME_STATE_API.md` - Runtime state API reference
- `LUA_CONFIG_STATUS.md` - Configuration status tracking
- `LUA_FILES_CHECKLIST.md` - Implementation file checklist

**Example Scripts** (`examples/`):
- `niri.lua` - Main example configuration
- `config_api_demo.lua`, `config_api_dump.lua`, `config_api_usage.lua` - Config API examples
- `runtime_state_api_demo.lua`, `runtime_state_query.lua` - Runtime API examples
- `query_windows.lua`, `query_workspaces.lua` - Query examples
- `event_system_demo.lua` - Event system example
- `config_recent_windows.lua` - Recent windows tracking example

> **Important Correction:** This spec originally proposed "Luau type definitions" with Luau's `declare module` syntax. Since niri uses **Lua 5.2 with LuaJIT** (not Luau), type definitions should use **EmmyLua annotations** instead - the same format used by Neovim and supported by lua_ls (lua-language-server).
>
> - **Luau syntax (incorrect for niri):** `declare module "niri" do function log(msg: string): nil end`
> - **EmmyLua syntax (correct):** `---@param msg string` / `---@return nil` / `---@class NiriConfig`

---

## Overview

Tier 6 bridges the gap between implementation and usability by providing:

1. **Type Definitions** - EmmyLua annotation stubs for IDE autocomplete and type checking (Lua 5.2/LuaJIT)
2. **LSP Support** - Language Server Protocol integration for Neovim/VS Code via lua_ls
3. **Documentation** - User guides, quick start, architecture docs
4. **Example Plugins** - 5+ real-world plugins with full source code
5. **Testing Framework** - Plugin testing infrastructure
6. **Interactive REPL** - Console for experimenting with Niri Lua API

---

## 1. Type Definitions (Luau)

### 1.1 Module Structure

Create `docs/niri.d.lua` with complete Luau type definitions:

```lua
-- docs/niri.d.lua
-- Niri Lua API Type Definitions (Luau)
-- 
-- This file provides type hints for IDE autocomplete and type checking.
-- Use with:
-- - Luau type checker
-- - VS Code (with Lua extension)
-- - Neovim (with lua_ls or emmylua)
-- - Zed, IntelliJ IDEA

declare module "niri" do
  -- ========== Logging ==========
  
  type LogLevel = "debug" | "info" | "warn" | "error"
  
  function log(msg: string, level: LogLevel?): nil
  function debug(msg: string): nil
  function info(msg: string): nil
  function warn(msg: string): nil
  function error(msg: string): nil
  
  -- ========== Version Info ==========
  
  type VersionInfo = {
    major: number,
    minor: number,
    patch: number,
    git_hash: string?,
    is_debug: boolean,
  }
  
  function version(): VersionInfo
  function version_string(): string
  
  -- ========== Process Management ==========
  
  type SpawnOptions = {
    cwd: string?,
    env: { [string]: string }?,
    stdin: boolean?,
    stdout: boolean?,
    stderr: boolean?,
  }
  
  function spawn(cmd: string, opts: SpawnOptions?): nil
  function spawn_blocking(cmd: string, opts: SpawnOptions?): number
  
  -- ========== Configuration API ==========
  
  namespace config do
    type AnimationCurve = "linear" | "ease_out_cubic" | "ease_out_back"
    
    type AnimationConfig = {
      window_open: {
        curve: AnimationCurve,
        duration_ms: number,
      },
      window_close: {
        curve: AnimationCurve,
        duration_ms: number,
      },
      window_movement: {
        curve: AnimationCurve,
        duration_ms: number,
      },
      window_resize: {
        curve: AnimationCurve,
        duration_ms: number,
      },
    }
    
    function get_animations(): AnimationConfig
    function set_animations(config: Partial<AnimationConfig>): boolean
    
    type InputConfig = {
      keyboard: {
        repeat_delay: number,
        repeat_rate: number,
        xkb_model: string,
        xkb_layout: string,
        xkb_variant: string?,
        xkb_options: string?,
      },
      mouse: {
        accel: { enabled: boolean, speed: number },
        natural_scroll: boolean,
      },
      touchpad: {
        accel: { enabled: boolean, speed: number },
        natural_scroll: boolean,
        tap_to_click: boolean,
      },
    }
    
    function get_input(): InputConfig
    function set_input(config: Partial<InputConfig>): boolean
    
    type LayoutConfig = {
      preset: "vertical" | "horizontal" | "paper",
      gaps: number,
      struts: { top: number, bottom: number, left: number, right: number },
    }
    
    function get_layout(): LayoutConfig
    function set_layout(config: Partial<LayoutConfig>): boolean
    
    function get_keybinds(): { [string]: string }
    function set_keybind(key_combo: string, action: string): boolean
    function remove_keybind(key_combo: string): boolean
    
    type AppearanceConfig = {
      border: {
        width: number,
        active_color: string,
        inactive_color: string,
        active_gradient_angle: number?,
        inactive_gradient_angle: number?,
      },
      background_image: string?,
      background_blur: number,
    }
    
    function get_appearance(): AppearanceConfig
    function set_appearance(config: Partial<AppearanceConfig>): boolean
  end
  
  -- ========== State Queries ==========
  
  namespace state do
    type Rect = {
      x: number,
      y: number,
      width: number,
      height: number,
    }
    
    type Window = {
      id: number,
      title: string,
      app_id: string,
      is_floating: boolean,
      workspace_id: number?,
      monitor_index: number?,
      bounds: Rect,
      focus_ring: Rect?,
    }
    
    type Workspace = {
      id: number,
      name: string,
      index: number,
      monitor_index: number,
      window_count: number,
      layout_mode: "tiling" | "floating",
    }
    
    type Monitor = {
      index: number,
      name: string,
      make: string,
      model: string,
      serial: string?,
      physical_size: { width: number, height: number }?,
      current_scale: number,
      preferred_scale: number,
      refresh_rate: number,
      layout: { x: number, y: number, width: number, height: number },
    }
    
    function windows(): Window[]
    function windows_on_workspace(ws_id: number): Window[]
    function window_by_id(id: number): Window?
    function window_by_app_id(app_id: string): Window?
    function active_window(): Window?
    
    function workspaces(): Workspace[]
    function workspace_by_name(name: string): Workspace?
    function workspace_by_id(id: number): Workspace?
    function active_workspace(): Workspace?
    
    function monitors(): Monitor[]
    function monitor_at_index(index: number): Monitor?
    function active_monitor(): Monitor?
    
    function screenshot(monitor_index: number?, callback: ((data: string) -> nil)): nil
  end
  
  -- ========== Event System ==========
  
  namespace events do
    type WindowEvent = {
      type: "open" | "close" | "focus" | "title_changed" | "app_id_changed",
      window: state.Window,
      old_focus: state.Window?,
    }
    
    type WorkspaceEvent = {
      type: "enter" | "leave" | "layout_changed",
      workspace: state.Workspace,
      old_workspace: state.Workspace?,
    }
    
    type MonitorEvent = {
      type: "connect" | "disconnect",
      monitor: state.Monitor,
    }
    
    type LayoutEvent = {
      type: "changed",
      new_layout: "tiling" | "floating",
    }
    
    type ActionEvent = {
      type: "executed",
      action: string,
      arg: string?,
    }
    
    type EventData =
      WindowEvent
      | WorkspaceEvent
      | MonitorEvent
      | LayoutEvent
      | ActionEvent
    
    type EventHandler = (event: EventData) -> ()
    
    function on(event_name: string, handler: EventHandler): number  -- Returns handler ID
    function once(event_name: string, handler: EventHandler): number
    function off(event_name: string, handler_id: number): boolean
    function emit(event_name: string, data: EventData): nil  -- For testing
  end
  
  -- ========== Plugin System ==========
  
  namespace plugins do
    type PluginMetadata = {
      name: string,
      version: string,
      author: string,
      description: string,
      license: string?,
      repository: string?,
      dependencies: { [number]: string },
      exports: { [string]: any },
    }
    
    type PluginInfo = {
      name: string,
      version: string,
      enabled: boolean,
      loaded: boolean,
      metadata_path: string,
      data_path: string,
    }
    
    function get_metadata(): PluginMetadata
    function get_plugin_info(name: string): PluginInfo?
    function list_plugins(): PluginInfo[]
    function enable_plugin(name: string): boolean
    function disable_plugin(name: string): boolean
    function install_plugin(url: string, opts: { force: boolean }?): boolean
    function uninstall_plugin(name: string): boolean
    function reload_plugin(name: string): boolean
    
    function get_plugin_state(name: string): { [string]: any }?
    function set_plugin_state(name: string, state: { [string]: any }): boolean
  end
  
  -- ========== Layout Operations ==========
  
  namespace layout do
    type LayoutCommand =
      "focus_left"
      | "focus_right"
      | "focus_up"
      | "focus_down"
      | "focus_first"
      | "focus_last"
      | "move_left"
      | "move_right"
      | "move_up"
      | "move_down"
      | "focus_column_left"
      | "focus_column_right"
      | "move_column_left"
      | "move_column_right"
      | "set_column_width"
      | "set_window_height"
      | "reset_window_height"
      | "toggle_fullscreen"
      | "toggle_floating"
    
    function execute(cmd: LayoutCommand, arg: string?): boolean
  end
  
  -- ========== Window Operations ==========
  
  namespace window do
    function close(window_id: number?): boolean
    function focus(window_id: number): boolean
    function move_to_workspace(window_id: number, ws_id: number): boolean
    function set_floating(window_id: number, floating: boolean): boolean
    function set_fullscreen(window_id: number, fullscreen: boolean): boolean
  end
  
  -- ========== Workspace Operations ==========
  
  namespace workspace do
    function activate(ws_id: number | string): boolean
    function activate_prev(): boolean
    function activate_next(): boolean
    function create(name: string): number?
    function delete(ws_id: number): boolean
    function rename(ws_id: number, name: string): boolean
  end
end

export type = module
```

### 1.2 Per-Module Type Files (Optional)

For better IDE organization, split into sub-modules:

```lua
-- docs/types/window.d.lua
declare global do
  type Window = {
    id: number,
    title: string,
    app_id: string,
    is_floating: boolean,
    bounds: { x: number, y: number, width: number, height: number },
  }
end
```

---

## 2. LSP Support

### 2.1 LSP Stub Generation

Create `tools/generate-lsp-stubs.rs` to generate Luau stubs for language servers:

```rust
// tools/generate-lsp-stubs.rs

/// Generate Luau definition files from Rust API
/// Usage: cargo run --bin generate-lsp-stubs > docs/niri.d.lua
///
/// Supports:
/// - VS Code Lua extension (sumneko/lua-language-server)
/// - Neovim lua_ls (neovim/neovim)
/// - Zed
/// - IntelliJ IDEA

use std::fs;

pub struct TypeDefinition {
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub methods: Vec<(String, String)>,
}

pub fn generate_window_types() -> String {
    // Generate Window, Workspace, Monitor types
    // Generate all method signatures
    // Output Luau syntax
    todo!()
}

fn main() {
    let output = generate_window_types();
    println!("{}", output);
}
```

### 2.2 Neovim lua_ls Configuration

```lua
-- .luarc.json (for lua_ls in Neovim)
{
  "runtime": {
    "version": "LuaJIT",
    "path": ["?.lua", "?/init.lua"]
  },
  "diagnostics": {
    "globals": ["niri", "vim"]
  },
  "workspace": {
    "library": ["./docs/types"],
    "maxPreload": 10000
  }
}
```

### 2.3 VS Code Settings

```json
{
  "[lua]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "sumneko.lua"
  },
  "Lua.runtime.version": "LuaJIT",
  "Lua.diagnostics.globals": ["niri"],
  "Lua.workspace.library": ["./docs/types"],
  "Lua.workspace.preloadFileSize": 10000
}
```

---

## 3. Documentation

### 3.1 LUA_GUIDE.md (Comprehensive User Guide)

**Sections:**
1. **Getting Started**
   - Installation and setup
   - Config file location
   - First plugin

2. **Core Concepts**
   - Module system
   - Event handling
   - Plugin lifecycle
   - Sandboxing

3. **Configuration**
   - Reading/writing config
   - Keybinds
   - Animations
   - Input settings

4. **State Queries**
   - Windows API
   - Workspaces API
   - Monitors API
   - Filtering examples

5. **Event Handling**
   - Event types
   - Handler registration
   - Error handling
   - Event ordering

6. **Plugin Development**
   - Plugin structure
   - Dependencies
   - State persistence
   - Plugin testing

7. **Advanced Topics**
   - Custom modules
   - Extending Niri
   - Performance optimization
   - Debugging

8. **Troubleshooting**
   - Common issues
   - Debug logging
   - Performance problems
   - Plugin conflicts

### 3.2 LUA_QUICKSTART.md (5-Minute Start)

**Content:**
- Install Niri with Lua support
- Create first plugin (hello world)
- Run plugin
- Modify keybinding via Lua
- Listen to window events

### 3.3 LUA_EMBEDDING.md (Architecture)

**Content:**
- How Lua is integrated into Niri
- mlua + LuaJIT architecture
- IPC communication
- Plugin isolation model
- Performance characteristics

---

## 4. Example Plugins

Create 5+ fully-featured example plugins in `examples/plugins/`:

### 4.1 Example: Window Switcher

```lua
-- examples/plugins/window-switcher/plugin.lua
-- 
-- Demonstrates:
-- - Window state queries
-- - Event handling
-- - Custom keybinds
-- - User interaction

local niri = require "niri"

local metadata = {
  name = "window-switcher",
  version = "1.0.0",
  author = "Niri Community",
  description = "Quick window switcher with preview",
  dependencies = {},
}

local function show_window_list()
  local windows = niri.state.windows()
  
  -- Display windows and handle selection
  for i, win in ipairs(windows) do
    niri.log(string.format("%d: %s (%s)", i, win.title, win.app_id))
  end
end

local function on_window_open(event)
  niri.log("Window opened: " .. event.window.title)
  show_window_list()
end

-- Register event handler
niri.events.on("window:open", on_window_open)

-- Register keybind
niri.config.set_keybind("Super+Tab", "window-switcher.show")

return metadata
```

### 4.2 Example: Workspace Tabs

```lua
-- examples/plugins/workspace-tabs/plugin.lua
-- 
-- Demonstrates:
-- - Workspace operations
-- - State persistence
-- - Custom rendering (if UI support added)

local niri = require "niri"

local metadata = {
  name = "workspace-tabs",
  version = "1.0.0",
  author = "Niri Community",
  description = "Tab-like workspace switcher",
}

local function on_workspace_change(event)
  niri.log("Active workspace: " .. event.workspace.name)
end

niri.events.on("workspace:enter", on_workspace_change)

return metadata
```

### 4.3 Example: Layout Manager

```lua
-- examples/plugins/layout-manager/plugin.lua

local niri = require "niri"

local metadata = {
  name = "layout-manager",
  version = "1.0.0",
  author = "Niri Community",
  description = "Quick layout switcher",
}

local layouts = { "vertical", "horizontal", "paper" }
local current = 1

local function cycle_layout()
  current = (current % #layouts) + 1
  niri.log("Switching to: " .. layouts[current])
  niri.config.set_layout({ preset = layouts[current] })
end

niri.config.set_keybind("Super+L", "layout-manager.cycle")

return metadata
```

### 4.4 Example: Notification Daemon

```lua
-- examples/plugins/notif-daemon/plugin.lua

local niri = require "niri"

local metadata = {
  name = "notif-daemon",
  version = "1.0.0",
  author = "Niri Community",
  description = "Simple notification daemon",
}

local notifications = {}

local function notify(title, message, duration)
  duration = duration or 3000
  
  table.insert(notifications, {
    title = title,
    message = message,
    created_at = os.time() * 1000,
    duration = duration,
  })
  
  niri.log(string.format("[%s] %s", title, message))
end

return {
  metadata = metadata,
  notify = notify,
}
```

### 4.5 Example: Keybind Helper

```lua
-- examples/plugins/keybind-helper/plugin.lua

local niri = require "niri"

local metadata = {
  name = "keybind-helper",
  version = "1.0.0",
  author = "Niri Community",
  description = "Show available keybinds overlay",
}

local function show_help()
  local binds = niri.config.get_keybinds()
  
  niri.log("=== Available Keybinds ===")
  for key, action in pairs(binds) do
    niri.log(string.format("%s => %s", key, action))
  end
end

niri.config.set_keybind("Super+?", "keybind-helper.show")

return metadata
```

---

## 5. Testing Framework

### 5.1 Plugin Testing Infrastructure

Create `docs/testing-framework.md`:

```lua
-- example-plugin-test.lua

local niri = require "niri"
local test = require "niri.testing"

test.describe("Window Operations", function()
  test.it("should list all windows", function()
    local windows = niri.state.windows()
    test.assert(#windows > 0, "Should have at least one window")
  end)
  
  test.it("should find active window", function()
    local active = niri.state.active_window()
    test.assert(active ~= nil, "Should have an active window")
  end)
end)

test.run()
```

### 5.2 Plugin Development Template

```bash
# Create plugin structure
mkdir -p my-plugin/{src,tests,docs}

# my-plugin/plugin.lua - Main entry point
# my-plugin/src/main.rs - If native code needed
# my-plugin/tests/test.lua - Test suite
# my-plugin/docs/README.md - Plugin documentation
```

---

## 6. Interactive REPL

### 6.1 REPL Features

Create `tools/niri-repl`:

```lua
-- Interactive console for testing Niri API
-- Usage: niri-repl

-- Features:
-- - Command history
-- - Tab completion (niri. prefix)
-- - Error pretty-printing
-- - Result formatting
-- - Help system (? or :help)

> niri.version_string()
"Niri 0.1.0 (abc1234)"

> niri.state.windows()
[
  { id: 1, title: "Firefox", app_id: "firefox" },
  { id: 2, title: "Neovim", app_id: "foot" },
]

> niri.config.get_layout()
{ preset: "vertical", gaps: 8 }

> niri.events.on("window:open", function(e) print(e.window.title) end)
123  -- handler ID

> :help events
...
```

---

## 7. Integration Checklist

### Code
- [ ] Create `docs/niri.d.lua` with complete type definitions
- [ ] Create `tools/generate-lsp-stubs.rs` for LSP generation
- [ ] Verify types work with `lua_ls` in Neovim
- [ ] Verify types work with VS Code Lua extension

### Documentation
- [ ] Write `LUA_GUIDE.md` (comprehensive guide)
- [ ] Write `LUA_QUICKSTART.md` (5-minute tutorial)
- [ ] Write `LUA_EMBEDDING.md` (architecture)
- [ ] Write `docs/testing-framework.md` (testing guide)
- [ ] Write README for each example plugin
- [ ] Add API reference to documentation site

### Examples
- [ ] Create window-switcher plugin
- [ ] Create workspace-tabs plugin
- [ ] Create layout-manager plugin
- [ ] Create notification-daemon plugin
- [ ] Create keybind-helper plugin

### Tools
- [ ] Implement REPL for interactive testing
- [ ] Create plugin development template
- [ ] Write installation script

### Testing
- [ ] Test type checking with Luau
- [ ] Test LSP integration with Neovim
- [ ] Test example plugins for correctness
- [ ] Performance test large plugin loads
- [ ] Verify all examples work end-to-end

---

## 8. Success Criteria

### Quantitative
- [ ] LSP/IDE autocomplete works in 3+ editors
- [ ] 5+ example plugins demonstrate all major features
- [ ] Documentation covers 100% of public API
- [ ] REPL supports at least 50 common use cases
- [ ] Zero type errors in example plugins

### Qualitative
- [ ] Users find API intuitive and discoverable
- [ ] Type definitions are comprehensive and accurate
- [ ] Examples are well-documented and easy to follow
- [ ] Community can easily create plugins
- [ ] Documentation is clear and accessible

---

## 9. Timeline

| Task | Duration | Dependencies |
|------|----------|--------------|
| Type Definitions | 2 days | Tiers 1-5 API finalized |
| LSP Generation | 1 day | Type definitions complete |
| Documentation | 3 days | All code complete |
| Example Plugins | 3 days | Tiers 1-5 implementation done |
| REPL Development | 2 days | Plugin system stable |
| Final Polish | 1 day | All above complete |
| **Total** | **2 weeks** | **Sequential** |

---

## 10. References

- **LUA_TIER1_SPEC.md** - Foundation
- **LUA_TIER2_SPEC.md** - Configuration API
- **LUA_TIER3_SPEC.md** - State queries
- **LUA_TIER4_SPEC.md** - Event system
- **LUA_TIER5_SPEC.md** - Plugin ecosystem
- **LUA_IMPLEMENTATION_ROADMAP.md** - Overall timeline

---

**Document Version:** 1.0  
**Last Updated:** November 15, 2025  
**Author:** OpenCode Assistant
