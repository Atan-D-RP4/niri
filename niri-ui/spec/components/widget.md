# Widget System Specification

> niri-ui component spec: Widget trait, core widgets, and composition patterns

## 1. Overview

The widget system provides the building blocks for niri-ui interfaces. All widgets implement a common `Widget` trait that handles layout, rendering, and input. Widgets are composable, allowing complex UIs to be built from simple primitives.

### 1.1 Design Principles

- **Constraint C1**: Widgets must never crash the compositor; invalid state → graceful degradation
- **Constraint C2**: All widget types contained within niri-ui crate
- **Constraint C3**: QtQuick-level flexibility for custom widget creation

## 2. Core Types

### 2.1 Widget Trait

```rust
use smithay::utils::{Logical, Point, Rectangle, Size};

/// Unique identifier for widget instances
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(u64);

impl WidgetId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Layout constraints passed from parent to child
#[derive(Debug, Clone, Copy)]
pub struct Constraints {
    pub min_width: f64,
    pub max_width: f64,
    pub min_height: f64,
    pub max_height: f64,
}

impl Constraints {
    pub const UNBOUNDED: Self = Self {
        min_width: 0.0,
        max_width: f64::INFINITY,
        min_height: 0.0,
        max_height: f64::INFINITY,
    };

    pub fn tight(size: Size<f64, Logical>) -> Self {
        Self {
            min_width: size.w,
            max_width: size.w,
            min_height: size.h,
            max_height: size.h,
        }
    }

    pub fn loose(size: Size<f64, Logical>) -> Self {
        Self {
            min_width: 0.0,
            max_width: size.w,
            min_height: 0.0,
            max_height: size.h,
        }
    }

    pub fn constrain(&self, size: Size<f64, Logical>) -> Size<f64, Logical> {
        Size::from((
            size.w.clamp(self.min_width, self.max_width),
            size.h.clamp(self.min_height, self.max_height),
        ))
    }

    pub fn is_bounded(&self) -> bool {
        self.max_width.is_finite() && self.max_height.is_finite()
    }
}

/// Context passed during layout
pub struct LayoutContext<'a> {
    pub scale: f64,
    pub clock: &'a Clock,
}

/// Context passed during rendering
pub struct RenderContext<'a> {
    pub cairo: &'a cairo::Context,
    pub scale: f64,
    pub clip: Rectangle<f64, Logical>,
}

/// Result of input handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputResult {
    /// Input was not handled, propagate to next widget
    Ignored,
    /// Input was handled, stop propagation
    Handled,
    /// Input was handled and requires a redraw
    HandledNeedsRedraw,
}

/// The core widget trait
pub trait Widget: Send + Sync {
    /// Unique identifier for this widget instance
    fn id(&self) -> WidgetId;

    /// Measure the widget's preferred size given constraints
    fn measure(&mut self, constraints: Constraints, ctx: &LayoutContext) -> Size<f64, Logical>;

    /// Position children after measurement (for container widgets)
    fn layout(&mut self, size: Size<f64, Logical>, ctx: &LayoutContext) {
        let _ = (size, ctx); // Default: no-op for leaf widgets
    }

    /// Render the widget to a Cairo context
    fn render(&self, ctx: &mut RenderContext);

    /// Handle pointer motion, return true if within bounds
    fn on_pointer_motion(&mut self, position: Point<f64, Logical>) -> bool {
        let _ = position;
        false
    }

    /// Handle pointer button press/release
    fn on_pointer_button(
        &mut self,
        position: Point<f64, Logical>,
        button: u32,
        pressed: bool,
    ) -> InputResult {
        let _ = (position, button, pressed);
        InputResult::Ignored
    }

    /// Handle scroll events
    fn on_scroll(&mut self, position: Point<f64, Logical>, delta: (f64, f64)) -> InputResult {
        let _ = (position, delta);
        InputResult::Ignored
    }

    /// Check if widget needs redraw
    fn needs_redraw(&self) -> bool {
        false
    }

    /// Clear the needs_redraw flag
    fn clear_redraw_flag(&mut self) {}

    /// Get the current bounds (set during layout)
    fn bounds(&self) -> Rectangle<f64, Logical>;

    /// Set bounds (called by parent during layout)
    fn set_bounds(&mut self, bounds: Rectangle<f64, Logical>);

    /// Check if point is within widget bounds
    fn hit_test(&self, point: Point<f64, Logical>) -> bool {
        self.bounds().contains(point)
    }

    /// Get child widgets (for traversal)
    fn children(&self) -> &[Box<dyn Widget>] {
        &[]
    }

    /// Get mutable child widgets
    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut []
    }
}
```

### 2.2 WidgetBase Helper

