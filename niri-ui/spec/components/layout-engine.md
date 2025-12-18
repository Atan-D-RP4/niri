# Layout Engine Specification

## Overview

The layout engine implements a flexbox-inspired two-pass layout algorithm for widget positioning. It handles constraint propagation, padding/margin, and alignment within the niri-ui widget system.

## Core Types

### Constraints

```rust
/// Layout constraints passed down from parent to child
#[derive(Debug, Clone, Copy)]
pub struct Constraints {
    /// Minimum allowed size
    pub min: Size<f64>,
    /// Maximum allowed size (f64::INFINITY for unbounded)
    pub max: Size<f64>,
}

impl Constraints {
    pub fn tight(size: Size<f64>) -> Self {
        Self { min: size, max: size }
    }
    
    pub fn loose(max: Size<f64>) -> Self {
        Self {
            min: Size::default(),
            max,
        }
    }
    
    pub fn unbounded() -> Self {
        Self {
            min: Size::default(),
            max: Size::new(f64::INFINITY, f64::INFINITY),
        }
    }
    
    /// Constrain a size to fit within these constraints
    pub fn constrain(&self, size: Size<f64>) -> Size<f64> {
        Size::new(
            size.width.clamp(self.min.width, self.max.width),
            size.height.clamp(self.min.height, self.max.height),
        )
    }
    
    /// Shrink constraints by padding/margin
    pub fn deflate(&self, insets: Insets) -> Self {
        Self {
            min: Size::new(
                (self.min.width - insets.horizontal()).max(0.0),
                (self.min.height - insets.vertical()).max(0.0),
            ),
            max: Size::new(
                (self.max.width - insets.horizontal()).max(0.0),
                (self.max.height - insets.vertical()).max(0.0),
            ),
        }
    }
}
```

### Size and Geometry

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect<T> {
    pub origin: Point<T>,
    pub size: Size<T>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Insets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Insets {
    pub fn all(value: f64) -> Self {
        Self { top: value, right: value, bottom: value, left: value }
    }
    
    pub fn symmetric(horizontal: f64, vertical: f64) -> Self {
        Self { top: vertical, right: horizontal, bottom: vertical, left: horizontal }
    }
    
    pub fn horizontal(&self) -> f64 { self.left + self.right }
    pub fn vertical(&self) -> f64 { self.top + self.bottom }
}
```

### LayoutContext

```rust
/// Context provided during layout operations
pub struct LayoutContext<'a> {
    /// Current output scale factor
    pub scale: f64,
    /// Font context for text measurement
    pub font_ctx: &'a FontContext,
    /// Whether layout is being performed for measurement only
    pub measuring: bool,
}
```

## Two-Pass Layout Algorithm

### Pass 1: Measure (Bottom-Up)

Children report their desired sizes given constraints from parents.

```rust
pub trait Widget {
    /// Measure the widget and return its desired size
    fn measure(&mut self, constraints: Constraints, ctx: &LayoutContext) -> Size<f64>;
}
```

### Pass 2: Position (Top-Down)

Parents assign positions to children based on measured sizes.

```rust
pub trait Widget {
    /// Position children within the allocated bounds
    fn layout(&mut self, bounds: Rect<f64>, ctx: &LayoutContext);
}
```

### Layout Flow

```
Parent.measure(constraints)
  │
  ├─► Child1.measure(child_constraints) → size1
  ├─► Child2.measure(child_constraints) → size2
  │
  └─► return computed_size

Parent.layout(bounds)
  │
  ├─► compute child positions
  ├─► Child1.layout(child1_bounds)
  └─► Child2.layout(child2_bounds)
