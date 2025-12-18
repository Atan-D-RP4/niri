# Lua Bindings Specification

## Overview

This specification defines how niri-ui widgets are exposed to Lua scripts, enabling users to create custom panels, launchers, notifications, and other UI elements through Lua configuration.

## Architecture

### Namespace Structure

```lua
niri.ui                    -- Widget factory and window management
niri.ui.label(props)       -- Create Label widget
niri.ui.box(props)         -- Create Box container
niri.ui.row(props)         -- Create Row layout
niri.ui.column(props)      -- Create Column layout
niri.ui.button(props)      -- Create Button widget
niri.ui.image(props)       -- Create Image widget
niri.ui.slider(props)      -- Create Slider widget
niri.ui.entry(props)       -- Create text Entry widget
niri.ui.progress(props)    -- Create ProgressBar widget
niri.ui.circular_progress(props) -- Create CircularProgress widget
niri.ui.revealer(props)    -- Create Revealer container
niri.ui.scrollable(props)  -- Create Scrollable container

niri.ui.window(props)      -- Create UI window
niri.ui.popup(props)       -- Create popup window

niri.ui.style(props)       -- Create style definition
niri.ui.animation(props)   -- Create animation definition
```

---

## Core Types

### 1. Widget Userdata

All widgets are represented as Lua userdata with methods:

```rust
/// Lua userdata wrapper for widgets
pub struct LuaWidget {
    /// Unique widget ID for tracking
    id: WidgetId,
    /// Reference to the actual widget (via Arc<Mutex<>>)
    inner: Arc<Mutex<dyn Widget>>,
}

impl LuaUserData for LuaWidget {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Property access
        methods.add_method("get", |_, this, key: String| {
            this.get_property(&key)
        });
        
        methods.add_method_mut("set", |_, this, (key, value): (String, LuaValue)| {
            this.set_property(&key, value)
        });
        
        // Event handling
        methods.add_method_mut("on", |lua, this, (event, handler): (String, LuaFunction)| {
            this.add_event_handler(&event, handler)
        });
        
        // Hierarchy
        methods.add_method_mut("add_child", |_, this, child: LuaWidget| {
            this.add_child(child)
        });
        
        methods.add_method_mut("remove_child", |_, this, child: LuaWidget| {
            this.remove_child(child)
        });
        
        // Lifecycle
        methods.add_method("invalidate", |_, this, ()| {
            this.invalidate()
        });
        
        methods.add_method("is_visible", |_, this, ()| {
            Ok(this.is_visible())
        });
    }
}
```

### 2. Window Userdata

```rust
/// Lua userdata wrapper for UI windows
pub struct LuaWindow {
    id: WindowId,
    inner: Arc<Mutex<UiWindow>>,
}

impl LuaUserData for LuaWindow {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("show", |_, this, ()| {
            this.show()
        });
        
        methods.add_method("hide", |_, this, ()| {
            this.hide()
        });
        
        methods.add_method("toggle", |_, this, ()| {
            this.toggle()
        });
        
        methods.add_method("is_visible", |_, this, ()| {
            Ok(this.is_visible())
        });
        
        methods.add_method_mut("set_content", |_, this, widget: LuaWidget| {
            this.set_content(widget)
        });
        
        methods.add_method("on", |lua, this, (event, handler): (String, LuaFunction)| {
            this.add_event_handler(&event, handler)
        });
    }
}
```

---

## Widget Factory Functions

### Label

```lua
-- Create a text label
local label = niri.ui.label({
    text = "Hello World",
    font_family = "sans-serif",  -- optional
    font_size = 14,              -- optional, in points
    font_weight = "normal",      -- optional: "normal", "bold", "light", etc.
    color = "#ffffff",           -- optional
    ellipsize = "end",           -- optional: "none", "start", "middle", "end"
    wrap = false,                -- optional: enable text wrapping
    max_width = 200,             -- optional: max width for wrapping
    markup = false,              -- optional: enable Pango markup
})

-- Update text dynamically
label:set("text", "Updated text")
```

