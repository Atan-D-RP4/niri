# Compositor Integration Specification

## Overview

This document specifies how niri-ui integrates with the niri compositor. The integration is designed to be minimal (<50 lines of changes to compositor code) while providing full access to rendering, input, and window management capabilities.

## Design Constraints

- **C1**: UI must never crash the compositor (graceful degradation)
- **C2**: All types/implementation inside niri-ui crate, minimal compositor changes
- **C3**: Aim for QtQuick-level flexibility

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Compositor (niri)                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │   Render    │  │    Input    │  │   Window    │              │
│  │   System    │  │   System    │  │   Stack     │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
│         │                │                │                      │
│         └────────────────┼────────────────┘                      │
│                          │                                       │
│                 ┌────────▼────────┐                              │
│                 │  UiCompositor   │  (trait, ~20 lines)          │
│                 │   Interface     │                              │
│                 └────────┬────────┘                              │
└──────────────────────────┼───────────────────────────────────────┘
                           │
┌──────────────────────────▼───────────────────────────────────────┐
│                        niri-ui crate                             │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                      UiManager                               │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │ │
│  │  │ Windows  │  │ Widgets  │  │ Services │  │ Effects  │    │ │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │ │
│  └─────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

## 1. UiCompositorInterface Trait

The compositor implements this trait to provide niri-ui with necessary capabilities.

### 1.1 Rust Types

```rust
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::output::Output;
use smithay::utils::{Logical, Point, Size};

/// Result of UI input processing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiInputResult {
    /// Input was consumed by UI
    Consumed,
    /// Input should be passed to compositor
    NotConsumed,
    /// Input was consumed and requests focus change
    ConsumedWithFocusChange,
}

/// Information about the current compositor state
#[derive(Debug, Clone)]
pub struct CompositorState {
    /// Current keyboard modifiers
    pub modifiers: Modifiers,
    /// Whether any grab is active
    pub grab_active: bool,
    /// Current pointer position (if any)
    pub pointer_position: Option<Point<f64, Logical>>,
    /// Currently focused output
    pub focused_output: Option<Output>,
}

/// Keyboard modifier state
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
}

/// Trait implemented by compositor to provide UI integration
pub trait UiCompositorInterface {
    /// Get the GlesRenderer for texture uploads
    fn renderer(&mut self) -> &mut GlesRenderer;
    
    /// Get all outputs
    fn outputs(&self) -> impl Iterator<Item = &Output>;
    
    /// Get the focused output
    fn focused_output(&self) -> Option<&Output>;
    
    /// Get current compositor state
    fn compositor_state(&self) -> CompositorState;
    
    /// Request a redraw for an output
    fn queue_redraw(&mut self, output: &Output);
    
    /// Request a redraw for all outputs
    fn queue_redraw_all(&mut self);
    
    /// Get the animation clock
    fn clock(&self) -> &Clock;
    
    /// Schedule a callback after duration
    fn schedule_callback(&mut self, delay: Duration, callback: Box<dyn FnOnce(&mut Self)>);
}
```

### 1.2 Compositor Implementation (Minimal)

The compositor implements this trait with approximately 30 lines:

```rust
// In src/niri.rs - approximately 30 lines of changes

impl UiCompositorInterface for Niri {
    fn renderer(&mut self) -> &mut GlesRenderer {
        self.backend.renderer()
    }
    
    fn outputs(&self) -> impl Iterator<Item = &Output> {
        self.global_space.outputs()
    }
    
    fn focused_output(&self) -> Option<&Output> {
        self.layout.focus().map(|f| f.output())
    }
    
    fn compositor_state(&self) -> CompositorState {
        CompositorState {
            modifiers: self.seat.get_keyboard().map(|k| k.modifier_state()).into(),
            grab_active: self.seat.get_pointer().map(|p| p.is_grabbed()).unwrap_or(false),
            pointer_position: self.seat.get_pointer().map(|p| p.current_location()),
            focused_output: self.focused_output().cloned(),
        }
    }
    
    fn queue_redraw(&mut self, output: &Output) {
        self.queue_redraw(output);
    }
    
    fn queue_redraw_all(&mut self) {
        for output in self.global_space.outputs() {
            self.queue_redraw(output);
        }
    }
    
    fn clock(&self) -> &Clock {
        &self.clock
    }
    
    fn schedule_callback(&mut self, delay: Duration, callback: Box<dyn FnOnce(&mut Self)>) {
        self.event_loop.insert_source(
            Timer::from_duration(delay),
            move |_, _, state| {
                callback(state);
                TimeoutAction::Drop
            },
        ).ok();
    }
}
```

