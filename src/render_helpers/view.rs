use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{GlesError, GlesFrame, GlesRenderer};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::TextureFilter;
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Physical, Point, Rectangle, Scale, Size, Transform};

use crate::backend::tty::{TtyFrame, TtyRenderer, TtyRendererError};
use crate::utils::view::OutputViewport;

/// Linear below threshold, Nearest at or above.
///
/// `Linear` is the renderer's resting default, so this is the filter for every frame;
/// only crossing the threshold changes what the sampler does.
pub fn view_filter(scale_factor: f64, threshold: f64) -> TextureFilter {
    debug_assert!(
        !scale_factor.is_nan() && !threshold.is_nan(),
        "view_filter called with NaN"
    );
    if scale_factor < threshold {
        TextureFilter::Linear
    } else {
        TextureFilter::Nearest
    }
}

/// Renders an element through an output view transform.
///
/// This Element's damage and opaque regions are relative to the inner element, that is, to the
/// position the element is drawn at.
#[derive(Debug)]
pub struct ViewElement<E> {
    element: E,
    scale: Scale<f64>,
    /// The view to render the element through.
    ///
    /// The view maps content space geometry to screen space by scaling the element by
    /// the viewport factor about the viewport focal point.
    viewport: OutputViewport,
    /// Overrides the position of the element. The size still comes from the view.
    ///
    /// This is used for the cursor, which is positioned by the pointer position
    /// rather than by the view.
    pointer_loc: Option<Point<i32, Physical>>,
    /// Enables plane-window mode against an output of this size.
    ///
    /// When the element mapped through the view covers the output rectangle entirely, its
    /// destination is pinned to the output rectangle and src() carries the fractional buffer
    /// window of the visible content instead. Direct scanout then sees a stable integer
    /// plane destination with a sub-pixel source window, so zoomed fullscreen content
    /// animates without destination jitter.
    output_size: Option<Size<i32, Physical>>,
}

impl<E: Element> ViewElement<E> {
    pub fn from_element(
        element: E,
        viewport: OutputViewport,
        scale: Scale<f64>,
        pointer_loc: Option<Point<i32, Physical>>,
        output_size: Option<Size<i32, Physical>>,
    ) -> Self {
        Self {
            element,
            viewport,
            scale,
            pointer_loc,
            output_size,
        }
    }

    /// Maps a rectangle from content space to screen space.
    fn apply_to_rect(&self, rect: Rectangle<i32, Physical>) -> Rectangle<f64, Physical> {
        let rect = rect.to_f64();
        self.viewport
            .content_to_screen_rect(rect.to_logical(self.scale))
            .to_physical(self.scale)
    }

    /// Maps a region relative to the wrapped element into a region relative to this
    /// element: made absolute in content space, mapped through the view, then made
    /// relative to the element's origin again.
    fn transform_relative_rect(
        &self,
        inner_geometry: Rectangle<i32, Physical>,
        rect: Rectangle<i32, Physical>,
    ) -> Rectangle<f64, Physical> {
        let mut absolute = rect;
        absolute.loc += inner_geometry.loc;
        let mut transformed = self.apply_to_rect(absolute);
        transformed.loc -= self.apply_to_rect(inner_geometry).loc;
        transformed
    }

    /// Plane-window mode: the output rectangle if the element mapped through the view
    /// covers it entirely, otherwise None.
    fn plane_window(
        &self,
        inner_geometry: Rectangle<i32, Physical>,
    ) -> Option<Rectangle<i32, Physical>> {
        let output_size = self.output_size.as_ref()?;
        if self.pointer_loc.is_some() || inner_geometry.size.w <= 0 || inner_geometry.size.h <= 0 {
            return None;
        }
        let mapped = self.apply_to_rect(inner_geometry);
        let covers = mapped.loc.x <= 0.
            && mapped.loc.y <= 0.
            && mapped.loc.x + mapped.size.w >= f64::from(output_size.w)
            && mapped.loc.y + mapped.size.h >= f64::from(output_size.h);
        covers.then(|| Rectangle::new(Point::from((0, 0)), *output_size))
    }
}

/// Maps a content-space window into buffer space through the src ↔ geometry correspondence
/// of an element with the given transform.
///
/// `geometry` is the content-space extent corresponding to `src`; `window` is the wanted
/// window in content space and must lie within `geometry`. Coordinates are normalized
/// within `geometry` and mapped to buffer axes through the inverse transform, so rotated
/// and flipped buffers come out right.
fn buffer_window(
    transform: Transform,
    src: Rectangle<f64, Buffer>,
    geometry: Rectangle<f64, Physical>,
    window: Rectangle<f64, Physical>,
) -> Rectangle<f64, Buffer> {
    let rel_loc: Point<f64, Physical> = Point::from((
        (window.loc.x - geometry.loc.x) / geometry.size.w,
        (window.loc.y - geometry.loc.y) / geometry.size.h,
    ));
    let rel_size: Size<f64, Physical> = Size::from((
        window.size.w / geometry.size.w,
        window.size.h / geometry.size.h,
    ));

    // The element transform maps buffer axes to content axes; apply its inverse to
    // normalized coordinates via Transform's own point mapping.
    let unit: Size<f64, Physical> = Size::from((1., 1.));
    let inverse = transform.invert();
    let p1 = inverse.transform_point_in(rel_loc, &unit);
    let p2 = inverse.transform_point_in(
        Point::from((rel_loc.x + rel_size.w, rel_loc.y + rel_size.h)),
        &unit,
    );
    let (x_min, x_max) = (p1.x.min(p2.x), p1.x.max(p2.x));
    let (y_min, y_max) = (p1.y.min(p2.y), p1.y.max(p2.y));

    Rectangle::new(
        Point::new(
            src.loc.x + x_min * src.size.w,
            src.loc.y + y_min * src.size.h,
        ),
        Size::new((x_max - x_min) * src.size.w, (y_max - y_min) * src.size.h),
    )
}

