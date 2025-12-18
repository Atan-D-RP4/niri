# Window System Specification

## Overview

The window system manages UI surfaces that host widget trees. Windows handle lifecycle, visibility animations, input routing, and per-output rendering. This spec covers `UiWindow` for persistent panels/overlays and `PopupWindow` for transient anchored surfaces.

## Core Types

### Window Configuration

```rust
/// Window layer determines z-ordering relative to compositor surfaces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowLayer {
    /// Below all windows (wallpaper, desktop widgets)
    Background,
    /// Above windows but below panels
    Bottom,
    /// Standard panel layer (bars, docks)
    Top,
    /// Above everything (notifications, OSDs, popups)
    Overlay,
}

/// Which outputs a window should appear on
#[derive(Debug, Clone)]
pub enum OutputTarget {
    /// Show on all outputs
    All,
    /// Show only on primary output
    Primary,
    /// Show on specific output by name
    Named(String),
    /// Show on output containing pointer
    Focused,
}

/// Window anchor point for positioning
#[derive(Debug, Clone, Copy)]
pub struct WindowAnchor {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl WindowAnchor {
    pub const TOP: Self = Self { top: true, bottom: false, left: true, right: true };
    pub const BOTTOM: Self = Self { top: false, bottom: true, left: true, right: true };
    pub const LEFT: Self = Self { top: true, bottom: true, left: true, right: false };
    pub const RIGHT: Self = Self { top: true, bottom: true, left: false, right: true };
    pub const CENTER: Self = Self { top: false, bottom: false, left: false, right: false };
    pub const TOP_LEFT: Self = Self { top: true, bottom: false, left: true, right: false };
    pub const TOP_RIGHT: Self = Self { top: true, bottom: false, left: false, right: true };
    pub const BOTTOM_LEFT: Self = Self { top: false, bottom: true, left: true, right: false };
    pub const BOTTOM_RIGHT: Self = Self { top: false, bottom: true, left: false, right: true };
}

/// Window margins from screen edges
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowMargins {
    pub top: i32,
    pub bottom: i32,
    pub left: i32,
    pub right: i32,
}

/// Configuration for creating a UI window
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Unique identifier
    pub id: String,
    /// Layer for z-ordering
    pub layer: WindowLayer,
    /// Which outputs to show on
    pub output: OutputTarget,
    /// Anchor point for positioning
    pub anchor: WindowAnchor,
    /// Margins from anchor edges
    pub margins: WindowMargins,
    /// Whether window reserves screen space (exclusive zone)
    pub exclusive: bool,
    /// Whether window accepts keyboard focus
    pub keyboard_interactivity: KeyboardInteractivity,
    /// Whether window accepts pointer/touch input
    pub pointer_interactivity: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardInteractivity {
    /// Never receive keyboard focus
    None,
    /// Receive focus only when explicitly requested
    OnDemand,
    /// Always receive keyboard focus when interacted with
    Exclusive,
}
```

### UiWindow