### Box (Container)

```lua
-- Create a styled container
local box = niri.ui.box({
    width = 300,                 -- optional: fixed width
    height = 200,                -- optional: fixed height
    min_width = 100,             -- optional
    min_height = 50,             -- optional
    padding = 10,                -- optional: uniform padding
    padding = { 10, 20 },        -- optional: vertical, horizontal
    padding = { 10, 20, 10, 20 },-- optional: top, right, bottom, left
    margin = 5,                  -- optional: same format as padding
    background = "#1a1a2e",      -- optional: solid color
    background = {               -- optional: gradient
        type = "linear",
        angle = 90,
        stops = {
            { 0.0, "#1a1a2e" },
            { 1.0, "#16213e" },
        }
    },
    border_radius = 8,           -- optional: uniform radius
    border_radius = { 8, 8, 0, 0 }, -- optional: per-corner
    border_width = 1,            -- optional
    border_color = "#ffffff33",  -- optional
    shadow = {                   -- optional: box shadow
        offset_x = 0,
        offset_y = 4,
        blur = 8,
        color = "#00000066",
    },
    children = { label },        -- optional: child widgets
})
```

### Row / Column (Layout Containers)

```lua
-- Horizontal layout
local row = niri.ui.row({
    spacing = 10,                -- gap between children
    align = "center",            -- cross-axis: "start", "center", "end", "stretch"
    justify = "space-between",   -- main-axis: "start", "center", "end", "space-between", "space-around"
    children = { widget1, widget2, widget3 },
})

-- Vertical layout
local column = niri.ui.column({
    spacing = 8,
    align = "stretch",
    justify = "start",
    children = { header, content, footer },
})
```

### Button

```lua
local button = niri.ui.button({
    child = niri.ui.label({ text = "Click Me" }),
    -- OR use text shorthand:
    text = "Click Me",
    
    style = {
        background = "#3b82f6",
        hover = { background = "#2563eb" },
        active = { background = "#1d4ed8" },
        disabled = { background = "#6b7280", opacity = 0.5 },
    },
    disabled = false,            -- optional
})

-- Event handling
button:on("click", function()
    print("Button clicked!")
end)

button:on("hover", function(hovered)
    print("Hover state:", hovered)
end)
```

### Image

```lua
local image = niri.ui.image({
    path = "/path/to/icon.png",  -- file path
    -- OR
    name = "firefox",            -- icon name (uses icon theme)
    -- OR
    data = raw_bytes,            -- raw image data
    
    width = 48,                  -- optional: display width
    height = 48,                 -- optional: display height
    scale_mode = "fit",          -- optional: "fit", "fill", "stretch", "none"
})
```

### Slider

```lua
local slider = niri.ui.slider({
    value = 50,                  -- current value (0-100 default)
    min = 0,                     -- optional
    max = 100,                   -- optional
    step = 1,                    -- optional: step increment
    orientation = "horizontal", -- optional: "horizontal", "vertical"
    
    track_color = "#374151",     -- optional
    fill_color = "#3b82f6",      -- optional
    thumb_color = "#ffffff",     -- optional
    thumb_size = 16,             -- optional
})

slider:on("change", function(value)
    print("Slider value:", value)
end)

slider:on("release", function(value)
    -- Called when user releases the slider
    niri.action.set_volume({ volume = value / 100 })
end)
```

### Entry (Text Input)

```lua
local entry = niri.ui.entry({
    text = "",                   -- initial text
    placeholder = "Search...",  -- placeholder text
    password = false,            -- optional: hide input
    max_length = 100,            -- optional
    
    style = {
        background = "#1f2937",
        color = "#ffffff",
        placeholder_color = "#6b7280",
        focus = { border_color = "#3b82f6" },
    },
})

entry:on("change", function(text)
    print("Text changed:", text)
end)

entry:on("submit", function(text)
    print("Submitted:", text)
end)

entry:on("focus", function(focused)
    print("Focus state:", focused)
end)
```

### ProgressBar

