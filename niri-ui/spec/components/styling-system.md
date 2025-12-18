# Styling System Specification

## Overview

The styling system provides a flexible, state-aware approach to widget appearance. It supports named styles, state variants (hover, active, focused, disabled), and a priority-based resolution system that merges base styles with state-specific overrides.

## Design Principles

1. **State-aware**: Styles respond to widget interaction states
2. **Composable**: Base styles can be extended with state variants
3. **Lua-friendly**: Easy to define in Lua tables with intuitive syntax
4. **Performance-conscious**: Style resolution is cached until state changes

## Core Types

### Color

```rust
/// RGBA color with alpha channel
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,  // 0.0 - 1.0
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    
    /// Parse from hex string: "#RGB", "#RGBA", "#RRGGBB", "#RRGGBBAA"
    pub fn from_hex(hex: &str) -> Result<Self, StyleError> { ... }
    
    /// Parse from CSS-style: "rgb(r, g, b)" or "rgba(r, g, b, a)"
    pub fn from_css(css: &str) -> Result<Self, StyleError> { ... }
    
    /// Interpolate between two colors for animations
    pub fn lerp(self, other: Color, t: f32) -> Color {
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }
    
    /// Convert to Cairo-compatible tuple
    pub fn to_cairo(&self) -> (f64, f64, f64, f64) {
        (self.r as f64, self.g as f64, self.b as f64, self.a as f64)
    }
}
```

### Gradient

```rust
/// Gradient stop for linear/radial gradients
#[derive(Debug, Clone)]
pub struct GradientStop {
    pub offset: f32,  // 0.0 - 1.0
    pub color: Color,
}

/// Gradient definition
#[derive(Debug, Clone)]
pub enum Gradient {
    Linear {
        angle: f32,  // degrees, 0 = left-to-right, 90 = top-to-bottom
        stops: Vec<GradientStop>,
    },
    Radial {
        center: (f32, f32),  // relative position (0.5, 0.5 = center)
        radius: f32,
        stops: Vec<GradientStop>,
    },
}

impl Gradient {
    /// Create Cairo pattern for rendering
    pub fn to_cairo_pattern(&self, bounds: Rect) -> cairo::Pattern { ... }
}
```

### Background

```rust
/// Widget background fill
#[derive(Debug, Clone)]
pub enum Background {
    None,
    Solid(Color),
    Gradient(Gradient),
    // Future: Image(ImageSource),
}

impl Background {
    pub fn is_none(&self) -> bool {
        matches!(self, Background::None)
    }
}
```

### Border

```rust
/// Border style for a single edge
#[derive(Debug, Clone, Copy)]
pub struct BorderSide {
    pub width: f32,
    pub color: Color,
}

/// Complete border specification
#[derive(Debug, Clone)]
pub struct Border {
    pub top: Option<BorderSide>,
    pub right: Option<BorderSide>,
    pub bottom: Option<BorderSide>,
    pub left: Option<BorderSide>,
    pub radius: BorderRadius,
}

/// Corner radii for rounded borders
#[derive(Debug, Clone, Copy, Default)]
pub struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl BorderRadius {
    pub fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }
    
    pub fn is_zero(&self) -> bool {
        self.top_left == 0.0 && self.top_right == 0.0 
            && self.bottom_right == 0.0 && self.bottom_left == 0.0
    }
}
```

### Shadow

```rust
/// Box shadow effect
#[derive(Debug, Clone)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: Color,
    pub inset: bool,
}
```

### TextStyle

```rust
/// Typography styling
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub color: Option<Color>,
    pub line_height: Option<f32>,
    pub letter_spacing: Option<f32>,
    pub text_align: Option<TextAlign>,
    pub text_decoration: Option<TextDecoration>,
}

#[derive(Debug, Clone, Copy)]
pub enum FontWeight {
    Thin,       // 100
    Light,      // 300
    Regular,    // 400
    Medium,     // 500
    SemiBold,   // 600
    Bold,       // 700
    ExtraBold,  // 800
    Black,      // 900
}

#[derive(Debug, Clone, Copy)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Clone, Copy)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy)]
pub enum TextDecoration {
    None,
    Underline,
    Overline,
    LineThrough,
}
```

## Widget State

