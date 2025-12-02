# Tier 6: Developer Experience - Remaining Work

**Status:** ⚙️ **PARTIAL IMPLEMENTATION**

## What's Already Implemented

| Component | Status |
|-----------|--------|
| Interactive REPL (`niri msg lua`) | ✅ Complete |
| Documentation (LUA_GUIDE.md, LUA_QUICKSTART.md, etc.) | ✅ Complete |
| Example Scripts (10 in `examples/`) | ✅ Complete |

## Remaining Work

### 1. EmmyLua Type Definitions

**File to create:** `docs/niri.lua` (type stubs)

Since niri uses **Lua 5.2 with LuaJIT** (not Luau), type definitions should use **EmmyLua annotations** - the same format used by Neovim and supported by lua_ls.

Example format:
```lua
---@class NiriConfig
---@field layout NiriLayoutConfig
---@field input NiriInputConfig

---@class NiriLayoutConfig
---@field gaps number

---@param msg string
---@return nil
function niri.utils.log(msg) end
```

### 2. LSP Configuration

**Files to create:**
- `.luarc.json` - lua_ls configuration for Neovim
- `.vscode/settings.json` - VS Code Lua extension settings

Example `.luarc.json`:
```json
{
  "runtime": { "version": "LuaJIT" },
  "diagnostics": { "globals": ["niri"] },
  "workspace": { "library": ["./docs"] }
}
```

### 3. Plugin Testing Framework

**File to create:** `niri-lua/src/testing.rs` or Lua module

Features needed:
- `test.describe()` / `test.it()` BDD-style test functions
- `test.assert()` with clear error messages
- Mock state for testing without running compositor
- Test runner that reports results

Example usage:
```lua
local test = require "niri.testing"

test.describe("Window Operations", function()
  test.it("should list windows", function()
    local windows = niri.state.windows()
    test.assert(type(windows) == "table")
  end)
end)

test.run()
```

### 4. Additional Example Plugins

Create in `examples/plugins/`:
- `column-stacker/` - Stack N windows in a column before creating new columns
- `workspace-namer/` - Auto-name workspaces based on open apps
- `app-launcher/` - Fuzzy app launcher with recent apps tracking
- `session-save/` - Save and restore window layouts

## Success Criteria

- [ ] IDE autocomplete works in Neovim/VS Code with lua_ls
- [ ] Type definitions cover all public APIs
- [ ] Plugin testing framework functional
- [ ] 5+ example plugins demonstrate major features
