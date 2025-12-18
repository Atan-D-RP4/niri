# Cursor Effects Specification

## Overview

The cursor effects system provides visual feedback for pointer interactions including click ripples, cursor trails, and custom cursor rendering. Effects are compositor-native with Lua configuration.

## Core Types

### CursorEffect Trait

```rust
/// A visual effect rendered at or near the cursor position
pub trait CursorEffect: Send + Sync {
    /// Update effect state, return true if still active
    fn tick(&mut self, clock: &Clock) -> bool;
    
    /// Check if effect should continue rendering
    fn is_active(&self) -> bool;
    
    /// Render effect to Cairo context
    fn render(&self, ctx: &cairo::Context, cursor_pos: Point<f64, Logical>);
    
    /// Get bounding box for damage tracking
    fn bounds(&self) -> Rectangle<f64, Logical>;
}
```

### ClickRipple

```rust
/// Expanding circle effect on mouse click
pub struct ClickRipple {
    /// Center position of the ripple
    center: Point<f64, Logical>,
    /// Animation controlling expansion and fade
    animation: Animation,
    /// Maximum radius when fully expanded
    max_radius: f64,
    /// Color of the ripple (with alpha for fade)
    color: Color,
    /// Current state
    state: RippleState,
}

pub enum RippleState {
    Expanding,
    Fading,
    Complete,
}

impl ClickRipple {
    pub fn new(
        center: Point<f64, Logical>,
        clock: &Clock,
        config: &RippleConfig,
    ) -> Self {
        Self {
            center,
            animation: Animation::new(clock, 0.0, 1.0, config.duration)
                .with_curve(Curve::EaseOutCubic),
            max_radius: config.max_radius,
            color: config.color,
            state: RippleState::Expanding,
        }
    }
}

impl CursorEffect for ClickRipple {
    fn tick(&mut self, clock: &Clock) -> bool {
        self.animation.tick(clock);
        
        if self.animation.is_done() {
            self.state = RippleState::Complete;
            false
        } else {
            true
        }
    }
    
    fn is_active(&self) -> bool {
        !matches!(self.state, RippleState::Complete)
    }
    
    fn render(&self, ctx: &cairo::Context, _cursor_pos: Point<f64, Logical>) {
        let progress = self.animation.value();
        let radius = self.max_radius * progress;
        let alpha = self.color.alpha * (1.0 - progress); // Fade as it expands
        
        ctx.arc(
            self.center.x,
            self.center.y,
            radius,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        
        ctx.set_source_rgba(
            self.color.red,
            self.color.green,
            self.color.blue,
            alpha,
        );
        ctx.fill().unwrap();
    }
    
    fn bounds(&self) -> Rectangle<f64, Logical> {
        let radius = self.max_radius;
        Rectangle::from_loc_and_size(
            (self.center.x - radius, self.center.y - radius),
            (radius * 2.0, radius * 2.0),
        )
    }
}
```

### CursorTrail