Common functionality extracted into a helper struct:

```rust
/// Common widget state
#[derive(Debug)]
pub struct WidgetBase {
    id: WidgetId,
    bounds: Rectangle<f64, Logical>,
    needs_redraw: bool,
    is_hovered: bool,
    is_pressed: bool,
    is_focused: bool,
    is_disabled: bool,
}

impl WidgetBase {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            bounds: Rectangle::default(),
            needs_redraw: true,
            is_hovered: false,
            is_pressed: false,
            is_focused: false,
            is_disabled: false,
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    pub fn bounds(&self) -> Rectangle<f64, Logical> {
        self.bounds
    }

    pub fn set_bounds(&mut self, bounds: Rectangle<f64, Logical>) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.needs_redraw = true;
        }
    }

    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    pub fn mark_dirty(&mut self) {
        self.needs_redraw = true;
    }

    pub fn clear_redraw_flag(&mut self) {
        self.needs_redraw = false;
    }

    /// Returns current style state for StyleResolver
    pub fn style_state(&self) -> StyleState {
        StyleState {
            hovered: self.is_hovered,
            pressed: self.is_pressed,
            focused: self.is_focused,
            disabled: self.is_disabled,
        }
    }

    pub fn set_hovered(&mut self, hovered: bool) -> bool {
        if self.is_hovered != hovered {
            self.is_hovered = hovered;
            self.needs_redraw = true;
            true
        } else {
            false
        }
    }

    pub fn set_pressed(&mut self, pressed: bool) -> bool {
        if self.is_pressed != pressed {
            self.is_pressed = pressed;
            self.needs_redraw = true;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StyleState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub disabled: bool,
}
```

## 3. Core Widgets

### 3.1 Label

Text display widget using Pango for rendering.

```rust
pub struct Label {
    base: WidgetBase,
    text: String,
    font: FontDescription,
    color: Color,
    alignment: Alignment,
    // Cached layout
    cached_layout: Option<CachedPangoLayout>,
}

struct CachedPangoLayout {
    text: String,
    font: FontDescription,
    scale: f64,
    size: Size<f64, Logical>,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            base: WidgetBase::new(),
            text: text.into(),
            font: FontDescription::from_string("sans 14px"),
            color: Color::WHITE,
            alignment: Alignment::Start,
            cached_layout: None,
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.text != text {
            self.text = text;
            self.cached_layout = None;
            self.base.mark_dirty();
        }
    }

    pub fn set_font(&mut self, font: FontDescription) {
        self.font = font;
        self.cached_layout = None;
        self.base.mark_dirty();
    }

    pub fn set_color(&mut self, color: Color) {
        if self.color != color {
            self.color = color;
            self.base.mark_dirty();
        }
    }
}

impl Widget for Label {
    fn id(&self) -> WidgetId {
        self.base.id()
    }

    fn measure(&mut self, constraints: Constraints, ctx: &LayoutContext) -> Size<f64, Logical> {
        // Check cache
        if let Some(cached) = &self.cached_layout {
            if cached.text == self.text && cached.scale == ctx.scale {
                return constraints.constrain(cached.size);
            }
        }

        // Create Pango layout for measurement
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1, 1).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();
        let layout = pangocairo::functions::create_layout(&cr);

        let mut font = self.font.clone();
        font.set_absolute_size(font.size() as f64 * ctx.scale);
        layout.set_font_description(Some(&font));
        layout.set_text(&self.text);

        if constraints.is_bounded() {
            layout.set_width((constraints.max_width * ctx.scale * pango::SCALE as f64) as i32);
        }

        let (width, height) = layout.pixel_size();
        let size = Size::from((width as f64 / ctx.scale, height as f64 / ctx.scale));

        // Cache the result
        self.cached_layout = Some(CachedPangoLayout {
            text: self.text.clone(),
            font: self.font.clone(),
            scale: ctx.scale,
            size,
        });

        constraints.constrain(size)
    }

    fn render(&self, ctx: &mut RenderContext) {
        let bounds = self.base.bounds();
        let layout = pangocairo::functions::create_layout(ctx.cairo);

        let mut font = self.font.clone();
        font.set_absolute_size(font.size() as f64 * ctx.scale);
        layout.set_font_description(Some(&font));
        layout.set_text(&self.text);

        ctx.cairo.move_to(bounds.loc.x, bounds.loc.y);
        ctx.cairo.set_source_rgba(
            self.color.r as f64,
            self.color.g as f64,
            self.color.b as f64,
            self.color.a as f64,
        );
        pangocairo::functions::show_layout(ctx.cairo, &layout);
    }

    fn bounds(&self) -> Rectangle<f64, Logical> {
        self.base.bounds()
    }

    fn set_bounds(&mut self, bounds: Rectangle<f64, Logical>) {
        self.base.set_bounds(bounds);
    }

    fn needs_redraw(&self) -> bool {
        self.base.needs_redraw()
    }

    fn clear_redraw_flag(&mut self) {
        self.base.clear_redraw_flag();
    }
}
```

