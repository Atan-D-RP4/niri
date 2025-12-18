# niri-ui Component Specifications

This directory contains detailed specifications for each component of the niri-ui widget toolkit.

## Overview

niri-ui is a Smithay-native widget toolkit integrated into the niri compositor, exposed to Lua for building panels, launchers, notifications, system trays, and OSDs.

### Design Constraints

- **C1**: UI must never crash the compositor (graceful degradation)
- **C2**: All types/implementation inside niri-ui crate, keep changes to main compositor < 50 lines
- **C3**: Aim for QtQuick-level flexibility

## Component Index

| Component | File | Description | Priority |
|-----------|------|-------------|----------|
| **Rendering Pipeline** | [rendering.md](rendering.md) | Cairo → TextureBuffer → RenderElement flow, per-output caching | P0 |
| **Animation System** | [animation.md](animation.md) | Animation/Clock types, state machines, easing curves | P0 |
| **Widget System** | [widget.md](widget.md) | Widget trait, core widgets (Label, Button, Image, etc.) | P0 |
| **Window Management** | [window.md](window.md) | UiWindow, PopupWindow, WindowManager, per-output visibility | P0 |
| **Layout Engine** | [layout-engine.md](layout-engine.md) | Flexbox-inspired two-pass layout algorithm | P0 |
| **Styling System** | [styling-system.md](styling-system.md) | Style types, state variants, StyleResolver | P1 |
| **Lua Bindings** | [lua_bindings.md](lua_bindings.md) | niri.ui namespace, widget factories, reactive patterns | P0 |
| **Service Layer** | [service-layer.md](service-layer.md) | D-Bus integration, system tray, notifications | P1 |
| **Compositor Integration** | [compositor-integration.md](compositor-integration.md) | UiManager API, 5 integration points | P0 |
| **Cursor Effects** | [cursor-effects.md](cursor-effects.md) | ClickRipple, CursorTrail, custom cursors | P2 |

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         Lua API Layer                           │
│  niri.ui.window()  niri.ui.label()  niri.ui.row()  niri.dbus   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Widget System                              │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐            │
│  │  Label  │  │ Button  │  │  Image  │  │   Box   │  ...       │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘            │
│       └────────────┴────────────┴────────────┘                  │
│                         │                                       │
│                    Widget Trait                                 │
└─────────────────────────────────────────────────────────────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          ▼                     ▼                     ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Layout Engine  │  │  Styling System │  │ Animation System│
│  (Two-pass)     │  │  (State-based)  │  │ (State machine) │
└─────────────────┘  └─────────────────┘  └─────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Rendering Pipeline                           │
│  Cairo/Pango → ImageSurface → TextureBuffer → RenderElement    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Compositor Integration                        │
│  UiManager → render_elements() → Smithay → GPU                 │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow

1. **Configuration**: Lua scripts define widgets, styles, and event handlers
2. **Layout**: Two-pass algorithm computes sizes and positions
3. **Styling**: StyleResolver merges base styles with state variants
4. **Animation**: State machine manages visibility/property transitions
5. **Rendering**: Cairo draws to ImageSurface, uploaded to GPU texture
6. **Composition**: UiManager provides render elements to compositor

## Data Contracts

The following primary data contracts are used across components:

- ImageData: { width: u32, height: u32, stride: u32, scale: f64, format: "ARGB8888", premultiplied: bool, data: Vec<u8> } — produced by niri-ui rendering and uploaded by the compositor to GPU textures.

- TrayItem: { id: String, title: String, icon_name: Option<String>, icon_pixmap: Option<IconPixmap>, tooltip: Option<String>, category: TrayCategory, status: TrayStatus, has_menu: bool }

- Notification: { id: u32, app_name: String, app_icon: Option<String>, summary: String, body: String, actions: Vec<(String,String)>, urgency: Urgency, timeout_ms: i32, timestamp: Instant, image: Option<NotificationImage>, hints: Map<String,Value> }

- Signal: { source: String, name: String, payload: serde_json::Value }

Render element contract: niri-ui should produce ImageData (preferred) and the compositor is responsible for uploading ImageData to a TextureBuffer in ARGB8888 format; helper helpers exist for upload but prefer the compositor to manage GPU resources and memory.

## Accessibility & Robustness

Accessibility and robustness are first-class concerns.

- Accessibility contract: every window and widget SHOULD expose the following minimal a11y attributes: `role` (string), `name` (string), `description` (optional), `actions` (list of action ids and labels), `focused` (bool), `enabled` (bool). The implementation SHOULD map these to platform accessibility APIs (AT-SPI where available) and provide Lua helpers to query and emit accessibility events.

- Error handling & isolation: user-provided Lua handlers and widget code MUST be sandboxed so that panics or runtime errors do not crash the compositor. All Lua errors should be caught, logged, and converted into safe fallbacks (e.g., draw an error placeholder and continue). Provide a diagnostic mode that surfaces recoverable errors in a visible developer overlay.