```lua
local progress = niri.ui.progress({
    value = 0.75,                -- 0.0 to 1.0
    orientation = "horizontal", -- optional
    
    track_color = "#374151",
    fill_color = "#10b981",
    height = 8,                  -- optional: bar height
    border_radius = 4,           -- optional
})

-- Update progress
progress:set("value", 0.9)
```

### CircularProgress

```lua
local circular = niri.ui.circular_progress({
    value = 0.65,                -- 0.0 to 1.0
    size = 64,                   -- diameter
    thickness = 6,               -- ring thickness
    
    track_color = "#374151",
    fill_color = "#3b82f6",
    start_angle = -90,           -- optional: start at top
    
    child = niri.ui.label({      -- optional: center content
        text = "65%",
        font_size = 12,
    }),
})
```

### Revealer

```lua
local revealer = niri.ui.revealer({
    revealed = false,            -- initial state
    transition = "slide_down",   -- "slide_down", "slide_up", "slide_left", "slide_right", "fade", "none"
    duration = 200,              -- animation duration in ms
    
    child = niri.ui.box({
        children = { content },
    }),
})

-- Toggle visibility with animation
revealer:set("revealed", true)
revealer:set("revealed", false)
```

### Scrollable

```lua
local scrollable = niri.ui.scrollable({
    direction = "vertical",      -- "vertical", "horizontal", "both"
    
    scrollbar = {                -- optional: scrollbar styling
        width = 8,
        track_color = "#1f2937",
        thumb_color = "#4b5563",
        hover = { thumb_color = "#6b7280" },
    },
    
    child = niri.ui.column({
        children = long_list,
    }),
})

-- Programmatic scrolling
scrollable:scroll_to(0, 500)     -- x, y position
scrollable:scroll_to_child(widget) -- scroll to make widget visible
```

---

## Window Creation

### Panel Window

```lua
local panel = niri.ui.window({
    name = "top-panel",          -- unique identifier
    
    anchor = { "top", "left", "right" }, -- edge anchoring
    -- OR single edge:
    anchor = "top",
    
    exclusive = 32,              -- reserve space (height/width based on anchor)
    -- OR
    exclusive = true,            -- auto-calculate from content
    
    layer = "top",               -- "background", "bottom", "top", "overlay"
    keyboard_mode = "none",      -- "none", "exclusive", "on_demand"
    
    margin = { 0, 10, 0, 10 },   -- optional: top, right, bottom, left
    
    output = "all",              -- "all", "focused", or specific output name
    
    content = niri.ui.row({
        children = { workspaces, clock, systray },
    }),
})

panel:show()
```

### Popup Window

```lua
local popup = niri.ui.popup({
    name = "app-menu",
    
    -- Anchor to another widget
    anchor = {
        widget = button,         -- anchor to this widget
        edge = "bottom",         -- which edge of the widget
        align = "start",         -- alignment along that edge
    },
    -- OR anchor to cursor
    anchor = "cursor",
    -- OR anchor to screen position
    anchor = { x = 100, y = 200, output = "focused" },
    
    -- Auto-dismiss behavior
    dismiss_on_click_outside = true,
    dismiss_on_escape = true,
    dismiss_on_focus_loss = true,
    
    -- Animation
    animation = {
        show = { type = "slide", direction = "down", duration = 150 },
        hide = { type = "fade", duration = 100 },
    },
    
    content = menu_widget,
})

-- Show popup
popup:show()

-- Toggle popup
popup:toggle()

-- Events
popup:on("dismiss", function(reason)
    print("Popup dismissed:", reason) -- "click_outside", "escape", "programmatic"
end)
```

---

## Style Definitions

### Inline Styles

```lua
local button = niri.ui.button({
    text = "Styled Button",
    style = {
        background = "#3b82f6",
        color = "#ffffff",
        padding = { 8, 16 },
        border_radius = 6,
        
        -- State variants
        hover = {
            background = "#2563eb",
        },
        active = {
            background = "#1d4ed8",
            transform = "scale(0.98)",
        },
        disabled = {
            background = "#6b7280",
            opacity = 0.5,
        },
    },
})
```