```rust
use smithay::output::Output;
use smithay::utils::{Logical, Point, Size};

/// Visibility state with animation tracking
#[derive(Debug)]
pub enum VisibilityState {
    Hidden,
    Showing(Animation),
    Shown,
    Hiding(Animation),
}

/// A persistent UI window (panel, bar, overlay)
pub struct UiWindow {
    /// Configuration
    config: WindowConfig,
    /// Root widget tree
    root: Box<dyn Widget>,
    /// Current visibility state
    visibility: VisibilityState,
    /// Per-output render cache
    output_cache: HashMap<WeakOutput, OutputWindowState>,
    /// Computed size
    size: Size<i32, Logical>,
    /// Whether layout needs recalculation
    needs_layout: bool,
    /// Whether texture needs re-render
    needs_render: bool,
}

/// Per-output state for a window
struct OutputWindowState {
    /// Cached texture at output's scale
    texture: Option<TextureBuffer>,
    /// Scale this texture was rendered at
    scale: f64,
    /// Position on this output
    position: Point<i32, Logical>,
}

impl UiWindow {
    pub fn new(config: WindowConfig, root: Box<dyn Widget>) -> Self {
        Self {
            config,
            root,
            visibility: VisibilityState::Hidden,
            output_cache: HashMap::new(),
            size: Size::default(),
            needs_layout: true,
            needs_render: true,
        }
    }

    /// Show the window with animation
    pub fn show(&mut self, clock: &Clock) {
        match &self.visibility {
            VisibilityState::Hidden | VisibilityState::Hiding(_) => {
                let anim = Animation::new(clock, 0.0, 1.0, 200.ms())
                    .with_curve(Curve::EaseOutCubic);
                self.visibility = VisibilityState::Showing(anim);
            }
            _ => {}
        }
    }

    /// Hide the window with animation
    pub fn hide(&mut self, clock: &Clock) {
        match &self.visibility {
            VisibilityState::Shown | VisibilityState::Showing(_) => {
                let anim = Animation::new(clock, 1.0, 0.0, 150.ms())
                    .with_curve(Curve::EaseOutQuad);
                self.visibility = VisibilityState::Hiding(anim);
            }
            _ => {}
        }
    }

    /// Toggle visibility
    pub fn toggle(&mut self, clock: &Clock) {
        match &self.visibility {
            VisibilityState::Hidden | VisibilityState::Hiding(_) => self.show(clock),
            VisibilityState::Shown | VisibilityState::Showing(_) => self.hide(clock),
        }
    }

    /// Check if window is visible (fully or animating)
    pub fn is_visible(&self) -> bool {
        !matches!(self.visibility, VisibilityState::Hidden)
    }

    /// Update animation state, returns true if still animating
    pub fn advance_animations(&mut self, clock: &Clock) -> bool {
        match &self.visibility {
            VisibilityState::Showing(anim) => {
                if anim.is_done(clock) {
                    self.visibility = VisibilityState::Shown;
                    false
                } else {
                    true
                }
            }
            VisibilityState::Hiding(anim) => {
                if anim.is_done(clock) {
                    self.visibility = VisibilityState::Hidden;
                    false
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    /// Mark window as needing re-layout
    pub fn invalidate_layout(&mut self) {
        self.needs_layout = true;
        self.needs_render = true;
    }

    /// Mark window as needing re-render (but not re-layout)
    pub fn invalidate_render(&mut self) {
        self.needs_render = true;
    }

    /// Perform layout if needed
    pub fn layout_if_needed(&mut self, available_size: Size<i32, Logical>) {
        if self.needs_layout {
            let constraints = Constraints {
                min: Size::default(),
                max: available_size,
            };
            self.size = self.root.measure(&constraints);
            self.root.layout(Point::default(), self.size);
            self.needs_layout = false;
            // Clear output cache since size may have changed
            self.output_cache.clear();
        }
    }

    /// Render for specific output, using cache if valid
    pub fn render_for_output(
        &mut self,
        output: &Output,
        renderer: &mut GlesRenderer,
    ) -> Option<PrimaryGpuTextureRenderElement> {
        if !self.is_visible() {
            return None;
        }

        let scale = output.current_scale().fractional_scale();
        let weak_output = WeakOutput::from(output.clone());
        
        // Check cache validity
        let cache = self.output_cache.entry(weak_output).or_insert_with(|| {
            OutputWindowState {
                texture: None,
                scale: 0.0,
                position: self.compute_position(output),
            }
        });

        // Re-render if scale changed or render invalidated
        if cache.scale != scale || self.needs_render || cache.texture.is_none() {
            cache.texture = Some(self.render_texture(scale));
            cache.scale = scale;
        }

        let texture = cache.texture.as_ref()?;
        let opacity = self.current_opacity();
        
        Some(create_render_element(
            renderer,
            texture,
            cache.position,
            opacity,
            scale,
        ))
    }

    /// Compute position on output based on anchor and margins
    fn compute_position(&self, output: &Output) -> Point<i32, Logical> {
        let output_size = output.geometry().size;
        let anchor = &self.config.anchor;
        let margins = &self.config.margins;

        let x = if anchor.left && anchor.right {
            // Centered horizontally
            (output_size.w - self.size.w) / 2
        } else if anchor.left {
            margins.left
        } else if anchor.right {
            output_size.w - self.size.w - margins.right
        } else {
            (output_size.w - self.size.w) / 2
        };

        let y = if anchor.top && anchor.bottom {
            // Centered vertically
            (output_size.h - self.size.h) / 2
        } else if anchor.top {
            margins.top
        } else if anchor.bottom {
            output_size.h - self.size.h - margins.bottom
        } else {
            (output_size.h - self.size.h) / 2
        };

        Point::from((x, y))
    }

    /// Get current opacity based on animation state
    fn current_opacity(&self) -> f32 {
        match &self.visibility {
            VisibilityState::Hidden => 0.0,
            VisibilityState::Shown => 1.0,
            VisibilityState::Showing(anim) | VisibilityState::Hiding(anim) => {
                anim.value() as f32
            }
        }
    }

    /// Render widget tree to texture
    fn render_texture(&self, scale: f64) -> TextureBuffer {
        let pixel_size = (
            (self.size.w as f64 * scale).ceil() as i32,
            (self.size.h as f64 * scale).ceil() as i32,
        );

        let surface = ImageSurface::create(Format::ARgb32, pixel_size.0, pixel_size.1)
            .expect("Failed to create surface");

        {
            let cr = cairo::Context::new(&surface).expect("Failed to create context");
            cr.scale(scale, scale);
            
            let ctx = RenderContext {
                cairo: &cr,
                scale,
                clip: None,
            };
            self.root.render(&ctx);
        }

        let data = surface.take_data().expect("Failed to get surface data");
        TextureBuffer::from_memory(
            data.as_ref(),
            Fourcc::Argb8888,
            (pixel_size.0, pixel_size.1),
            false,
            scale as i32,
            Transform::Normal,
        )
    }

    /// Handle pointer event, returns true if consumed
    pub fn handle_pointer(&mut self, position: Point<f64, Logical>, event: PointerEvent) -> bool {
        if !self.config.pointer_interactivity || !self.is_visible() {
            return false;
        }
        self.root.handle_pointer(position, event)
    }
}
```