## 2. UiManager

Central manager for all UI components within niri-ui.

### 2.1 Rust Types

```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Central manager for all UI components
pub struct UiManager {
    /// All registered windows
    windows: HashMap<WindowId, Arc<RwLock<UiWindow>>>,
    /// Window render order (back to front)
    window_order: Vec<WindowId>,
    /// Service manager
    services: ServiceManager,
    /// Cursor effects manager
    cursor_effects: CursorEffectsManager,
    /// Global style registry
    styles: StyleRegistry,
    /// Whether any window needs redraw
    needs_redraw: bool,
    /// ID counter
    next_id: u64,
}

impl UiManager {
    /// Create a new UI manager
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            window_order: Vec::new(),
            services: ServiceManager::new(),
            cursor_effects: CursorEffectsManager::new(),
            styles: StyleRegistry::new(),
            needs_redraw: false,
            next_id: 0,
        }
    }
    
    /// Register a new window
    pub fn register_window(&mut self, window: UiWindow) -> WindowId {
        let id = WindowId(self.next_id);
        self.next_id += 1;
        self.windows.insert(id, Arc::new(RwLock::new(window)));
        self.window_order.push(id);
        self.needs_redraw = true;
        id
    }
    
    /// Unregister a window
    pub fn unregister_window(&mut self, id: WindowId) -> Option<Arc<RwLock<UiWindow>>> {
        self.window_order.retain(|&wid| wid != id);
        self.windows.remove(&id)
    }
    
    /// Get a window by ID
    pub fn window(&self, id: WindowId) -> Option<&Arc<RwLock<UiWindow>>> {
        self.windows.get(&id)
    }
    
    /// Iterate windows in render order (back to front)
    pub fn windows_render_order(&self) -> impl Iterator<Item = &Arc<RwLock<UiWindow>>> {
        self.window_order.iter().filter_map(|id| self.windows.get(id))
    }
    
    /// Raise a window to the top
    pub fn raise_window(&mut self, id: WindowId) {
        if let Some(pos) = self.window_order.iter().position(|&wid| wid == id) {
            self.window_order.remove(pos);
            self.window_order.push(id);
        }
    }
    
    /// Mark that a redraw is needed
    pub fn invalidate(&mut self) {
        self.needs_redraw = true;
    }
    
    /// Check and clear redraw flag
    pub fn take_needs_redraw(&mut self) -> bool {
        std::mem::take(&mut self.needs_redraw)
    }
}
```

### 2.2 Lifecycle Methods

```rust
impl UiManager {
    /// Initialize the UI manager with compositor interface
    pub fn init<C: UiCompositorInterface>(&mut self, compositor: &mut C) -> Result<(), UiError> {
        // Initialize services (D-Bus connections)
        self.services.init()?;
        
        // Initialize cursor effects
        self.cursor_effects.init()?;
        
        Ok(())
    }
    
    /// Shutdown the UI manager
    pub fn shutdown(&mut self) {
        // Close all windows
        for window in self.windows.values() {
            if let Ok(mut w) = window.write() {
                w.close();
            }
        }
        self.windows.clear();
        self.window_order.clear();
        
        // Shutdown services
        self.services.shutdown();
    }
    
    /// Tick animations and effects
    pub fn tick<C: UiCompositorInterface>(&mut self, compositor: &C) {
        let clock = compositor.clock();
        
        // Tick all windows
        for window in self.windows.values() {
            if let Ok(mut w) = window.write() {
                if w.tick(clock) {
                    self.needs_redraw = true;
                }
            }
        }
        
        // Tick cursor effects
        if self.cursor_effects.tick(clock) {
            self.needs_redraw = true;
        }
        
        // Remove closed windows
        self.window_order.retain(|id| {
            self.windows.get(id)
                .and_then(|w| w.read().ok())
                .map(|w| !w.is_closed())
                .unwrap_or(false)
        });
        self.windows.retain(|_, w| {
            w.read().ok().map(|w| !w.is_closed()).unwrap_or(false)
        });
    }
}
```

## 3. Integration Points

### 3.1 Rendering Integration