```rust
bitflags::bitflags! {
    /// Widget interaction state flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct WidgetState: u8 {
        const HOVERED  = 0b0001;
        const ACTIVE   = 0b0010;  // Mouse button held
        const FOCUSED  = 0b0100;
        const DISABLED = 0b1000;
    }
}

impl WidgetState {
    pub fn is_hovered(self) -> bool { self.contains(Self::HOVERED) }
    pub fn is_active(self) -> bool { self.contains(Self::ACTIVE) }
    pub fn is_focused(self) -> bool { self.contains(Self::FOCUSED) }
    pub fn is_disabled(self) -> bool { self.contains(Self::DISABLED) }
}
```

## Style Structure

```rust
/// Complete style definition for a widget
#[derive(Debug, Clone, Default)]
pub struct Style {
    // Box model
    pub padding: Option<Edges>,
    pub margin: Option<Edges>,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    
    // Visual
    pub background: Option<Background>,
    pub border: Option<Border>,
    pub shadow: Option<Vec<Shadow>>,
    pub opacity: Option<f32>,
    
    // Typography (for text widgets)
    pub text: Option<TextStyle>,
    
    // Cursor
    pub cursor: Option<CursorIcon>,
    
    // Transitions (for animated style changes)
    pub transition_duration: Option<Duration>,
    pub transition_easing: Option<Curve>,
}

/// Edge values (padding, margin)
#[derive(Debug, Clone, Copy, Default)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub fn uniform(value: f32) -> Self {
        Self { top: value, right: value, bottom: value, left: value }
    }
    
    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self { top: vertical, right: horizontal, bottom: vertical, left: horizontal }
    }
    
    pub fn horizontal(&self) -> f32 { self.left + self.right }
    pub fn vertical(&self) -> f32 { self.top + self.bottom }
}
```

## Stateful Style

```rust
/// Style with state variants
#[derive(Debug, Clone, Default)]
pub struct StatefulStyle {
    /// Base style (always applied)
    pub base: Style,
    
    /// State-specific overrides
    pub hovered: Option<Style>,
    pub active: Option<Style>,
    pub focused: Option<Style>,
    pub disabled: Option<Style>,
}

impl StatefulStyle {
    /// Resolve final style for given widget state
    pub fn resolve(&self, state: WidgetState) -> ResolvedStyle {
        StyleResolver::resolve(self, state)
    }
}
```

## Style Resolution

```rust
/// Priority-based style resolver
pub struct StyleResolver;

impl StyleResolver {
    /// Resolution priority (highest to lowest):
    /// 1. disabled (if set)
    /// 2. active (if set)
    /// 3. focused (if set)
    /// 4. hovered (if set)
    /// 5. base
    pub fn resolve(style: &StatefulStyle, state: WidgetState) -> ResolvedStyle {
        let mut resolved = ResolvedStyle::from(&style.base);
        
        // Apply state variants in priority order (lowest to highest)
        if state.is_hovered() {
            if let Some(ref hover) = style.hovered {
                resolved.merge(hover);
            }
        }
        
        if state.is_focused() {
            if let Some(ref focused) = style.focused {
                resolved.merge(focused);
            }
        }
        
        if state.is_active() {
            if let Some(ref active) = style.active {
                resolved.merge(active);
            }
        }
        
        // Disabled has highest priority
        if state.is_disabled() {
            if let Some(ref disabled) = style.disabled {
                resolved.merge(disabled);
            }
        }
        
        resolved
    }
}

/// Fully resolved style with no Option fields
#[derive(Debug, Clone)]
pub struct ResolvedStyle {
    pub padding: Edges,
    pub margin: Edges,
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub background: Background,
    pub border: Border,
    pub shadows: Vec<Shadow>,
    pub opacity: f32,
    pub text: ResolvedTextStyle,
    pub cursor: CursorIcon,
    pub transition_duration: Duration,
    pub transition_easing: Curve,
}

impl ResolvedStyle {
    /// Merge another style on top, overwriting non-None fields
    fn merge(&mut self, other: &Style) {
        if let Some(ref padding) = other.padding { self.padding = *padding; }
        if let Some(ref margin) = other.margin { self.margin = *margin; }
        if let Some(bg) = other.background.clone() { self.background = bg; }
        if let Some(ref border) = other.border { self.border = border.clone(); }
        if let Some(opacity) = other.opacity { self.opacity = opacity; }
        // ... etc for all fields
    }
}
```

## Lua API

### Style Definition

