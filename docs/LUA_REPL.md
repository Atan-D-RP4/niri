# Niri Lua REPL

**Execute Lua code in the running Niri compositor**

The Niri Lua REPL allows you to execute Lua scripts within the Niri compositor's runtime context via the `niri msg lua` command. This connects to the running Niri instance via IPC and executes Lua code with full access to all Niri APIs.

---

## Usage

Execute Lua code directly:

```bash
niri msg lua "print(niri.version_string())"
```

Execute multiple statements:

```bash
niri msg lua "
local windows = niri.state.windows()
print('Open windows: ' .. #windows)
for i, win in ipairs(windows) do
    print(i .. ': ' .. win.title .. ' (' .. win.app_id .. ')')
end
"
```

Or using a here-document:

```bash
niri msg lua <<'EOF'
local windows = niri.state.windows()
print("Open windows: " .. #windows)
for i, win in ipairs(windows) do
    print(i .. ": " .. win.title .. " (" .. win.app_id .. ")")
end
EOF
```

### JSON Output

For machine-readable output, use the `--json` flag:

```bash
niri msg lua --json "niri.state.windows()"
```

This returns structured JSON that includes both the output and execution status.

---

## Output and Error Handling

The `niri msg lua` command prints output from `print()` statements:

```bash
$ niri msg lua "print('Hello'); print('World')"
Hello
World
```

If execution fails, the command returns a non-zero exit code:

```bash
$ niri msg lua "error('Something went wrong')"
Error: Lua execution failed
```

---

## Examples

### Query Compositor State

Get information about open windows:

```bash
niri msg lua "
local windows = niri.state.windows()
for i, win in ipairs(windows) do
    print(i .. '. ' .. win.title .. ' (' .. win.app_id .. ')')
end
"
```

Output:
```
1. Firefox (firefox)
2. Neovim (foot)
3. Niri (niri)
```

Or use pretty-printing to see the full window structure:

```bash
niri msg lua "niri.print(niri.state.windows())"
```

Output:
```
{
  { app_id = "firefox", id = 1, is_floating = false, title = "Firefox", ... },
  { app_id = "foot", id = 2, is_floating = true, title = "Neovim", ... },
  { app_id = "niri", id = 3, is_floating = false, title = "Niri", ... }
}
```

Get active workspace:

```bash
niri msg lua "
local ws = niri.state.active_workspace()
print('Workspace: ' .. ws.name .. ', Windows: ' .. ws.window_count)
"
```

Pretty-print the active workspace:

```bash
niri msg lua "niri.print(niri.state.active_workspace())"
```

List all outputs:

```bash
niri msg lua "
local outputs = niri.state.monitors()
for _, out in ipairs(outputs) do
    print(out.name .. ' @ ' .. out.current_scale .. 'x')
end
"
```

Pretty-print all monitors:

```bash
niri msg lua "niri.print(niri.state.monitors())"
```

### Configure Keybindings

Add a keybinding dynamically:

```bash
niri msg lua "niri.config.set_keybind('Super+X', 'spawn xterm')"
```

### Control Animations

Adjust animation settings:

```bash
niri msg lua "
niri.config.set_animations({
  window_open = { curve = 'ease_out_cubic', duration_ms = 300 },
  window_close = { curve = 'ease_out_cubic', duration_ms = 200 },
})
"
```

### Complex Scripting

Build and execute complex behaviors:

```bash
niri msg lua "
local function count_windows_per_app()
  local counts = {}
  for _, win in ipairs(niri.state.windows()) do
    local app_id = win.app_id or 'unknown'
    counts[app_id] = (counts[app_id] or 0) + 1
  end
  for app_id, count in pairs(counts) do
    print(app_id .. ': ' .. count)
  end
end
count_windows_per_app()
"
```

---

## API Reference

### State Queries

#### `niri.state.windows()`

Returns a table of all open windows:

```lua
{
  { id = 1, title = "Firefox", app_id = "firefox", is_floating = false, ... },
  { id = 2, title = "Neovim", app_id = "foot", is_floating = true, ... },
}
```

#### `niri.state.active_window()`

Returns the currently focused window or nil.

#### `niri.state.workspaces()`

Returns a table of all workspaces:

```lua
{
  { id = 1, name = "workspace-1", index = 0, window_count = 3, ... },
  { id = 2, name = "workspace-2", index = 1, window_count = 1, ... },
}
```

#### `niri.state.active_workspace()`

Returns the currently active workspace.

#### `niri.state.monitors()`

Returns a table of all connected monitors:

```lua
{
  { index = 0, name = "HDMI-1", current_scale = 1, refresh_rate = 60, ... },
  { index = 1, name = "eDP-1", current_scale = 1.5, refresh_rate = 144, ... },
}
```

### Configuration

#### `niri.config.set_keybind(key, action)`

Sets a keybinding. Returns true on success.

```lua
niri.config.set_keybind("Super+Alt+T", "spawn alacritty")
```

#### `niri.config.get_keybinds()`

Returns all current keybindings as a table.

#### `niri.config.set_appearance(config)`

Sets appearance settings. Accepts a partial configuration table:

```lua
niri.config.set_appearance({
  border = {
    width = 4,
    active_color = "#00ff00",
    inactive_color = "#333333",
  }
})
```

#### `niri.config.set_layout(config)`

Sets layout configuration:

```lua
niri.config.set_layout({
  preset = "vertical",
  gaps = 8,
})
```

#### `niri.config.set_animations(config)`

Sets animation configuration:

```lua
niri.config.set_animations({
  window_open = { curve = "ease_out_cubic", duration_ms = 300 },
})
```