- Resource budgets: the compositor SHOULD be able to set per-window/widget CPU and memory budgets (configurable). The UI system SHOULD enforce soft limits and gracefully degrade (throttle animations, drop expensive snapshots) rather than fail hard.

- Security: D-Bus interactions MUST follow least privilege: only necessary methods and signals are called; services with elevated privileges require explicit opt-in.

## Quickshell/QtQuick Comparison

This section maps Quickshell/QtQuick features to niri-ui design decisions and highlights capability gaps and priorities:

- Per-screen windows: Quickshell's "Variants" maps to niri-ui's `output` parameter for window creation (`WindowConfig::output`).
- Layer-shell positioning: Quickshell `PanelWindow` maps to niri-ui's `UiWindow` anchored with `WindowAnchor` and exclusive zones.
- Popup windows: Quickshell `PopupWindow` capability is implemented by `PopupWindow` with `PopupAnchor` variants (widget, cursor, position, screen). Popups must support stacking, auto-flip, and fast show/hide animations (100–200ms).
- Hot reload & plugin system: Quickshell hot-reload (EngineGeneration) and plugin systems are powerful features; in niri-ui these are planned as post-v1 features (hot-reload P2, plugins P2).
- Service layer: Quickshell exposes services (SystemTray, Notifications, MPRIS) as reactive singletons; niri-ui uses a zbus-based `ServiceManager` with subscription-based events and Lua-friendly bindings.

Capability gaps:
- PopupWindow: addressed in this spec (Phase 3).
- Hot reload and plugin system: planned post-v1 (P2).

## Key Patterns

### Per-Output Texture Caching
```rust
struct CachedTexture {
    texture: TextureBuffer,
    scale: smithay::output::Scale,
    size: Size<i32, Logical>,
}
textures: HashMap<WeakOutput, CachedTexture>
```

### Animation State Machine
```rust
enum VisibilityState {
    Hidden,
    Showing(Animation),
    Shown(Option<Duration>),  // auto-hide timeout
    Hiding(Animation),
}
```

### Widget Trait
```rust
trait Widget: Send + Sync {
    fn measure(&mut self, ctx: &LayoutContext, constraints: Constraints) -> Size<f64, Logical>;
    fn layout(&mut self, ctx: &LayoutContext, size: Size<f64, Logical>);
    fn render(&self, ctx: &mut RenderContext);
    fn handle_event(&mut self, event: &UiEvent) -> EventResult;
}
```

## Observability & Versioning

### Observability & Metrics

- Metrics to track: render_time_ms, upload_time_ms, cache_memory_bytes, cache_entry_count, fps, layout_time_ms.
- Target budgets: 16ms/frame (60fps); recommended render_time_ms <= 10ms, upload_time_ms <= 4ms. Track and surface budget violations via compositor instrumentation.
- Expose metrics via the compositor's debug or telemetry endpoint; tests should assert on budgets in CI where possible.
- Font/Pango layout caching: cache PangoLayout objects keyed by (text, font_description, scale) to reduce layout CPU time and allocations.

### Versioning & Migration

- Versioning: Start with `niri.ui` v1 for the initial Lua API. Annotate major API and spec changes with `Since` notes (e.g., "Since v1.0").
- Deprecation policy: Keep deprecated Lua APIs as compatibility shims for one minor release and provide migration guidance in the spec and docs.
- Migration helpers: Provide high-level migration utilities in `niri-lua` to ease transitions across versions.

## Implementation Order

### Phase 1: Core Foundation (P0)
1. Rendering pipeline with per-output caching
2. Basic Widget trait and WidgetBase
3. Layout engine (Row, Column, Box)
4. UiWindow and WindowManager
5. Compositor integration points

### Phase 2: Widget Library (P0-P1)
1. Label, Button, Image widgets
2. Animation system integration
3. Styling system with state variants
4. Lua bindings for core widgets

### Phase 3: Services (P1)
1. D-Bus manager
2. System tray (StatusNotifierItem)
3. Notification service
4. Popup windows

### Phase 4: Polish (P2)
1. Advanced widgets (Slider, CircularProgress, Revealer)
2. Cursor effects
3. Performance optimization
4. Additional input methods (touch, tablet)

## Testing Strategy

Each spec includes:
- **Unit tests**: Individual component behavior
- **Integration tests**: Component interactions
- **Snapshot tests**: Visual regression with insta
- **Property tests**: Layout algorithm invariants

## Related Documentation

- [niri-ui Brief](../../brief/niri-ui-brief.md) - Original project brief
- [niri-lua AGENTS.md](../../../../niri-lua/AGENTS.md) - Lua binding patterns
- [src/ui/](../../../../src/ui/) - Existing UI implementations (hotkey_overlay, etc.)