### PopupWindow

```rust
/// Anchor reference for popup positioning
#[derive(Debug, Clone)]
pub enum PopupAnchor {
    /// Anchor to a widget's bounds
    Widget {
        window_id: String,
        widget_id: String,
        edge: PopupEdge,
    },
    /// Anchor to absolute screen position
    Position(Point<i32, Logical>),
    /// Anchor to current cursor position
    Cursor,
    /// Anchor to screen edge/corner
    Screen {
        output: OutputTarget,
        anchor: WindowAnchor,
    },
}

/// Edge of anchor to attach popup to
#[derive(Debug, Clone, Copy)]
pub enum PopupEdge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Configuration for popup windows
#[derive(Debug, Clone)]
pub struct PopupConfig {
    pub id: String,
    pub anchor: PopupAnchor,
    /// Offset from anchor point
    pub offset: Point<i32, Logical>,
    /// Close when clicking outside
    pub close_on_outside_click: bool,
    /// Close when escape is pressed
    pub close_on_escape: bool,
    /// Close when another popup opens
    pub close_on_other_popup: bool,
}

/// A transient popup window
pub struct PopupWindow {
    config: PopupConfig,
    root: Box<dyn Widget>,
    visibility: VisibilityState,
    /// Resolved position (computed from anchor)
    position: Point<i32, Logical>,
    size: Size<i32, Logical>,
    /// Output this popup is on
    output: Option<WeakOutput>,
    /// Cached texture
    texture: Option<TextureBuffer>,
    texture_scale: f64,
    needs_layout: bool,
    needs_render: bool,
}

impl PopupWindow {
    pub fn new(config: PopupConfig, root: Box<dyn Widget>) -> Self {
        Self {
            config,
            root,
            visibility: VisibilityState::Hidden,
            position: Point::default(),
            size: Size::default(),
            output: None,
            texture: None,
            texture_scale: 0.0,
            needs_layout: true,
            needs_render: true,
        }
    }

    /// Show popup, computing position from anchor
    pub fn show(&mut self, clock: &Clock, anchor_resolver: &impl AnchorResolver) {
        // Resolve anchor to position
        let (position, output) = anchor_resolver.resolve(&self.config.anchor);
        self.position = Point::from((
            position.x + self.config.offset.x,
            position.y + self.config.offset.y,
        ));
        self.output = Some(output);
        
        let anim = Animation::new(clock, 0.0, 1.0, 150.ms())
            .with_curve(Curve::EaseOutQuad);
        self.visibility = VisibilityState::Showing(anim);
    }

    /// Hide popup
    pub fn hide(&mut self, clock: &Clock) {
        if matches!(self.visibility, VisibilityState::Shown | VisibilityState::Showing(_)) {
            let anim = Animation::new(clock, 1.0, 0.0, 100.ms())
                .with_curve(Curve::EaseOutQuad);
            self.visibility = VisibilityState::Hiding(anim);
        }
    }

    /// Toggle popup visibility
    pub fn toggle(&mut self, clock: &Clock, anchor_resolver: &impl AnchorResolver) {
        match &self.visibility {
            VisibilityState::Hidden | VisibilityState::Hiding(_) => {
                self.show(clock, anchor_resolver)
            }
            _ => self.hide(clock),
        }
    }

    // ... similar render and input methods as UiWindow
}

/// Resolves popup anchors to screen positions
pub trait AnchorResolver {
    fn resolve(&self, anchor: &PopupAnchor) -> (Point<i32, Logical>, WeakOutput);
}
```