```

## Container Widgets

### Row (Horizontal Layout)

```rust
pub struct Row {
    children: Vec<Box<dyn Widget>>,
    spacing: f64,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    // Cached layout results
    child_sizes: Vec<Size<f64>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum MainAxisAlignment {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum CrossAxisAlignment {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

impl Widget for Row {
    fn measure(&mut self, constraints: Constraints, ctx: &LayoutContext) -> Size<f64> {
        let mut total_width = 0.0;
        let mut max_height = 0.0;
        self.child_sizes.clear();
        
        // Measure each child with unbounded width
        let child_constraints = Constraints {
            min: Size::default(),
            max: Size::new(f64::INFINITY, constraints.max.height),
        };
        
        for child in &mut self.children {
            let size = child.measure(child_constraints, ctx);
            self.child_sizes.push(size);
            total_width += size.width;
            max_height = max_height.max(size.height);
        }
        
        // Add spacing
        if self.children.len() > 1 {
            total_width += self.spacing * (self.children.len() - 1) as f64;
        }
        
        constraints.constrain(Size::new(total_width, max_height))
    }
    
    fn layout(&mut self, bounds: Rect<f64>, ctx: &LayoutContext) {
        let total_child_width: f64 = self.child_sizes.iter().map(|s| s.width).sum();
        let total_spacing = self.spacing * (self.children.len().saturating_sub(1)) as f64;
        let extra_space = (bounds.size.width - total_child_width - total_spacing).max(0.0);
        
        // Calculate starting position based on alignment
        let (mut x, spacing) = match self.main_axis_alignment {
            MainAxisAlignment::Start => (bounds.origin.x, self.spacing),
            MainAxisAlignment::Center => (bounds.origin.x + extra_space / 2.0, self.spacing),
            MainAxisAlignment::End => (bounds.origin.x + extra_space, self.spacing),
            MainAxisAlignment::SpaceBetween if self.children.len() > 1 => {
                (bounds.origin.x, extra_space / (self.children.len() - 1) as f64 + self.spacing)
            }
            MainAxisAlignment::SpaceAround => {
                let gap = extra_space / self.children.len() as f64;
                (bounds.origin.x + gap / 2.0, gap + self.spacing)
            }
            MainAxisAlignment::SpaceEvenly => {
                let gap = extra_space / (self.children.len() + 1) as f64;
                (bounds.origin.x + gap, gap + self.spacing)
            }
            _ => (bounds.origin.x, self.spacing),
        };
        
        for (child, size) in self.children.iter_mut().zip(&self.child_sizes) {
            let y = match self.cross_axis_alignment {
                CrossAxisAlignment::Start => bounds.origin.y,
                CrossAxisAlignment::Center => bounds.origin.y + (bounds.size.height - size.height) / 2.0,
                CrossAxisAlignment::End => bounds.origin.y + bounds.size.height - size.height,
                CrossAxisAlignment::Stretch => bounds.origin.y,
            };
            
            let height = match self.cross_axis_alignment {
                CrossAxisAlignment::Stretch => bounds.size.height,
                _ => size.height,
            };
            
            child.layout(Rect {
                origin: Point { x, y },
                size: Size::new(size.width, height),
            }, ctx);
            
            x += size.width + spacing;
        }
    }
}
```

### Column (Vertical Layout)

```rust
pub struct Column {
    children: Vec<Box<dyn Widget>>,
    spacing: f64,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    child_sizes: Vec<Size<f64>>,
}

impl Widget for Column {
    fn measure(&mut self, constraints: Constraints, ctx: &LayoutContext) -> Size<f64> {
        let mut total_height = 0.0;
        let mut max_width = 0.0;
        self.child_sizes.clear();
        
        let child_constraints = Constraints {
            min: Size::default(),
            max: Size::new(constraints.max.width, f64::INFINITY),
        };
        
        for child in &mut self.children {
            let size = child.measure(child_constraints, ctx);
            self.child_sizes.push(size);
            total_height += size.height;
            max_width = max_width.max(size.width);
        }
        
        if self.children.len() > 1 {
            total_height += self.spacing * (self.children.len() - 1) as f64;
        }
        
        constraints.constrain(Size::new(max_width, total_height))
    }
    
    fn layout(&mut self, bounds: Rect<f64>, ctx: &LayoutContext) {
        // Similar to Row but on vertical axis
        let total_child_height: f64 = self.child_sizes.iter().map(|s| s.height).sum();
        let total_spacing = self.spacing * (self.children.len().saturating_sub(1)) as f64;
        let extra_space = (bounds.size.height - total_child_height - total_spacing).max(0.0);
        
        let (mut y, spacing) = match self.main_axis_alignment {
            MainAxisAlignment::Start => (bounds.origin.y, self.spacing),
            MainAxisAlignment::Center => (bounds.origin.y + extra_space / 2.0, self.spacing),
            MainAxisAlignment::End => (bounds.origin.y + extra_space, self.spacing),
            MainAxisAlignment::SpaceBetween if self.children.len() > 1 => {
                (bounds.origin.y, extra_space / (self.children.len() - 1) as f64 + self.spacing)
            }
            MainAxisAlignment::SpaceAround => {
                let gap = extra_space / self.children.len() as f64;
                (bounds.origin.y + gap / 2.0, gap + self.spacing)
            }
            MainAxisAlignment::SpaceEvenly => {
                let gap = extra_space / (self.children.len() + 1) as f64;
                (bounds.origin.y + gap, gap + self.spacing)
            }
            _ => (bounds.origin.y, self.spacing),
        };
        
        for (child, size) in self.children.iter_mut().zip(&self.child_sizes) {
            let x = match self.cross_axis_alignment {
                CrossAxisAlignment::Start => bounds.origin.x,
                CrossAxisAlignment::Center => bounds.origin.x + (bounds.size.width - size.width) / 2.0,
                CrossAxisAlignment::End => bounds.origin.x + bounds.size.width - size.width,
                CrossAxisAlignment::Stretch => bounds.origin.x,
            };
            
            let width = match self.cross_axis_alignment {
                CrossAxisAlignment::Stretch => bounds.size.width,
                _ => size.width,
            };
            
            child.layout(Rect {
                origin: Point { x, y },
                size: Size::new(width, size.height),
            }, ctx);
            
            y += size.height + spacing;
        }
    }
}
```

### Flexible (Flex Children)

```rust
/// A widget that can expand to fill available space
pub struct Flexible {
    child: Box<dyn Widget>,
    flex: f64,
    fit: FlexFit,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum FlexFit {
    #[default]
    Loose,  // Child can be smaller than allocated space
    Tight,  // Child must fill allocated space
}
```

### Stack (Z-axis Layout)

```rust
pub struct Stack {
    children: Vec<StackChild>,
    alignment: Alignment,
}

pub struct StackChild {
    widget: Box<dyn Widget>,
    positioned: Option<StackPosition>,
}

pub struct StackPosition {
    pub top: Option<f64>,
    pub right: Option<f64>,
    pub bottom: Option<f64>,
    pub left: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Alignment {
    pub x: f64,  // -1.0 (left) to 1.0 (right)
    pub y: f64,  // -1.0 (top) to 1.0 (bottom)
}

impl Alignment {
    pub const TOP_LEFT: Self = Self { x: -1.0, y: -1.0 };
    pub const TOP_CENTER: Self = Self { x: 0.0, y: -1.0 };
    pub const TOP_RIGHT: Self = Self { x: 1.0, y: -1.0 };
    pub const CENTER_LEFT: Self = Self { x: -1.0, y: 0.0 };
    pub const CENTER: Self = Self { x: 0.0, y: 0.0 };
    pub const CENTER_RIGHT: Self = Self { x: 1.0, y: 0.0 };
    pub const BOTTOM_LEFT: Self = Self { x: -1.0, y: 1.0 };
    pub const BOTTOM_CENTER: Self = Self { x: 0.0, y: 1.0 };
    pub const BOTTOM_RIGHT: Self = Self { x: 1.0, y: 1.0 };
}
```

## Padding and Margin

```rust
pub struct Padding {
    child: Box<dyn Widget>,
    insets: Insets,
}

impl Widget for Padding {
    fn measure(&mut self, constraints: Constraints, ctx: &LayoutContext) -> Size<f64> {
        let child_constraints = constraints.deflate(self.insets);
        let child_size = self.child.measure(child_constraints, ctx);
        
        Size::new(
            child_size.width + self.insets.horizontal(),
            child_size.height + self.insets.vertical(),
        )
    }
    
    fn layout(&mut self, bounds: Rect<f64>, ctx: &LayoutContext) {
        let child_bounds = Rect {
            origin: Point {
                x: bounds.origin.x + self.insets.left,
                y: bounds.origin.y + self.insets.top,
            },
            size: Size::new(
                bounds.size.width - self.insets.horizontal(),
                bounds.size.height - self.insets.vertical(),
            ),
        };
        self.child.layout(child_bounds, ctx);
    }
}
```

## Lua API

### Basic Layout

```lua
local ui = niri.ui

-- Row with children
local row = ui.row({
    spacing = 8,
    align = "center",      -- main axis: start, center, end, space-between, space-around, space-evenly
    cross_align = "center", -- cross axis: start, center, end, stretch
    children = {
        ui.label({ text = "Hello" }),
        ui.label({ text = "World" }),
    }
})

-- Column with children
local column = ui.column({
    spacing = 4,
    children = {
        ui.label({ text = "Line 1" }),
        ui.label({ text = "Line 2" }),
    }
})

-- Padding
local padded = ui.padding({
    all = 16,  -- or: top, right, bottom, left, horizontal, vertical
    child = ui.label({ text = "Padded content" })
})

-- Stack for overlays
local stack = ui.stack({
    alignment = "center",
    children = {
        ui.box({ width = 200, height = 200, background = "#333" }),
        ui.label({ text = "Centered" }),
    }
})
```

### Flexible Children

```lua
-- Expanding child in a row
local row = ui.row({
    children = {
        ui.label({ text = "Fixed" }),
        ui.flexible({
            flex = 1,
            child = ui.box({ background = "#444" })
        }),
        ui.label({ text = "Fixed" }),
    }
})

-- Multiple flex children with different weights
local row = ui.row({
    children = {
        ui.flexible({ flex = 1, child = ui.box({ background = "#f00" }) }),
        ui.flexible({ flex = 2, child = ui.box({ background = "#0f0" }) }),
        ui.flexible({ flex = 1, child = ui.box({ background = "#00f" }) }),
    }
})
```

### Positioned Stack Children

```lua
local stack = ui.stack({
    children = {
        -- Background fills entire stack
        ui.box({ background = "#222" }),
        
        -- Positioned in top-right corner
        ui.positioned({
            top = 8,
            right = 8,
            child = ui.label({ text = "Close" })
        }),
        
        -- Positioned at bottom, stretched horizontally
        ui.positioned({
            left = 0,
            right = 0,
            bottom = 0,
            child = ui.box({ height = 32, background = "#333" })
        }),
    }
})
```

## Acceptance Criteria

### AC-LAYOUT-001: Constraint Propagation
```
GIVEN a parent widget with max size 400x300
WHEN a child requests unbounded size
THEN the child receives constraints with max 400x300
```

### AC-LAYOUT-002: Row Spacing
```
GIVEN a Row with 3 children and spacing=10
WHEN measured with unbounded constraints
THEN total width = child1.width + child2.width + child3.width + 20
```

### AC-LAYOUT-003: MainAxisAlignment.SpaceBetween
```
GIVEN a Row with 3 children, total width 100, children width 60
WHEN alignment is SpaceBetween
THEN children positioned at x=0, x=20, x=40 (20px gaps between)
```

### AC-LAYOUT-004: CrossAxisAlignment.Stretch
```
GIVEN a Row with height 100 and cross_align=Stretch
WHEN laying out a child with natural height 50
THEN child receives height 100
```

### AC-LAYOUT-005: Padding Deflation
```
GIVEN Padding with insets 10 all sides
AND parent constraints max 200x200
WHEN child measures
THEN child receives max constraints 180x180
```

### AC-LAYOUT-006: Flexible Distribution
```
GIVEN a Row width 300
AND two Flexible children with flex=1 and flex=2
WHEN layout completes
THEN child1 width=100, child2 width=200
```

### AC-LAYOUT-007: Stack Positioning
```
GIVEN a Stack 200x200
AND a Positioned child with top=10, right=10
WHEN layout completes  
THEN child positioned at (200 - child.width - 10, 10)
```

## Test Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_constraints_constrain() {
        let c = Constraints {
            min: Size::new(50.0, 50.0),
            max: Size::new(200.0, 200.0),
        };
        
        assert_eq!(c.constrain(Size::new(100.0, 100.0)), Size::new(100.0, 100.0));
        assert_eq!(c.constrain(Size::new(10.0, 10.0)), Size::new(50.0, 50.0));
        assert_eq!(c.constrain(Size::new(300.0, 300.0)), Size::new(200.0, 200.0));
    }
    
    #[test]
    fn test_constraints_deflate() {
        let c = Constraints::loose(Size::new(200.0, 200.0));
        let deflated = c.deflate(Insets::all(20.0));
        
        assert_eq!(deflated.max.width, 160.0);
        assert_eq!(deflated.max.height, 160.0);
    }
    
    #[test]
    fn test_row_measure_spacing() {
        let ctx = LayoutContext { scale: 1.0, font_ctx: &FontContext::new(), measuring: true };
        let mut row = Row {
            children: vec![
                Box::new(FixedSize::new(50.0, 30.0)),
                Box::new(FixedSize::new(50.0, 40.0)),
                Box::new(FixedSize::new(50.0, 35.0)),
            ],
            spacing: 10.0,
            ..Default::default()
        };
        
        let size = row.measure(Constraints::unbounded(), &ctx);
        assert_eq!(size.width, 170.0);  // 50 + 50 + 50 + 10 + 10
        assert_eq!(size.height, 40.0);   // max height
    }
    
    #[test]
    fn test_row_layout_space_between() {
        let ctx = LayoutContext { scale: 1.0, font_ctx: &FontContext::new(), measuring: false };
        let mut row = Row {
            children: vec![
                Box::new(FixedSize::new(20.0, 20.0)),
                Box::new(FixedSize::new(20.0, 20.0)),
                Box::new(FixedSize::new(20.0, 20.0)),
            ],
            spacing: 0.0,
            main_axis_alignment: MainAxisAlignment::SpaceBetween,
            ..Default::default()
        };
        
        row.measure(Constraints::unbounded(), &ctx);
        row.layout(Rect {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size::new(100.0, 50.0),
        }, &ctx);
        
        // Children at 0, 40, 80 (20px gaps for 40px extra space / 2 gaps)
        // Verify via child bounds tracking
    }
}
```

### Snapshot Tests

```rust
#[test]
fn test_complex_layout_snapshot() {
    let tree = column([
        row([label("A"), flexible(box_widget()), label("B")]),
        padding(10.0, label("Padded")),
        stack([
            box_widget().size(100.0, 100.0),
            positioned(top(5.0).right(5.0), label("X")),
        ]),
    ]);
    
    let layout_result = compute_layout(&tree, Size::new(400.0, 300.0));
    insta::assert_debug_snapshot!(layout_result);
}
```

## Performance Considerations

1. **Layout Caching**: Cache measure results when constraints haven't changed
2. **Dirty Tracking**: Only re-layout subtrees with dirty flags set
3. **Avoid Allocations**: Reuse child_sizes vectors between layouts
4. **Batch Updates**: Defer layout until all property changes are applied

## Related Specifications

- [Widget System](widget.md) - Widget trait that layout operates on
- [Rendering Pipeline](rendering.md) - How layout bounds feed into rendering
- [Styling System](styling-system.md) - Padding/margin from styles