impl<E: Element> Element for ViewElement<E> {
    fn id(&self) -> &Id {
        self.element.id()
    }

    fn current_commit(&self) -> CommitCounter {
        self.element.current_commit()
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        let inner_src = self.element.src();
        let inner_geometry = self.element.geometry(self.scale);
        let Some(output_rect) = self.plane_window(inner_geometry) else {
            return inner_src;
        };

        // The visible content window: the output rectangle mapped from screen space back
        // into content space. Coverage guarantees it lies within the inner geometry.
        // src() has no scale parameter, so the geometry is queried with the construction
        // scale, matching how the wrapping render path builds this element.
        let content_window = self
            .viewport
            .screen_to_content_rect(output_rect.to_f64().to_logical(self.scale))
            .to_physical(self.scale);

        buffer_window(
            self.element.transform(),
            inner_src,
            inner_geometry.to_f64(),
            content_window,
        )
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        let inner_geometry = self.element.geometry(scale);
        if let Some(output_rect) = self.plane_window(inner_geometry) {
            // Destination pinned to the output rectangle: a stable plane destination, with
            // the visible content carried by the fractional src() window.
            return output_rect;
        }
        let mut mapped = self.apply_to_rect(inner_geometry).to_i32_round();
        if let Some(location) = self.pointer_loc {
            mapped.loc = location;
        }
        mapped
    }

    fn transform(&self) -> Transform {
        self.element.transform()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        let inner_geometry = self.element.geometry(scale);
        let inner_damage = self.element.damage_since(scale, commit);
        let mapped = inner_damage.iter().map(|rect| {
            self.transform_relative_rect(inner_geometry, *rect)
                .to_i32_round()
        });

        if let Some(output_rect) = self.plane_window(inner_geometry) {
            // The destination is pinned to the output rectangle; clip the mapped damage to
            // it. View-driven damage is carried by src() changes, which the output damage
            // tracker reports as damage to the element's whole geometry.
            mapped
                .filter_map(|rect| rect.intersection(output_rect))
                .collect()
        } else {
            mapped.collect()
        }
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        // The opaque regions are rebased by the element's transformed origin, but the inner rects
        // are rooted at the raw element geometry.
        let inner_geometry = self.element.geometry(scale);

        let inner_opaque = self.element.opaque_regions(scale);
        let mapped = inner_opaque.iter().map(|rect| {
            self.transform_relative_rect(inner_geometry, *rect)
                .to_i32_round()
        });

        if let Some(output_rect) = self.plane_window(inner_geometry) {
            // Same pinning as damage_since: clip to the output rectangle.
            mapped
                .filter_map(|rect| rect.intersection(output_rect))
                .collect()
        } else {
            mapped.collect()
        }
    }

    fn alpha(&self) -> f32 {
        self.element.alpha()
    }

    fn kind(&self) -> Kind {
        self.element.kind()
    }

    fn is_framebuffer_effect(&self) -> bool {
        self.element.is_framebuffer_effect()
    }
}

impl<E: RenderElement<GlesRenderer>> RenderElement<GlesRenderer> for ViewElement<E> {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        self.element
            .draw(frame, src, dst, damage, opaque_regions, cache)
    }

    fn underlying_storage(&self, renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        self.element.underlying_storage(renderer)
    }

    fn capture_framebuffer(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), GlesError> {
        self.element.capture_framebuffer(frame, src, dst, cache)
    }
}

impl<'render, E: RenderElement<TtyRenderer<'render>>> RenderElement<TtyRenderer<'render>>
    for ViewElement<E>
{
    fn draw(
        &self,
        frame: &mut TtyFrame<'render, '_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), TtyRendererError<'render>> {
        self.element
            .draw(frame, src, dst, damage, opaque_regions, cache)
    }

    fn underlying_storage(
        &self,
        renderer: &mut TtyRenderer<'render>,
    ) -> Option<UnderlyingStorage<'_>> {
        self.element.underlying_storage(renderer)
    }

    fn capture_framebuffer(
        &self,
        frame: &mut TtyFrame<'render, '_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), TtyRendererError<'render>> {
        self.element.capture_framebuffer(frame, src, dst, cache)
    }
}