### WindowManager

```rust
/// Manages all UI windows
pub struct WindowManager {
    windows: HashMap<String, UiWindow>,
    popups: Vec<PopupWindow>,
    /// Window draw order (back to front)
    draw_order: Vec<String>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            popups: Vec::new(),
            draw_order: Vec::new(),
        }
    }

    /// Register a new window
    pub fn add_window(&mut self, window: UiWindow) {
        let id = window.config.id.clone();
        self.windows.insert(id.clone(), window);
        self.update_draw_order();
    }

    /// Remove a window
    pub fn remove_window(&mut self, id: &str) -> Option<UiWindow> {
        let window = self.windows.remove(id);
        if window.is_some() {
            self.draw_order.retain(|i| i != id);
        }
        window
    }

    /// Get window by ID
    pub fn get_window(&self, id: &str) -> Option<&UiWindow> {
        self.windows.get(id)
    }

    /// Get mutable window by ID
    pub fn get_window_mut(&mut self, id: &str) -> Option<&mut UiWindow> {
        self.windows.get_mut(id)
    }

    /// Show a popup
    pub fn show_popup(&mut self, popup: PopupWindow, clock: &Clock, resolver: &impl AnchorResolver) {
        // Close other popups if configured
        if popup.config.close_on_other_popup {
            for p in &mut self.popups {
                p.hide(clock);
            }
        }
        
        let mut popup = popup;
        popup.show(clock, resolver);
        self.popups.push(popup);
    }

    /// Close all popups
    pub fn close_all_popups(&mut self, clock: &Clock) {
        for popup in &mut self.popups {
            popup.hide(clock);
        }
    }

    /// Update all animations, returns true if any still animating
    pub fn advance_animations(&mut self, clock: &Clock) -> bool {
        let mut animating = false;
        
        for window in self.windows.values_mut() {
            animating |= window.advance_animations(clock);
        }
        
        for popup in &mut self.popups {
            animating |= popup.advance_animations(clock);
        }
        
        // Remove fully hidden popups
        self.popups.retain(|p| p.is_visible());
        
        animating
    }

    /// Render all windows for an output
    pub fn render_for_output(
        &mut self,
        output: &Output,
        renderer: &mut GlesRenderer,
    ) -> Vec<PrimaryGpuTextureRenderElement> {
        let mut elements = Vec::new();

        // Render windows in draw order
        for id in &self.draw_order {
            if let Some(window) = self.windows.get_mut(id) {
                if let Some(elem) = window.render_for_output(output, renderer) {
                    elements.push(elem);
                }
            }
        }

        // Render popups on top
        for popup in &mut self.popups {
            if let Some(elem) = popup.render_for_output(output, renderer) {
                elements.push(elem);
            }
        }

        elements
    }

    /// Handle pointer event, returns result indicating consumption
    pub fn handle_pointer(
        &mut self,
        output: &Output,
        position: Point<f64, Logical>,
        event: PointerEvent,
    ) -> UiInputResult {
        // Check popups first (topmost)
        for popup in self.popups.iter_mut().rev() {
            if popup.contains_point(position) {
                if popup.handle_pointer(position, event) {
                    return UiInputResult::Consumed;
                }
            } else if matches!(event, PointerEvent::Button { pressed: true, .. }) {
                if popup.config.close_on_outside_click {
                    popup.hide(&Clock::now());
                    return UiInputResult::Consumed;
                }
            }
        }

        // Check windows in reverse draw order
        for id in self.draw_order.iter().rev() {
            if let Some(window) = self.windows.get_mut(id) {
                if window.handle_pointer(position, event) {
                    return UiInputResult::Consumed;
                }
            }
        }

        UiInputResult::NotConsumed
    }

    /// Update draw order based on window layers
    fn update_draw_order(&mut self) {
        self.draw_order = self.windows.keys().cloned().collect();
        self.draw_order.sort_by_key(|id| {
            self.windows.get(id).map(|w| w.config.layer as i32).unwrap_or(0)
        });
    }
}

/// Result of UI input handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiInputResult {
    /// Input was consumed by UI
    Consumed,
    /// Input was not consumed, pass to compositor
    NotConsumed,
}
```

