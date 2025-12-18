# Rendering Pipeline Specification

## Overview

The niri-ui rendering pipeline converts widget trees into GPU textures that can be composited by niri. It follows the established pattern used by existing niri UI elements (hotkey overlay, config error notification).

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Rendering Pipeline                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   Widget Tree                                                       │
│       │                                                             │
│       ▼                                                             │
│   ┌───────────────────┐                                             │
│   │   Layout Pass     │  Measure → Position                         │
│   └─────────┬─────────┘                                             │
│             │                                                       │
│             ▼                                                       │
│   ┌───────────────────┐                                             │
│   │  Cairo Context    │  ImageSurface (ARgb32)                      │
│   │  + Pango Layout   │  Text rendering via pangocairo              │
│   └─────────┬─────────┘                                             │
│             │                                                       │
│             ▼                                                       │
│   ┌───────────────────┐                                             │
│   │  TextureBuffer    │  from_memory() with Fourcc::Argb8888        │
│   │  <GlesTexture>    │  Stores scale, transform, opaque_regions    │
│   └─────────┬─────────┘                                             │
│             │                                                       │
│             ▼                                                       │
│   ┌───────────────────┐                                             │
│   │TextureRenderElement│  Adds location, alpha, src rect, size      │
│   └─────────┬─────────┘                                             │
│             │                                                       │
│             ▼                                                       │
│   ┌───────────────────┐                                             │
│   │PrimaryGpuTexture  │  Wrapper for compositor integration         │
│   │  RenderElement    │                                             │
│   └───────────────────┘                                             │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Core Types

### RenderContext

Encapsulates all rendering state for a single frame.

```rust
use pangocairo::cairo::{self, Context, ImageSurface};
use pangocairo::pango::{FontDescription, Layout};
use smithay::utils::{Logical, Physical, Point, Rectangle, Scale, Size};

/// Context for rendering widgets to a Cairo surface.
pub struct RenderContext {
    /// The Cairo context for drawing operations.
    cr: Context,
    /// The underlying image surface.
    surface: ImageSurface,
    /// Current output scale factor.
    scale: f64,
    /// Pango font description for text rendering.
    font: FontDescription,
    /// Current clip region (logical coordinates).
    clip: Option<Rectangle<f64, Logical>>,
}

impl RenderContext {
    /// Creates a new render context with the given dimensions and scale.
    pub fn new(width: i32, height: i32, scale: f64) -> anyhow::Result<Self> {
        let surface = ImageSurface::create(cairo::Format::ARgb32, width, height)?;
        let cr = Context::new(&surface)?;
        
        let mut font = FontDescription::from_string("sans 14px");
        font.set_absolute_size(to_physical_precise_round(scale, font.size()));
        
        Ok(Self {
            cr,
            surface,
            scale,
            font,
            clip: None,
        })
    }
    
    /// Returns the Cairo context for direct drawing.
    pub fn cairo(&self) -> &Context {
        &self.cr
    }
    
    /// Returns the current scale factor.
    pub fn scale(&self) -> f64 {
        self.scale
    }
    
    /// Creates a Pango layout for text rendering.
    pub fn create_layout(&self) -> Layout {
        let layout = pangocairo::functions::create_layout(&self.cr);
        layout.context().set_round_glyph_positions(false);
        layout.set_font_description(Some(&self.font));
        layout
    }
    
    /// Draws text at the current position.
    pub fn draw_text(&self, text: &str) {
        let layout = self.create_layout();
        layout.set_text(text);
        pangocairo::functions::show_layout(&self.cr, &layout);
    }
    
    /// Draws text with Pango markup at the current position.
    pub fn draw_markup(&self, markup: &str) {
        let layout = self.create_layout();
        layout.set_markup(markup);
        pangocairo::functions::show_layout(&self.cr, &layout);
    }
    
    /// Measures text dimensions without drawing.
    pub fn measure_text(&self, text: &str) -> Size<i32, Physical> {
        let layout = self.create_layout();
        layout.set_text(text);
        let (w, h) = layout.pixel_size();
        Size::from((w, h))
    }
    
    /// Sets a clip rectangle (in logical coordinates).
    pub fn push_clip(&mut self, rect: Rectangle<f64, Logical>) {
        self.cr.save().ok();
        let physical = rect.to_physical_precise_round(Scale::from(self.scale));
        self.cr.rectangle(
            physical.loc.x.into(),
            physical.loc.y.into(),
            physical.size.w.into(),
            physical.size.h.into(),
        );
        self.cr.clip();
        self.clip = Some(rect);
    }
    
    /// Removes the current clip rectangle.
    pub fn pop_clip(&mut self) {
        self.cr.restore().ok();
        self.clip = None;
    }
    
    /// Finishes rendering and returns the surface data.
    pub fn finish(self) -> anyhow::Result<Vec<u8>> {
        drop(self.cr);
        Ok(self.surface.take_data()?.to_vec())
    }
}
```