### 3.2 Box (Container)

A styled container with background, border, and padding.

```rust
pub struct Box {
    base: WidgetBase,
    child: Option<Box<dyn Widget>>,
    style: BoxStyle,
    padding: Edges,
}

#[derive(Debug, Clone)]
pub struct BoxStyle {
    pub background: Option<Background>,
    pub border: Option<Border>,
    pub corner_radius: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Edges {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Edges {
    pub const ZERO: Self = Self { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 };

    pub fn all(value: f64) -> Self {
        Self { top: value, right: value, bottom: value, left: value }
    }

    pub fn symmetric(horizontal: f64, vertical: f64) -> Self {
        Self { top: vertical, right: horizontal, bottom: vertical, left: horizontal }
    }

    pub fn horizontal(&self) -> f64 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f64 {
        self.top + self.bottom
    }
}

impl Box {
    pub fn new() -> Self {
        Self {
            base: WidgetBase::new(),
            child: None,
            style: BoxStyle {
                background: None,
                border: None,
                corner_radius: 0.0,
            },
            padding: Edges::ZERO,
        }
    }

    pub fn with_child(mut self, child: impl Widget + 'static) -> Self {
        self.child = Some(std::boxed::Box::new(child));
        self
    }

    pub fn with_padding(mut self, padding: Edges) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_background(mut self, background: Background) -> Self {
        self.style.background = Some(background);
        self
    }

    pub fn with_border(mut self, border: Border) -> Self {
        self.style.border = Some(border);
        self
    }

    pub fn with_corner_radius(mut self, radius: f64) -> Self {
        self.style.corner_radius = radius;
        self
    }
}

impl Widget for Box {
    fn id(&self) -> WidgetId {
        self.base.id()
    }

    fn measure(&mut self, constraints: Constraints, ctx: &LayoutContext) -> Size<f64, Logical> {
        let padding_h = self.padding.horizontal();
        let padding_v = self.padding.vertical();

        let child_constraints = Constraints {
            min_width: (constraints.min_width - padding_h).max(0.0),
            max_width: (constraints.max_width - padding_h).max(0.0),
            min_height: (constraints.min_height - padding_v).max(0.0),
            max_height: (constraints.max_height - padding_v).max(0.0),
        };

        let child_size = if let Some(child) = &mut self.child {
            child.measure(child_constraints, ctx)
        } else {
            Size::from((0.0, 0.0))
        };

        constraints.constrain(Size::from((
            child_size.w + padding_h,
            child_size.h + padding_v,
        )))
    }

    fn layout(&mut self, size: Size<f64, Logical>, ctx: &LayoutContext) {
        if let Some(child) = &mut self.child {
            let child_bounds = Rectangle::new(
                Point::from((self.padding.left, self.padding.top)),
                Size::from((
                    size.w - self.padding.horizontal(),
                    size.h - self.padding.vertical(),
                )),
            );
            child.set_bounds(child_bounds);
            child.layout(child_bounds.size, ctx);
        }
    }

    fn render(&self, ctx: &mut RenderContext) {
        let bounds = self.base.bounds();

        // Draw background
        if let Some(bg) = &self.style.background {
            draw_rounded_rect(ctx.cairo, bounds, self.style.corner_radius);
            bg.apply(ctx.cairo, bounds);
            ctx.cairo.fill().unwrap();
        }

        // Draw child
        if let Some(child) = &self.child {
            child.render(ctx);
        }

        // Draw border
        if let Some(border) = &self.style.border {
            draw_rounded_rect(ctx.cairo, bounds, self.style.corner_radius);
            ctx.cairo.set_source_rgba(
                border.color.r as f64,
                border.color.g as f64,
                border.color.b as f64,
                border.color.a as f64,
            );
            ctx.cairo.set_line_width(border.width);
            ctx.cairo.stroke().unwrap();
        }
    }

    fn bounds(&self) -> Rectangle<f64, Logical> {
        self.base.bounds()
    }

    fn set_bounds(&mut self, bounds: Rectangle<f64, Logical>) {
        self.base.set_bounds(bounds);
    }

    fn on_pointer_motion(&mut self, position: Point<f64, Logical>) -> bool {
        if let Some(child) = &mut self.child {
            child.on_pointer_motion(position)
        } else {
            false
        }
    }

    fn on_pointer_button(
        &mut self,
        position: Point<f64, Logical>,
        button: u32,
        pressed: bool,
    ) -> InputResult {
        if let Some(child) = &mut self.child {
            child.on_pointer_button(position, button, pressed)
        } else {
            InputResult::Ignored
        }
    }
}

fn draw_rounded_rect(cr: &cairo::Context, rect: Rectangle<f64, Logical>, radius: f64) {
    if radius <= 0.0 {
        cr.rectangle(rect.loc.x, rect.loc.y, rect.size.w, rect.size.h);
        return;
    }

    let x = rect.loc.x;
    let y = rect.loc.y;
    let w = rect.size.w;
    let h = rect.size.h;
    let r = radius.min(w / 2.0).min(h / 2.0);

    cr.new_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
    cr.close_path();
}
```