```rust
/// Trail of fading points following cursor movement
pub struct CursorTrail {
    /// Historical cursor positions with timestamps
    points: VecDeque<TrailPoint>,
    /// Maximum number of points to keep
    max_points: usize,
    /// How long each point lives (ms)
    point_lifetime: Duration,
    /// Trail style configuration
    style: TrailStyle,
}

pub struct TrailPoint {
    position: Point<f64, Logical>,
    timestamp: Duration,
}

pub struct TrailStyle {
    pub color: Color,
    pub start_width: f64,
    pub end_width: f64,
    pub interpolation: TrailInterpolation,
}

pub enum TrailInterpolation {
    Linear,
    CatmullRom,
    Bezier,
}

impl CursorTrail {
    pub fn new(config: &TrailConfig) -> Self {
        Self {
            points: VecDeque::with_capacity(config.max_points),
            max_points: config.max_points,
            point_lifetime: config.point_lifetime,
            style: config.style.clone(),
        }
    }
    
    /// Add a new point to the trail
    pub fn add_point(&mut self, pos: Point<f64, Logical>, clock: &Clock) {
        self.points.push_back(TrailPoint {
            position: pos,
            timestamp: clock.now(),
        });
        
        // Limit trail length
        while self.points.len() > self.max_points {
            self.points.pop_front();
        }
    }
}

impl CursorEffect for CursorTrail {
    fn tick(&mut self, clock: &Clock) -> bool {
        let now = clock.now();
        
        // Remove expired points
        while let Some(front) = self.points.front() {
            if now.saturating_sub(front.timestamp) > self.point_lifetime {
                self.points.pop_front();
            } else {
                break;
            }
        }
        
        !self.points.is_empty()
    }
    
    fn is_active(&self) -> bool {
        !self.points.is_empty()
    }
    
    fn render(&self, ctx: &cairo::Context, _cursor_pos: Point<f64, Logical>) {
        if self.points.len() < 2 {
            return;
        }
        
        let points: Vec<_> = self.points.iter().collect();
        
        for (i, window) in points.windows(2).enumerate() {
            let progress = i as f64 / (points.len() - 1) as f64;
            let width = self.style.start_width 
                + (self.style.end_width - self.style.start_width) * progress;
            let alpha = self.style.color.alpha * (1.0 - progress);
            
            ctx.set_line_width(width);
            ctx.set_source_rgba(
                self.style.color.red,
                self.style.color.green,
                self.style.color.blue,
                alpha,
            );
            
            ctx.move_to(window[0].position.x, window[0].position.y);
            ctx.line_to(window[1].position.x, window[1].position.y);
            ctx.stroke().unwrap();
        }
    }
    
    fn bounds(&self) -> Rectangle<f64, Logical> {
        if self.points.is_empty() {
            return Rectangle::default();
        }
        
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        
        for point in &self.points {
            min_x = min_x.min(point.position.x);
            min_y = min_y.min(point.position.y);
            max_x = max_x.max(point.position.x);
            max_y = max_y.max(point.position.y);
        }
        
        let padding = self.style.start_width;
        Rectangle::from_loc_and_size(
            (min_x - padding, min_y - padding),
            (max_x - min_x + padding * 2.0, max_y - min_y + padding * 2.0),
        )
    }
}
```

### CursorEffectsManager

```rust
/// Manages all active cursor effects
pub struct CursorEffectsManager {
    /// Active effects
    effects: Vec<Box<dyn CursorEffect>>,
    /// Persistent trail (if enabled)
    trail: Option<CursorTrail>,
    /// Configuration
    config: CursorEffectsConfig,
    /// Per-output render cache
    render_cache: HashMap<WeakOutput, RenderedEffects>,
}

pub struct CursorEffectsConfig {
    pub ripple: Option<RippleConfig>,
    pub trail: Option<TrailConfig>,
    pub custom_cursor: Option<CustomCursorConfig>,
}

pub struct RippleConfig {
    pub enabled: bool,
    pub max_radius: f64,
    pub duration: Duration,
    pub color: Color,
    pub on_left_click: bool,
    pub on_right_click: bool,
    pub on_middle_click: bool,
}

pub struct TrailConfig {
    pub enabled: bool,
    pub max_points: usize,
    pub point_lifetime: Duration,
    pub style: TrailStyle,
}

impl CursorEffectsManager {
    pub fn new(config: CursorEffectsConfig) -> Self {
        let trail = config.trail.as_ref()
            .filter(|c| c.enabled)
            .map(CursorTrail::new);
        
        Self {
            effects: Vec::new(),
            trail,
            config,
            render_cache: HashMap::new(),
        }
    }
    
    /// Handle mouse button press
    pub fn on_button_press(
        &mut self,
        button: MouseButton,
        pos: Point<f64, Logical>,
        clock: &Clock,
    ) {
        if let Some(ref config) = self.config.ripple {
            let should_ripple = match button {
                MouseButton::Left => config.on_left_click,
                MouseButton::Right => config.on_right_click,
                MouseButton::Middle => config.on_middle_click,
                _ => false,
            };
            
            if should_ripple {
                self.effects.push(Box::new(ClickRipple::new(pos, clock, config)));
                self.invalidate_cache();
            }
        }
    }
    
    /// Handle cursor movement
    pub fn on_cursor_move(&mut self, pos: Point<f64, Logical>, clock: &Clock) {
        if let Some(ref mut trail) = self.trail {
            trail.add_point(pos, clock);
            self.invalidate_cache();
        }
    }
    
    /// Tick all effects, remove completed ones
    pub fn tick(&mut self, clock: &Clock) -> bool {
        let mut needs_redraw = false;
        
        // Tick and retain active effects
        self.effects.retain_mut(|effect| {
            let active = effect.tick(clock);
            if active {
                needs_redraw = true;
            }
            active
        });
        
        // Tick trail
        if let Some(ref mut trail) = self.trail {
            if trail.tick(clock) {
                needs_redraw = true;
            }
        }
        
        if needs_redraw {
            self.invalidate_cache();
        }
        
        needs_redraw
    }
    
    /// Check if any effects are active
    pub fn has_active_effects(&self) -> bool {
        !self.effects.is_empty() 
            || self.trail.as_ref().map_or(false, |t| t.is_active())
    }
    
    /// Render all effects for an output
    pub fn render(
        &mut self,
        output: &Output,
        cursor_pos: Point<f64, Logical>,
    ) -> Option<PrimaryGpuTextureRenderElement> {
        if !self.has_active_effects() {
            return None;
        }
        
        let scale = output.current_scale().fractional_scale();
        let output_size = output.current_mode().unwrap().size;
        
        // Check cache
        let weak_output = output.downgrade();
        if let Some(cached) = self.render_cache.get(&weak_output) {
            if cached.scale == scale {
                return Some(cached.element.clone());
            }
        }
        
        // Render to new texture
        let physical_size = output_size.to_f64().to_physical(scale);
        let surface = cairo::ImageSurface::create(
            cairo::Format::ARgb32,
            physical_size.w as i32,
            physical_size.h as i32,
        ).unwrap();
        
        let ctx = cairo::Context::new(&surface).unwrap();
        ctx.scale(scale, scale);
        
        // Render trail first (underneath)
        if let Some(ref trail) = self.trail {
            trail.render(&ctx, cursor_pos);
        }
        
        // Render other effects
        for effect in &self.effects {
            effect.render(&ctx, cursor_pos);
        }
        
        drop(ctx);
        surface.flush();
        
        // Upload to GPU
        let data = surface.data().unwrap();
        let texture = TextureBuffer::from_memory(
            &*data,
            Fourcc::Argb8888,
            (physical_size.w as i32, physical_size.h as i32),
            false,
            scale as i32,
            Transform::Normal,
            None,
        ).unwrap();
        
        let element = PrimaryGpuTextureRenderElement::from_texture_buffer(
            texture,
            Point::from((0.0, 0.0)),
            1.0, // alpha
            None,
            None,
            Kind::Unspecified,
        );
        
        // Cache
        self.render_cache.insert(weak_output, RenderedEffects {
            element: element.clone(),
            scale,
        });
        
        Some(element)
    }
    
    fn invalidate_cache(&mut self) {
        self.render_cache.clear();
    }
}

struct RenderedEffects {
    element: PrimaryGpuTextureRenderElement,
    scale: f64,
}
```