```lua
-- Simple style
local button_style = {
    padding = 12,  -- uniform padding
    background = "#3498db",
    border = {
        radius = 8,
        width = 1,
        color = "#2980b9"
    },
    text = {
        color = "#ffffff",
        font_size = 14,
        font_weight = "medium"
    }
}

-- Stateful style with variants
local interactive_button = {
    -- Base style
    padding = { top = 8, right = 16, bottom = 8, left = 16 },
    background = "#3498db",
    border = { radius = 6 },
    text = { color = "#ffffff" },
    transition_duration = 150,  -- ms
    
    -- State variants (only specify overrides)
    hovered = {
        background = "#2980b9",
    },
    active = {
        background = "#1a5276",
        transform = { scale = 0.98 }
    },
    focused = {
        border = { width = 2, color = "#f39c12" }
    },
    disabled = {
        background = "#bdc3c7",
        text = { color = "#7f8c8d" },
        opacity = 0.6
    }
}
```

### Color Formats

```lua
-- Hex colors
local c1 = "#RGB"           -- Short hex
local c2 = "#RRGGBB"        -- Full hex
local c3 = "#RRGGBBAA"      -- Hex with alpha

-- CSS-style
local c4 = "rgb(52, 152, 219)"
local c5 = "rgba(52, 152, 219, 0.8)"

-- Named colors (subset supported)
local c6 = "transparent"
local c7 = "white"
local c8 = "black"

-- Table format
local c9 = { r = 0.2, g = 0.6, b = 0.86, a = 1.0 }
```

### Gradient Syntax

```lua
-- Linear gradient
local gradient1 = {
    type = "linear",
    angle = 90,  -- top to bottom
    stops = {
        { offset = 0, color = "#3498db" },
        { offset = 1, color = "#2c3e50" }
    }
}

-- Radial gradient
local gradient2 = {
    type = "radial",
    center = { 0.5, 0.5 },
    radius = 1.0,
    stops = {
        { offset = 0, color = "#ffffff" },
        { offset = 1, color = "#3498db" }
    }
}

-- Usage
local panel_style = {
    background = gradient1
}
```

### Shadow Syntax

```lua
local card_style = {
    background = "#ffffff",
    shadow = {
        -- Single shadow
        { x = 0, y = 2, blur = 4, color = "rgba(0,0,0,0.1)" },
        -- Multiple shadows supported
        { x = 0, y = 4, blur = 8, spread = 2, color = "rgba(0,0,0,0.05)" }
    }
}
```

### Applying Styles to Widgets

```lua
local niri = require("niri")
local ui = niri.ui

-- Via constructor
local btn = ui.button({
    label = "Click Me",
    style = interactive_button
})

-- Via method
btn:set_style(new_style)

-- Partial update (merges with existing)
btn:update_style({ background = "#e74c3c" })
```

## Style Caching

```rust
/// Per-widget style cache
pub struct StyleCache {
    /// Last resolved style
    resolved: Option<ResolvedStyle>,
    /// State when style was resolved
    cached_state: WidgetState,
    /// Dirty flag for invalidation
    dirty: bool,
}

impl StyleCache {
    pub fn get_or_resolve(
        &mut self,
        style: &StatefulStyle,
        state: WidgetState,
    ) -> &ResolvedStyle {
        if self.dirty || self.cached_state != state || self.resolved.is_none() {
            self.resolved = Some(StyleResolver::resolve(style, state));
            self.cached_state = state;
            self.dirty = false;
        }
        self.resolved.as_ref().unwrap()
    }
    
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }
}
```

## Style Transitions

When widget state changes, styles can animate smoothly:

```rust
/// Animated style transition
pub struct StyleTransition {
    from: ResolvedStyle,
    to: ResolvedStyle,
    animation: Animation,
}

impl StyleTransition {
    pub fn new(from: ResolvedStyle, to: ResolvedStyle, duration: Duration, curve: Curve) -> Self {
        Self {
            from,
            to,
            animation: Animation::new(0.0, 1.0, duration).with_curve(curve),
        }
    }
    
    pub fn advance(&mut self, clock: &Clock) -> bool {
        self.animation.set_current_time(clock.now());
        !self.animation.is_done()
    }
    
    pub fn current(&self) -> ResolvedStyle {
        let t = self.animation.value();
        self.from.interpolate(&self.to, t as f32)
    }
}
```

## Acceptance Criteria

### AC-STYLE-1: Color Parsing
```
GIVEN a color string in hex format "#RRGGBB"
WHEN Color::from_hex() is called
THEN the correct RGBA values are returned

GIVEN an invalid color string
WHEN Color::from_hex() is called
THEN an appropriate error is returned
```