### 3.3 Button

Interactive button with click handling.

```rust
pub struct Button {
    base: WidgetBase,
    child: Box<dyn Widget>,
    style: ButtonStyle,
    on_click: Option<Box<dyn Fn() + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub normal: BoxStyle,
    pub hovered: BoxStyle,
    pub pressed: BoxStyle,
    pub disabled: BoxStyle,
    pub padding: Edges,
}

impl Button {
    pub fn new(child: impl Widget + 'static) -> Self {
        Self {
            base: WidgetBase::new(),
            child: std::boxed::Box::new(child),
            style: ButtonStyle::default(),
            on_click: None,
        }
    }

    pub fn on_click(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_click = Some(std::boxed::Box::new(handler));
        self
    }

    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    fn current_style(&self) -> &BoxStyle {
        let state = self.base.style_state();
        if state.disabled {
            &self.style.disabled
        } else if state.pressed {
            &self.style.pressed
        } else if state.hovered {
            &self.style.hovered
        } else {
            &self.style.normal
        }
    }
}

impl Widget for Button {
    fn id(&self) -> WidgetId {
        self.base.id()
    }

    fn measure(&mut self, constraints: Constraints, ctx: &LayoutContext) -> Size<f64, Logical> {
        let padding = &self.style.padding;
        let padding_h = padding.horizontal();
        let padding_v = padding.vertical();

        let child_constraints = Constraints {
            min_width: (constraints.min_width - padding_h).max(0.0),
            max_width: (constraints.max_width - padding_h).max(0.0),
            min_height: (constraints.min_height - padding_v).max(0.0),
            max_height: (constraints.max_height - padding_v).max(0.0),
        };

        let child_size = self.child.measure(child_constraints, ctx);

        constraints.constrain(Size::from((
            child_size.w + padding_h,
            child_size.h + padding_v,
        )))
    }

    fn layout(&mut self, size: Size<f64, Logical>, ctx: &LayoutContext) {
        let padding = &self.style.padding;
        let child_bounds = Rectangle::new(
            Point::from((padding.left, padding.top)),
            Size::from((
                size.w - padding.horizontal(),
                size.h - padding.vertical(),
            )),
        );
        self.child.set_bounds(child_bounds);
        self.child.layout(child_bounds.size, ctx);
    }

    fn render(&self, ctx: &mut RenderContext) {
        let bounds = self.base.bounds();
        let style = self.current_style();

        // Draw background
        if let Some(bg) = &style.background {
            draw_rounded_rect(ctx.cairo, bounds, style.corner_radius);
            bg.apply(ctx.cairo, bounds);
            ctx.cairo.fill().unwrap();
        }

        // Draw child
        self.child.render(ctx);

        // Draw border
        if let Some(border) = &style.border {
            draw_rounded_rect(ctx.cairo, bounds, style.corner_radius);
            ctx.cairo.set_source_rgba(
                border.color.r as f64,
                border.color.g as f64,
                border.color.b as f64,
                border.color.a as f64,
            );
            ctx.cairo.set_line_width(border.width);
            ctx.cairo.stroke().unwrap();
        }
    }

    fn on_pointer_motion(&mut self, position: Point<f64, Logical>) -> bool {
        let in_bounds = self.hit_test(position);
        self.base.set_hovered(in_bounds);
        in_bounds
    }

    fn on_pointer_button(
        &mut self,
        position: Point<f64, Logical>,
        button: u32,
        pressed: bool,
    ) -> InputResult {
        if button != 0x110 {
            // BTN_LEFT
            return InputResult::Ignored;
        }

        if !self.hit_test(position) {
            self.base.set_pressed(false);
            return InputResult::Ignored;
        }

        if pressed {
            self.base.set_pressed(true);
            InputResult::HandledNeedsRedraw
        } else {
            if self.base.style_state().pressed {
                self.base.set_pressed(false);
                if let Some(on_click) = &self.on_click {
                    on_click();
                }
                InputResult::HandledNeedsRedraw
            } else {
                InputResult::Ignored
            }
        }
    }

    fn bounds(&self) -> Rectangle<f64, Logical> {
        self.base.bounds()
    }

    fn set_bounds(&mut self, bounds: Rectangle<f64, Logical>) {
        self.base.set_bounds(bounds);
    }

    fn needs_redraw(&self) -> bool {
        self.base.needs_redraw() || self.child.needs_redraw()
    }

    fn clear_redraw_flag(&mut self) {
        self.base.clear_redraw_flag();
        self.child.clear_redraw_flag();
    }
}
```