## Lua API

### Configuration

```lua
-- Configure cursor effects
niri.cursor.configure({
    ripple = {
        enabled = true,
        max_radius = 30,
        duration_ms = 400,
        color = "#ffffff40",  -- white with 25% opacity
        on_left_click = true,
        on_right_click = false,
        on_middle_click = false,
    },
    trail = {
        enabled = false,
        max_points = 20,
        point_lifetime_ms = 200,
        color = "#ffffff60",
        start_width = 4,
        end_width = 1,
        interpolation = "linear",  -- "linear", "catmull_rom", "bezier"
    },
})
```

### Runtime Control

```lua
-- Enable/disable effects at runtime
niri.cursor.set_ripple_enabled(true)
niri.cursor.set_trail_enabled(false)

-- Trigger manual ripple (for custom interactions)
niri.cursor.trigger_ripple({
    x = 100,
    y = 200,
    color = "#ff000080",
    radius = 50,
    duration_ms = 300,
})

-- Get current cursor position
local pos = niri.cursor.position()
print(pos.x, pos.y)

-- Custom cursor (future)
niri.cursor.set_cursor({
    image = "/path/to/cursor.png",
    hotspot = { x = 0, y = 0 },
    size = 24,
})
niri.cursor.reset_cursor()
```

### Event Hooks

```lua
-- React to cursor events
niri.events.on("cursor_move", function(event)
    -- event.x, event.y, event.output
end)

niri.events.on("button_press", function(event)
    -- event.button, event.x, event.y
    -- Custom ripple on specific conditions
    if event.button == "left" and some_condition then
        niri.cursor.trigger_ripple({
            x = event.x,
            y = event.y,
            color = "#00ff00",
        })
    end
end)
```

## Acceptance Criteria

### AC1: Click Ripple Effect
```
GIVEN cursor effects are enabled with ripple configuration
WHEN user clicks the left mouse button
THEN a circular ripple expands from click position
AND the ripple fades as it expands
AND the ripple completes within configured duration
```

### AC2: Cursor Trail Effect
```
GIVEN cursor effects are enabled with trail configuration
WHEN user moves the cursor
THEN a trail of points follows the cursor
AND older points fade out over time
AND trail respects max_points limit
```