### Reusable Styles

```lua
-- Define reusable style
local primary_button_style = niri.ui.style({
    background = "#3b82f6",
    color = "#ffffff",
    padding = { 8, 16 },
    border_radius = 6,
    font_weight = "medium",
    
    hover = { background = "#2563eb" },
    active = { background = "#1d4ed8" },
    disabled = { background = "#6b7280", opacity = 0.5 },
})

-- Apply to widgets
local btn1 = niri.ui.button({ text = "Primary", style = primary_button_style })
local btn2 = niri.ui.button({ text = "Another", style = primary_button_style })

-- Extend/override styles
local danger_button_style = primary_button_style:extend({
    background = "#ef4444",
    hover = { background = "#dc2626" },
    active = { background = "#b91c1c" },
})
```

---

## Event Handling

### Widget Events

```lua
-- Mouse events
widget:on("click", function(event)
    print("Clicked at:", event.x, event.y)
    print("Button:", event.button) -- 1=left, 2=middle, 3=right
end)

widget:on("hover", function(hovered)
    -- hovered: boolean
end)

widget:on("scroll", function(event)
    print("Scroll delta:", event.delta_x, event.delta_y)
end)

-- Keyboard events (for focusable widgets)
widget:on("key", function(event)
    print("Key:", event.key, "Pressed:", event.pressed)
end)

-- Focus events
widget:on("focus", function(focused)
    -- focused: boolean
end)
```

### Window Events

```lua
window:on("show", function() end)
window:on("hide", function() end)
window:on("output_change", function(output_name) end)
```

### Removing Event Handlers

```lua
local handler_id = widget:on("click", handler_fn)
widget:off(handler_id)

-- Or remove all handlers for an event type
widget:off("click")
```

---

## Reactive Patterns

### Property Binding

```lua
-- Create reactive state
local state = niri.ui.state({
    count = 0,
    visible = true,
})

-- Bind widget property to state
local label = niri.ui.label({
    text = state:bind("count", function(count)
        return "Count: " .. count
    end),
})

-- Update state (automatically updates bound widgets)
state:set("count", state:get("count") + 1)
```

### Computed Properties

```lua
local state = niri.ui.state({
    volume = 50,
    muted = false,
})

-- Computed binding
local icon = niri.ui.image({
    name = state:computed(function(s)
        if s.muted then return "audio-volume-muted" end
        if s.volume > 66 then return "audio-volume-high" end
        if s.volume > 33 then return "audio-volume-medium" end
        return "audio-volume-low"
    end),
})
```

---

## Runtime Query Behavior & niri-lua API Enhancements

### Dual-mode queries & snapshot staleness

niri-lua uses a dual-mode runtime query architecture (Event Handler Mode uses a pre-captured StateSnapshot; Normal Mode performs an idle callback into the main thread). This has direct implications for UI code:

- Event handlers see a pre-captured snapshot: do not rely on re-querying full state immediately after performing synchronous actions inside an event handler.
- Preferred patterns:
  - Include sufficient event data (ids, rects, small payloads) with emitted events so handlers don't need to re-query.
  - Use niri.utils.defer() or schedule an idle callback to perform follow-up queries when up-to-date state is required.
  - For reactive UI, prefer `niri.state.watch(path_or_selector, callback)` when waiting for state changes.

### Required runtime functions (recommended additions to niri-lua)

The UI project requires a small set of targeted runtime helpers to make common tasks efficient and robust. Add the following functions to `niri.ui` (and register action proxies in `niri.action` where appropriate):

- niri.ui.hit_test(x: number, y: number) -> table | nil
  - Returns: { window = <WindowId>, widget_id = <string>, local_x = <number>, local_y = <number>, global_x = <number>, global_y = <number> }
  - Use: quick lookups for context menus and pointer-driven popups without scanning the whole widget tree.

- niri.ui.get_widget_bounds(widget_id: string) -> table | nil
  - Returns: { x, y, width, height, window, output, scale }
  - Use: precise popup anchoring and animation source rectangles.