### 3.4 Image

Image display widget.

```rust
pub struct Image {
    base: WidgetBase,
    source: ImageSource,
    fit: ImageFit,
    cached_surface: Option<cairo::ImageSurface>,
}

pub enum ImageSource {
    Path(PathBuf),
    Data(Vec<u8>),
    Surface(cairo::ImageSurface),
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ImageFit {
    #[default]
    Contain,
    Cover,
    Fill,
    None,
}

impl Image {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            base: WidgetBase::new(),
            source: ImageSource::Path(path.into()),
            fit: ImageFit::default(),
            cached_surface: None,
        }
    }

    pub fn with_fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    fn load_surface(&mut self) -> Option<&cairo::ImageSurface> {
        if self.cached_surface.is_none() {
            self.cached_surface = match &self.source {
                ImageSource::Path(path) => {
                    cairo::ImageSurface::create_from_png(&mut std::fs::File::open(path).ok()?).ok()
                }
                ImageSource::Data(data) => {
                    cairo::ImageSurface::create_from_png(&mut data.as_slice()).ok()
                }
                ImageSource::Surface(surface) => Some(surface.clone()),
            };
        }
        self.cached_surface.as_ref()
    }
}

impl Widget for Image {
    fn id(&self) -> WidgetId {
        self.base.id()
    }

    fn measure(&mut self, constraints: Constraints, _ctx: &LayoutContext) -> Size<f64, Logical> {
        let intrinsic = if let Some(surface) = self.load_surface() {
            Size::from((surface.width() as f64, surface.height() as f64))
        } else {
            Size::from((0.0, 0.0))
        };

        constraints.constrain(intrinsic)
    }

    fn render(&self, ctx: &mut RenderContext) {
        let Some(surface) = &self.cached_surface else {
            return;
        };

        let bounds = self.base.bounds();
        let img_w = surface.width() as f64;
        let img_h = surface.height() as f64;

        let (scale_x, scale_y, offset_x, offset_y) = match self.fit {
            ImageFit::Fill => (bounds.size.w / img_w, bounds.size.h / img_h, 0.0, 0.0),
            ImageFit::Contain => {
                let scale = (bounds.size.w / img_w).min(bounds.size.h / img_h);
                let offset_x = (bounds.size.w - img_w * scale) / 2.0;
                let offset_y = (bounds.size.h - img_h * scale) / 2.0;
                (scale, scale, offset_x, offset_y)
            }
            ImageFit::Cover => {
                let scale = (bounds.size.w / img_w).max(bounds.size.h / img_h);
                let offset_x = (bounds.size.w - img_w * scale) / 2.0;
                let offset_y = (bounds.size.h - img_h * scale) / 2.0;
                (scale, scale, offset_x, offset_y)
            }
            ImageFit::None => (1.0, 1.0, 0.0, 0.0),
        };

        ctx.cairo.save().unwrap();
        ctx.cairo.translate(bounds.loc.x + offset_x, bounds.loc.y + offset_y);
        ctx.cairo.scale(scale_x, scale_y);
        ctx.cairo.set_source_surface(surface, 0.0, 0.0).unwrap();
        ctx.cairo.paint().unwrap();
        ctx.cairo.restore().unwrap();
    }

    fn bounds(&self) -> Rectangle<f64, Logical> {
        self.base.bounds()
    }

    fn set_bounds(&mut self, bounds: Rectangle<f64, Logical>) {
        self.base.set_bounds(bounds);
    }
}
```

### 3.5 Slider

Interactive slider widget for value selection.