## Lua API

### Creating Windows

```lua
-- Create a panel window
local panel = niri.ui.window({
    id = "top-panel",
    layer = "top",
    output = "all",
    anchor = "top",
    margins = { top = 0, left = 8, right = 8 },
    exclusive = true,
    keyboard = "none",
}, function()
    return niri.ui.box({
        orientation = "horizontal",
        style = { background = "#1a1a1aee", padding = 8 },
        children = {
            workspaces_widget(),
            niri.ui.spacer(),
            clock_widget(),
            niri.ui.spacer(),
            tray_widget(),
        },
    })
end)

-- Show/hide
panel:show()
panel:hide()
panel:toggle()

-- Check visibility
if panel:is_visible() then
    print("Panel is visible")
end

-- Update content
panel:set_content(function()
    return new_widget_tree()
end)

-- Destroy window
panel:destroy()
```

### Creating Popups

```lua
-- Create a popup anchored to a widget
local menu = niri.ui.popup({
    id = "app-menu",
    anchor = {
        type = "widget",
        window = "top-panel",
        widget = "menu-button",
        edge = "bottom",
    },
    offset = { x = 0, y = 4 },
    close_on_outside_click = true,
    close_on_escape = true,
}, function()
    return niri.ui.box({
        orientation = "vertical",
        style = menu_style,
        children = {
            menu_item("Settings", on_settings),
            menu_item("About", on_about),
            niri.ui.separator(),
            menu_item("Quit", on_quit),
        },
    })
end)

-- Anchor to cursor position
local context_menu = niri.ui.popup({
    id = "context-menu",
    anchor = { type = "cursor" },
    close_on_outside_click = true,
}, menu_content)

-- Anchor to screen corner
local notification = niri.ui.popup({
    id = "notification",
    anchor = {
        type = "screen",
        output = "primary",
        position = "top_right",
    },
    offset = { x = -16, y = 16 },
}, notification_content)

-- Show popup
menu:show()
context_menu:show()

-- Close all popups
niri.ui.close_all_popups()
```

### Window Events

```lua
-- Window lifecycle events
panel:on("show", function()
    print("Panel shown")
end)

panel:on("hide", function()
    print("Panel hidden")
end)

panel:on("output_changed", function(output)
    print("Panel moved to output: " .. output.name)
end)

-- Popup events
menu:on("close", function(reason)
    -- reason: "outside_click", "escape", "programmatic"
    print("Menu closed: " .. reason)
end)
```

## Acceptance Criteria

### Window Lifecycle
```
GIVEN a window is created with valid config
WHEN show() is called
THEN window enters Showing state with animation
AND window becomes visible immediately (opacity animating from 0)

GIVEN a window is in Shown state
WHEN hide() is called
THEN window enters Hiding state with animation
AND window remains visible until animation completes
AND window enters Hidden state when animation finishes
```