- niri.ui.get_window(name_or_id) -> Window userdata | nil
  - A targeted alternative to `niri.state.windows()` that avoids copying the entire list.

- Popup control helpers:
  - niri.ui.show_popup(name)
  - niri.ui.hide_popup(name)
  - niri.ui.toggle_popup(name)
  - niri.ui.update_popup_position(name, { x, y })
  - niri.ui.is_popup_visible(name) -> bool

- Actions (niri.action) for remote/permissioned invocation (recommended):
  - Action::ShowPopup { name }
  - Action::HidePopup { name }
  - Action::TogglePopup { name }

### Event payload shapes (standardize payloads)

- `widget:click` -> { widget_id, window, x, y, button, modifiers }
- `popup:show` -> { name, window, anchor_rect = { x, y, w, h }, position = { x, y }, output, scale }
- `popup:dismiss` -> { name, reason = "outside_click" | "escape" | "programmatic" }

Recommendation: Emit these payloads with events so handlers can act without additional queries.

### Extractors & API Schema

- Add targeted extractors to `niri-lua/src/extractors.rs` for UI types to validate and canonicalize Lua tables into Rust types:
  - `extract_popup_config(table: &LuaTable) -> LuaResult<Option<PopupConfig>>`
  - `extract_slider_config(table: &LuaTable) -> LuaResult<Option<SliderConfig>>`
  - `extract_style(table: &LuaTable) -> LuaResult<Option<Style>>`

- Update `niri-lua/src/api_data.rs` to include `niri.ui` module schema entries and types (PopupConfig, PopupAnchor, PopupWindow, HitTestResult, WidgetBounds) so the generated API docs and EmmyLua types remain accurate.

### Tests & Acceptance Criteria (Lua bindings)

- Unit tests for extractor functions (valid/invalid inputs), and for each new `niri.ui.*` helper.
- Integration tests that simulate pointer events and verify `hit_test()` results and popup show/hide behavior (use the existing `create_test_environment()` utilities).

### Implementation notes

- Expose both convenience functions on `niri.ui.*` (for direct script use) and actions via `niri.action` (for permissioned/IPC usage).
- Prefer sending rich event payloads rather than encouraging event handlers to re-query state during the synchronous event handler phase.

### niri-lua integration checklist (PRIORITY: P0)

The UI project depends on a small set of targeted capabilities in `niri-lua`. These should be implemented and prioritized as P0 to enable reliable, low-latency UI features (popups, hit-testing, widget anchoring, reactive bindings).

- Implement runtime helpers in `niri-lua`:
  - `niri.ui.hit_test(x: number, y: number) -> HitTestResult | nil` (fast, use snapshots for handler mode)
  - `niri.ui.get_widget_bounds(widget_id: string) -> WidgetBounds | nil`
  - `niri.ui.get_window(name_or_id) -> Window | nil` (targeted query to avoid listing all windows)
  - Popup control helpers: `niri.ui.show_popup(name)`, `niri.ui.hide_popup(name)`, `niri.ui.toggle_popup(name)`, `niri.ui.update_popup_position(name, {x,y})`, `niri.ui.is_popup_visible(name)`
  - Register corresponding actions under `niri.action` (Action::ShowPopup/HidePopup/TogglePopup)

- Implement `niri.state.watch(selector_or_opts, callback)` as a Lua-level helper (P0) rather than a Rust core feature. Rationale and design below.

### Reactive watch helper (recommended: Lua-level, P0)

Rationale
- A core Rust `watch` registry is powerful but requires intrusive hooks, delta detection, quotas, and careful synchronization (P2 complexity).
- Most UI use-cases are covered by a small Lua helper that composes `niri.events:on()` with targeted queries and optional debounce logic. This delivers a fast developer experience with minimal core changes.

API shape (suggested)