```rust
pub struct Slider {
    base: WidgetBase,
    value: f64,
    min: f64,
    max: f64,
    step: Option<f64>,
    orientation: Orientation,
    style: SliderStyle,
    on_change: Option<Box<dyn Fn(f64) + Send + Sync>>,
    dragging: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub struct SliderStyle {
    pub track_color: Color,
    pub track_height: f64,
    pub fill_color: Color,
    pub thumb_color: Color,
    pub thumb_radius: f64,
    pub thumb_hover_color: Color,
}

impl Slider {
    pub fn new(min: f64, max: f64, value: f64) -> Self {
        Self {
            base: WidgetBase::new(),
            value: value.clamp(min, max),
            min,
            max,
            step: None,
            orientation: Orientation::default(),
            style: SliderStyle::default(),
            on_change: None,
            dragging: false,
        }
    }

    pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
        self.on_change = Some(std::boxed::Box::new(handler));
        self
    }

    pub fn with_step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    pub fn set_value(&mut self, value: f64) {
        let new_value = self.snap_value(value.clamp(self.min, self.max));
        if (self.value - new_value).abs() > f64::EPSILON {
            self.value = new_value;
            self.base.mark_dirty();
        }
    }

    fn snap_value(&self, value: f64) -> f64 {
        if let Some(step) = self.step {
            ((value - self.min) / step).round() * step + self.min
        } else {
            value
        }
    }

    fn value_to_position(&self, value: f64) -> f64 {
        (value - self.min) / (self.max - self.min)
    }

    fn position_to_value(&self, position: f64) -> f64 {
        position * (self.max - self.min) + self.min
    }

    fn thumb_position(&self) -> Point<f64, Logical> {
        let bounds = self.base.bounds();
        let ratio = self.value_to_position(self.value);

        match self.orientation {
            Orientation::Horizontal => Point::from((
                bounds.loc.x + ratio * bounds.size.w,
                bounds.loc.y + bounds.size.h / 2.0,
            )),
            Orientation::Vertical => Point::from((
                bounds.loc.x + bounds.size.w / 2.0,
                bounds.loc.y + (1.0 - ratio) * bounds.size.h,
            )),
        }
    }
}

impl Widget for Slider {
    fn id(&self) -> WidgetId {
        self.base.id()
    }

    fn measure(&mut self, constraints: Constraints, _ctx: &LayoutContext) -> Size<f64, Logical> {
        let preferred = match self.orientation {
            Orientation::Horizontal => Size::from((200.0, 24.0)),
            Orientation::Vertical => Size::from((24.0, 200.0)),
        };
        constraints.constrain(preferred)
    }

    fn render(&self, ctx: &mut RenderContext) {
        let bounds = self.base.bounds();
        let style = &self.style;

        // Draw track
        let track_rect = match self.orientation {
            Orientation::Horizontal => Rectangle::new(
                Point::from((bounds.loc.x, bounds.loc.y + (bounds.size.h - style.track_height) / 2.0)),
                Size::from((bounds.size.w, style.track_height)),
            ),
            Orientation::Vertical => Rectangle::new(
                Point::from((bounds.loc.x + (bounds.size.w - style.track_height) / 2.0, bounds.loc.y)),
                Size::from((style.track_height, bounds.size.h)),
            ),
        };

        draw_rounded_rect(ctx.cairo, track_rect, style.track_height / 2.0);
        ctx.cairo.set_source_rgba(
            style.track_color.r as f64,
            style.track_color.g as f64,
            style.track_color.b as f64,
            style.track_color.a as f64,
        );
        ctx.cairo.fill().unwrap();

        // Draw fill
        let ratio = self.value_to_position(self.value);
        let fill_rect = match self.orientation {
            Orientation::Horizontal => Rectangle::new(
                track_rect.loc,
                Size::from((track_rect.size.w * ratio, track_rect.size.h)),
            ),
            Orientation::Vertical => {
                let h = track_rect.size.h * ratio;
                Rectangle::new(
                    Point::from((track_rect.loc.x, track_rect.loc.y + track_rect.size.h - h)),
                    Size::from((track_rect.size.w, h)),
                )
            }
        };

        draw_rounded_rect(ctx.cairo, fill_rect, style.track_height / 2.0);
        ctx.cairo.set_source_rgba(
            style.fill_color.r as f64,
            style.fill_color.g as f64,
            style.fill_color.b as f64,
            style.fill_color.a as f64,
        );
        ctx.cairo.fill().unwrap();

        // Draw thumb
        let thumb_pos = self.thumb_position();
        let thumb_color = if self.base.style_state().hovered || self.dragging {
            &style.thumb_hover_color
        } else {
            &style.thumb_color
        };

        ctx.cairo.arc(thumb_pos.x, thumb_pos.y, style.thumb_radius, 0.0, 2.0 * std::f64::consts::PI);
        ctx.cairo.set_source_rgba(
            thumb_color.r as f64,
            thumb_color.g as f64,
            thumb_color.b as f64,
            thumb_color.a as f64,
        );
        ctx.cairo.fill().unwrap();
    }

    fn on_pointer_motion(&mut self, position: Point<f64, Logical>) -> bool {
        let in_bounds = self.hit_test(position);
        self.base.set_hovered(in_bounds);

        if self.dragging {
            let bounds = self.base.bounds();
            let ratio = match self.orientation {
                Orientation::Horizontal => {
                    ((position.x - bounds.loc.x) / bounds.size.w).clamp(0.0, 1.0)
                }
                Orientation::Vertical => {
                    (1.0 - (position.y - bounds.loc.y) / bounds.size.h).clamp(0.0, 1.0)
                }
            };

            let new_value = self.snap_value(self.position_to_value(ratio));
            if (self.value - new_value).abs() > f64::EPSILON {
                self.value = new_value;
                self.base.mark_dirty();
                if let Some(on_change) = &self.on_change {
                    on_change(self.value);
                }
            }
        }

        in_bounds || self.dragging
    }

    fn on_pointer_button(
        &mut self,
        position: Point<f64, Logical>,
        button: u32,
        pressed: bool,
    ) -> InputResult {
        if button != 0x110 {
            return InputResult::Ignored;
        }

        if pressed && self.hit_test(position) {
            self.dragging = true;
            // Jump to click position
            self.on_pointer_motion(position);
            InputResult::HandledNeedsRedraw
        } else if !pressed && self.dragging {
            self.dragging = false;
            InputResult::HandledNeedsRedraw
        } else {
            InputResult::Ignored
        }
    }

    fn bounds(&self) -> Rectangle<f64, Logical> {
        self.base.bounds()
    }

    fn set_bounds(&mut self, bounds: Rectangle<f64, Logical>) {
        self.base.set_bounds(bounds);
    }

    fn needs_redraw(&self) -> bool {
        self.base.needs_redraw()
    }

    fn clear_redraw_flag(&mut self) {
        self.base.clear_redraw_flag();
    }
}
```