### ImageData contract

Every rendered surface produced by niri-ui is represented by an ImageData object. This defines the pixel buffer contract between niri-ui and the compositor. The expected format is ARGB8888 (premultiplied alpha) and pixel data is given row-major with a stride measured in bytes.

```rust
/// Image data produced by niri-ui rendering
pub struct ImageData {
    /// Width in physical pixels
    pub width: u32,
    /// Height in physical pixels
    pub height: u32,
    /// Row stride in bytes
    pub stride: u32,
    /// Output scale used to render this image (e.g., 1.0, 1.5, 2.0)
    pub scale: f64,
    /// Pixel format (must be "ARGB8888")
    pub format: &'static str,
    /// Whether the alpha channel is premultiplied
    pub premultiplied: bool,
    /// Pixel buffer (length = stride * height)
    pub data: Vec<u8>,
}
```

Responsibility split:

- niri-ui is responsible for producing ImageData via Cairo/Pango rendering (RenderContext::finish() or helpers).
- The compositor (niri) is responsible for converting ImageData into a GPU texture (TextureBuffer) with Fourcc::Argb8888 and uploading it to the renderer. This keeps GPU/renderer concerns within the compositor while keeping niri-ui focused on layout, styling, and CPU rendering.

Guidance:

- Use TextureBuffer::from_memory or compositor helper functions to create textures from ImageData.
- Ensure proper stride and premultiplied alpha semantics when creating textures.

### TextureCache

Per-output texture caching with scale-aware invalidation.

```rust
use std::cell::RefCell;
use std::collections::HashMap;

use smithay::backend::renderer::gles::GlesTexture;
use smithay::output::{Output, WeakOutput};
use smithay::utils::Scale;

use crate::render_helpers::texture::TextureBuffer;

/// Cached texture for a single output.
pub struct CachedTexture {
    /// The rendered texture buffer.
    pub buffer: Option<TextureBuffer<GlesTexture>>,
    /// Version number for invalidation.
    pub version: u64,
}

/// Manages per-output texture caching.
pub struct TextureCache {
    textures: RefCell<HashMap<WeakOutput, CachedTexture>>,
    current_version: u64,
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            textures: RefCell::new(HashMap::new()),
            current_version: 0,
        }
    }
    
    /// Invalidates all cached textures.
    pub fn invalidate(&mut self) {
        self.current_version += 1;
    }
    
    /// Gets or creates a texture for the given output.
    /// 
    /// Returns `None` if the cache is valid, `Some(scale)` if rendering is needed.
    pub fn needs_render(&self, output: &Output) -> Option<f64> {
        let scale = output.current_scale().fractional_scale();
        let weak = output.downgrade();
        
        let mut textures = self.textures.borrow_mut();
        
        // Remove dead outputs
        textures.retain(|output, _| output.is_alive());
        
        // Check if we need to re-render
        if let Some(cached) = textures.get(&weak) {
            if cached.version == self.current_version {
                if let Some(buffer) = &cached.buffer {
                    if buffer.texture_scale() == Scale::from(scale) {
                        return None; // Cache is valid
                    }
                }
            }
        }
        
        Some(scale)
    }
    
    /// Stores a rendered texture for the given output.
    pub fn store(&self, output: &Output, buffer: Option<TextureBuffer<GlesTexture>>) {
        let weak = output.downgrade();
        self.textures.borrow_mut().insert(weak, CachedTexture {
            buffer,
            version: self.current_version,
        });
    }
    
    /// Retrieves the cached texture for the given output.
    pub fn get(&self, output: &Output) -> Option<TextureBuffer<GlesTexture>> {
        let weak = output.downgrade();
        self.textures
            .borrow()
            .get(&weak)
            .and_then(|c| c.buffer.clone())
    }
}

impl Default for TextureCache {
    fn default() -> Self {
        Self::new()
    }
}

## Cache Policy

Per-output texture caches must implement a budgeted LRU eviction policy to bound memory usage. Recommended defaults:

- Default memory budget: 64 MiB per window
- Max textures per window: 128
- Eviction: LRU across cached textures; evict least-recently-used textures until under budget.
- Invalidation: When window size, scale, or content changes, increment the cache version and mark entries as stale.
- Weak-output keys: Use WeakOutput keys so entries are removed when outputs are destroyed.

Implementation notes:

- Provide APIs to inspect cache usage (memory bytes, entry count) for diagnostics and testing.
- Prefer keeping GPU texture lifetime management inside the compositor; caches should hold ImageData (CPU) and optionally weak references to GPU textures managed by the compositor.

```