```rust
impl UiManager {
    /// Produce render elements for an output
    pub fn render_elements<C: UiCompositorInterface>(
        &mut self,
        compositor: &mut C,
        output: &Output,
    ) -> Vec<PrimaryGpuTextureRenderElement> {
        let mut elements = Vec::new();
        let scale = output.current_scale().fractional_scale();
        let output_geo = compositor.output_geometry(output);
        
        // Render windows in order (back to front)
        for window in self.windows_render_order() {
            if let Ok(window) = window.read() {
                if window.is_visible_on(output) {
                    match window.render_element(compositor.renderer(), output, scale) {
                        Ok(element) => elements.push(element),
                        Err(e) => {
                            // C1: Never crash - log and skip
                            tracing::warn!("Failed to render UI window: {}", e);
                        }
                    }
                }
            }
        }
        
        // Render cursor effects
        if let Some(pos) = compositor.compositor_state().pointer_position {
            elements.extend(self.cursor_effects.render_elements(
                compositor.renderer(),
                output,
                pos,
                scale,
            ));
        }
        
        elements
    }
}
```

### 3.2 Input Integration

```rust
impl UiManager {
    /// Handle pointer motion
    pub fn on_pointer_motion(
        &mut self,
        position: Point<f64, Logical>,
        output: &Output,
    ) -> UiInputResult {
        // Check windows in reverse order (front to back)
        for id in self.window_order.iter().rev().copied() {
            if let Some(window) = self.windows.get(&id) {
                if let Ok(mut window) = window.write() {
                    if window.is_visible_on(output) && window.contains_point(position) {
                        let local_pos = window.to_local(position);
                        if window.on_pointer_motion(local_pos) {
                            self.needs_redraw = true;
                            return UiInputResult::Consumed;
                        }
                    }
                }
            }
        }
        UiInputResult::NotConsumed
    }
    
    /// Handle pointer button
    pub fn on_pointer_button(
        &mut self,
        position: Point<f64, Logical>,
        button: u32,
        state: ButtonState,
        output: &Output,
    ) -> UiInputResult {
        // Check windows in reverse order (front to back)
        for id in self.window_order.iter().rev().copied() {
            if let Some(window) = self.windows.get(&id) {
                if let Ok(mut window) = window.write() {
                    if window.is_visible_on(output) && window.contains_point(position) {
                        let local_pos = window.to_local(position);
                        let result = window.on_pointer_button(local_pos, button, state);
                        if result != UiInputResult::NotConsumed {
                            self.needs_redraw = true;
                            
                            // Raise window on click
                            if state == ButtonState::Pressed {
                                self.raise_window(id);
                            }
                            
                            return result;
                        }
                    }
                }
            }
        }
        
        // Trigger click ripple effect
        if state == ButtonState::Pressed {
            self.cursor_effects.trigger_click_ripple(position);
            self.needs_redraw = true;
        }
        
        UiInputResult::NotConsumed
    }
    
    /// Handle keyboard input
    pub fn on_keyboard_key(
        &mut self,
        key: u32,
        state: KeyState,
        modifiers: Modifiers,
    ) -> UiInputResult {
        // Find focused window
        if let Some(focused_id) = self.focused_window_id() {
            if let Some(window) = self.windows.get(&focused_id) {
                if let Ok(mut window) = window.write() {
                    let result = window.on_keyboard_key(key, state, modifiers);
                    if result != UiInputResult::NotConsumed {
                        self.needs_redraw = true;
                        return result;
                    }
                }
            }
        }
        UiInputResult::NotConsumed
    }
    
    /// Handle scroll
    pub fn on_scroll(
        &mut self,
        position: Point<f64, Logical>,
        delta: Point<f64, Logical>,
        output: &Output,
    ) -> UiInputResult {
        for id in self.window_order.iter().rev().copied() {
            if let Some(window) = self.windows.get(&id) {
                if let Ok(mut window) = window.write() {
                    if window.is_visible_on(output) && window.contains_point(position) {
                        let local_pos = window.to_local(position);
                        if window.on_scroll(local_pos, delta) {
                            self.needs_redraw = true;
                            return UiInputResult::Consumed;
                        }
                    }
                }
            }
        }
        UiInputResult::NotConsumed
    }
    
    /// Get the currently focused window ID
    fn focused_window_id(&self) -> Option<WindowId> {
        // Last window in order that accepts focus
        self.window_order.iter().rev().copied().find(|id| {
            self.windows.get(id)
                .and_then(|w| w.read().ok())
                .map(|w| w.accepts_focus())
                .unwrap_or(false)
        })
    }
}
```

### 3.3 Output Change Handling