### 3.6 CircularProgress

Circular progress indicator.

```rust
pub struct CircularProgress {
    base: WidgetBase,
    value: f64,  // 0.0 to 1.0
    style: CircularProgressStyle,
    indeterminate: bool,
    animation: Option<Animation>,
}

#[derive(Debug, Clone)]
pub struct CircularProgressStyle {
    pub track_color: Color,
    pub fill_color: Color,
    pub track_width: f64,
    pub size: f64,
}

impl CircularProgress {
    pub fn new(value: f64) -> Self {
        Self {
            base: WidgetBase::new(),
            value: value.clamp(0.0, 1.0),
            style: CircularProgressStyle::default(),
            indeterminate: false,
            animation: None,
        }
    }

    pub fn indeterminate() -> Self {
        Self {
            base: WidgetBase::new(),
            value: 0.0,
            style: CircularProgressStyle::default(),
            indeterminate: true,
            animation: None,
        }
    }

    pub fn set_value(&mut self, value: f64) {
        let new_value = value.clamp(0.0, 1.0);
        if (self.value - new_value).abs() > f64::EPSILON {
            self.value = new_value;
            self.base.mark_dirty();
        }
    }
}

impl Widget for CircularProgress {
    fn id(&self) -> WidgetId {
        self.base.id()
    }

    fn measure(&mut self, constraints: Constraints, _ctx: &LayoutContext) -> Size<f64, Logical> {
        let size = self.style.size;
        constraints.constrain(Size::from((size, size)))
    }

    fn render(&self, ctx: &mut RenderContext) {
        let bounds = self.base.bounds();
        let style = &self.style;

        let center_x = bounds.loc.x + bounds.size.w / 2.0;
        let center_y = bounds.loc.y + bounds.size.h / 2.0;
        let radius = (bounds.size.w.min(bounds.size.h) - style.track_width) / 2.0;

        // Draw track
        ctx.cairo.set_line_width(style.track_width);
        ctx.cairo.arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
        ctx.cairo.set_source_rgba(
            style.track_color.r as f64,
            style.track_color.g as f64,
            style.track_color.b as f64,
            style.track_color.a as f64,
        );
        ctx.cairo.stroke().unwrap();

        // Draw progress arc
        let start_angle = -std::f64::consts::FRAC_PI_2;
        let end_angle = if self.indeterminate {
            // Spinning animation
            let anim_value = self.animation.as_ref().map(|a| a.value()).unwrap_or(0.0);
            start_angle + anim_value * 2.0 * std::f64::consts::PI
        } else {
            start_angle + self.value * 2.0 * std::f64::consts::PI
        };

        ctx.cairo.arc(center_x, center_y, radius, start_angle, end_angle);
        ctx.cairo.set_source_rgba(
            style.fill_color.r as f64,
            style.fill_color.g as f64,
            style.fill_color.b as f64,
            style.fill_color.a as f64,
        );
        ctx.cairo.stroke().unwrap();
    }

    fn bounds(&self) -> Rectangle<f64, Logical> {
        self.base.bounds()
    }

    fn set_bounds(&mut self, bounds: Rectangle<f64, Logical>) {
        self.base.set_bounds(bounds);
    }

    fn needs_redraw(&self) -> bool {
        self.indeterminate || self.base.needs_redraw()
    }
}
```