### Rendering Functions

Helper functions to convert rendered content to compositor elements.

```rust
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::utils::{Logical, Point, Transform};

use crate::render_helpers::primary_gpu_texture::PrimaryGpuTextureRenderElement;
use crate::render_helpers::texture::{TextureBuffer, TextureRenderElement};

/// Creates a TextureBuffer from raw ARGB data.
pub fn create_texture_buffer(
    renderer: &mut GlesRenderer,
    data: &[u8],
    width: i32,
    height: i32,
    scale: f64,
) -> anyhow::Result<TextureBuffer<GlesTexture>> {
    TextureBuffer::from_memory(
        renderer,
        data,
        Fourcc::Argb8888,
        (width, height),
        false,
        scale,
        Transform::Normal,
        Vec::new(),
    )
    .map_err(|e| anyhow::anyhow!("Failed to create texture: {e:?}"))
}

/// Creates a render element from a texture buffer.
pub fn create_render_element(
    buffer: TextureBuffer<GlesTexture>,
    location: Point<f64, Logical>,
    alpha: f32,
) -> PrimaryGpuTextureRenderElement {
    let elem = TextureRenderElement::from_texture_buffer(
        buffer,
        location,
        alpha,
        None,  // src rect
        None,  // size override
        Kind::Unspecified,
    );
    PrimaryGpuTextureRenderElement(elem)
}

/// Converts logical units to physical units with rounding.
pub fn to_physical_precise_round<N>(scale: f64, value: N) -> i32
where
    N: Into<f64>,
{
    (value.into() * scale).round() as i32
}
```

## Widget Rendering Contract

Every widget must implement the `render` method:

```rust
pub trait Widget {
    /// Renders the widget to the given context.
    /// 
    /// # Arguments
    /// * `ctx` - The render context with Cairo/Pango access
    /// * `bounds` - The allocated rectangle for this widget (logical coordinates)
    /// 
    /// # Contract
    /// - Widget MUST NOT draw outside `bounds`
    /// - Widget MUST respect current clip region
    /// - Widget MUST handle its own background/border if styled
    fn render(&self, ctx: &mut RenderContext, bounds: Rectangle<f64, Logical>);
}
```

## Drawing Primitives

### Colors

```rust
/// RGBA color with components in 0.0-1.0 range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }
    
    pub const fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }
    
    /// Creates a color from a hex string (e.g., "#ff0000" or "#ff0000ff").
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        let (r, g, b, a) = match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                (r, g, b, 255)
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                (r, g, b, a)
            }
            _ => return None,
        };
        Some(Self {
            r: r as f64 / 255.0,
            g: g as f64 / 255.0,
            b: b as f64 / 255.0,
            a: a as f64 / 255.0,
        })
    }
    
    /// Applies this color to a Cairo context.
    pub fn apply(&self, cr: &Context) {
        cr.set_source_rgba(self.r, self.g, self.b, self.a);
    }
}
```

### Shapes