```rust
impl UiManager {
    /// Handle output added
    pub fn on_output_added(&mut self, output: &Output) {
        // Notify all windows
        for window in self.windows.values() {
            if let Ok(mut w) = window.write() {
                w.on_output_added(output);
            }
        }
        self.needs_redraw = true;
    }
    
    /// Handle output removed
    pub fn on_output_removed(&mut self, output: &Output) {
        // Notify all windows
        for window in self.windows.values() {
            if let Ok(mut w) = window.write() {
                w.on_output_removed(output);
            }
        }
        // Invalidate cached textures for this output
        for window in self.windows.values() {
            if let Ok(mut w) = window.write() {
                w.invalidate_output_cache(output);
            }
        }
    }
    
    /// Handle output scale change
    pub fn on_output_scale_changed(&mut self, output: &Output, new_scale: f64) {
        // Invalidate cached textures for this output
        for window in self.windows.values() {
            if let Ok(mut w) = window.write() {
                w.invalidate_output_cache(output);
            }
        }
        self.needs_redraw = true;
    }
}
```

## 4. Compositor Integration Code

The actual code changes needed in the compositor:

### 4.1 State Initialization (~5 lines)

```rust
// In src/niri.rs, add to Niri struct:
pub struct Niri {
    // ... existing fields ...
    
    /// UI manager for custom widgets
    pub ui_manager: UiManager,
}

// In Niri::new():
let ui_manager = UiManager::new();
ui_manager.init(&mut *self)?;
```

### 4.2 Render Integration (~10 lines)

```rust
// In rendering code, after rendering windows but before cursor:
let ui_elements = self.ui_manager.render_elements(&mut *self, output);
for element in ui_elements {
    frame.render_element(element, output_transform)?;
}
```

### 4.3 Input Integration (~15 lines)

```rust
// In input handling:
fn on_pointer_motion(&mut self, event: PointerMotionEvent) {
    let position = event.position;
    let output = self.output_at(position);
    
    // Check UI first
    if self.ui_manager.on_pointer_motion(position, &output) == UiInputResult::Consumed {
        return;
    }
    
    // ... existing pointer handling ...
}

fn on_pointer_button(&mut self, event: PointerButtonEvent) {
    let position = self.pointer_position();
    let output = self.output_at(position);
    
    // Check UI first
    if self.ui_manager.on_pointer_button(position, event.button, event.state, &output) 
        != UiInputResult::NotConsumed 
    {
        return;
    }
    
    // ... existing button handling ...
}
```

### 4.4 Tick Integration (~5 lines)

```rust
// In main loop or frame callback:
self.ui_manager.tick(&*self);
if self.ui_manager.take_needs_redraw() {
    self.queue_redraw_all();
}
```

## 5. Error Handling

All operations follow constraint C1 (never crash):

```rust
impl UiManager {
    /// Safe wrapper for operations that might fail
    fn with_window<F, R>(&self, id: WindowId, f: F) -> Option<R>
    where
        F: FnOnce(&UiWindow) -> R,
    {
        self.windows.get(&id)
            .and_then(|w| w.read().ok())
            .map(|w| f(&*w))
    }
    
    /// Safe wrapper for mutable operations
    fn with_window_mut<F, R>(&self, id: WindowId, f: F) -> Option<R>
    where
        F: FnOnce(&mut UiWindow) -> R,
    {
        self.windows.get(&id)
            .and_then(|w| w.write().ok())
            .map(|mut w| f(&mut *w))
    }
}
```

## 6. Acceptance Criteria

### 6.1 Initialization

```gherkin
GIVEN the compositor is starting up
WHEN UiManager::init() is called
THEN the UI manager initializes successfully
AND no errors are propagated to the compositor
AND the compositor can continue without UI if init fails

GIVEN the compositor is shutting down
WHEN UiManager::shutdown() is called
THEN all windows are closed gracefully
AND all services are disconnected
AND no resources are leaked
```

### 6.2 Rendering

```gherkin
GIVEN a UI window is visible on an output
WHEN render_elements() is called for that output
THEN the window's render element is included
AND the element is positioned correctly
AND the element uses the correct scale

GIVEN a UI window render fails
WHEN render_elements() is called
THEN the error is logged
AND no element is produced for that window
AND the compositor continues normally
```

### 6.3 Input