## 4. Lua API

### 4.1 Widget Factory

```lua
-- Create widgets via niri.ui namespace
local label = niri.ui.label({
    text = "Hello, World!",
    font = "sans 16px",
    color = "#ffffff",
})

local button = niri.ui.button({
    child = label,
    on_click = function()
        print("Clicked!")
    end,
    style = {
        background = "#333333",
        corner_radius = 8,
        padding = 12,
    },
})

local slider = niri.ui.slider({
    min = 0,
    max = 100,
    value = 50,
    on_change = function(value)
        print("Value: " .. value)
    end,
})

local progress = niri.ui.circular_progress({
    value = 0.75,
    size = 48,
    track_width = 4,
})
```

### 4.2 Widget Methods

```lua
-- All widgets have common methods
widget:set_visible(visible)
widget:is_visible()
widget:bounds()
widget:invalidate()  -- Request redraw

-- Label-specific
label:set_text(text)
label:set_color(color)
label:set_font(font)

-- Button-specific
button:set_enabled(enabled)
button:is_enabled()

-- Slider-specific
slider:set_value(value)
slider:value()
slider:set_range(min, max)

-- CircularProgress-specific
progress:set_value(value)
progress:set_indeterminate(indeterminate)
```

## 5. Acceptance Criteria

### 5.1 Widget Trait

```gherkin
GIVEN a custom widget implementing the Widget trait
WHEN the widget is added to a window
THEN it participates in layout, rendering, and input handling

GIVEN a widget with children
WHEN layout is performed
THEN children are measured and positioned within parent bounds

GIVEN a widget that calls mark_dirty()
WHEN the next frame is rendered
THEN the widget and its ancestors are re-rendered
```

### 5.2 Core Widgets

```gherkin
GIVEN a Label widget
WHEN text is set
THEN the label re-measures and redraws with new text

GIVEN a Button widget
WHEN pointer enters bounds
THEN style changes to hovered state

GIVEN a Button widget with on_click handler
WHEN button is clicked (press + release within bounds)
THEN on_click callback is invoked

GIVEN a Slider widget
WHEN user drags the thumb
THEN value updates and on_change callback fires

GIVEN a CircularProgress in indeterminate mode
WHEN rendered each frame
THEN the progress arc animates continuously
```

## 6. Test Strategy

### 6.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraints_constrain() {
        let constraints = Constraints {
            min_width: 10.0,
            max_width: 100.0,
            min_height: 20.0,
            max_height: 200.0,
        };

        assert_eq!(
            constraints.constrain(Size::from((5.0, 15.0))),
            Size::from((10.0, 20.0))
        );
        assert_eq!(
            constraints.constrain(Size::from((150.0, 250.0))),
            Size::from((100.0, 200.0))
        );
    }

    #[test]
    fn test_widget_base_dirty_flag() {
        let mut base = WidgetBase::new();
        assert!(base.needs_redraw()); // Initially dirty

        base.clear_redraw_flag();
        assert!(!base.needs_redraw());

        base.mark_dirty();
        assert!(base.needs_redraw());
    }

    #[test]
    fn test_label_text_change() {
        let mut label = Label::new("Hello");
        label.set_text("World");
        assert!(label.needs_redraw());
    }

    #[test]
    fn test_slider_value_clamping() {
        let mut slider = Slider::new(0.0, 100.0, 50.0);
        slider.set_value(150.0);
        assert!((slider.value - 100.0).abs() < f64::EPSILON);

        slider.set_value(-50.0);
        assert!((slider.value - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_slider_step_snapping() {
        let slider = Slider::new(0.0, 100.0, 0.0).with_step(10.0);
        assert!((slider.snap_value(23.0) - 20.0).abs() < f64::EPSILON);
        assert!((slider.snap_value(27.0) - 30.0).abs() < f64::EPSILON);
    }
}
```

## 7. Dependencies

- `pangocairo`: Text rendering
- `cairo`: 2D graphics
- `smithay::utils`: Geometry types (Point, Size, Rectangle)

## 8. References

- Existing niri UI: `src/ui/hotkey_overlay.rs`
- Smithay render elements: `src/render_helpers/texture.rs`
- Animation system: `src/animation/mod.rs`