```lua
-- Simple options-based form
local sub = niri.state.watch({
  events = {"window:move", "window:resize"}, -- or patterns
  filter = function(payload) return payload.id == 42 end, -- optional
  immediate = true,  -- invoke once with current state
  debounce_ms = 50,  -- coalesce rapid updates
}, function(payload)
  -- callback invoked with event payload
end)

-- Cancel subscription
sub:cancel()
```

Implementation sketch (pseudo-Lua)

```lua
function niri.state.watch(opts, cb)
  local events = opts.events or {"window:open","window:close"}
  local debounce_ms = opts.debounce_ms
  local filter = opts.filter
  local immediate = opts.immediate

  local handler_ids = niri.events:on(events, function(payload)
    if filter and not filter(payload) then return end
    if debounce_ms and debounce_ms > 0 then
      -- use a timer to coalesce events
      if timer then timer:stop() end
      timer = niri.loop.new_timer(function() if active then cb(payload) end end, debounce_ms)
    else
      cb(payload)
    end
  end)

  if immediate then
    -- schedule immediate delivery with targeted queries to avoid heavy lists
    niri.utils.defer(function()
      if active then cb({ immediate = true, /* include targeted state snapshot */ }) end
    end)
  end

  local sub = {
    cancel = function()
      active = false
      niri.events:off(handler_ids)
      if timer then timer:stop() end
    end,
    is_active = function() return active end,
  }
  return sub
end
```

Guidance & constraints
- Events emitted by the compositor should include rich, small payloads (id, rect, output, scale) so watchers avoid full-list queries.
- `immediate=true` should use targeted queries (`niri.state.get_window(id)` or `niri.ui.get_widget_bounds`) rather than returning entire collections.
- Provide `cancel()` and `is_active()` on the subscription object and ensure a `__gc` finalizer for auto-cleanup.

Acceptance criteria
- AC-1: Immediate delivery — `immediate=true` invokes callback once with current state.
- AC-2: Debounce correctness — multiple rapid events coalesce into a single callback when `debounce_ms` is set.
- AC-3: Cancellation — `sub:cancel()` prevents future callbacks and removes event handlers.
- AC-4: GC safety — subscription is cleaned up if garbage-collected without explicit cancel.

When to implement a Rust-level registry (P2)
- Only consider after profiling demonstrates that the Lua helper is a bottleneck for high-frequency updates. A core-level implementation should provide quotas, efficient per-field hooks, and strong cancellation semantics.

- Add targeted extractors to `niri-lua/src/extractors.rs`:
  - `extract_popup_config(table: &LuaTable) -> LuaResult<Option<PopupConfig>>`
  - `extract_slider_config(table: &LuaTable) -> LuaResult<Option<SliderConfig>>`
  - `extract_style(table: &LuaTable) -> LuaResult<Option<Style>>`

- Add API schema entries and types to `niri-lua/src/api_data.rs` for the `niri.ui` module (PopupConfig, PopupAnchor, PopupWindow, HitTestResult, WidgetBounds) so generated docs and LSP types are available.

- Emit popup lifecycle events in `lua_event_hooks.rs` (or equivalent): `popup:show`, `popup:dismiss`, with standardized payloads (see Event payload shapes above).

- Ensure `register_ui_api(lua)` (or equivalent) is called during runtime setup (`setup_runtime()` / `lua_integration.rs`) so `niri.ui` is available to scripts.

- Tests & Acceptance criteria:
  - Unit tests for new extractors (valid/invalid inputs).
  - Unit tests for `hit_test()` and `get_widget_bounds()` semantics.
  - Integration tests that simulate pointer events to assert popup show/hide, correct anchor positioning, and emitted event payloads.

### Security & Observability

- Expose UI control actions through `niri.action` when they have global or privileged effects to centralize permissioning/auditing.
- Instrument key metrics (popup_show_count, popup_show_time_ms, ui_hit_test_latency_ns) and surface them via the compositor telemetry.


## Integration with niri APIs

### Actions

```lua
-- Use niri actions from UI handlers
button:on("click", function()
    niri.action.spawn({ command = "firefox" })
end)

-- Workspace switching
ws_button:on("click", function()
    niri.action.focus_workspace({ index = 1 })
end)
```