```gherkin
GIVEN a UI window is under the pointer
WHEN a pointer button is pressed
THEN the window receives the event first
AND UiInputResult::Consumed is returned if handled
AND the compositor does not process the event

GIVEN no UI window is under the pointer
WHEN a pointer button is pressed
THEN UiInputResult::NotConsumed is returned
AND the compositor processes the event normally
```

### 6.4 Output Changes

```gherkin
GIVEN UI windows have cached textures for an output
WHEN the output's scale changes
THEN all cached textures for that output are invalidated
AND windows re-render at the new scale

GIVEN UI windows are visible on an output
WHEN the output is removed
THEN windows are notified
AND windows update their visibility state
```

## 7. Test Strategy

### 7.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    struct MockCompositor {
        outputs: Vec<Output>,
        redraw_requested: bool,
    }
    
    impl UiCompositorInterface for MockCompositor {
        // ... mock implementation ...
    }
    
    #[test]
    fn test_window_registration() {
        let mut manager = UiManager::new();
        let window = UiWindow::new(/* ... */);
        
        let id = manager.register_window(window);
        assert!(manager.window(id).is_some());
        
        manager.unregister_window(id);
        assert!(manager.window(id).is_none());
    }
    
    #[test]
    fn test_input_routing() {
        let mut manager = UiManager::new();
        let mut compositor = MockCompositor::new();
        
        // Create window at (100, 100) with size (200, 200)
        let window = UiWindow::new(/* ... */);
        manager.register_window(window);
        
        // Click inside window
        let result = manager.on_pointer_button(
            Point::from((150.0, 150.0)),
            BTN_LEFT,
            ButtonState::Pressed,
            &compositor.outputs[0],
        );
        assert_eq!(result, UiInputResult::Consumed);
        
        // Click outside window
        let result = manager.on_pointer_button(
            Point::from((50.0, 50.0)),
            BTN_LEFT,
            ButtonState::Pressed,
            &compositor.outputs[0],
        );
        assert_eq!(result, UiInputResult::NotConsumed);
    }
    
    #[test]
    fn test_window_ordering() {
        let mut manager = UiManager::new();
        
        let id1 = manager.register_window(UiWindow::new(/* ... */));
        let id2 = manager.register_window(UiWindow::new(/* ... */));
        
        // id2 should be on top
        assert_eq!(manager.window_order.last(), Some(&id2));
        
        // Raise id1
        manager.raise_window(id1);
        assert_eq!(manager.window_order.last(), Some(&id1));
    }
    
    #[test]
    fn test_graceful_degradation() {
        let mut manager = UiManager::new();
        
        // Register a window that will fail to render
        let window = UiWindow::new_failing(/* ... */);
        manager.register_window(window);
        
        let mut compositor = MockCompositor::new();
        
        // Should not panic, should return empty vec
        let elements = manager.render_elements(&mut compositor, &compositor.outputs[0]);
        // May be empty if render failed, but should not crash
    }
}
```

### 7.2 Integration Tests

```rust
#[test]
fn test_full_lifecycle() {
    let mut compositor = MockCompositor::new();
    let mut manager = UiManager::new();
    
    // Initialize
    manager.init(&mut compositor).unwrap();
    
    // Create window via Lua-like API
    let window = UiWindow::builder()
        .position(100, 100)
        .size(200, 200)
        .build();
    let id = manager.register_window(window);
    
    // Simulate frame
    manager.tick(&compositor);
    let elements = manager.render_elements(&mut compositor, &compositor.outputs[0]);
    assert!(!elements.is_empty());
    
    // Simulate click
    let result = manager.on_pointer_button(
        Point::from((150.0, 150.0)),
        BTN_LEFT,
        ButtonState::Pressed,
        &compositor.outputs[0],
    );
    assert_eq!(result, UiInputResult::Consumed);
    
    // Close window
    manager.unregister_window(id);
    
    // Shutdown
    manager.shutdown();
}
```

## 8. Performance Considerations

1. **Window lookup**: Use `HashMap` for O(1) window lookup by ID
2. **Render order**: Maintain separate `Vec` for render order to avoid sorting
3. **Input hit testing**: Test windows front-to-back, exit early on hit
4. **Texture caching**: Per-output texture caches prevent re-rendering
5. **Lazy invalidation**: Only re-render when `needs_redraw` is set

## References

- [rendering.md](rendering.md) - Rendering pipeline details
- [window.md](window.md) - Window system specification
- [widget.md](widget.md) - Widget system specification
- [cursor-effects.md](cursor-effects.md) - Cursor effects specification