### AC-STYLE-2: Style Resolution Priority
```
GIVEN a StatefulStyle with base, hovered, and disabled variants
WHEN the widget state is HOVERED | DISABLED
THEN disabled styles override hovered styles

GIVEN a StatefulStyle with base and active variants
WHEN the widget state is HOVERED | ACTIVE
THEN active styles override hovered styles
```

### AC-STYLE-3: Style Merging
```
GIVEN a base style with background="#blue" and padding=10
AND a hovered variant with only background="#darkblue"
WHEN the style is resolved for HOVERED state
THEN background is "#darkblue" AND padding remains 10
```

### AC-STYLE-4: Lua Style Parsing
```
GIVEN a Lua table with style properties
WHEN the style is converted to Rust
THEN all properties are correctly parsed including nested objects
```

### AC-STYLE-5: Style Transitions
```
GIVEN a widget with transition_duration = 200ms
WHEN the widget state changes from normal to hovered
THEN the style animates smoothly over 200ms
```

## Test Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_hex() {
        assert_eq!(
            Color::from_hex("#ff0000").unwrap(),
            Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }
        );
        assert_eq!(
            Color::from_hex("#00ff0080").unwrap(),
            Color { r: 0.0, g: 1.0, b: 0.0, a: 0.5 }
        );
    }
    
    #[test]
    fn test_style_resolution_priority() {
        let style = StatefulStyle {
            base: Style { 
                background: Some(Background::Solid(Color::from_hex("#blue").unwrap())),
                ..Default::default()
            },
            hovered: Some(Style {
                background: Some(Background::Solid(Color::from_hex("#lightblue").unwrap())),
                ..Default::default()
            }),
            disabled: Some(Style {
                background: Some(Background::Solid(Color::from_hex("#gray").unwrap())),
                ..Default::default()
            }),
            ..Default::default()
        };
        
        // Disabled should win over hovered
        let state = WidgetState::HOVERED | WidgetState::DISABLED;
        let resolved = style.resolve(state);
        assert!(matches!(resolved.background, Background::Solid(c) if c == Color::from_hex("#gray").unwrap()));
    }
    
    #[test]
    fn test_edge_uniform() {
        let edges = Edges::uniform(10.0);
        assert_eq!(edges.top, 10.0);
        assert_eq!(edges.horizontal(), 20.0);
    }
    
    #[test]
    fn test_color_lerp() {
        let black = Color::BLACK;
        let white = Color::WHITE;
        let gray = black.lerp(white, 0.5);
        assert!((gray.r - 0.5).abs() < 0.001);
    }
}
```

### Lua Integration Tests

```lua
-- test_styling.lua
local niri = require("niri")
local ui = niri.ui

-- Test color parsing
local label = ui.label({
    text = "Test",
    style = { text = { color = "#ff0000" } }
})
assert(label ~= nil, "Label with hex color should be created")

-- Test gradient parsing
local panel = ui.box({
    style = {
        background = {
            type = "linear",
            angle = 45,
            stops = {
                { offset = 0, color = "#000" },
                { offset = 1, color = "#fff" }
            }
        }
    }
})
assert(panel ~= nil, "Panel with gradient should be created")

-- Test state variants
local button = ui.button({
    label = "Test",
    style = {
        background = "#3498db",
        hovered = { background = "#2980b9" },
        active = { background = "#1a5276" }
    }
})
assert(button ~= nil, "Button with state variants should be created")
```

## Implementation Notes

1. **Cairo Integration**: Use `cairo::Context::set_source_rgba()` for colors, create `cairo::LinearGradient`/`cairo::RadialGradient` for gradient fills

2. **Pango Integration**: Convert `TextStyle` to `pango::FontDescription` and `pango::AttrList`

3. **Performance**: Cache `ResolvedStyle` per widget, invalidate only on style or state change

4. **Memory**: Use `Arc<StatefulStyle>` for shared styles between similar widgets

5. **Defaults**: Provide sensible defaults (transparent background, no border, full opacity)

## Related Documents

- [widget.md](widget.md) - Widget trait and base implementation
- [rendering.md](rendering.md) - How styles are rendered via Cairo
- [animation.md](animation.md) - Animation curves for style transitions
- [lua_bindings.md](lua_bindings.md) - Lua API design patterns