### State Queries

```lua
-- Query compositor state
local windows = niri.state.windows()
local workspaces = niri.state.workspaces()
local focused = niri.state.focused_window()

-- Build UI from state
local ws_buttons = {}
for _, ws in ipairs(workspaces) do
    table.insert(ws_buttons, niri.ui.button({
        text = ws.name or tostring(ws.idx),
        style = ws.is_active and active_style or inactive_style,
        on_click = function()
            niri.action.focus_workspace({ id = ws.id })
        end,
    }))
end
```

### Event Integration

```lua
-- React to compositor events
niri.on("workspace:activate", function(data)
    update_workspace_indicator(data.id)
end)

niri.on("window:focus", function(data)
    update_focused_window_title(data.title)
end)
```

---

## Rust Implementation

### Widget Factory Registration

```rust
/// Register all widget factory functions to Lua
pub fn register_ui_api(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();
    let niri: LuaTable = globals.get("niri")?;
    
    let ui = lua.create_table()?;
    
    // Register widget constructors
    ui.set("label", lua.create_function(create_label)?)?;
    ui.set("box", lua.create_function(create_box)?)?;
    ui.set("row", lua.create_function(create_row)?)?;
    ui.set("column", lua.create_function(create_column)?)?;
    ui.set("button", lua.create_function(create_button)?)?;
    ui.set("image", lua.create_function(create_image)?)?;
    ui.set("slider", lua.create_function(create_slider)?)?;
    ui.set("entry", lua.create_function(create_entry)?)?;
    ui.set("progress", lua.create_function(create_progress)?)?;
    ui.set("circular_progress", lua.create_function(create_circular_progress)?)?;
    ui.set("revealer", lua.create_function(create_revealer)?)?;
    ui.set("scrollable", lua.create_function(create_scrollable)?)?;
    
    // Window constructors
    ui.set("window", lua.create_function(create_window)?)?;
    ui.set("popup", lua.create_function(create_popup)?)?;
    
    // Utilities
    ui.set("style", lua.create_function(create_style)?)?;
    ui.set("state", lua.create_function(create_reactive_state)?)?;
    
    niri.set("ui", ui)?;
    Ok(())
}

/// Create a Label widget from Lua props
fn create_label(lua: &Lua, props: LuaTable) -> LuaResult<LuaWidget> {
    let text: String = props.get("text").unwrap_or_default();
    let font_size: Option<f64> = props.get("font_size").ok();
    let font_family: Option<String> = props.get("font_family").ok();
    let color: Option<Color> = extract_color(&props, "color")?;
    
    let mut label = Label::new(&text);
    
    if let Some(size) = font_size {
        label.set_font_size(size);
    }
    if let Some(family) = font_family {
        label.set_font_family(&family);
    }
    if let Some(c) = color {
        label.set_color(c);
    }
    
    Ok(LuaWidget::new(Box::new(label)))
}
```

### Event Handler Storage

```rust
/// Stores Lua event handlers for widgets
pub struct LuaEventHandlers {
    handlers: HashMap<(WidgetId, String), Vec<LuaRegistryKey>>,
}

impl LuaEventHandlers {
    pub fn register(
        &mut self,
        lua: &Lua,
        widget_id: WidgetId,
        event: &str,
        handler: LuaFunction,
    ) -> LuaResult<HandlerId> {
        let key = lua.create_registry_value(handler)?;
        let handlers = self.handlers
            .entry((widget_id, event.to_string()))
            .or_default();
        let id = handlers.len();
        handlers.push(key);
        Ok(HandlerId(id))
    }
    
    pub fn emit(
        &self,
        lua: &Lua,
        widget_id: WidgetId,
        event: &str,
        args: impl IntoLuaMulti,
    ) -> LuaResult<()> {
        if let Some(handlers) = self.handlers.get(&(widget_id, event.to_string())) {
            for key in handlers {
                let handler: LuaFunction = lua.registry_value(key)?;
                handler.call(args.clone())?;
            }
        }
        Ok(())
    }
}
```