### Events

#### `niri.events.on(event_name, handler)`

Registers an event handler. Returns a handler ID that can be used to unregister.

Available events: `window:open`, `window:close`, `window:focus`, `window:blur`, `workspace:activate`, `layout:mode_changed`, `monitor:connect`, `monitor:disconnect`, etc.

```lua
local handler_id = niri.events.on("window:open", function(event)
  print("Window: " .. event.window.title)
end)
```

#### `niri.events.once(event_name, handler)`

Registers a one-time event handler that automatically unregisters after the first event.

#### `niri.events.off(event_name, handler_id)`

Unregisters a previously registered event handler.

### Other

#### `niri.print(...)`

Pretty-prints values similar to `vim.print()`. This function formats tables, arrays, and other values in a human-readable way.

For arrays and simple values:
```lua
> niri.print({ 1, 2, 3, 4, 5 })
{ 1, 2, 3, 4, 5 }
```

For objects with key-value pairs:
```lua
> niri.print({ name = "firefox", count = 3, active = true })
{ name = "firefox", count = 3, active = true }
```

For complex nested structures, it automatically switches to multi-line formatting:
```lua
> niri.print({
    { name = "firefox", count = 3 },
    { name = "neovim", count = 1 },
    { name = "foot", count = 2 }
  })
{
  { name = "firefox", count = 3 },
  { name = "neovim", count = 1 },
  { name = "foot", count = 2 }
}
```

You can print multiple values at once (separated by tabs):
```lua
> niri.print("Windows:", #niri.state.windows(), "Active:", niri.state.active_window().title)
Windows:	2	Active:	Firefox
```

#### `niri.version_string()`

Returns the Niri version string:

```lua
> niri.version_string()
"Niri 0.1.0 (abc1234)"
```

#### `niri.log(message)`

Logs a message to the Niri logs (visible in `journalctl -eu niri`):

```lua
> niri.log("Hello from REPL!")
nil
```

---

## Security Considerations

⚠️ **IMPORTANT:** The Lua REPL has **NO SANDBOXING**. It has full access to:

- All Niri APIs
- The Lua standard library (including `os.execute()`)
- The system (can spawn processes, read/write files, etc.)

**Only execute code from trusted sources.**

In particular:
- Do NOT use the REPL with untrusted scripts or code from unknown sources
- The REPL has the same privileges as the Niri process
- Malicious Lua code could potentially crash Niri or compromise your system

For production use, consider:
1. Using a dedicated user account for Niri with limited privileges
2. Only using pre-vetted Lua scripts
3. Regularly reviewing any Lua code before execution

---

## Error Handling

If Lua code raises an error, the REPL displays it clearly:

```lua
> error("test error")
Error: Error: test error
```

Syntax errors are also caught:

```lua
> print(1 +
Error: Error: [string "<input>"]:1: '=' expected near '<eof>'
```

---

## Performance Tips

1. **Print output**: Avoid printing extremely large data structures, as output formatting can be slow
2. **Lua loops**: Simple loops in Lua are reasonably fast; for heavy computation, consider using Niri's native APIs
3. **IPC overhead**: Each REPL interaction sends data via IPC; batch operations when possible

---

## Troubleshooting

### "NIRI_SOCKET is not set, are you running this within niri?"

The REPL couldn't find the Niri IPC socket. This means either:
- Niri is not running
- You're not running within a Niri session
- The `NIRI_SOCKET` environment variable is not set

Solution: Start Niri and make sure you're in the Niri session environment.

### "Connection refused"

The Niri IPC socket exists but Niri refused the connection. Usually means:
- Niri is starting up
- Lua runtime is not initialized

Solution: Wait for Niri to fully start, or check the logs with `journalctl -eu niri`.

### Changes to keybinds don't persist

Keybinds set via the REPL are not saved to the config file. They only apply to the running Niri session. To persist changes, edit your `config.lua` and reload it.

### Event handlers not firing

Event handlers registered in the REPL only respond to events that occur after they're registered. Events that happened before registration are not replayed.

---

## Examples

### Build a Simple Window Switcher

```lua
> local function switch_to_firefox()
  local wins = niri.state.windows()
  for _, win in ipairs(wins) do
    if win.app_id == "firefox" then
      niri.window.focus(win.id)
      return
    end
  end
  print("Firefox not found")
end

> switch_to_firefox()
```

### Monitor Window Operations

```lua
> niri.events.on("window:open", function(e)
  print("New: " .. e.window.title)
end)

> niri.events.on("window:close", function(e)
  print("Closed: " .. e.window.title)
end)
```

### Create Dynamic Keybinds

```lua
> local layouts = { "vertical", "horizontal", "paper" }
> local current = 1

> niri.config.set_keybind("Super+L", "niri msg action next-layout")

> local function next_layout()
  current = (current % #layouts) + 1
  niri.log("Layout: " .. layouts[current])
  niri.config.set_layout({ preset = layouts[current] })
end

> next_layout()
```

---

## See Also

- **Lua Guide**: `docs/LUA_GUIDE.md` - Comprehensive Lua scripting documentation
- **Lua API Reference**: `docs/LUA_TIER3_SPEC.md` - Detailed API specifications
- **Event System**: `docs/LUA_TIER4_SPEC.md` - Event handling documentation
- **Niri Configuration**: `docs/niri-config.md` - Configuration options

---

## Contributing

Issues and feature requests for the REPL can be filed at:
https://github.com/sodiboo/niri/issues

For security issues, please report privately to the maintainers.

---

**Happy scripting! 🚀**