### AC3: Per-Output Rendering
```
GIVEN cursor effects are active
WHEN effects need to render on multiple outputs
THEN each output gets its own texture at correct scale
AND cache is invalidated when scale changes
```

### AC4: Graceful Degradation
```
GIVEN cursor effects system is running
WHEN an effect fails to render
THEN the failure is logged
AND other effects continue to render
AND compositor remains stable
```

### AC5: Lua Configuration
```
GIVEN default cursor effects configuration
WHEN Lua script calls niri.cursor.configure()
THEN effect parameters are updated
AND changes apply to new effects immediately
```

## Test Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_click_ripple_lifecycle() {
        let clock = Clock::new();
        let config = RippleConfig {
            max_radius: 30.0,
            duration: Duration::from_millis(400),
            color: Color::WHITE,
            ..Default::default()
        };
        
        let mut ripple = ClickRipple::new(
            Point::from((100.0, 100.0)),
            &clock,
            &config,
        );
        
        assert!(ripple.is_active());
        
        // Simulate time passing
        let clock = clock.advance(Duration::from_millis(200));
        assert!(ripple.tick(&clock));
        assert!(ripple.is_active());
        
        // Complete animation
        let clock = clock.advance(Duration::from_millis(250));
        assert!(!ripple.tick(&clock));
        assert!(!ripple.is_active());
    }
    
    #[test]
    fn test_cursor_trail_point_expiry() {
        let config = TrailConfig {
            max_points: 10,
            point_lifetime: Duration::from_millis(100),
            ..Default::default()
        };
        
        let mut trail = CursorTrail::new(&config);
        let clock = Clock::new();
        
        // Add points
        trail.add_point(Point::from((0.0, 0.0)), &clock);
        trail.add_point(Point::from((10.0, 10.0)), &clock);
        
        assert_eq!(trail.points.len(), 2);
        
        // Advance past lifetime
        let clock = clock.advance(Duration::from_millis(150));
        trail.tick(&clock);
        
        assert!(trail.points.is_empty());
    }
    
    #[test]
    fn test_effects_manager_button_press() {
        let config = CursorEffectsConfig {
            ripple: Some(RippleConfig {
                enabled: true,
                on_left_click: true,
                ..Default::default()
            }),
            trail: None,
            custom_cursor: None,
        };
        
        let mut manager = CursorEffectsManager::new(config);
        let clock = Clock::new();
        
        assert!(!manager.has_active_effects());
        
        manager.on_button_press(
            MouseButton::Left,
            Point::from((50.0, 50.0)),
            &clock,
        );
        
        assert!(manager.has_active_effects());
        assert_eq!(manager.effects.len(), 1);
    }
    
    #[test]
    fn test_ripple_bounds() {
        let clock = Clock::new();
        let config = RippleConfig {
            max_radius: 30.0,
            ..Default::default()
        };
        
        let ripple = ClickRipple::new(
            Point::from((100.0, 100.0)),
            &clock,
            &config,
        );
        
        let bounds = ripple.bounds();
        assert_eq!(bounds.loc.x, 70.0);  // 100 - 30
        assert_eq!(bounds.loc.y, 70.0);
        assert_eq!(bounds.size.w, 60.0); // 30 * 2
        assert_eq!(bounds.size.h, 60.0);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_cursor_effects_render_pipeline() {
    let config = CursorEffectsConfig::default();
    let mut manager = CursorEffectsManager::new(config);
    
    // Create mock output
    let output = create_test_output(1920, 1080, 1.0);
    let clock = Clock::new();
    
    // Add effect
    manager.on_button_press(
        MouseButton::Left,
        Point::from((100.0, 100.0)),
        &clock,
    );
    
    // Render
    let element = manager.render(&output, Point::from((100.0, 100.0)));
    assert!(element.is_some());
    
    // Verify cached
    let element2 = manager.render(&output, Point::from((100.0, 100.0)));
    assert!(element2.is_some());
}
```

## Performance Considerations

1. **Effect Pooling**: Reuse effect instances to avoid allocations
2. **Bounds Tracking**: Only re-render damaged regions
3. **Cache Management**: Per-output caching with scale awareness
4. **Trail Optimization**: Use VecDeque for efficient point management
5. **Render Batching**: Combine multiple effects into single texture

## Future Extensions

- Custom cursor images from Lua
- Cursor scaling animations
- Hover highlight effects
- Drag visual feedback
- Touch ripples for touchscreen
- Accessibility cursor enlargement