### Per-Output Rendering
```
GIVEN a window is configured for output "all"
WHEN window is rendered
THEN each output receives a separate texture at its native scale
AND texture cache is invalidated when output scale changes

GIVEN a window is configured for output "primary"
WHEN outputs change
THEN window only appears on current primary output
AND window moves if primary output changes
```

### Popup Positioning
```
GIVEN a popup with widget anchor
WHEN popup is shown
THEN popup position is computed relative to anchor widget's bounds
AND popup remains within screen bounds (constrained)

GIVEN a popup with cursor anchor
WHEN popup is shown
THEN popup appears at current cursor position
AND popup is constrained to screen bounds
```

### Input Handling
```
GIVEN multiple overlapping windows
WHEN pointer click occurs
THEN topmost window at click position receives event first
AND event propagation stops if window consumes event

GIVEN a popup with close_on_outside_click = true
WHEN click occurs outside popup bounds
THEN popup begins hide animation
AND click is consumed (not passed to windows below)
```

## Test Strategy

### Unit Tests
```rust
#[test]
fn test_window_visibility_state_machine() {
    let clock = Clock::new();
    let mut window = create_test_window();
    
    assert!(matches!(window.visibility, VisibilityState::Hidden));
    
    window.show(&clock);
    assert!(matches!(window.visibility, VisibilityState::Showing(_)));
    
    // Advance time past animation
    clock.advance(Duration::from_millis(300));
    window.advance_animations(&clock);
    assert!(matches!(window.visibility, VisibilityState::Shown));
    
    window.hide(&clock);
    assert!(matches!(window.visibility, VisibilityState::Hiding(_)));
}

#[test]
fn test_window_anchor_positioning() {
    let output = create_test_output(1920, 1080);
    
    // Top anchor
    let window = create_window_with_anchor(WindowAnchor::TOP, Size::from((100, 50)));
    let pos = window.compute_position(&output);
    assert_eq!(pos.y, 0);
    assert_eq!(pos.x, (1920 - 100) / 2); // Centered horizontally
    
    // Bottom-right anchor with margins
    let window = create_window_with_config(WindowConfig {
        anchor: WindowAnchor::BOTTOM_RIGHT,
        margins: WindowMargins { right: 10, bottom: 10, ..Default::default() },
        ..Default::default()
    }, Size::from((100, 50)));
    let pos = window.compute_position(&output);
    assert_eq!(pos.x, 1920 - 100 - 10);
    assert_eq!(pos.y, 1080 - 50 - 10);
}

#[test]
fn test_popup_outside_click_closes() {
    let mut manager = WindowManager::new();
    let popup = create_popup_with_config(PopupConfig {
        close_on_outside_click: true,
        ..Default::default()
    });
    
    manager.show_popup(popup, &Clock::new(), &TestResolver);
    
    // Click outside popup
    let result = manager.handle_pointer(
        &output,
        Point::from((1000.0, 1000.0)), // Outside popup bounds
        PointerEvent::Button { pressed: true, button: 0x110 },
    );
    
    assert_eq!(result, UiInputResult::Consumed);
    // Popup should be hiding
    assert!(manager.popups[0].is_hiding());
}
```

### Integration Tests
```rust
#[test]
fn test_window_output_cache_invalidation() {
    let mut window = create_test_window();
    let output1 = create_output_with_scale(1.0);
    let output2 = create_output_with_scale(2.0);
    
    window.show(&Clock::new());
    window.layout_if_needed(Size::from((1920, 1080)));
    
    // Render for output1
    let elem1 = window.render_for_output(&output1, &mut renderer);
    assert!(elem1.is_some());
    
    // Render for output2 (different scale)
    let elem2 = window.render_for_output(&output2, &mut renderer);
    assert!(elem2.is_some());
    
    // Each output should have its own cached texture
    assert_eq!(window.output_cache.len(), 2);
}
```

## Related Specifications

- [Rendering Pipeline](./rendering.md) - Texture creation and render elements
- [Animation System](./animation.md) - Visibility animations
- [Widget System](./widget.md) - Widget trees hosted by windows
- [Compositor Integration](./compositor-integration.md) - Integration points with niri
