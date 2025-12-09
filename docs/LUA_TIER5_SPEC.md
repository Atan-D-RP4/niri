# Tier 5: Plugin Ecosystem - Remaining Work

**Status:** ⚙️ **PARTIAL IMPLEMENTATION**

## What's Already Implemented

| Component | File | Status |
|-----------|------|--------|
| Plugin System (basic) | `niri-lua/src/plugin_system.rs` | ✅ Complete |
| Module Loader | `niri-lua/src/module_loader.rs` | ✅ Complete |

## Remaining Work

### 1. Plugin Manager (Full Lifecycle)

**File to create:** `niri-lua/src/plugin_manager.rs`

Features needed:
- `enable_plugin(name)` / `disable_plugin(name)` - Toggle plugin state
- `on_load()`, `on_enable()`, `on_disable()`, `on_unload()` lifecycle hooks
- Plugin state persistence across reloads (JSON storage in `~/.local/share/niri/plugins/`)

### 2. Plugin Sandbox

**File to create:** `niri-lua/src/plugin_sandbox.rs`

Features needed:
- Isolated Lua environment per plugin using `_ENV`
- Permission-based API access (e.g., `access_state`, `spawn_processes`, `filesystem_read`)
- Prevent plugins from interfering with each other

### 3. Dependency Resolution

Features needed:
- Parse `dependencies` from plugin metadata (`init.toml`)
- Topological sort for load order
- Version constraint checking (semver)
- Circular dependency detection

### 4. IPC Plugin Commands

Add to `src/ipc/server.rs`:

```
niri msg plugin list          # List all plugins with status
niri msg plugin enable <name> # Enable a plugin
niri msg plugin disable <name># Disable a plugin
niri msg plugin info <name>   # Get plugin details
```

## Plugin Metadata Format

```toml
# ~/.config/niri/plugins/my-plugin/init.toml
[plugin]
name = "my-plugin"
version = "0.1.0"
author = "Your Name"
description = "What the plugin does"

[plugin.dependencies]
"other-plugin" = ">=0.1.0"

[plugin.permissions]
access_state = true
spawn_processes = false
```

## Success Criteria

- [ ] Plugins can be enabled/disabled via IPC
- [ ] Plugin state persists across reloads
- [ ] Dependencies resolved correctly
- [ ] Plugin errors don't crash other plugins