```rust
impl RenderContext {
    /// Fills a rectangle with the current source.
    pub fn fill_rect(&self, rect: Rectangle<f64, Logical>) {
        let physical = rect.to_physical_precise_round(Scale::from(self.scale));
        self.cr.rectangle(
            physical.loc.x.into(),
            physical.loc.y.into(),
            physical.size.w.into(),
            physical.size.h.into(),
        );
        self.cr.fill().ok();
    }
    
    /// Strokes a rectangle border with the current source.
    pub fn stroke_rect(&self, rect: Rectangle<f64, Logical>, line_width: f64) {
        let physical = rect.to_physical_precise_round(Scale::from(self.scale));
        // Keep the border width even to avoid blurry edges
        let physical_width = (line_width * self.scale / 2.0).round() * 2.0;
        self.cr.set_line_width(physical_width);
        self.cr.rectangle(
            physical.loc.x.into(),
            physical.loc.y.into(),
            physical.size.w.into(),
            physical.size.h.into(),
        );
        self.cr.stroke().ok();
    }
    
    /// Draws a rounded rectangle path.
    pub fn rounded_rect(&self, rect: Rectangle<f64, Logical>, radius: f64) {
        let physical = rect.to_physical_precise_round(Scale::from(self.scale));
        let r = (radius * self.scale).round();
        let x = physical.loc.x as f64;
        let y = physical.loc.y as f64;
        let w = physical.size.w as f64;
        let h = physical.size.h as f64;
        
        use std::f64::consts::PI;
        
        self.cr.new_path();
        self.cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
        self.cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
        self.cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
        self.cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
        self.cr.close_path();
    }
}
```

## Acceptance Criteria

### AC1: Basic Rendering
```
GIVEN a widget tree with a Label containing "Hello"
WHEN render() is called with scale=1.0
THEN a TextureBuffer is produced with correct pixel dimensions
AND the text is rendered using Pango
AND the texture format is Argb8888
```

### AC2: Scale Handling
```
GIVEN a cached texture rendered at scale=1.0
WHEN the output scale changes to 2.0
THEN TextureCache.needs_render() returns Some(2.0)
AND after re-render, the new texture has scale=2.0
```

### AC3: Cache Invalidation
```
GIVEN a valid cached texture
WHEN TextureCache.invalidate() is called
THEN the next needs_render() call returns Some(scale)
AND the texture is re-rendered
```

### AC4: Clip Regions
```
GIVEN a widget with children that extend beyond its bounds
WHEN the widget sets a clip before rendering children
THEN child content outside the clip is not visible
AND pop_clip() restores the previous state
```

### AC5: Color Parsing
```
GIVEN various color formats
WHEN Color::from_hex() is called with "#ff0000"
THEN Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 } is returned
AND "#ff000080" returns Color { r: 1.0, g: 0.0, b: 0.0, a: 0.502 }
```

## Test Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_color_from_hex() {
        let red = Color::from_hex("#ff0000").unwrap();
        assert_eq!(red.r, 1.0);
        assert_eq!(red.g, 0.0);
        assert_eq!(red.b, 0.0);
        assert_eq!(red.a, 1.0);
        
        let semi_transparent = Color::from_hex("#ff000080").unwrap();
        assert!((semi_transparent.a - 0.502).abs() < 0.01);
    }
    
    #[test]
    fn test_to_physical_precise_round() {
        assert_eq!(to_physical_precise_round(1.0, 10), 10);
        assert_eq!(to_physical_precise_round(2.0, 10), 20);
        assert_eq!(to_physical_precise_round(1.5, 10), 15);
        assert_eq!(to_physical_precise_round(1.25, 10), 13); // rounds 12.5 to 13
    }
    
    #[test]
    fn test_texture_cache_invalidation() {
        let mut cache = TextureCache::new();
        // Initial state requires render
        // After store, cache is valid
        // After invalidate, cache requires render again
        cache.invalidate();
        assert_eq!(cache.current_version, 1);
    }
}
```

### Integration Tests

- Test rendering a complete widget tree to texture
- Test per-output caching with mock outputs at different scales
- Test memory usage with large widget trees

## Performance Considerations

1. **Minimize Cairo surface allocations**: Reuse surfaces when dimensions don't change
2. **Batch text rendering**: Use a single Pango layout for multiple text operations
3. **Lazy invalidation**: Only re-render when content actually changes
4. **Per-output caching**: Each output maintains its own texture at its native scale

## References

- `src/ui/hotkey_overlay.rs:306-456` - Reference rendering implementation
- `src/render_helpers/texture.rs` - TextureBuffer and TextureRenderElement types
- `src/render_helpers/primary_gpu_texture.rs` - PrimaryGpuTextureRenderElement wrapper