---

## Acceptance Criteria

### AC-1: Widget Creation
```
GIVEN a Lua script with `niri.ui.label({ text = "Hello" })`
WHEN the script executes
THEN a Label widget userdata is returned
AND the widget has text property "Hello"
```

### AC-2: Property Updates
```
GIVEN a created Label widget
WHEN `label:set("text", "Updated")` is called
THEN the widget's text property changes to "Updated"
AND the widget is marked for re-render
```

### AC-3: Event Handling
```
GIVEN a Button widget with click handler registered
WHEN the button receives a click event
THEN the Lua handler function is called
AND the event data contains x, y coordinates and button number
```

### AC-4: Window Display
```
GIVEN a window created with `niri.ui.window({ content = widget })`
WHEN `window:show()` is called
THEN the window becomes visible on the configured output(s)
AND the content widget is rendered
```

### AC-5: Popup Anchoring
```
GIVEN a popup with widget anchor
WHEN the popup is shown
THEN it positions relative to the anchor widget
AND respects the specified edge and alignment
```

### AC-6: Style State Variants
```
GIVEN a button with hover style variant
WHEN the cursor enters the button bounds
THEN the hover styles are applied
AND when cursor exits, base styles are restored
```

### AC-7: Reactive State
```
GIVEN a state object with bound widget
WHEN `state:set("value", new_value)` is called
THEN the bound widget property updates automatically
AND the widget re-renders with new value
```

---

## Test Strategy

### Unit Tests

```rust
#[test]
fn test_label_creation_from_lua() {
    let lua = create_test_lua();
    register_ui_api(&lua).unwrap();
    
    let result: LuaWidget = lua.load(r#"
        return niri.ui.label({ text = "Test", font_size = 16 })
    "#).eval().unwrap();
    
    assert_eq!(result.get_property("text"), LuaValue::String("Test".into()));
}

#[test]
fn test_event_handler_registration() {
    let lua = create_test_lua();
    register_ui_api(&lua).unwrap();
    
    lua.load(r#"
        local clicked = false
        local btn = niri.ui.button({ text = "Click" })
        btn:on("click", function() clicked = true end)
        _G.btn = btn
        _G.clicked = function() return clicked end
    "#).exec().unwrap();
    
    // Simulate click
    let btn: LuaWidget = lua.globals().get("btn").unwrap();
    btn.emit_event(&lua, "click", ()).unwrap();
    
    let clicked: bool = lua.load("return clicked()").eval().unwrap();
    assert!(clicked);
}
```

### Integration Tests

```rust
#[test]
fn test_window_creation_and_display() {
    let (runtime, ui_manager) = create_test_environment();
    
    runtime.load_string(r#"
        local panel = niri.ui.window({
            name = "test-panel",
            anchor = "top",
            content = niri.ui.label({ text = "Panel" }),
        })
        panel:show()
    "#).unwrap();
    
    assert!(ui_manager.has_window("test-panel"));
    assert!(ui_manager.get_window("test-panel").unwrap().is_visible());
}
```

---

## Error Handling

All Lua API functions follow graceful degradation (Constraint C1):

```rust
fn create_widget_safe<F>(lua: &Lua, props: LuaTable, constructor: F) -> LuaResult<LuaWidget>
where
    F: FnOnce(LuaTable) -> Result<Box<dyn Widget>, WidgetError>,
{
    match constructor(props) {
        Ok(widget) => Ok(LuaWidget::new(widget)),
        Err(e) => {
            // Log error but don't crash
            tracing::warn!("Widget creation failed: {}", e);
            // Return placeholder widget
            Ok(LuaWidget::new(Box::new(ErrorPlaceholder::new(e.to_string()))))
        }
    }
}
```

---

## File References

- Existing Lua patterns: `niri-lua/src/runtime.rs`, `niri-lua/src/config_api.rs`
- Event system: `niri-lua/src/event_system.rs`
- Type extraction: `niri-lua/src/extractors.rs`
- API schema: `niri-lua/src/api_data.rs`
