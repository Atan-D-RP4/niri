# Niri UI Module Specification

**Version**: 0.1.0 (Draft)  
**Status**: Design Phase  
**Target**: niri 25.12+  
**Crate**: `niri-ui`

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Goals and Non-Goals](#2-goals-and-non-goals)
3. [Architecture Overview](#3-architecture-overview)
4. [Widget System](#4-widget-system)
5. [Rendering Pipeline](#5-rendering-pipeline)
6. [Layout Engine](#6-layout-engine)
7. [Styling System](#7-styling-system)
8. [Service Layer](#8-service-layer)
9. [Lua API](#9-lua-api)
10. [Integration with Compositor](#10-integration-with-compositor)
11. [Safety and Stability](#11-safety-and-stability)
12. [Implementation Roadmap](#12-implementation-roadmap)
13. [API Reference](#13-api-reference)
14. [Examples](#14-examples)
15. [Cursor Effects](#15-cursor-effects)
16. [Shell Project Analysis](#16-shell-project-analysis)
17. [Appendix A: Comparison with Astrum](#appendix-a-comparison-with-astrum)
18. [Appendix B: Comparison with QtQuick/Shell](#appendix-b-comparison-with-qtquickshell)
19. [Appendix C: Comparison with Pinnacle/Snowcap](#appendix-c-comparison-with-pinnaclesnowcap)
20. [Appendix D: Main Compositor Changes Summary](#appendix-d-main-compositor-changes-summary)
21. [Appendix E: File Checklist](#appendix-e-file-checklist)

---

## 1. Executive Summary

### 1.1 What is niri-ui?

`niri-ui` is a Smithay-native widget toolkit integrated into the niri compositor, exposed to Lua for building desktop shell components such as:

- Status bars and panels
- Application launchers
- Notification centers
- System trays
- On-screen displays (OSD)
- Custom overlays

### 1.2 Design Philosophy

1. **Lean and Focused**: Minimal dependencies, built on existing niri infrastructure
2. **Compositor-First**: Tight integration with niri's rendering and event systems
3. **Lua-Driven**: User-configurable UI through Lua scripting
4. **Stability-Oriented**: UI crashes must never take down the compositor
5. **Smithay-Native**: Uses niri's existing rendering pipeline (Cairo/Pango → GlowRenderer)
6. **Upstream-Friendly**: Minimal changes to main compositor code for easy upstream merging
7. **QtQuick-Level Flexibility**: Aim for capability parity with QtQuick/Shell for full desktop shell development

### 1.3 Critical Design Constraints

These constraints are non-negotiable and must be respected throughout implementation:

#### C1: Layer-Shell Must Not Break Compositor

The UI layer-shell system must be completely isolated from core compositor functionality:

- UI windows are **additive only** - they don't modify existing layer-shell behavior
- Compositor continues to function normally even if all UI windows fail
- UI rendering failures result in graceful degradation, not compositor crashes
- Input routing to UI windows must not interfere with regular window input handling
- UI windows must respect the existing layer-shell protocol semantics

#### C2: Minimal Main Compositor Changes

To facilitate easy merging of upstream niri changes:

- **All types, traits, and implementations must be defined in `niri-ui/`**
- Main compositor code (`src/`) should only:
  - Import and instantiate `niri_ui` types
  - Call into `niri_ui` API at specific integration points
  - Never contain UI-specific logic
- Integration points should be clearly marked and minimal
- Use trait objects and callbacks to avoid coupling

#### C3: QtQuick/Shell-Level Flexibility

The system must be capable enough to build desktop shells comparable to:

- **Noctalia Shell** (KDE Plasma shell)
- **DankMaterialShell** 
- **QtQuick-based shells**

This requires:
- Arbitrary widget composition and nesting
- Complex layout capabilities
- Animation system
- Full D-Bus integration for system services
- Extensibility through Lua modules

### 1.4 Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| UI Framework | Smithay-native | Zero new deps, perfect integration |
| Architecture | Integrated (single binary) | Shared Lua runtime, direct state access |
| Styling | Named styles (Lua tables) | Themeable, future CSS support |
| Lua API | Table configuration | Simple, declarative, familiar |
| Services | D-Bus in Rust, exposed to Lua | Reliability + flexibility |
| Async | Calloop (no Tokio) | Consistent with niri |

---

## 2. Goals and Non-Goals

### 2.1 Goals

- **G1**: Create layer-shell windows from Lua
- **G2**: Provide core widget set (container, row, column, text, button, image, space)
- **G3**: Implement flexbox-inspired layout engine
- **G4**: Support named styles with state variants (normal, hover, active)
- **G5**: Expose D-Bus primitives for service integration
- **G6**: Integrate with niri's existing event system
- **G7**: Support animations consistent with niri's animation system
- **G8**: Enable full desktop shell development (comparable to Noctalia, QtQuick/Shell)
- **G9**: Maintain minimal changes to main compositor code (<50 lines total)
- **G10**: Layer-shell UI must not interfere with compositor stability or external layer-shell clients

### 2.2 Non-Goals

- **NG1**: General-purpose GUI toolkit (not for standalone apps)
- **NG2**: HTML/CSS rendering engine
- **NG3**: Complex text editing (beyond simple input fields)
- **NG4**: 3D graphics or game-oriented rendering
- **NG5**: Backward compatibility with Astrum or other toolkits

### 2.3 Future Goals (Post-v1)

- **FG1**: Embedded views (webview, external renderers)
- **FG2**: CSS file import for styling
- **FG3**: Visual designer tool
- **FG4**: Animation DSL

---

## 3. Architecture Overview

### 3.1 Crate Structure

```
niri-ui/
├── Cargo.toml
├── AGENTS.md
├── src/
│   ├── lib.rs                 # Public API
│   ├── window.rs              # Layer-shell window management
│   ├── widget.rs              # Widget trait and base types
│   ├── widgets/               # Widget implementations
│   │   ├── mod.rs
│   │   ├── container.rs       # Box with background/border
│   │   ├── row.rs             # Horizontal flex layout
│   │   ├── column.rs          # Vertical flex layout
│   │   ├── text.rs            # Text label
│   │   ├── button.rs          # Clickable button
│   │   ├── image.rs           # Image/icon display
│   │   ├── space.rs           # Flexible/fixed spacing
│   │   ├── scroll.rs          # Scrollable container
│   │   └── input.rs           # Text input (future)
│   ├── layout/                # Layout algorithms
│   │   ├── mod.rs
│   │   ├── flex.rs            # Flexbox-inspired layout
│   │   └── constraints.rs     # Size constraints
│   ├── render/                # Rendering
│   │   ├── mod.rs
│   │   ├── painter.rs         # Cairo painting context
│   │   ├── texture_cache.rs   # Per-output texture caching
│   │   └── elements.rs        # Smithay render elements
│   ├── style/                 # Styling system
│   │   ├── mod.rs
│   │   ├── types.rs           # Style types (Color, Border, etc.)
│   │   ├── resolver.rs        # Style resolution with states
│   │   └── theme.rs           # Theme management
│   ├── input/                 # Input handling
│   │   ├── mod.rs
│   │   ├── pointer.rs         # Mouse/touch events
│   │   └── keyboard.rs        # Keyboard events
│   ├── services/              # D-Bus and system services
│   │   ├── mod.rs
│   │   ├── dbus.rs            # D-Bus connection management
│   │   ├── tray.rs            # System tray (SNI)
│   │   └── notifications.rs   # Notification daemon
│   ├── lua_bindings/          # Lua API exposure
│   │   ├── mod.rs
│   │   ├── window.rs          # niri.ui.window()
│   │   ├── widgets.rs         # niri.ui.* widgets
│   │   ├── styles.rs          # Style definitions
│   │   ├── services.rs        # Service subscriptions
│   │   └── types.rs           # Lua ↔ Rust type conversion
│   └── error.rs               # Error types
```

### 3.2 Dependency Graph

```
                    ┌─────────────┐
                    │    niri     │ (compositor binary)
                    └──────┬──────┘
                           │ uses
           ┌───────────────┼───────────────┐
           │               │               │
           ▼               ▼               ▼
    ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
    │ niri-config │ │  niri-lua   │ │  niri-ipc   │
    └─────────────┘ └──────┬──────┘ └─────────────┘
                           │ uses
                           ▼
                    ┌─────────────┐
                    │   niri-ui   │ (NEW)
                    └─────────────┘
                           │ uses
           ┌───────────────┼───────────────┐
           │               │               │
           ▼               ▼               ▼
      [smithay]       [pango]         [zbus]
    (rendering)    (text layout)     (D-Bus)
```

### 3.3 Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                         Lua Script                              │
│  niri.ui.window({ view = function() return widgets end })       │
└────────────────────────────┬────────────────────────────────────┘
                             │ (1) view() called
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Widget Tree (Rust)                         │
│  Container → Row → [Text, Button, Space, Text]                  │
└────────────────────────────┬────────────────────────────────────┘
                             │ (2) layout()
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Layout Pass                                │
│  Constraints → Measure → Position                               │
└────────────────────────────┬────────────────────────────────────┘
                             │ (3) paint()
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Cairo Surface                              │
│  ImageSurface (ARGB32)                                          │
└────────────────────────────┬────────────────────────────────────┘
                             │ (4) upload texture
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      TextureBuffer                              │
│  Cached per output/scale                                        │
└────────────────────────────┬────────────────────────────────────┘
                             │ (5) render element
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      GlowRenderer                               │
│  Composited with other windows                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Widget System

### 4.1 Widget Trait

```rust
/// Core widget trait - all widgets implement this
pub trait Widget: Send + Sync {
    /// Unique identifier for this widget instance
    fn id(&self) -> WidgetId;
    
    /// Calculate preferred size given constraints
    fn layout(&mut self, ctx: &mut LayoutContext, constraints: Constraints) -> Size;
    
    /// Paint the widget to the given painter
    fn paint(&self, ctx: &mut PaintContext, bounds: Rectangle);
    
    /// Handle input event, return signal if emitted
    fn handle_event(&mut self, event: WidgetEvent, bounds: Rectangle) -> Option<Signal>;
    
    /// Child widgets (for tree traversal)
    fn children(&self) -> &[Box<dyn Widget>];
    
    /// Mutable child access (for layout)
    fn children_mut(&mut self) -> &mut [Box<dyn Widget>];
}
```

### 4.2 Core Widgets

#### Container
A box that can have background, border, padding, and contain one child.

```rust
pub struct Container {
    pub child: Option<Box<dyn Widget>>,
    pub style: ContainerStyle,
    // Resolved at layout time
    computed_bounds: Rectangle,
}

pub struct ContainerStyle {
    pub background: Option<Background>,
    pub border: Option<BorderStyle>,
    pub border_radius: Option<f64>,
    pub padding: Padding,
    pub margin: Margin,
    pub min_width: Option<f64>,
    pub min_height: Option<f64>,
    pub max_width: Option<f64>,
    pub max_height: Option<f64>,
}
```

#### Row
Horizontal flex container with configurable spacing and alignment.

```rust
pub struct Row {
    pub children: Vec<Box<dyn Widget>>,
    pub spacing: f64,
    pub main_axis_alignment: MainAxisAlignment,  // start, center, end, space_between, space_around
    pub cross_axis_alignment: CrossAxisAlignment, // start, center, end, stretch
}
```

#### Column
Vertical flex container (same API as Row).

```rust
pub struct Column {
    pub children: Vec<Box<dyn Widget>>,
    pub spacing: f64,
    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
}
```

#### Text
Text label with styling.

```rust
pub struct Text {
    pub content: String,
    pub style: TextStyle,
}

pub struct TextStyle {
    pub font_family: Option<String>,
    pub font_size: f64,
    pub font_weight: FontWeight,
    pub color: Color,
    pub line_height: Option<f64>,
    pub ellipsize: Ellipsize,  // none, start, middle, end
    pub max_width: Option<f64>,
}
```

#### Button
Clickable container that emits signals.

```rust
pub struct Button {
    pub child: Box<dyn Widget>,
    pub style: ButtonStyle,
    pub signal: String,  // Signal name to emit on click
    pub signal_data: Option<LuaValue>,  // Optional data to pass
    // State
    state: ButtonState,  // Normal, Hovered, Pressed
}

pub struct ButtonStyle {
    pub normal: ContainerStyle,
    pub hover: Option<ContainerStyle>,
    pub active: Option<ContainerStyle>,
}
```

#### Image
Image or icon display.

```rust
pub struct Image {
    pub source: ImageSource,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub scale_mode: ScaleMode,  // fit, fill, stretch, none
}

pub enum ImageSource {
    Path(PathBuf),
    Icon(String),  // Icon name from theme
    Svg(String),   // Inline SVG
    Cached(TextureId),
}
```

#### Space
Flexible or fixed spacing.

```rust
pub struct Space {
    pub width: SpaceSize,
    pub height: SpaceSize,
}

pub enum SpaceSize {
    Fixed(f64),
    Flexible(f64),  // flex factor
}
```

### 4.3 Widget Tree

Widgets form a tree structure owned by `UiWindow`:

```rust
pub struct UiWindow {
    id: WindowId,
    name: String,
    layer_surface: LayerSurface,
    settings: LayerSettings,
    
    // Widget tree
    root: Box<dyn Widget>,
    
    // Cached render state
    texture_cache: OutputMap<TextureBuffer>,
    needs_layout: bool,
    needs_paint: bool,
    
    // Lua references
    view_function: RegistryKey,
    signal_handlers: HashMap<String, RegistryKey>,
    
    // Styles
    styles: HashMap<String, Style>,
}
```

---

## 5. Rendering Pipeline

### 5.1 Reference: Existing Niri Pattern

From `src/ui/hotkey_overlay.rs`, niri uses this pattern for compositor-internal UI:

```rust
// 1. Create Cairo ImageSurface
let surface = ImageSurface::create(Format::ARgb32, width, height)?;
let cr = cairo::Context::new(&surface)?;

// 2. Draw with Cairo/Pango
cr.set_source_rgba(r, g, b, a);
cr.rectangle(x, y, w, h);
cr.fill()?;

let layout = pangocairo::create_layout(&cr);
layout.set_text("Hello");
pangocairo::show_layout(&cr, &layout);

// 3. Convert to Smithay texture
drop(cr);
let data = surface.take_data()?;
let buffer = TextureBuffer::from_memory(
    renderer,
    &data,
    Fourcc::Argb8888,
    (width, height),
    false,
    scale,
    Transform::Normal,
    None,
)?;

// 4. Create render element
let element = TextureRenderElement::from_texture_buffer(
    buffer,
    location,
    alpha,
    None,
    None,
    Kind::Unspecified,
);
```

**Key patterns from hotkey overlay that niri-ui should follow:**

1. **Per-output/per-scale caching**: Textures are cached with `OutputMap<TextureBuffer>`
2. **Animation state machine**: Uses `enum State { Hidden, Showing, Shown, Hiding }` with `Clock`
3. **Render returns Option**: `fn render(&self, output) -> Option<RenderElement>` - skip if hidden
4. **No Wayland protocol**: Hotkey overlay is compositor-internal, not a layer-shell client
5. **Pango for text**: Uses `pangocairo` for all text rendering with proper font handling

**This pattern is directly applicable to niri-ui** - we're building more sophisticated versions of what hotkey_overlay and config_error_notification already do.

### 5.2 Niri-UI Rendering Abstraction

```rust
/// High-level painter that wraps Cairo
pub struct Painter<'a> {
    cr: &'a cairo::Context,
    pango: pango::Context,
    scale: f64,
    bounds: Rectangle,
}

impl<'a> Painter<'a> {
    /// Fill rectangle with color
    pub fn fill_rect(&self, rect: Rectangle, color: Color);
    
    /// Draw rounded rectangle
    pub fn rounded_rect(&self, rect: Rectangle, radii: BorderRadius, color: Color);
    
    /// Draw border
    pub fn stroke_rect(&self, rect: Rectangle, border: &BorderStyle);
    
    /// Draw text
    pub fn draw_text(&self, text: &str, style: &TextStyle, bounds: Rectangle);
    
    /// Draw image
    pub fn draw_image(&self, source: &ImageSource, bounds: Rectangle, scale: ScaleMode);
    
    /// Push clip region
    pub fn push_clip(&self, rect: Rectangle);
    
    /// Pop clip region
    pub fn pop_clip(&self);
}
```

### 5.3 Texture Caching

Following niri's pattern, textures are cached per-output and per-scale:

```rust
pub struct TextureCache {
    /// Map from (output_id, scale) to cached texture
    textures: HashMap<(OutputId, OrderedFloat<f64>), CachedTexture>,
}

pub struct CachedTexture {
    buffer: TextureBuffer,
    size: Size,
    last_used: Instant,
}

impl TextureCache {
    /// Get or create texture for output
    pub fn get_or_create<F>(
        &mut self,
        output: &Output,
        size: Size,
        paint_fn: F,
    ) -> &TextureBuffer
    where
        F: FnOnce(&mut Painter),
    {
        let key = (output.id(), output.current_scale());
        
        if !self.textures.contains_key(&key) || self.needs_repaint {
            // Create new surface and paint
            let surface = self.create_surface(size, output.current_scale());
            let cr = cairo::Context::new(&surface)?;
            let mut painter = Painter::new(&cr, output.current_scale());
            paint_fn(&mut painter);
            
            // Upload texture
            let buffer = self.upload_texture(&surface, renderer)?;
            self.textures.insert(key, CachedTexture::new(buffer, size));
        }
        
        &self.textures.get(&key).unwrap().buffer
    }
}
```

### 5.4 Animation Integration

UI windows integrate with niri's animation system:

```rust
pub struct AnimatedValue {
    current: f64,
    target: f64,
    animation: Option<Animation>,
}

impl UiWindow {
    /// Check if any animations are ongoing
    pub fn are_animations_ongoing(&self, clock: &Clock) -> bool {
        // Check widget animations
        self.check_animations_recursive(&self.root, clock)
    }
    
    /// Advance animations
    pub fn advance_animations(&mut self, clock: &Clock) {
        // Update animated values
        // Mark needs_paint if changed
    }
}
```

---

## 6. Layout Engine

### 6.1 Flexbox-Inspired Model

The layout engine uses a simplified flexbox model:

```rust
/// Constraints passed down during layout
pub struct Constraints {
    pub min_width: f64,
    pub max_width: f64,
    pub min_height: f64,
    pub max_height: f64,
}

impl Constraints {
    pub fn unbounded() -> Self;
    pub fn tight(size: Size) -> Self;
    pub fn loose(max: Size) -> Self;
}

/// Size returned from layout
pub struct Size {
    pub width: f64,
    pub height: f64,
}
```

### 6.2 Layout Algorithm (Two-Pass)

**Pass 1: Measure**
```rust
fn layout(&mut self, ctx: &mut LayoutContext, constraints: Constraints) -> Size {
    // For Row:
    let mut remaining_width = constraints.max_width;
    let mut max_height = 0.0;
    let mut flex_total = 0.0;
    
    // First pass: measure non-flex children
    for child in &mut self.children {
        if child.is_flexible() {
            flex_total += child.flex_factor();
        } else {
            let child_size = child.layout(ctx, Constraints::loose(remaining_width, f64::INFINITY));
            remaining_width -= child_size.width + self.spacing;
            max_height = max_height.max(child_size.height);
        }
    }
    
    // Second pass: distribute remaining space to flex children
    let flex_space = remaining_width / flex_total;
    for child in &mut self.children {
        if child.is_flexible() {
            let child_width = flex_space * child.flex_factor();
            let child_size = child.layout(ctx, Constraints::tight(child_width, max_height));
        }
    }
    
    Size { width: constraints.max_width, height: max_height }
}
```

**Pass 2: Position**
```rust
fn position_children(&mut self, bounds: Rectangle) {
    let mut x = bounds.x + self.padding.left;
    
    for child in &mut self.children {
        child.set_position(Point { x, y: bounds.y + self.padding.top });
        x += child.size().width + self.spacing;
    }
}
```

### 6.3 Layout Context

```rust
pub struct LayoutContext<'a> {
    /// Pango context for text measurement
    pub pango: &'a pango::Context,
    
    /// Current scale factor
    pub scale: f64,
    
    /// Style resolver
    pub styles: &'a StyleResolver,
}
```

---

## 7. Styling System

### 7.1 Style Definition (Lua)

```lua
niri.ui.window({
    name = "panel",
    
    styles = {
        -- Named style
        panel_bg = {
            background = "#1e1e2e",
            padding = { 8, 16 },  -- vertical, horizontal
            border = {
                width = 1,
                color = "#313244",
                radius = 12,
            },
        },
        
        -- Style with state variants
        workspace_button = {
            background = "transparent",
            padding = { 4, 12 },
            border = { radius = 8 },
            
            -- State variants (merged with base)
            hover = {
                background = "#313244",
            },
            active = {
                background = "#45475a",
            },
        },
        
        -- Text style
        time_text = {
            color = "#cdd6f4",
            font_size = 14,
            font_weight = "bold",
        },
    },
    
    view = function()
        return niri.ui.container({
            style = "panel_bg",
            child = niri.ui.text({
                content = os.date("%H:%M"),
                style = "time_text",
            })
        })
    end,
})
```

### 7.2 Style Types (Rust)

```rust
/// Complete style definition
pub struct Style {
    // Container properties
    pub background: Option<Background>,
    pub border: Option<BorderStyle>,
    pub padding: Option<Padding>,
    pub margin: Option<Margin>,
    
    // Text properties
    pub color: Option<Color>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<FontWeight>,
    
    // State variants
    pub hover: Option<Box<Style>>,
    pub active: Option<Box<Style>>,
    pub focused: Option<Box<Style>>,
    pub disabled: Option<Box<Style>>,
}

pub enum Background {
    Solid(Color),
    Gradient(Gradient),
}

pub struct BorderStyle {
    pub width: f64,
    pub color: Color,
    pub radius: BorderRadius,
}

pub struct BorderRadius {
    pub top_left: f64,
    pub top_right: f64,
    pub bottom_left: f64,
    pub bottom_right: f64,
}

pub struct Padding {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}
```

### 7.3 Style Resolution

```rust
pub struct StyleResolver {
    styles: HashMap<String, Style>,
}

impl StyleResolver {
    /// Resolve style by name with state
    pub fn resolve(&self, name: &str, state: WidgetState) -> ResolvedStyle {
        let base = self.styles.get(name).cloned().unwrap_or_default();
        
        // Merge state variant
        let merged = match state {
            WidgetState::Normal => base,
            WidgetState::Hovered => base.merge_with(base.hover.as_deref()),
            WidgetState::Active => base.merge_with(base.active.as_deref()),
            WidgetState::Focused => base.merge_with(base.focused.as_deref()),
            WidgetState::Disabled => base.merge_with(base.disabled.as_deref()),
        };
        
        ResolvedStyle::from(merged)
    }
}
```

### 7.4 Future: CSS Import

```lua
-- Future syntax
niri.ui.window({
    name = "panel",
    stylesheet = "~/.config/niri/panel.css",
    -- ...
})
```

---

## 8. Service Layer

### 8.1 Philosophy

- **D-Bus integration in Rust**: Reliable, type-safe
- **Exposed to Lua**: Flexible, user-customizable
- **Services implemented in Lua where possible**: Maximum flexibility
- **Rust-only where required**: System tray, notification daemon

### 8.2 D-Bus Foundation (Rust)

```rust
/// D-Bus connection manager
pub struct DbusManager {
    session: Connection,
    system: Connection,
}

impl DbusManager {
    /// Create proxy for a D-Bus interface
    pub async fn proxy<'a, P: From<Proxy<'a>>>(
        &'a self,
        bus: Bus,
        destination: &str,
        path: &str,
    ) -> Result<P>;
    
    /// Watch for name owner changes
    pub fn watch_name(&self, name: &str, callback: impl Fn(bool));
    
    /// Call method
    pub async fn call<R>(
        &self,
        bus: Bus,
        destination: &str,
        path: &str,
        interface: &str,
        method: &str,
        args: impl Serialize,
    ) -> Result<R>;
    
    /// Get property
    pub async fn get_property<T>(
        &self,
        bus: Bus,
        destination: &str,
        path: &str,
        interface: &str,
        property: &str,
    ) -> Result<T>;
}
```

### 8.3 Lua D-Bus API

```lua
-- Query MPRIS players
local players = niri.dbus.list_names("session", "org.mpris.MediaPlayer2.*")

for _, player in ipairs(players) do
    local metadata = niri.dbus.get_property(
        "session",
        player,
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
        "Metadata"
    )
    print(metadata["xesam:title"])
end

-- Subscribe to signal
niri.dbus.subscribe(
    "session",
    "org.freedesktop.DBus.Properties",
    "PropertiesChanged",
    function(interface, changed, invalidated)
        if interface == "org.mpris.MediaPlayer2.Player" then
            niri.ui.redraw("media_widget")
        end
    end
)

-- Call method
niri.dbus.call(
    "session",
    "org.mpris.MediaPlayer2.spotify",
    "/org/mpris/MediaPlayer2",
    "org.mpris.MediaPlayer2.Player",
    "PlayPause"
)
```

### 8.4 Rust-Only Services

#### System Tray (StatusNotifierItem)

Must be implemented in Rust because it requires:
1. Hosting `org.kde.StatusNotifierWatcher` D-Bus service
2. Handling `org.kde.StatusNotifierItem` registrations
3. Managing tray item lifecycle

```rust
pub struct TrayService {
    watcher: StatusNotifierWatcher,
    items: Vec<TrayItem>,
    event_sender: Sender<TrayEvent>,
}

pub struct TrayItem {
    pub id: String,
    pub title: String,
    pub icon: Icon,
    pub menu: Option<DbusMenu>,
    pub status: TrayStatus,
}

// Exposed to Lua
impl TrayService {
    /// Get all tray items
    pub fn items(&self) -> &[TrayItem];
    
    /// Activate item (left click)
    pub fn activate(&self, id: &str, x: i32, y: i32);
    
    /// Show context menu
    pub fn context_menu(&self, id: &str, x: i32, y: i32);
}
```

Lua API:
```lua
local items = niri.tray.items()

for _, item in ipairs(items) do
    print(item.id, item.title)
end

-- React to tray changes
niri.events:on("tray:item_added", function(item)
    niri.ui.redraw("tray_widget")
end)
```

#### Notification Daemon

Must be implemented in Rust because it requires:
1. Hosting `org.freedesktop.Notifications` D-Bus service
2. Handling notification methods (Notify, CloseNotification)
3. Managing notification lifecycle and timeouts

```rust
pub struct NotificationService {
    notifications: Vec<Notification>,
    event_sender: Sender<NotificationEvent>,
}

pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub icon: Option<Icon>,
    pub actions: Vec<(String, String)>,
    pub urgency: Urgency,
    pub timeout: Duration,
}

// Exposed to Lua
impl NotificationService {
    /// Get active notifications
    pub fn list(&self) -> &[Notification];
    
    /// Dismiss notification
    pub fn dismiss(&self, id: u32);
    
    /// Invoke action
    pub fn invoke_action(&self, id: u32, action: &str);
}
```

Lua API:
```lua
-- React to new notification
niri.events:on("notification:new", function(notification)
    -- Custom handling
    if notification.urgency == "critical" then
        niri.action.spawn("play-sound", "/usr/share/sounds/critical.wav")
    end
    niri.ui.redraw("notification_center")
end)

-- Query notifications
local notifications = niri.notifications.list()
```

### 8.5 Lua-Implementable Services

These can be fully implemented in Lua using timers and D-Bus:

```lua
-- Battery service (example implementation)
local battery = {
    percent = 0,
    status = "unknown",
    time_remaining = 0,
}

local function update_battery()
    -- Query UPower via D-Bus
    local devices = niri.dbus.call(
        "system",
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower",
        "EnumerateDevices"
    )
    
    for _, path in ipairs(devices) do
        if path:match("battery") then
            battery.percent = niri.dbus.get_property(
                "system", "org.freedesktop.UPower", path,
                "org.freedesktop.UPower.Device", "Percentage"
            )
            battery.status = niri.dbus.get_property(
                "system", "org.freedesktop.UPower", path,
                "org.freedesktop.UPower.Device", "State"
            )
        end
    end
    
    niri.ui.redraw("battery_widget")
end

-- Poll every 30 seconds
niri.loop.new_timer():after(0):every(30000):start(update_battery)

-- Export for use in widgets
return battery
```

---

## 9. Lua API

### 9.1 Module Structure

```lua
niri.ui = {
    -- Window management
    window = function(config) end,
    redraw = function(window_name) end,
    close = function(window_name) end,
    
    -- Widgets (return widget tables, not userdata)
    container = function(props) end,
    row = function(props) end,
    column = function(props) end,
    text = function(props) end,
    button = function(props) end,
    image = function(props) end,
    space = function(props) end,
    
    -- Services (Rust-backed)
    tray = TrayService,
    notifications = NotificationService,
}

niri.dbus = {
    -- D-Bus primitives
    call = function(bus, dest, path, interface, method, ...) end,
    get_property = function(bus, dest, path, interface, property) end,
    set_property = function(bus, dest, path, interface, property, value) end,
    subscribe = function(bus, interface, signal, callback) end,
    list_names = function(bus, pattern) end,
}
```

### 9.2 Window Configuration

```lua
niri.ui.window({
    -- Identity
    name = "panel",              -- Required, unique identifier
    
    -- Layer shell settings
    layer = "top",               -- "background" | "bottom" | "top" | "overlay"
    anchor = { "top", "left", "right" },  -- Edges to anchor to
    exclusive_zone = 32,         -- Pixels to reserve, -1 for full, 0 for none
    margin = { top = 0, right = 0, bottom = 0, left = 0 },
    keyboard_interactivity = "none",  -- "none" | "exclusive" | "on_demand"
    
    -- Output targeting
    output = nil,                -- nil = all outputs, or specific output name
    
    -- Styles (named styles available in view)
    styles = {
        -- ... style definitions
    },
    
    -- View function (called on redraw)
    view = function()
        return niri.ui.container({ ... })
    end,
    
    -- Signal handlers (widget interactions)
    signals = {
        button_clicked = function(data)
            -- Handle button click
        end,
    },
    
    -- Event subscriptions (compositor events)
    events = {
        ["workspace:activate"] = function(event)
            niri.ui.redraw("panel")
        end,
    },
    
    -- Subscriptions (service updates)
    subscriptions = {
        tray = true,          -- Subscribe to tray updates
        notifications = true, -- Subscribe to notification updates
    },
})
```

### 9.3 Widget Props

```lua
-- Container
niri.ui.container({
    style = "style_name",     -- Named style
    child = widget,           -- Single child widget
    
    -- Or inline styles (merged with named style)
    background = "#1e1e2e",
    padding = 8,              -- All sides
    -- padding = { 8, 16 },   -- Vertical, horizontal
    -- padding = { 8, 16, 8, 16 },  -- Top, right, bottom, left
    border = {
        width = 1,
        color = "#313244",
        radius = 12,
    },
})

-- Row
niri.ui.row({
    style = "style_name",
    children = { widget1, widget2, widget3 },
    spacing = 8,
    align = "center",         -- Cross-axis: "start" | "center" | "end" | "stretch"
    justify = "space-between", -- Main-axis: "start" | "center" | "end" | "space-between" | "space-around"
})

-- Column (same as Row but vertical)
niri.ui.column({
    children = { widget1, widget2 },
    spacing = 4,
})

-- Text
niri.ui.text({
    content = "Hello World",
    style = "text_style",
    
    -- Or inline
    color = "#cdd6f4",
    font_size = 14,
    font_weight = "bold",    -- "normal" | "bold" | number (100-900)
    font_family = "monospace",
    ellipsize = "end",       -- "none" | "start" | "middle" | "end"
    max_width = 200,
})

-- Button
niri.ui.button({
    child = niri.ui.text({ content = "Click me" }),
    style = "button_style",
    signal = "button_clicked",
    signal_data = { action = "toggle" },
})

-- Image
niri.ui.image({
    -- One of:
    path = "/path/to/image.png",
    icon = "firefox",         -- Icon name from theme
    svg = "<svg>...</svg>",   -- Inline SVG
    
    width = 24,
    height = 24,
    scale = "fit",            -- "fit" | "fill" | "stretch" | "none"
})

-- Space
niri.ui.space({
    width = 8,                -- Fixed width
    -- or
    flex = 1,                 -- Flexible, takes remaining space
})
```

### 9.4 Reactive Patterns

```lua
-- State management
local state = {
    active_workspace = 1,
    time = "",
    battery = { percent = 100, status = "full" },
}

-- View function is called on every redraw
local function view()
    return niri.ui.row({
        children = {
            -- Workspaces
            niri.ui.row({
                children = build_workspace_buttons(state.active_workspace),
            }),
            
            -- Flexible space
            niri.ui.space({ flex = 1 }),
            
            -- Right side
            niri.ui.row({
                children = {
                    niri.ui.text({ content = state.battery.percent .. "%" }),
                    niri.ui.text({ content = state.time }),
                },
                spacing = 16,
            }),
        },
    })
end

-- Update state and trigger redraw
niri.events:on("workspace:activate", function(event)
    state.active_workspace = event.workspace_id
    niri.ui.redraw("panel")
end)

niri.loop.new_timer():every(1000):start(function()
    state.time = os.date("%H:%M")
    niri.ui.redraw("panel")
end)
```

---

## 10. Integration with Compositor

### 10.0 Integration Philosophy

**Critical Requirement**: Changes to main compositor code must be minimal to allow easy merging from upstream niri.

**Principle**: All types, traits, and implementations are defined in `niri-ui/`. The main compositor only:
1. Imports `niri_ui` types
2. Instantiates `UiManager` 
3. Calls into `niri_ui` API at clearly defined integration points

### 10.1 Types Defined in niri-ui (NOT in main compositor)

All of the following are defined in `niri-ui/src/` and imported into the compositor:

```rust
// niri-ui/src/lib.rs - Public API surface

/// Main entry point - the only type compositor needs to manage
pub struct UiManager { /* ... */ }

/// Configuration for creating a UI manager
pub struct UiManagerConfig {
    pub limits: UiLimits,
}

/// Render element type (re-exported or type alias)
pub type UiRenderElement = /* ... */;

/// Input result from UI handling
pub enum UiInputResult {
    /// Input was consumed by UI
    Consumed,
    /// Input should be passed to regular handling
    NotConsumed,
    /// Input consumed, signal emitted
    Signal(Signal),
}

/// Integration trait for compositor to implement
pub trait UiCompositorInterface {
    /// Get current outputs
    fn outputs(&self) -> Vec<OutputInfo>;
    
    /// Get renderer for an output
    fn renderer(&mut self, output: &Output) -> &mut GlowRenderer;
    
    /// Schedule redraw for output
    fn queue_redraw(&mut self, output: &Output);
    
    /// Get current clock
    fn clock(&self) -> &Clock;
    
    /// Emit Lua signal
    fn emit_signal(&mut self, signal: Signal);
}
```

### 10.2 Minimal Compositor Changes

The main `niri` crate changes are confined to these minimal integration points:

```rust
// In niri/src/niri.rs - MINIMAL CHANGES

use niri_ui::{UiManager, UiManagerConfig, UiInputResult, UiCompositorInterface};

pub struct Niri {
    // Existing fields unchanged...
    
    // NEW: Single field for UI management
    pub ui: Option<UiManager>,  // Option allows graceful degradation
}

impl Niri {
    pub fn new(/* ... */) -> Self {
        // ... existing initialization ...
        
        // NEW: Initialize UI manager (2 lines)
        let ui = UiManager::new(UiManagerConfig::default())
            .map_err(|e| warn!("UI system disabled: {}", e))
            .ok();
        
        Self {
            // ... existing fields ...
            ui,
        }
    }
}

// Implement the interface trait
impl UiCompositorInterface for Niri {
    fn outputs(&self) -> Vec<OutputInfo> {
        self.layout.outputs().map(OutputInfo::from).collect()
    }
    
    fn renderer(&mut self, output: &Output) -> &mut GlowRenderer {
        &mut self.backend.renderer()
    }
    
    fn queue_redraw(&mut self, output: &Output) {
        self.queue_redraw(output);
    }
    
    fn clock(&self) -> &Clock {
        &self.clock
    }
    
    fn emit_signal(&mut self, signal: Signal) {
        // Route to Lua runtime
        self.lua_runtime.emit_signal(signal);
    }
}
```

### 10.3 Integration Points (Exhaustive List)

Only **5 integration points** in main compositor code:

#### Point 1: Initialization (niri.rs)
```rust
// In Niri::new()
let ui = UiManager::new(config).ok();
```

#### Point 2: Render Loop (niri.rs)
```rust
// In render_output() or equivalent
fn render_output(&mut self, output: &Output) -> Vec<RenderElement> {
    let mut elements = Vec::new();
    
    // Existing rendering...
    elements.extend(self.layout.render_elements(output));
    
    // NEW: Add UI elements (3 lines)
    if let Some(ref mut ui) = self.ui {
        elements.extend(ui.render_elements(self, output));
    }
    
    elements
}
```

#### Point 3: Input Handling (input/mod.rs)
```rust
// In pointer button handler
fn handle_pointer_button(&mut self, event: PointerButtonEvent) {
    // NEW: Check UI first (4 lines)
    if let Some(ref mut ui) = self.niri.ui {
        match ui.handle_pointer_button(self.pointer_position, &event) {
            UiInputResult::Consumed => return,
            UiInputResult::Signal(s) => { self.emit_signal(s); return; }
            UiInputResult::NotConsumed => {}
        }
    }
    
    // Existing input handling continues unchanged...
}
```

#### Point 4: Animation Tick (niri.rs)
```rust
// In refresh_and_flush_clients() or animation tick
fn advance_animations(&mut self) {
    // Existing animations...
    
    // NEW: Tick UI animations (2 lines)
    if let Some(ref mut ui) = self.ui {
        ui.advance_animations(self);
    }
}
```

#### Point 5: Lua Runtime Registration (niri-lua integration)
```rust
// In Lua runtime setup
fn setup_lua_ui_module(lua: &Lua, ui: &Arc<Mutex<UiManager>>) {
    // This is in niri-lua, not main compositor
    niri_ui::lua_bindings::register(lua, ui);
}
```

### 10.4 Layer-Shell Safety

UI windows use layer-shell but are isolated from normal layer-shell handling:

```rust
// In niri-ui/src/window.rs

impl UiWindow {
    /// UI windows are rendered as overlays, not as layer-shell clients
    /// This prevents any interference with external layer-shell clients
    pub fn render(&self, output: &Output, renderer: &mut GlowRenderer) -> UiRenderElement {
        // UI windows are rendered directly to textures
        // They don't use the Wayland layer-shell protocol at all
        // This is a key safety measure - we're compositor-internal
        
        let texture = self.texture_cache.get_or_create(output, || {
            self.paint_to_surface(output)
        });
        
        // Position according to layer-shell-like semantics, but internal
        let location = self.calculate_position(output);
        
        TextureRenderElement::from_texture_buffer(
            texture,
            location,
            1.0,
            None,
            None,
            Kind::Unspecified,
        )
    }
}
```

**Key insight**: UI windows are **not** Wayland layer-shell clients. They are compositor-internal overlays that **mimic** layer-shell positioning semantics. This completely isolates them from external layer-shell clients (like waybar).

### 10.5 Why This Approach Works

1. **Upstream merges**: Main compositor changes are <50 lines total, easy to resolve conflicts
2. **Feature flag possible**: Can compile without UI support via feature flag
3. **No protocol changes**: Doesn't affect Wayland protocol handling
4. **Crash isolation**: `Option<UiManager>` means UI can fail without compositor impact
5. **Clear boundaries**: All UI logic is in `niri-ui/`, none leaks into compositor

---

## 11. Safety and Stability

### 11.1 Crash Isolation

UI code must never crash the compositor:

```rust
impl UiWindowManager {
    /// Safe wrapper for Lua view function
    fn call_view_function(&self, window: &str) -> Result<WidgetTree, UiError> {
        let lua = self.lua.upgrade().ok_or(UiError::LuaDropped)?;
        let lua = lua.lock().map_err(|_| UiError::LuaLocked)?;
        
        // Call Lua in protected mode
        match lua.scope(|scope| {
            let view_fn: Function = lua.registry_value(&self.view_key)?;
            view_fn.call::<Table>(())
        }) {
            Ok(table) => self.parse_widget_tree(table),
            Err(e) => {
                // Log error, don't crash
                warn!("UI view function failed: {}", e);
                Err(UiError::ViewFunctionFailed(e))
            }
        }
    }
}
```

### 11.2 Resource Limits

```rust
pub struct UiLimits {
    /// Maximum number of UI windows
    pub max_windows: usize,  // Default: 32
    
    /// Maximum widget tree depth
    pub max_tree_depth: usize,  // Default: 64
    
    /// Maximum widgets per window
    pub max_widgets: usize,  // Default: 1000
    
    /// Maximum texture cache size per window
    pub max_texture_cache_mb: usize,  // Default: 64
}
```

### 11.3 Error Recovery

```rust
impl UiWindow {
    fn render_with_fallback(&self, output: &Output) -> RenderElement {
        match self.try_render(output) {
            Ok(element) => element,
            Err(e) => {
                // Render error indicator
                warn!("UI window '{}' render failed: {}", self.name, e);
                self.render_error_fallback(output, &e)
            }
        }
    }
    
    fn render_error_fallback(&self, output: &Output, error: &UiError) -> RenderElement {
        // Red background with error text
        let surface = ImageSurface::create(Format::ARgb32, 200, 50)?;
        let cr = cairo::Context::new(&surface)?;
        cr.set_source_rgb(0.8, 0.2, 0.2);
        cr.paint()?;
        // ... render error text
    }
}
```

### 11.4 Graceful Degradation

```rust
/// When a UI window fails, it doesn't affect others
impl UiWindowManager {
    pub fn render_all(&self, output: &Output) -> Vec<RenderElement> {
        self.windows
            .values()
            .filter_map(|window| {
                match window.render(output) {
                    Ok(element) => Some(element),
                    Err(e) => {
                        // Log and skip this window
                        warn!("Skipping UI window '{}': {}", window.name, e);
                        None
                    }
                }
            })
            .collect()
    }
}
```

---

## 12. Implementation Roadmap

### Phase 1: Foundation (Weeks 1-2)

**Goal**: Basic infrastructure, one widget, one window

| Task | Priority | Estimate |
|------|----------|----------|
| Create `niri-ui` crate structure | P0 | 2h |
| Implement `Widget` trait | P0 | 4h |
| Implement `Container` widget | P0 | 4h |
| Implement `Text` widget | P0 | 4h |
| Implement `Painter` (Cairo wrapper) | P0 | 8h |
| Implement `UiWindow` | P0 | 8h |
| Basic Lua bindings (`niri.ui.window`, `niri.ui.container`, `niri.ui.text`) | P0 | 8h |
| Integration with compositor render loop | P0 | 4h |
| **Milestone**: Render "Hello World" text in layer-shell window | | |

### Phase 2: Layout & Widgets (Weeks 3-4)

**Goal**: Full widget set, working layout engine

| Task | Priority | Estimate |
|------|----------|----------|
| Implement layout engine (Constraints, Size) | P0 | 8h |
| Implement `Row` widget | P0 | 4h |
| Implement `Column` widget | P0 | 4h |
| Implement `Space` widget | P0 | 2h |
| Implement `Button` widget | P0 | 6h |
| Implement `Image` widget | P1 | 6h |
| Implement texture caching | P0 | 6h |
| Implement `niri.ui.redraw()` | P0 | 2h |
| **Milestone**: Render multi-widget panel with clickable buttons | | |

### Phase 3: Styling & Input (Weeks 5-6)

**Goal**: Named styles, full input handling

| Task | Priority | Estimate |
|------|----------|----------|
| Implement `Style` types | P0 | 4h |
| Implement `StyleResolver` | P0 | 4h |
| Implement style state variants (hover, active) | P0 | 6h |
| Implement pointer input handling | P0 | 8h |
| Implement signal emission to Lua | P0 | 4h |
| Implement keyboard input handling | P1 | 6h |
| Add gradient background support | P1 | 4h |
| **Milestone**: Interactive panel with hover effects | | |

### Phase 4: D-Bus & Services (Weeks 7-8)

**Goal**: D-Bus primitives, system tray, notifications

| Task | Priority | Estimate |
|------|----------|----------|
| Implement `DbusManager` | P0 | 8h |
| Implement `niri.dbus` Lua API | P0 | 8h |
| Implement `TrayService` (StatusNotifierWatcher) | P0 | 16h |
| Implement `NotificationService` | P0 | 12h |
| Lua bindings for tray and notifications | P0 | 4h |
| **Milestone**: Working system tray and notifications | | |

### Phase 5: Polish & Examples (Weeks 9-10)

**Goal**: Production-ready, example shells

| Task | Priority | Estimate |
|------|----------|----------|
| Animation support | P1 | 8h |
| Scroll widget | P1 | 8h |
| Icon theme integration | P1 | 6h |
| Example: Minimal bar | P0 | 4h |
| Example: Full shell (workspaces, tray, clock) | P0 | 8h |
| Example: Notification center | P1 | 6h |
| Documentation | P0 | 8h |
| Testing | P0 | 8h |
| **Milestone**: v0.1.0 release | | |

### Phase 6: Future Enhancements

| Feature | Priority | Notes |
|---------|----------|-------|
| CSS file import | P2 | Parse CSS subset |
| Text input widget | P2 | For launchers |
| Embedded views | P2 | WebView, etc. |
| Visual designer | P3 | Optional tooling |
| Animation DSL | P3 | Declarative animations |

---

## 13. API Reference

### 13.1 `niri.ui` Module

#### `niri.ui.window(config)`

Creates a new UI window.

**Parameters:**
- `config.name` (string, required): Unique window identifier
- `config.layer` (string): Layer shell layer ("background", "bottom", "top", "overlay")
- `config.anchor` (table): Edges to anchor to (e.g., {"top", "left", "right"})
- `config.exclusive_zone` (number): Exclusive zone size in pixels
- `config.margin` (table): Margins {top, right, bottom, left}
- `config.keyboard_interactivity` (string): "none", "exclusive", "on_demand"
- `config.output` (string|nil): Output name or nil for all outputs
- `config.styles` (table): Named style definitions
- `config.view` (function): Function returning widget tree
- `config.signals` (table): Signal handler functions
- `config.events` (table): Event handler functions

**Returns:** Window handle

#### `niri.ui.redraw(name)`

Triggers redraw of a window.

**Parameters:**
- `name` (string): Window name

#### `niri.ui.close(name)`

Closes a window.

**Parameters:**
- `name` (string): Window name

### 13.2 Widgets

See Section 9.3 for detailed widget props.

### 13.3 `niri.dbus` Module

#### `niri.dbus.call(bus, dest, path, interface, method, ...)`

Calls a D-Bus method.

**Parameters:**
- `bus` (string): "session" or "system"
- `dest` (string): Destination name
- `path` (string): Object path
- `interface` (string): Interface name
- `method` (string): Method name
- `...`: Method arguments

**Returns:** Method return value(s)

#### `niri.dbus.get_property(bus, dest, path, interface, property)`

Gets a D-Bus property.

**Returns:** Property value

#### `niri.dbus.set_property(bus, dest, path, interface, property, value)`

Sets a D-Bus property.

#### `niri.dbus.subscribe(bus, interface, signal, callback)`

Subscribes to a D-Bus signal.

**Parameters:**
- `callback` (function): Called with signal arguments

**Returns:** Subscription handle

#### `niri.dbus.list_names(bus, pattern)`

Lists D-Bus names matching pattern.

**Returns:** Table of matching names

---

## 14. Examples

### 14.1 Minimal Status Bar

```lua
-- ~/.config/niri/ui/bar.lua

niri.ui.window({
    name = "bar",
    layer = "top",
    anchor = { "top", "left", "right" },
    exclusive_zone = 32,
    
    styles = {
        bar_bg = {
            background = "#1e1e2e",
            padding = { 4, 16 },
        },
        time_text = {
            color = "#cdd6f4",
            font_size = 14,
        },
    },
    
    view = function()
        return niri.ui.container({
            style = "bar_bg",
            child = niri.ui.row({
                children = {
                    niri.ui.text({ content = "niri", style = "time_text" }),
                    niri.ui.space({ flex = 1 }),
                    niri.ui.text({ 
                        content = os.date("%H:%M"), 
                        style = "time_text" 
                    }),
                },
            }),
        })
    end,
})

-- Update time every second
niri.loop.new_timer():every(1000):start(function()
    niri.ui.redraw("bar")
end)
```

### 14.2 Workspace Indicator

```lua
-- Workspace state
local state = {
    workspaces = {},
    active = 1,
}

-- Update from compositor
niri.events:on("workspace:activate", function(e)
    state.active = e.workspace_id
    niri.ui.redraw("workspaces")
end)

-- Build workspace buttons
local function workspace_button(ws)
    local is_active = ws.id == state.active
    return niri.ui.button({
        style = is_active and "ws_active" or "ws_inactive",
        signal = "workspace_click",
        signal_data = { id = ws.id },
        child = niri.ui.text({ content = tostring(ws.id) }),
    })
end

niri.ui.window({
    name = "workspaces",
    layer = "top",
    anchor = { "bottom", "left" },
    margin = { bottom = 8, left = 8 },
    
    styles = {
        ws_inactive = {
            background = "#313244",
            padding = { 4, 12 },
            border = { radius = 4 },
            hover = { background = "#45475a" },
        },
        ws_active = {
            background = "#cba6f7",
            padding = { 4, 12 },
            border = { radius = 4 },
        },
    },
    
    view = function()
        local buttons = {}
        for _, ws in ipairs(state.workspaces) do
            table.insert(buttons, workspace_button(ws))
        end
        return niri.ui.row({ children = buttons, spacing = 4 })
    end,
    
    signals = {
        workspace_click = function(data)
            niri.action.focus_workspace(data.id)
        end,
    },
})
```

### 14.3 System Tray

```lua
niri.ui.window({
    name = "tray",
    layer = "top",
    anchor = { "top", "right" },
    margin = { top = 4, right = 4 },
    
    styles = {
        tray_bg = {
            background = "#1e1e2e",
            padding = 4,
            border = { radius = 8 },
        },
        tray_item = {
            padding = 4,
            border = { radius = 4 },
            hover = { background = "#313244" },
        },
    },
    
    view = function()
        local items = {}
        for _, item in ipairs(niri.tray.items()) do
            table.insert(items, niri.ui.button({
                style = "tray_item",
                signal = "tray_click",
                signal_data = { id = item.id },
                child = niri.ui.image({
                    icon = item.icon,
                    width = 20,
                    height = 20,
                }),
            }))
        end
        
        return niri.ui.container({
            style = "tray_bg",
            child = niri.ui.row({ children = items, spacing = 2 }),
        })
    end,
    
    signals = {
        tray_click = function(data)
            niri.tray.activate(data.id)
        end,
    },
    
    subscriptions = {
        tray = true,  -- Auto-redraw on tray changes
    },
})
```

---

## 15. Cursor Effects

### 15.1 Overview

Cursor effects provide visual feedback tied to cursor movement and interactions. This system enables:

- **Click ripples**: Animated ripples emanating from click locations
- **Cursor trails**: Persistent visual trails following cursor movement
- **Custom cursors**: User-defined cursor images/animations
- **Hover effects**: Visual feedback when hovering over UI elements

### 15.2 Architecture

Cursor effects are compositor-level overlays, not per-window features. They integrate with the existing cursor rendering pipeline.

```rust
// niri-ui/src/cursor_effects/mod.rs

pub struct CursorEffects {
    /// Active effect instances
    effects: Vec<Box<dyn CursorEffect>>,
    
    /// Configuration
    config: CursorEffectsConfig,
    
    /// Texture cache for effect rendering
    texture_cache: TextureCache,
}

pub trait CursorEffect: Send + Sync {
    /// Update effect state
    fn tick(&mut self, cursor_pos: Point, clock: &Clock);
    
    /// Check if effect is still active
    fn is_active(&self) -> bool;
    
    /// Render effect
    fn render(&self, renderer: &mut GlowRenderer) -> Vec<RenderElement>;
}
```

### 15.3 Effect Types

#### Click Ripple

```rust
pub struct ClickRipple {
    center: Point,
    start_time: Duration,
    duration: Duration,
    max_radius: f64,
    color: Color,
    style: RippleStyle,
}

pub enum RippleStyle {
    /// Solid expanding circle
    Solid,
    /// Ring that expands and fades
    Ring { thickness: f64 },
    /// Multiple concentric rings
    Concentric { count: u32 },
    /// Material Design ripple
    Material,
}
```

#### Cursor Trail

```rust
pub struct CursorTrail {
    /// History of cursor positions
    positions: VecDeque<(Point, Duration)>,
    
    /// Trail configuration
    config: TrailConfig,
}

pub struct TrailConfig {
    /// Number of trail points
    length: usize,
    
    /// Trail style
    style: TrailStyle,
    
    /// Fade duration
    fade_duration: Duration,
    
    /// Color (or gradient)
    color: TrailColor,
}

pub enum TrailStyle {
    /// Connected line segments
    Line { width: f64 },
    /// Dots at each position
    Dots { radius: f64 },
    /// Tapered line (thick → thin)
    Tapered { start_width: f64, end_width: f64 },
    /// Glow effect
    Glow { radius: f64, intensity: f64 },
}

pub enum TrailColor {
    Solid(Color),
    Gradient { start: Color, end: Color },
    Rainbow,
}
```

#### Custom Cursor

```rust
pub struct CustomCursor {
    /// Cursor image/animation
    source: CursorSource,
    
    /// Hotspot offset
    hotspot: Point,
    
    /// Scale factor
    scale: f64,
}

pub enum CursorSource {
    /// Static image
    Image(PathBuf),
    
    /// Animated (multiple frames)
    Animated {
        frames: Vec<PathBuf>,
        frame_duration: Duration,
    },
    
    /// SVG (scalable)
    Svg(String),
    
    /// Cursor from XCursor theme
    Theme { name: String, theme: String },
}
```

### 15.4 Lua API

```lua
-- Enable click ripples
niri.cursor.enable_ripple({
    color = "#cba6f780",  -- Semi-transparent accent
    duration = 400,        -- ms
    radius = 30,
    style = "material",    -- "solid" | "ring" | "concentric" | "material"
})

-- Disable ripples
niri.cursor.disable_ripple()

-- Enable cursor trail
niri.cursor.enable_trail({
    length = 10,
    style = "tapered",     -- "line" | "dots" | "tapered" | "glow"
    color = "#cba6f7",
    fade = 200,            -- ms
    width = 3,
})

-- Disable trail
niri.cursor.disable_trail()

-- Set custom cursor
niri.cursor.set_cursor({
    path = "~/.config/niri/cursors/custom.png",
    hotspot = { x = 0, y = 0 },
    scale = 1.0,
})

-- Animated cursor
niri.cursor.set_cursor({
    frames = {
        "~/.config/niri/cursors/frame1.png",
        "~/.config/niri/cursors/frame2.png",
        "~/.config/niri/cursors/frame3.png",
    },
    frame_duration = 100,  -- ms per frame
    hotspot = { x = 0, y = 0 },
})

-- Reset to default cursor
niri.cursor.reset()

-- Conditional effects (e.g., only during drag)
niri.events:on("pointer:button", function(event)
    if event.state == "pressed" then
        niri.cursor.enable_trail({ length = 20, style = "glow" })
    else
        niri.cursor.disable_trail()
    end
end)
```

### 15.5 Integration Points

Cursor effects require two additional integration points in the compositor:

```rust
// In render_output() - render cursor effects above windows, below UI
fn render_output(&mut self, output: &Output) -> Vec<RenderElement> {
    let mut elements = Vec::new();
    
    // Window layers...
    elements.extend(self.layout.render_elements(output));
    
    // NEW: Cursor effects (2 lines)
    #[cfg(feature = "ui")]
    if let Some(ref ui) = self.ui {
        elements.extend(ui.cursor_effects.render(self.cursor_position, output));
    }
    
    // UI windows (on top)
    // ...
}

// In input handling - trigger click ripple
fn handle_pointer_button(&mut self, event: PointerButtonEvent) {
    // NEW: Trigger ripple on click (3 lines)
    #[cfg(feature = "ui")]
    if event.state == ButtonState::Pressed {
        if let Some(ref mut ui) = self.niri.ui {
            ui.cursor_effects.trigger_ripple(self.pointer_position);
        }
    }
    
    // ... existing handling
}
```

### 15.6 Performance Considerations

- **Trail position sampling**: Use fixed-rate sampling (e.g., 60 Hz) regardless of pointer event rate
- **Effect culling**: Don't render off-screen effects
- **Texture reuse**: Pre-render effect textures (e.g., ripple ring) and scale/transform
- **Animation batching**: Update all effects in single pass

### 15.7 Roadmap

| Phase | Feature | Priority |
|-------|---------|----------|
| Phase 5 | Click ripples | P1 |
| Phase 5 | Cursor trails | P2 |
| Future | Custom cursors | P2 |
| Future | Animated cursors | P3 |

---

## 16. Shell Project Analysis

### 16.1 Overview

To achieve QtQuick/Shell-level flexibility, niri-ui must support building shells comparable to:

1. **AGS/Astal** - GJS/TypeScript-based shell framework
2. **end-4/dots-hyprland** - Quickshell-based Hyprland rice
3. **Quickshell** - QML-based shell framework

This section analyzes these projects to identify required capabilities.

### 16.2 AGS/Astal Analysis

**Framework**: GJS (GNOME JavaScript) + GTK + JSX
**Styling**: CSS (full GTK CSS support)
**Services**: Astal libraries (D-Bus proxies)

#### Key Features

| Feature | AGS Implementation | niri-ui Equivalent |
|---------|-------------------|-------------------|
| JSX Components | `<box>`, `<button>`, `<label>` | `niri.ui.row()`, `niri.ui.button()`, `niri.ui.text()` |
| Reactive State | GObject signals + bindings | Lua state + `niri.ui.redraw()` |
| CSS Styling | Full GTK CSS | Named styles (future: CSS subset) |
| Layer Shell | `Astal.Window` | `niri.ui.window({ layer = "..." })` |
| Conditional Rendering | `<With>` component | Lua `if/else` in view function |
| List Rendering | `<For>` component | Lua `for` loop in view function |

#### Service Libraries (Astal)

| Astal Service | niri-ui Equivalent | Implementation |
|--------------|-------------------|----------------|
| `AstalWp` (Audio) | `niri.audio` or D-Bus | Lua via D-Bus |
| `AstalNetwork` | `niri.network` or D-Bus | Lua via D-Bus |
| `AstalNotifd` | `niri.notifications` | Rust (daemon) |
| `AstalBattery` | D-Bus | Lua via D-Bus |
| `AstalBluetooth` | D-Bus | Lua via D-Bus |
| `AstalMpris` | D-Bus | Lua via D-Bus |
| `AstalTray` | `niri.tray` | Rust (SNI watcher) |
| `AstalApps` | App list query | Lua (freedesktop.org) |

#### Widgets Required

From AGS intrinsic elements:
- ✅ `box` → `container`
- ✅ `button` → `button`
- ✅ `label` → `text`
- ✅ `centerbox` → `row` with flex spacing
- ⏳ `slider` → Planned (Phase 5+)
- ⏳ `circularprogress` → Planned (Phase 5+)
- ⏳ `entry` → `input` (future)
- ⏳ `scrollable` → `scroll`
- ⏳ `revealer` → Animation system
- ✅ `icon`/`image` → `image`

### 16.3 end-4/dots-hyprland (Quickshell) Analysis

**Framework**: Quickshell (QML-based)
**Styling**: QML properties + Qt styling
**Services**: Qt APIs + D-Bus

#### UI Component Structure

```
Shell Architecture:
├── ShellRoot (main orchestrator)
├── PanelLoader (conditional loading)
├── Panel Families
│   ├── ii (traditional)
│   └── waffle (Windows 11-inspired)
└── Singleton Services
    ├── Config (JSON persistence)
    └── Looks (theming)
```

#### niri-ui Mapping

| Quickshell Component | niri-ui Equivalent |
|---------------------|-------------------|
| `ShellRoot` | Main Lua config file |
| `PanelLoader` | Conditional `niri.ui.window()` |
| `BarButton` | `niri.ui.button()` with style |
| `SystemButton` | Composite widget (Lua function) |
| `BarPopup` | Animated overlay window |
| `ConfigSwitch` | Button + state binding |
| `MaterialSymbol` | `niri.ui.image({ icon = "..." })` |
| `SysTray` | `niri.tray.items()` + rendering |

#### Required Capabilities

1. **Modular Panel System**: Multiple windows composable from Lua
2. **JSON Config Persistence**: Lua `json` module + file I/O
3. **Theming System**: Named styles with runtime switching
4. **Animated Popups**: Window show/hide animations
5. **Overlay Widgets**: Floating windows for OSD, recorders, etc.

#### Service Integrations

| Service | end-4 Usage | niri-ui Approach |
|---------|------------|-----------------|
| Notifications | Toast display + center | `niri.notifications` |
| System Tray | Pinnable tray items | `niri.tray` |
| Audio | Volume slider, mute | D-Bus (WirePlumber) |
| Battery | Status, low warnings | D-Bus (UPower) |
| Network | Wi-Fi/Ethernet indicator | D-Bus (NetworkManager) |
| Brightness | Slider control | D-Bus or `/sys/class/backlight` |
| Hyprland IPC | Window/workspace info | niri events system |
| Material Theming | Wallpaper color extraction | External tool + Lua |

### 16.4 Capability Gap Analysis

Based on analysis, niri-ui requires these capabilities to match popular shells:

#### Core Requirements (Phase 1-5)

| Capability | Status | Notes |
|------------|--------|-------|
| Basic widgets (container, row, column, text, button, image, space) | Phase 1-2 | Core set |
| Named styles with states | Phase 3 | hover, active, disabled |
| Flexbox layout | Phase 2 | Row/Column with flex |
| Input handling (pointer) | Phase 3 | Click, hover |
| Texture caching | Phase 2 | Per-output/scale |
| Animations (show/hide) | Phase 5 | Window transitions |
| D-Bus primitives | Phase 4 | Generic D-Bus access |
| System tray (SNI) | Phase 4 | Rust service |
| Notifications daemon | Phase 4 | Rust service |

#### Extended Requirements (Post-v1)

| Capability | Priority | Notes |
|------------|----------|-------|
| Slider widget | P1 | Volume, brightness |
| Circular progress | P2 | CPU, battery |
| Text input | P2 | Launchers, search |
| Scroll container | P1 | Notification lists |
| Animations (value) | P1 | Smooth transitions |
| Blur/transparency | P2 | Modern aesthetics |
| SVG icon rendering | P1 | Icon themes |
| Revealer (animated show/hide) | P2 | Smooth expand/collapse |
| Popup windows | P1 | Dropdown menus |

#### Service Abstractions (Lua Libraries)

These can be implemented as Lua modules using `niri.dbus`:

```lua
-- Example: niri/services/battery.lua
local M = {}

local UPOWER = "org.freedesktop.UPower"
local DEVICE_IFACE = "org.freedesktop.UPower.Device"

function M.get_battery()
    local devices = niri.dbus.call(
        "system", UPOWER, "/org/freedesktop/UPower",
        "org.freedesktop.UPower", "EnumerateDevices"
    )
    
    for _, path in ipairs(devices) do
        if path:match("BAT") then
            return {
                percent = niri.dbus.get_property("system", UPOWER, path, DEVICE_IFACE, "Percentage"),
                state = niri.dbus.get_property("system", UPOWER, path, DEVICE_IFACE, "State"),
                time_to_empty = niri.dbus.get_property("system", UPOWER, path, DEVICE_IFACE, "TimeToEmpty"),
            }
        end
    end
    return nil
end

return M
```

### 16.5 Shell Building Patterns

#### Pattern 1: Simple Status Bar

```lua
-- Minimal bar with time and workspaces
local function view()
    return niri.ui.container({
        style = "bar",
        child = niri.ui.row({
            children = {
                workspace_buttons(),
                niri.ui.space({ flex = 1 }),
                niri.ui.text({ content = os.date("%H:%M") }),
            },
        }),
    })
end
```

#### Pattern 2: Modular Shell (AGS-style)

```lua
-- main.lua
require("services.battery")
require("services.audio")
require("services.network")

require("windows.bar")
require("windows.notification_center")
require("windows.launcher")
```

```lua
-- windows/bar.lua
local battery = require("services.battery")
local audio = require("services.audio")

niri.ui.window({
    name = "bar",
    layer = "top",
    anchor = { "top", "left", "right" },
    exclusive_zone = 32,
    view = function()
        return niri.ui.row({
            children = {
                widgets.workspaces(),
                niri.ui.space({ flex = 1 }),
                widgets.tray(),
                widgets.audio(audio.get()),
                widgets.battery(battery.get()),
                widgets.clock(),
            },
        })
    end,
})
```

#### Pattern 3: Runtime-Switchable Themes (end-4 style)

```lua
-- themes/catppuccin.lua
return {
    bar_bg = { background = "#1e1e2e" },
    text_primary = { color = "#cdd6f4" },
    accent = { color = "#cba6f7" },
}

-- themes/gruvbox.lua
return {
    bar_bg = { background = "#282828" },
    text_primary = { color = "#ebdbb2" },
    accent = { color = "#d79921" },
}

-- main.lua
local theme_name = niri.config.get("theme", "catppuccin")
local theme = require("themes." .. theme_name)

niri.ui.window({
    name = "bar",
    styles = theme,
    -- ...
})
```

### 16.6 Recommendations

Based on this analysis, the following additions strengthen the roadmap:

1. **Phase 5 additions**:
   - Slider widget (critical for volume/brightness)
   - Scroll container (notification lists)
   - Circular progress (battery, CPU indicators)

2. **Lua library ecosystem**:
   - Ship example service modules (battery, audio, network)
   - Document pattern for creating service wrappers

3. **Theme system enhancement**:
   - Support loading styles from separate Lua files
   - Theme hot-reloading via `niri.ui.reload_styles()`

4. **Popup/dropdown support**:
   - Child windows anchored to UI elements
   - Automatic dismissal on outside click

---

## Appendix A: Comparison with Astrum

| Feature | Astrum | niri-ui |
|---------|--------|---------|
| Runtime | Tokio | Calloop |
| UI Framework | libcosmic/iced | Smithay-native |
| Integration | Standalone binary | Compositor-integrated |
| Memory | ~300MB | Target <50MB |
| Dependencies | ~200 crates | ~10 new crates |
| Widget Definition | Rust + Lua bridge | Rust (Lua exposed) |
| Styling | Global mutex | Per-window, named |
| Services | Rust implementations | D-Bus primitives + Lua |

---

## Appendix B: Comparison with QtQuick/Shell

niri-ui aims for QtQuick/Shell-level capability. Here's how features map:

| QtQuick/Shell Feature | niri-ui Equivalent | Status |
|----------------------|-------------------|--------|
| QML declarative UI | Lua table configuration | Core |
| Anchors layout | `anchor` in window config | Core |
| Row/Column layouts | `niri.ui.row()`, `niri.ui.column()` | Core |
| Repeater | Lua `for` loop in view function | Core |
| PropertyAnimation | Animation system (Phase 5) | Planned |
| Loader | Lazy widget loading (Lua) | Possible |
| Component | Lua functions returning widgets | Core |
| PlasmaCore.DataSource | `niri.dbus` API | Core |
| PlasmaExtras.* | Service layer (tray, notifications) | Core |
| SystemTray | `niri.tray` | Core |
| Notifications | `niri.notifications` | Core |
| JS/QML scripting | Lua scripting | Core |

### QtQuick Features NOT Planned

| Feature | Reason |
|---------|--------|
| QML compiler | Lua is interpreted, acceptable |
| Qt Widgets embedding | Use Smithay-native only |
| 3D transforms | Not needed for shell UI |
| Qt Quick Controls | Build equivalent in niri-ui |
| QML debugging | Lua error reporting sufficient |

### Path to QtQuick Parity

**Phase 1-3**: Basic widget system - matches QML basics
**Phase 4**: D-Bus/services - matches PlasmaCore.DataSource
**Phase 5**: Animations - matches PropertyAnimation
**Future**: Embedded views, CSS - extends beyond QML

---

## Appendix C: Comparison with Pinnacle/Snowcap

### Overview

Pinnacle is a Wayland compositor with Snowcap, a separate widget system. Understanding Snowcap's architecture provides valuable contrast to niri-ui's integrated approach.

### Snowcap Architecture

| Aspect | Snowcap | niri-ui |
|--------|---------|---------|
| **Process Model** | Separate process | Same process (compositor-integrated) |
| **UI Framework** | Iced (Rust GUI) | Smithay-native (Cairo/Pango) |
| **Communication** | gRPC over Unix socket | Direct function calls |
| **Widget Definition** | Protocol Buffers | Lua tables → Rust structs |
| **Layer Shell** | wlr-layer-shell client | Compositor-internal overlays |
| **State Sync** | Message passing | Shared state |

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     SNOWCAP ARCHITECTURE                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌──────────────┐         gRPC/Unix Socket        ┌──────────┐ │
│   │   Snowcap    │◄───────────────────────────────►│ Pinnacle │ │
│   │   (Client)   │                                  │(Compositor│ │
│   └──────┬───────┘                                  └──────────┘ │
│          │                                                       │
│          ▼                                                       │
│   ┌──────────────┐                                               │
│   │    Iced      │                                               │
│   │  (Rendering) │                                               │
│   └──────┬───────┘                                               │
│          │                                                       │
│          ▼                                                       │
│   ┌──────────────┐                                               │
│   │ Layer-Shell  │  (wlr-layer-shell protocol)                   │
│   │   Surface    │                                               │
│   └──────────────┘                                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      NIRI-UI ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │                        niri                              │   │
│   │   ┌─────────────┐    ┌─────────────┐    ┌───────────┐   │   │
│   │   │   niri-ui   │◄──►│  niri-lua   │◄──►│   Lua     │   │   │
│   │   │  (Widgets)  │    │  (Bindings) │    │ (Config)  │   │   │
│   │   └──────┬──────┘    └─────────────┘    └───────────┘   │   │
│   │          │                                               │   │
│   │          ▼ (direct render)                               │   │
│   │   ┌─────────────┐                                        │   │
│   │   │  GlowRenderer│                                       │   │
│   │   │ (Compositor) │                                       │   │
│   │   └─────────────┘                                        │   │
│   │                                                          │   │
│   └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Snowcap Widget Definition (gRPC/Protobuf)

```protobuf
// snowcap.proto (simplified)
message Widget {
    oneof widget {
        Column column = 1;
        Container container = 2;
        Row row = 3;
        Text text = 4;
        Scrollable scrollable = 5;
    }
}

message Column {
    Alignment alignment = 1;
    Length width = 2;
    Length height = 3;
    Padding padding = 4;
    float spacing = 5;
    repeated Widget children = 6;
}

message Text {
    string text = 1;
    float size = 2;
    Color color = 3;
    Font font = 4;
}
```

### niri-ui Widget Definition (Lua)

```lua
-- Equivalent in niri-ui
niri.ui.column({
    align = "center",
    width = 200,
    height = "fill",
    padding = 8,
    spacing = 4,
    children = {
        niri.ui.text({
            content = "Hello",
            font_size = 16,
            color = "#ffffff",
        }),
    },
})
```

### Trade-offs

#### Snowcap Advantages

1. **Process isolation**: UI crashes don't affect compositor
2. **Iced ecosystem**: Access to Iced's widget library
3. **Compositor-agnostic**: Can work with any wlr-layer-shell compositor
4. **Language flexibility**: gRPC allows any language client

#### niri-ui Advantages

1. **Lower latency**: No IPC overhead, direct state access
2. **Simpler deployment**: Single binary, no socket management
3. **Deeper integration**: Direct access to compositor internals
4. **Lower memory**: No duplicate rendering contexts
5. **Synchronous updates**: No race conditions between UI and compositor state
6. **No protocol complexity**: Lua → Rust is simpler than Lua → gRPC → Rust

### Why niri-ui Chose Integration

The key reason for choosing compositor-integration over Snowcap's approach:

1. **niri already has compositor-internal UI**: hotkey_overlay, config_error_notification use the same Cairo → texture pattern. niri-ui extends this existing pattern.

2. **Shared Lua runtime**: niri-lua already provides Lua scripting. Adding UI to the same runtime avoids duplicating the scripting layer.

3. **No layer-shell complications**: By rendering UI as compositor-internal textures (like hotkey_overlay), we avoid any potential conflicts with external layer-shell clients.

4. **Simpler state management**: UI can directly read compositor state (workspaces, windows, outputs) without serialization/deserialization.

### When Snowcap's Approach is Better

- **Multi-compositor support**: If you want the same UI code to work on Pinnacle, Sway, and others
- **Crash resilience**: Critical if UI complexity risks crashes
- **Independent development**: UI and compositor teams can work separately
- **Hot-reloading**: Restart UI without restarting compositor

### Migration Path

If niri-ui ever needs to move to a separate process:

1. `niri.ui` API would remain unchanged
2. Implementation would switch from direct calls to IPC
3. Widget trees would serialize to a wire format
4. Rendering would move to a separate layer-shell client

The Lua API abstraction means users wouldn't need to change their configurations.

---

## Appendix D: Main Compositor Changes Summary

This appendix documents the **exact** changes required in the main niri crate.

### Files Modified (4 files)

1. **Cargo.toml** - Add dependency
```toml
[dependencies]
niri-ui = { path = "niri-ui", optional = true }

[features]
default = ["ui", ...]
ui = ["niri-ui"]
```

2. **src/niri.rs** - Add field and initialization (~15 lines)
```rust
use niri_ui::{UiManager, UiManagerConfig};

pub struct Niri {
    // ... existing fields ...
    #[cfg(feature = "ui")]
    pub ui: Option<UiManager>,
}

impl Niri {
    pub fn new(/* ... */) -> Self {
        #[cfg(feature = "ui")]
        let ui = UiManager::new(UiManagerConfig::default()).ok();
        
        Self {
            // ... existing fields ...
            #[cfg(feature = "ui")]
            ui,
        }
    }
}

impl UiCompositorInterface for Niri {
    // ~20 lines of trait implementation
}
```

3. **src/niri.rs** - Render integration (~5 lines)
```rust
fn render_output(&mut self, output: &Output) -> Vec<RenderElement> {
    // ... existing code ...
    
    #[cfg(feature = "ui")]
    if let Some(ref mut ui) = self.ui {
        elements.extend(ui.render_elements(self, output));
    }
    
    elements
}
```

4. **src/input/mod.rs** - Input routing (~8 lines)
```rust
fn handle_pointer_button(&mut self, event: PointerButtonEvent) {
    #[cfg(feature = "ui")]
    if let Some(ref mut ui) = self.niri.ui {
        if let UiInputResult::Consumed | UiInputResult::Signal(_) = 
            ui.handle_pointer_button(pos, &event) 
        {
            return;
        }
    }
    
    // ... existing code unchanged ...
}
```

### Total Lines Changed in Main Compositor

| File | Lines Added | Lines Modified |
|------|-------------|----------------|
| Cargo.toml | 4 | 0 |
| src/niri.rs | 30 | 0 |
| src/input/mod.rs | 10 | 0 |
| **Total** | **~44** | **0** |

All existing code remains unchanged. Changes are additive only.

---

## Appendix E: File Checklist

```
niri-ui/
├── Cargo.toml                    [ ]
├── AGENTS.md                     [ ]
├── README.md                     [ ]
├── src/
│   ├── lib.rs                    [ ]
│   ├── error.rs                  [ ]
│   ├── window.rs                 [ ]
│   ├── widget.rs                 [ ]
│   ├── widgets/
│   │   ├── mod.rs                [ ]
│   │   ├── container.rs          [ ]
│   │   ├── row.rs                [ ]
│   │   ├── column.rs             [ ]
│   │   ├── text.rs               [ ]
│   │   ├── button.rs             [ ]
│   │   ├── image.rs              [ ]
│   │   ├── space.rs              [ ]
│   │   ├── scroll.rs             [ ]
│   │   ├── slider.rs             [ ] (Phase 5+)
│   │   ├── progress.rs           [ ] (Phase 5+)
│   │   └── input.rs              [ ] (Future)
│   ├── layout/
│   │   ├── mod.rs                [ ]
│   │   ├── flex.rs               [ ]
│   │   └── constraints.rs        [ ]
│   ├── render/
│   │   ├── mod.rs                [ ]
│   │   ├── painter.rs            [ ]
│   │   ├── texture_cache.rs      [ ]
│   │   └── elements.rs           [ ]
│   ├── style/
│   │   ├── mod.rs                [ ]
│   │   ├── types.rs              [ ]
│   │   ├── resolver.rs           [ ]
│   │   └── theme.rs              [ ]
│   ├── input/
│   │   ├── mod.rs                [ ]
│   │   ├── pointer.rs            [ ]
│   │   └── keyboard.rs           [ ]
│   ├── cursor_effects/           (NEW - Phase 5)
│   │   ├── mod.rs                [ ]
│   │   ├── ripple.rs             [ ]
│   │   ├── trail.rs              [ ]
│   │   └── custom_cursor.rs      [ ]
│   ├── services/
│   │   ├── mod.rs                [ ]
│   │   ├── dbus.rs               [ ]
│   │   ├── tray.rs               [ ]
│   │   └── notifications.rs      [ ]
│   └── lua_bindings/
│       ├── mod.rs                [ ]
│       ├── window.rs             [ ]
│       ├── widgets.rs            [ ]
│       ├── styles.rs             [ ]
│       ├── cursor.rs             [ ] (NEW)
│       ├── services.rs           [ ]
│       └── types.rs              [ ]
```

---

*Document generated for niri-ui v0.1.0 specification*
